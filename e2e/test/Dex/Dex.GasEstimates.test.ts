import { JsonRpcProvider, Provider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Contract, Wallet, utils } from "ethers";
import { ethers } from "hardhat";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  DEX_PRECOMPILE_ABI,
  DEX_PRECOMPILE_ADDRESS,
  GAS_TOKEN_ID,
  GasCosts,
  NodeProcess,
  assetIdToERC20ContractAddress,
  getNextAssetId,
  saveGasCosts,
  startNode,
  typedefs,
} from "../../common";

import { MockERC20, IUniswapV2Router01, UniswapV2Factory, UniswapV2Router02, WETH9 } from "../../typechain-types";

describe("Dex Gas Estimation", function () {
  let TOKEN_ID_1: number;
  let TOKEN_ID_2: number;

  let api: ApiPromise;
  let node: NodeProcess;

  let alith: KeyringPair;
  let jsonProvider: Provider;
  let alithSigner: Wallet;
  let bobSigner: Wallet;
  let dexPrecompile: IUniswapV2Router01;

  let owner: Wallet;
  let user: Wallet;
  let alpha: MockERC20;
  let beta: MockERC20;
  let weth: WETH9;
  let uniswapV2Factory: UniswapV2Factory;
  let uniswapV2Router02: UniswapV2Router02;

  const allCosts: { [key: string]: GasCosts } = {};

  // Setup api instance
  before(async () => {
    node = await startNode();

    // prepare works for precompile and extrinsic calls
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    alith = new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    // setup JSON RPC
    jsonProvider = new JsonRpcProvider(`http://localhost:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(jsonProvider);
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(jsonProvider);
    owner = Wallet.createRandom().connect(jsonProvider);
    user = Wallet.createRandom().connect(jsonProvider);

    TOKEN_ID_1 = await getNextAssetId(api);
    TOKEN_ID_2 = await getNextAssetId(api, +(await api.query.assetsExt.nextAssetId()).toString() + 1);

    // create alpha and beta assets via Substrate
    const txs = [
      api.tx.assetsExt.createAsset("subAlpha", "SUBALPHA", 18, 1, alith.address), // create asset
      api.tx.assetsExt.createAsset("subBeta", "SUBBETA", 18, 1, alith.address), // create asset

      api.tx.assets.mint(TOKEN_ID_1, alith.address, utils.parseEther("10000000").toString()),
      api.tx.assets.mint(TOKEN_ID_1, owner.address, utils.parseEther("10000000").toString()),
      api.tx.assets.mint(TOKEN_ID_1, user.address, utils.parseEther("10000000").toString()),

      api.tx.assets.mint(TOKEN_ID_2, alith.address, utils.parseEther("10000000").toString()),
      api.tx.assets.mint(TOKEN_ID_2, owner.address, utils.parseEther("10000000").toString()),
      api.tx.assets.mint(TOKEN_ID_2, user.address, utils.parseEther("10000000").toString()),

      api.tx.assets.mint(GAS_TOKEN_ID, alith.address, utils.parseEther("10000000").toString()),
      api.tx.assets.mint(GAS_TOKEN_ID, owner.address, utils.parseEther("10000000").toString()),
      api.tx.assets.mint(GAS_TOKEN_ID, user.address, utils.parseEther("10000000").toString()),
    ];
    await new Promise<void>((resolve, reject) => {
      api.tx.utility
        .batch(txs)
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        })
        .catch((err) => reject(err));
    });

    // setup & deploy all contracts on Root EVM
    dexPrecompile = new Contract(DEX_PRECOMPILE_ADDRESS, DEX_PRECOMPILE_ABI, alithSigner) as IUniswapV2Router01;

    const ERC20Factory = await ethers.getContractFactory("MockERC20", alithSigner);
    alpha = ERC20Factory.connect(alithSigner).attach(assetIdToERC20ContractAddress(TOKEN_ID_1));
    beta = ERC20Factory.connect(alithSigner).attach(assetIdToERC20ContractAddress(TOKEN_ID_2));

    const WETH9ERC20Factory = await ethers.getContractFactory("WETH9", owner);
    weth = await WETH9ERC20Factory.connect(alithSigner).deploy();
    await weth.deployed();

    const UniswapV2FactoryFactory = await ethers.getContractFactory("UniswapV2Factory");
    uniswapV2Factory = await UniswapV2FactoryFactory.connect(alithSigner).deploy(owner.address);
    await uniswapV2Factory.deployed();

    const UniswapV2Router02Factory = await ethers.getContractFactory("UniswapV2Router02");
    uniswapV2Router02 = await UniswapV2Router02Factory.connect(alithSigner).deploy(uniswapV2Factory.address, weth.address);
    await uniswapV2Router02.deployed();

    // alpha = new Contract(assetIdToERC20ContractAddress(TOKEN_ID_1), MockERC20Contract.abi, owner) as MockERC20;
    // beta = new Contract(assetIdToERC20ContractAddress(TOKEN_ID_2), MockERC20Contract.abi, owner) as MockERC20;

    // // deploy the AlphaERC20
    // alpha = await ERC20Factory.deploy();
    // await alpha.deployed();

    // // deploy the BetaERC20
    // beta = await ERC20Factory.deploy();
    // await beta.deployed();

    let gas = await weth.connect(alithSigner).estimateGas.deposit({ value: utils.parseEther("100") });
    let tx = await weth.connect(alithSigner).deposit({ value: utils.parseEther("100"), gasLimit: gas });
    await tx.wait();

    // alpha approve router address
    gas = await alpha.connect(owner).estimateGas.approve(uniswapV2Router02.address, utils.parseEther("10000000"));
    tx = await alpha.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"), { gasLimit: gas });
    await tx.wait();

    // beta approve router address
    gas = await beta.connect(owner).estimateGas.approve(uniswapV2Router02.address, utils.parseEther("10000000"));
    tx = await beta.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"), { gasLimit: gas });
    await tx.wait();

    // add alpha and beta liquidity for removeLiquidity calls and swaps
    gas = await uniswapV2Router02
      .connect(owner)
      .estimateGas
      .addLiquidity(
        alpha.address,
        beta.address,
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        owner.address,
        ethers.constants.MaxUint256,
      );
    tx = await uniswapV2Router02
      .connect(owner)
      .addLiquidity(
        alpha.address,
        beta.address,
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        owner.address,
        ethers.constants.MaxUint256,
        { gasLimit: gas },
      );
    await tx.wait();

    const pairAddress = await uniswapV2Factory.getPair(alpha.address, beta.address);
    const lpToken: MockERC20 = await ethers.getContractAt("MockERC20", pairAddress);
    gas = await lpToken.connect(owner).estimateGas.approve(uniswapV2Router02.address, utils.parseEther("10000000"));
    tx = await lpToken.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"), { gasLimit: gas });
    await tx.wait();

    // add alpha and weth liquidity for removeLiquidity calls and swaps
    // gas = await uniswapV2Router02
    //   .connect(owner)
    //   .estimateGas
    //   .addLiquidityETH(
    //     alpha.address,
    //     utils.parseEther("1000").toString(),
    //     utils.parseEther("1000").toString(),
    //     utils.parseEther("250").toString(),
    //     owner.address,
    //     ethers.constants.MaxUint256,
    //     {
    //       value: utils.parseEther("250"),
    //     },
    //   );
    // tx = await uniswapV2Router02
    //   .connect(owner)
    //   .addLiquidityETH(
    //     alpha.address,
    //     utils.parseEther("1000").toString(),
    //     utils.parseEther("1000").toString(),
    //     utils.parseEther("250").toString(),
    //     owner.address,
    //     ethers.constants.MaxUint256,
    //     {
    //       value: utils.parseEther("250"),
    //       gasLimit: gas,
    //     },
    //   );
    // await tx.wait();

    // const pairAddressETH = await uniswapV2Factory.getPair(alpha.address, weth.address);
    // const lpTokenETH: MockERC20 = await ethers.getContractAt("MockERC20", pairAddressETH);
    // gas = await lpTokenETH.connect(owner).estimateGas.approve(uniswapV2Router02.address, utils.parseEther("10000000"));
    // tx = await lpTokenETH.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"), { gasLimit: gas });
    // await tx.wait();
  });

  after(async () => {
    saveGasCosts(allCosts, "Dex/GasCosts.md", "Dex Precompiles");
    await node.stop();
  });

  /*//////////////////////////////////////////////////////////////
                    Dex functions (transactions)
  //////////////////////////////////////////////////////////////*/
  
  it("addLiquidity gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas
      .addLiquidity(
        alpha.address,
        beta.address,
        utils.parseEther("1000"),
        utils.parseEther("250"),
        utils.parseEther("1000"),
        utils.parseEther("250"),
        owner.address,
        ethers.constants.MaxUint256,
      );

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(alithSigner)
      .estimateGas
      .addLiquidity(
        alpha.address,
        beta.address,
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        alithSigner.address,
        20000,
      );

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .addLiquidity(
          TOKEN_ID_1,
          TOKEN_ID_2,
          utils.parseEther("1000").toString(),
          utils.parseEther("250").toString(),
          utils.parseEther("1000").toString(),
          utils.parseEther("250").toString(),
          alithSigner.address,
          20000,
        )
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await jsonProvider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["addLiquidity"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  // TODO
  it.skip("addLiquidityETH gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas.addLiquidityETH(
        alpha.address,
        utils.parseEther("1000").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        owner.address,
        ethers.constants.MaxUint256,
        {
          value: utils.parseEther("250"),
        },
      );

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(alithSigner)
      .estimateGas.addLiquidityETH(
        alpha.address,
        utils.parseEther("1000").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        alithSigner.address,
        20000,
        {
          value: utils.parseEther("250"),
        },
      );

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .addLiquidity(
          TOKEN_ID_1,
          GAS_TOKEN_ID,
          utils.parseEther("1000").toString(),
          utils.parseEther("250").toString(),
          utils.parseEther("1000").toString(),
          utils.parseEther("250").toString(),
          alithSigner.address,
          20000,
        )
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter).sub(utils.parseEther("250").mul(10 ** 12));
    const fees = await jsonProvider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["addLiquidityETH"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  // dependent on 'addLiquidity gas estimates' test
  it("removeLiquidity gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas
      .removeLiquidity(
        alpha.address,
        beta.address,
        utils.parseEther("100").toString(),
        0,
        0,
        owner.address,
        ethers.constants.MaxUint256,
      );

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(alithSigner)
      .estimateGas
      .removeLiquidity(
        alpha.address,
        beta.address,
        utils.parseEther("100").toString(),
        0,
        0,
        alithSigner.address,
        20000,
      );

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .removeLiquidity(
          TOKEN_ID_1,
          TOKEN_ID_2,
          utils.parseEther("100").toString(),
          0,
          0,
          alithSigner.address,
          20000,
        )
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });

    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await jsonProvider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["removeLiquidity"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  // TODO
  it.skip("removeLiquidityETH gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas.removeLiquidityETH(
        alpha.address,
        utils.parseEther("100").toString(),
        0,
        0,
        owner.address,
        ethers.constants.MaxUint256,
      );

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(alithSigner)
      .estimateGas.removeLiquidityETH(
        alpha.address,
        utils.parseEther("100").toString(),
        0,
        0,
        alithSigner.address,
        20000,
      );

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .removeLiquidity(
          TOKEN_ID_1,
          GAS_TOKEN_ID,
          utils.parseEther("100").toString(),
          0,
          0,
          bobSigner.address,
          20000,
        )
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });

    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await jsonProvider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["removeLiquidityETH"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  // dependent on 'addLiquidity gas estimates' test
  it("swapExactTokensForTokens gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas
      .swapExactTokensForTokens(
        utils.parseEther("100"),
        0,
        [alpha.address, beta.address],
        user.address,
        ethers.constants.MaxUint256,
      );

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(alithSigner)
      .estimateGas
      .swapExactTokensForTokens(
        utils.parseEther("100"),
        0,
        [alpha.address, beta.address],
        bobSigner.address,
        20000,
      );

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .swapWithExactSupply(
          utils.parseEther("100").toString(),
          0,
          [TOKEN_ID_1, TOKEN_ID_2],
          bobSigner.address,
          20000,
        )
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await jsonProvider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["swapExactTokensForTokens"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  // TODO
  it.skip("swapExactTokensForETH gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas
      .swapExactTokensForETH(
        utils.parseEther("100"),
        0,
        [alpha.address, weth.address],
        user.address,
        ethers.constants.MaxUint256,
      );

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(alithSigner)
      .estimateGas
      .swapExactTokensForETH(
        utils.parseEther("100"),
        0,
        [alpha.address, assetIdToERC20ContractAddress(GAS_TOKEN_ID)],
        bobSigner.address,
        20000,
      );

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .swapWithExactSupply(
          utils.parseEther("100").toString(),
          0,
          [TOKEN_ID_1, GAS_TOKEN_ID],
          bobSigner.address,
          20000,
        )
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await jsonProvider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["swapExactTokensForETH"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  // TODO
  it.skip("swapExactETHForTokens gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas.swapExactETHForTokens(0, [weth.address, alpha.address], user.address, ethers.constants.MaxUint256, {
        value: utils.parseEther("100"),
      });

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(alithSigner)
      .estimateGas.swapExactETHForTokens(
        0,
        [assetIdToERC20ContractAddress(GAS_TOKEN_ID), alpha.address],
        bobSigner.address,
        20000,
        {
          value: utils.parseEther("100"),
        },
      );

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .swapWithExactSupply(
          utils.parseEther("5").toString(),
          0,
          [GAS_TOKEN_ID, TOKEN_ID_1],
          bobSigner.address,
          20000,
        )
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter).sub(utils.parseEther("5").mul(10 ** 12));
    const fees = await jsonProvider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["swapExactETHForTokens"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  // dependent on 'addLiquidity gas estimates' test
  it("swapTokensForExactTokens gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas
      .swapTokensForExactTokens(
        utils.parseEther("100"),
        utils.parseEther("10000"),
        [alpha.address, beta.address],
        user.address,
        ethers.constants.MaxUint256,
      );

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(alithSigner)
      .estimateGas
      .swapTokensForExactTokens(
        utils.parseEther("100"),
        utils.parseEther("10000"),
        [alpha.address, beta.address],
        bobSigner.address,
        20000,
      );

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .swapWithExactTarget(
          utils.parseEther("100").toString(),
          utils.parseEther("10000").toString(),
          [TOKEN_ID_1, TOKEN_ID_2],
          bobSigner.address,
          20000,
        )
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await jsonProvider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["swapTokensForExactTokens"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  // TODO
  it.skip("swapTokensForExactETH gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas.swapTokensForExactETH(
        utils.parseEther("100"),
        utils.parseEther("10000"),
        [alpha.address, weth.address],
        user.address,
        ethers.constants.MaxUint256,
      );

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(alithSigner)
      .estimateGas.swapTokensForExactETH(
        utils.parseEther("100"),
        utils.parseEther("10000"),
        [alpha.address, assetIdToERC20ContractAddress(GAS_TOKEN_ID)],
        bobSigner.address,
        20000,
      );

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .swapWithExactTarget(
          utils.parseEther("100").toString(),
          utils.parseEther("10000").toString(),
          [TOKEN_ID_1, GAS_TOKEN_ID],
          bobSigner.address,
          20000,
        )
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await jsonProvider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["swapTokensForExactETH"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  // TODO
  it.skip("swapETHForExactTokens gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas.swapETHForExactTokens(
        utils.parseEther("25"),
        [weth.address, alpha.address],
        user.address,
        ethers.constants.MaxUint256,
        {
          value: utils.parseEther("10000"),
        },
      );

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(alithSigner)
      .estimateGas.swapETHForExactTokens(
        utils.parseEther("25"),
        [assetIdToERC20ContractAddress(GAS_TOKEN_ID), alpha.address],
        bobSigner.address,
        20000,
        {
          value: utils.parseEther("10000"),
        },
      );

    const [ethInSub] = await dexPrecompile
      .connect(bobSigner)
      .getAmountsIn(utils.parseEther("25"), [
        assetIdToERC20ContractAddress(GAS_TOKEN_ID),
        alpha.address,
      ]);

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .swapWithExactTarget(
          utils.parseEther("25").toString(),
          utils.parseEther("10000").toString(),
          [GAS_TOKEN_ID, TOKEN_ID_1],
          bobSigner.address,
          20000,
        )
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter).sub(ethInSub.mul(10 ** 12));
    const fees = await jsonProvider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["swapETHForExactTokens"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  /*//////////////////////////////////////////////////////////////
                    Dex pure and view functions
  //////////////////////////////////////////////////////////////*/

  it("quote gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(user)
      .estimateGas
      .quote(utils.parseEther("5"), utils.parseEther("200"), utils.parseEther("120"));

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(bobSigner)
      .estimateGas
      .quote(utils.parseEther("5"), utils.parseEther("200"), utils.parseEther("120"));

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["quote"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
    };
  });

  it("getAmountOut gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(user)
      .estimateGas
      .getAmountOut(utils.parseEther("5"), utils.parseEther("200"), utils.parseEther("120"));

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(bobSigner)
      .estimateGas
      .getAmountOut(utils.parseEther("5"), utils.parseEther("200"), utils.parseEther("120"));

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["getAmountOut"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });

  // dependent on 'addLiquidity gas estimates' test
  it("getAmountsOut gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(user)
      .estimateGas
      .getAmountsOut(utils.parseEther("5"), [alpha.address, beta.address ]);

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(bobSigner)
      .estimateGas
      .getAmountsOut(utils.parseEther("5"), [ alpha.address, beta.address ]);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["getAmountsOut"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });

  // dependent on 'addLiquidity gas estimates' test
  it("getAmountsIn gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(user)
      .estimateGas
      .getAmountsIn(utils.parseEther("5"), [alpha.address, beta.address ]);

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(bobSigner)
      .estimateGas
      .getAmountsIn(utils.parseEther("5"), [ alpha.address, beta.address ]);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["getAmountsIn"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });
});
