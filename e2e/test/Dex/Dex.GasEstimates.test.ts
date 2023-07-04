import { JsonRpcProvider, Provider } from "@ethersproject/providers";
import { SignerWithAddress } from "@nomiclabs/hardhat-ethers/signers";
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
  saveGasCosts,
  startNode,
  typedefs,
} from "../../common";
import { CustomERC20, UniswapV2Factory, UniswapV2Router02, WETH9 } from "../../typechain-types";

const TOKEN_ID_SUBALPHA = 1124;
const TOKEN_ID_SUBBETA = 2148;

describe("Dex Gas Estimation", function () {
  let node: NodeProcess;
  let api: ApiPromise;

  let alithSigner: Wallet;
  let bobSigner: Wallet;
  let alith: KeyringPair;
  let dexPrecompile: Contract;
  let jsonProvider: Provider;

  let owner: SignerWithAddress;
  let user: SignerWithAddress;
  let alpha: CustomERC20;
  let beta: CustomERC20;
  let weth: WETH9;
  let uniswapV2Factory: UniswapV2Factory;
  let uniswapV2Router02: UniswapV2Router02;

  const allCosts: { [key: string]: GasCosts } = {};

  // Setup api instance
  before(async () => {
    node = await startNode();

    await node.wait(); // wait for the node to be ready

    // setup JSON RPC
    jsonProvider = new JsonRpcProvider(`http://localhost:${node.httpPort}`);
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(jsonProvider); // 'development' seed
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(jsonProvider); // 'development' seed

    // deploy all contracts
    const WETH9ERC20Factory = await ethers.getContractFactory("WETH9", owner);
    weth = await WETH9ERC20Factory.deploy();
    await weth.deployed();

    const ERC20Factory = await ethers.getContractFactory("CustomERC20");

    // deploy the AlphaERC20
    alpha = await ERC20Factory.deploy("Alpha", "ALPHA");
    await alpha.deployed();

    // deploy the BetaERC20
    beta = await ERC20Factory.deploy("Beta", "BETA");
    await beta.deployed();

    // Set up owner for re-use
    const [_owner, _user] = await ethers.getSigners();
    owner = _owner;
    user = _user;

    // mint some tokens to the owner
    await alpha.mint(owner.address, utils.parseEther("10000000"));
    await beta.mint(owner.address, utils.parseEther("10000000"));

    // deposit weth to owner
    await weth.connect(owner).deposit({ value: utils.parseEther("100") });

    // deploy the UniswapV2Factory
    const UniswapV2FactoryFactory = await ethers.getContractFactory("UniswapV2Factory");
    uniswapV2Factory = await UniswapV2FactoryFactory.deploy(owner.address);
    await uniswapV2Factory.deployed();

    // deploy the UniswapV2Router02
    const UniswapV2Router02Factory = await ethers.getContractFactory("UniswapV2Router02");
    uniswapV2Router02 = await UniswapV2Router02Factory.deploy(uniswapV2Factory.address, weth.address);
    await uniswapV2Router02.deployed();

    // approve router address
    await alpha.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));
    await beta.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));

    // add alpha and beta liquidity for calls like removeLiquidity and swaps
    const callResAddLiquidity = await uniswapV2Router02
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
      );

    await callResAddLiquidity;

    const pairAddress = await uniswapV2Factory.getPair(alpha.address, beta.address);
    const lpToken: CustomERC20 = await ethers.getContractAt("CustomERC20", pairAddress);
    await lpToken.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));

    // add alpha and weth liquidity for calls like removeLiquidity and swaps
    const callResAddLiquidityETH = await uniswapV2Router02
      .connect(owner)
      .addLiquidityETH(
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

    await callResAddLiquidityETH;

    const pairAddressETH = await uniswapV2Factory.getPair(alpha.address, weth.address);
    const lpTokenETH: CustomERC20 = await ethers.getContractAt("CustomERC20", pairAddressETH);
    await lpTokenETH.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));

    // prepare works for precompile and extrinsic calls
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    api = await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
    });

    dexPrecompile = new Contract(DEX_PRECOMPILE_ADDRESS, DEX_PRECOMPILE_ABI, alithSigner);

    // add alith to keyring
    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    // create alpha and beta assets via Substrate
    const txs = [
      api.tx.assetsExt.createAsset("subAlpha", "SUBALPHA", 18, 1, alithSigner.address), // create asset
      api.tx.assetsExt.createAsset("subBeta", "SUBBETA", 18, 1, alithSigner.address), // create asset
      api.tx.assets.mint(TOKEN_ID_SUBALPHA, alithSigner.address, utils.parseEther("10000000").toString()),
      api.tx.assets.mint(TOKEN_ID_SUBBETA, alithSigner.address, utils.parseEther("10000000").toString()),
      api.tx.assets.mint(GAS_TOKEN_ID, alithSigner.address, utils.parseEther("10000000").toString()),
    ];

    await new Promise<void>((resolve, reject) => {
      api.tx.utility
        .batch(txs)
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) {
            console.log(`setup block hash: ${status.asInBlock}`);
            resolve();
          }
        })
        .catch((err) => reject(err));
    });
  });

  after(async () => {
    saveGasCosts(allCosts, "Dex/GasCosts.md", "Dex Precompiles");

    await node.stop();
  });

  // Dex functions (transactions)
  it("addLiquidity gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas.addLiquidity(
        alpha.address,
        beta.address,
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        owner.address,
        ethers.constants.MaxUint256,
      );

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(alithSigner)
      .estimateGas.addLiquidity(
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA),
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
          TOKEN_ID_SUBALPHA,
          TOKEN_ID_SUBBETA,
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

  it("addLiquidityETH gas estimates", async () => {
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
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
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
          TOKEN_ID_SUBALPHA,
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

  it("removeLiquidity gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas.removeLiquidity(
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
      .estimateGas.removeLiquidity(
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA),
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
          TOKEN_ID_SUBALPHA,
          TOKEN_ID_SUBBETA,
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

  it("removeLiquidityETH gas estimates", async () => {
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
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
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
          TOKEN_ID_SUBALPHA,
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

  it("swapExactTokensForTokens gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas.swapExactTokensForTokens(
        utils.parseEther("100"),
        0,
        [alpha.address, beta.address],
        user.address,
        ethers.constants.MaxUint256,
      );

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(alithSigner)
      .estimateGas.swapExactTokensForTokens(
        utils.parseEther("100"),
        0,
        [assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA), assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA)],
        bobSigner.address,
        20000,
      );

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .swapWithExactSupply(
          utils.parseEther("100").toString(),
          0,
          [TOKEN_ID_SUBALPHA, TOKEN_ID_SUBBETA],
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

  it("swapExactTokensForETH gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas.swapExactTokensForETH(
        utils.parseEther("100"),
        0,
        [alpha.address, weth.address],
        user.address,
        ethers.constants.MaxUint256,
      );

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(alithSigner)
      .estimateGas.swapExactTokensForETH(
        utils.parseEther("100"),
        0,
        [assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA), assetIdToERC20ContractAddress(GAS_TOKEN_ID)],
        bobSigner.address,
        20000,
      );

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .swapWithExactSupply(
          utils.parseEther("100").toString(),
          0,
          [TOKEN_ID_SUBALPHA, GAS_TOKEN_ID],
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

  it("swapExactETHForTokens gas estimates", async () => {
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
        [assetIdToERC20ContractAddress(GAS_TOKEN_ID), assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA)],
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
          [GAS_TOKEN_ID, TOKEN_ID_SUBALPHA],
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

  it("swapTokensForExactTokens gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(owner)
      .estimateGas.swapTokensForExactTokens(
        utils.parseEther("100"),
        utils.parseEther("10000"),
        [alpha.address, beta.address],
        user.address,
        ethers.constants.MaxUint256,
      );

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(alithSigner)
      .estimateGas.swapTokensForExactTokens(
        utils.parseEther("100"),
        utils.parseEther("10000"),
        [assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA), assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA)],
        bobSigner.address,
        20000,
      );

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .swapWithExactTarget(
          utils.parseEther("100").toString(),
          utils.parseEther("10000").toString(),
          [TOKEN_ID_SUBALPHA, TOKEN_ID_SUBBETA],
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

  it("swapTokensForExactETH gas estimates", async () => {
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
        [assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA), assetIdToERC20ContractAddress(GAS_TOKEN_ID)],
        bobSigner.address,
        20000,
      );

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .swapWithExactTarget(
          utils.parseEther("100").toString(),
          utils.parseEther("10000").toString(),
          [TOKEN_ID_SUBALPHA, GAS_TOKEN_ID],
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

  it("swapETHForExactTokens gas estimates", async () => {
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
        [assetIdToERC20ContractAddress(GAS_TOKEN_ID), assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA)],
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
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
      ]);

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.dex
        .swapWithExactTarget(
          utils.parseEther("25").toString(),
          utils.parseEther("10000").toString(),
          [GAS_TOKEN_ID, TOKEN_ID_SUBALPHA],
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

  // Dex pure and view functions
  it("quote gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(user)
      .estimateGas.quote(utils.parseEther("5"), utils.parseEther("200"), utils.parseEther("120"));

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(bobSigner)
      .estimateGas.quote(utils.parseEther("5"), utils.parseEther("200"), utils.parseEther("120"));

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
      .estimateGas.getAmountOut(utils.parseEther("5"), utils.parseEther("200"), utils.parseEther("120"));

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(bobSigner)
      .estimateGas.getAmountOut(utils.parseEther("5"), utils.parseEther("200"), utils.parseEther("120"));

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["getAmountOut"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });

  it("getAmountsOut gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(user)
      .estimateGas.getAmountsOut(utils.parseEther("5"), [alpha.address, beta.address]);

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(bobSigner)
      .estimateGas.getAmountsOut(utils.parseEther("5"), [
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA),
      ]);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["getAmountsOut"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });

  it("getAmountsIn gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await uniswapV2Router02
      .connect(user)
      .estimateGas.getAmountsIn(utils.parseEther("5"), [alpha.address, beta.address]);

    // Estimate precompile call
    const precompileGasEstimate = await dexPrecompile
      .connect(bobSigner)
      .estimateGas.getAmountsIn(utils.parseEther("5"), [
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA),
      ]);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["getAmountsIn"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });
});
