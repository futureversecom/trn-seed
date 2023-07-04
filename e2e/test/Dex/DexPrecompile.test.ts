import { JsonRpcProvider, Provider } from "@ethersproject/providers";
import { SignerWithAddress } from "@nomiclabs/hardhat-ethers/signers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { BigNumber, Contract, ContractFactory, ContractReceipt, ContractTransaction, Wallet, utils } from "ethers";
import { ethers } from "hardhat";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  DEX_PRECOMPILE_ABI,
  DEX_PRECOMPILE_ADDRESS,
  GAS_TOKEN_ID,
  NodeProcess,
  assetIdToERC20ContractAddress,
  startNode,
  typedefs,
} from "../../common";
import {
  CustomERC20,
  UniswapV2Factory,
  UniswapV2Pair,
  UniswapV2Pair__factory,
  UniswapV2Router02,
  WETH9,
} from "../../typechain-types";
import { token } from "../../typechain-types/factories/@openzeppelin/contracts";

const TOKEN_ID_SUBALPHA = 1124;
const TOKEN_ID_SUBBETA = 2148;

describe("DEX Precompile", function () {
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
  let etherProvider: Provider;

  // Setup api instance
  before(async () => {
    node = await startNode();

    await node.wait(); // wait for the node to be ready

    // setup JSON RPC
    jsonProvider = new JsonRpcProvider(`http://localhost:${node.httpPort}`);
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(jsonProvider); // 'development' seed
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(jsonProvider); // 'development' seed

    // deploy all contracts
    etherProvider = ethers.provider;
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

  after(async () => await node.stop());

  it("names", async () => {
    expect(await alpha.name()).to.equal("Alpha");
    expect(await beta.name()).to.equal("Beta");

    let subAlphaMetadata: any = (await api.query.assets.metadata(TOKEN_ID_SUBALPHA)).toJSON();
    console.log(subAlphaMetadata);
    console.log(alithSigner.address);
    console.log(assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA));
    console.log(assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA));
    //expect(subAlphaMetadata.name.toString()).to.equal("subAlpha");
    let subBetaMetadata: any = (await api.query.assets.metadata(TOKEN_ID_SUBBETA)).toJSON();
    //expect(subBetaMetadata.name).to.equal("subBeta");
  });

  it("add liquidity", async () => {
    // add liquidity on uniswap
    await alpha.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));
    await beta.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));

    const res = await uniswapV2Router02
      .connect(owner)
      .callStatic.addLiquidity(
        alpha.address,
        beta.address,
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        owner.address,
        ethers.constants.MaxUint256,
      );
    console.log("Add liquidity result from uniswap call: ", res);

    // run the function on blockchain
    const uniAddLiquidity = await uniswapV2Router02
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

    const receipt = await uniAddLiquidity.wait();

    //console.log("add liquidity uni events: ", receipt?.events);

    // check events
    //expect((receipt?.events as any)[0].event).to.equal("Mint");
    //expect((receipt?.events as any)[0].args.sender).to.equal(alithSigner.address);
    //expect((receipt?.events as any)[0].args.amount0).to.equal(res.amountA);
    //expect((receipt?.events as any)[0].args.amount1).to.equal(res.amountB);

    const pairAddress = await uniswapV2Factory.getPair(alpha.address, beta.address);
    const lpToken: CustomERC20 = await ethers.getContractAt("CustomERC20", pairAddress);
    const lpBalance = await lpToken.balanceOf(owner.address);
    expect(lpBalance).to.eq(BigNumber.from("499999999999999999000"));

    // add liquidity via precompile
    const gasEstimated = await dexPrecompile
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

    console.log("Add liquidity gas: ", gasEstimated);

    const resPrecompile = await dexPrecompile
      .connect(alithSigner)
      .callStatic.addLiquidity(
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        alithSigner.address,
        20000,
        {
          gasLimit: gasEstimated,
        },
      );

    console.log("Add liquidity result from precompile: ", resPrecompile);

    const addLiquidity = await dexPrecompile
      .connect(alithSigner)
      .addLiquidity(
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        alithSigner.address,
        20000,
        {
          gasLimit: gasEstimated,
        },
      );

    const receiptSub = await addLiquidity.wait();

    //console.log("dex events: ", receiptSub?.events);
    // check events
    //expect((receiptSub?.events as any)[0].event).to.equal("Mint");
    //expect((receiptSub?.events as any)[0].args.sender).to.equal(alithSigner.address);
    //expect((receiptSub?.events as any)[0].args.amount0).to.equal(resPrecompile.amountA);
    //expect((receiptSub?.events as any)[0].args.amount1).to.equal(resPrecompile.amountB);

    const lpTokenSubstrate: any = (
      await api.query.dex.tradingPairLPToken([TOKEN_ID_SUBALPHA, TOKEN_ID_SUBBETA])
    ).toJSON();
    const lpAccountSubstrate: any = (await api.query.assets.account(lpTokenSubstrate, alithSigner.address)).toJSON();
    const lpBalanceSubstrate = lpAccountSubstrate.balance;
    expect(lpBalanceSubstrate).to.eq(BigNumber.from("499999999999999999000"));

    // the two results should be equaled
    expect(res.amountA).to.eq(resPrecompile.amountA);
    expect(res.amountB).to.eq(resPrecompile.amountB);
    expect(res.liquidity).to.eq(resPrecompile.liquidity);
    expect(lpBalance).to.eq(lpBalanceSubstrate);
  });

  it("add liquidity eth", async () => {
    // add liquidity on uniswap

    const gasEstimated = await uniswapV2Router02
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

    console.log("Add liquidity eth gas: ", gasEstimated);

    const res = await uniswapV2Router02
      .connect(owner)
      .callStatic.addLiquidityETH(
        alpha.address,
        utils.parseEther("1000").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        owner.address,
        ethers.constants.MaxUint256,
        {
          gasLimit: gasEstimated,
          value: utils.parseEther("250"),
        },
      );

    console.log("Add liquidity eth result from uniswap call: ", res);

    await uniswapV2Router02
      .connect(owner)
      .addLiquidityETH(
        alpha.address,
        utils.parseEther("1000").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        owner.address,
        ethers.constants.MaxUint256,
        {
          gasLimit: gasEstimated,
          value: utils.parseEther("250"),
        },
      );

    const pairAddress = await uniswapV2Factory.getPair(alpha.address, weth.address);
    const lpToken: CustomERC20 = await ethers.getContractAt("CustomERC20", pairAddress);
    const lpBalance = await lpToken.balanceOf(owner.address);
    expect(lpBalance).to.eq(BigNumber.from("499999999999999999000"));

    // add liquidity via precompile
    const gasEstimatedPrecompile = await dexPrecompile
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

    console.log("Add liquidity gas: ", gasEstimatedPrecompile);

    const resPrecompile = await dexPrecompile
      .connect(alithSigner)
      .callStatic.addLiquidityETH(
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        utils.parseEther("1000").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        alithSigner.address,
        20000,
        {
          gasLimit: gasEstimatedPrecompile,
          value: utils.parseEther("250"),
        },
      );

    console.log("Add liquidity result from precompile: ", resPrecompile);

    const addLiquidityETH = await dexPrecompile
      .connect(alithSigner)
      .addLiquidityETH(
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        utils.parseEther("1000").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        alithSigner.address,
        20000,
        {
          gasLimit: gasEstimatedPrecompile,
          value: utils.parseEther("250"),
        },
      );

    await addLiquidityETH.wait();

    const lpTokenSubstrate: any = (
      await api.query.dex.tradingPairLPToken([TOKEN_ID_SUBALPHA, TOKEN_ID_SUBBETA])
    ).toJSON();
    const lpAccountSubstrate: any = (await api.query.assets.account(lpTokenSubstrate, alithSigner.address)).toJSON();
    const lpBalanceSubstrate = lpAccountSubstrate.balance;
    expect(lpBalanceSubstrate).to.eq(BigNumber.from("499999999999999999000"));

    // the two results should be equaled
    expect(res.amountToken).to.eq(resPrecompile.amountToken);
    expect(res.amountETH).to.eq(resPrecompile.amountETH);
    expect(res.liquidity).to.eq(resPrecompile.liquidity);
    expect(lpBalance).to.eq(lpBalanceSubstrate);
  });

  it("remove liquidity", async () => {
    // 1. remove liquidity on uniswap
    // 1.1 verify liquidity balance
    const pairAddress = await uniswapV2Factory.getPair(alpha.address, beta.address);
    const lpToken: CustomERC20 = await ethers.getContractAt("CustomERC20", pairAddress);
    let lpBalance = await lpToken.balanceOf(owner.address);
    expect(lpBalance).to.eq(BigNumber.from("499999999999999999000"));

    // 1.2 allow owner to send funds to router - this is required to burn LP tokens which removes liquidity
    await lpToken.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));

    // 1.3 remove liquidity callstatic
    const res = await uniswapV2Router02
      .connect(owner)
      .callStatic.removeLiquidity(
        alpha.address,
        beta.address,
        utils.parseEther("100").toString(),
        0,
        0,
        owner.address,
        ethers.constants.MaxUint256,
      );

    console.log("Remove liquidity result from uniswap call: ", res);

    // 1.4 run the and finalize function on blockchain
    await uniswapV2Router02
      .connect(owner)
      .removeLiquidity(
        alpha.address,
        beta.address,
        utils.parseEther("100").toString(),
        0,
        0,
        owner.address,
        ethers.constants.MaxUint256,
      );

    // 1.5 verify the results on chain
    const lpBalanceAfter = await lpToken.balanceOf(owner.address);
    const alphaBalance = await alpha.balanceOf(owner.address);
    const betaBalance = await beta.balanceOf(owner.address);

    expect(lpBalanceAfter).to.eq(BigNumber.from("399999999999999999000"));
    expect(alphaBalance).to.eq(BigNumber.from("9998200000000000000000000"));
    expect(betaBalance).to.eq(BigNumber.from("9999800000000000000000000"));

    // 2. add liquidity via precompile
    // 2.1 estimate gas usage
    const gasEstimated = await dexPrecompile
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

    console.log("Remove liquidity gas: ", gasEstimated);

    // 2.2 run callstatic for return results
    const resPrecompile = await dexPrecompile
      .connect(alithSigner)
      .callStatic.removeLiquidity(
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA),
        utils.parseEther("100").toString(),
        0,
        0,
        alithSigner.address,
        20000,
        {
          gasLimit: gasEstimated,
        },
      );

    console.log("Add liquidity result from precompile: ", resPrecompile);

    // 2.3 run and finalize it on blockchain
    const removeLiquidity = await dexPrecompile
      .connect(alithSigner)
      .removeLiquidity(
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA),
        utils.parseEther("100").toString(),
        0,
        0,
        alithSigner.address,
        20000,
        {
          gasLimit: gasEstimated,
        },
      );

    await removeLiquidity.wait();

    // 2.4 verify the results on chain
    const lpTokenSubstrate: any = (
      await api.query.dex.tradingPairLPToken([TOKEN_ID_SUBALPHA, TOKEN_ID_SUBBETA])
    ).toJSON();
    const lpAccountSubstrate: any = (await api.query.assets.account(lpTokenSubstrate, alithSigner.address)).toJSON();
    const alphaAccountSubstrate: any = (
      await api.query.assets.account(TOKEN_ID_SUBALPHA, alithSigner.address)
    ).toJSON();
    const betaAccountSubstrate: any = (await api.query.assets.account(TOKEN_ID_SUBBETA, alithSigner.address)).toJSON();
    expect(lpAccountSubstrate.balance).to.eq(BigNumber.from("399999999999999999000"));
    expect(alphaAccountSubstrate.balance).to.eq(BigNumber.from("9998200000000000000000000"));
    expect(betaAccountSubstrate.balance).to.eq(BigNumber.from("9999800000000000000000000"));

    // 3. the two results should be equaled
    expect(res.amountA).to.eq(resPrecompile.amountA);
    expect(res.amountB).to.eq(resPrecompile.amountB);
  });

  it("remove liquidity eth", async () => {
    // remove liquidity on uniswap
    const pairAddress = await uniswapV2Factory.getPair(alpha.address, weth.address);
    const lpToken: CustomERC20 = await ethers.getContractAt("CustomERC20", pairAddress);
    const lpBalance = await lpToken.balanceOf(owner.address);
    expect(lpBalance).to.eq(BigNumber.from("499999999999999999000"));

    // allow user to send funds to router - this is required to burn LP tokens which removes liquidity
    await lpToken.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));

    const res = await uniswapV2Router02
      .connect(owner)
      .callStatic.removeLiquidityETH(
        alpha.address,
        utils.parseEther("100").toString(),
        0,
        0,
        owner.address,
        ethers.constants.MaxUint256,
      );

    console.log("Remove liquidity eth result from uniswap call: ", res);

    // run the function on blockchain
    await uniswapV2Router02
      .connect(owner)
      .removeLiquidityETH(
        alpha.address,
        utils.parseEther("100").toString(),
        0,
        0,
        owner.address,
        ethers.constants.MaxUint256,
      );

    // remove liquidity eth via precompile
    const gasEstimated = await dexPrecompile
      .connect(alithSigner)
      .estimateGas.removeLiquidityETH(
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        utils.parseEther("100").toString(),
        0,
        0,
        alithSigner.address,
        20000,
      );

    console.log("Remove liquidity gas: ", gasEstimated);

    const resPrecompile = await dexPrecompile
      .connect(alithSigner)
      .callStatic.removeLiquidityETH(
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        utils.parseEther("100").toString(),
        0,
        0,
        alithSigner.address,
        20000,
        {
          gasLimit: gasEstimated,
        },
      );

    console.log("Add liquidity result from precompile: ", resPrecompile);

    const removeLiquidity = await dexPrecompile
      .connect(alithSigner)
      .removeLiquidityETH(
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        utils.parseEther("100").toString(),
        0,
        0,
        alithSigner.address,
        20000,
        {
          gasLimit: gasEstimated,
        },
      );

    await removeLiquidity.wait();

    // the two results should be equaled
    expect(res.amountToken).to.eq(resPrecompile.amountToken);
    expect(res.amountETH).to.eq(resPrecompile.amountETH);
  });

  it("swap exact token for token", async () => {
    // 1. swap in uniswap

    // 1.1 user approves router to spend tokens
    await alpha.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));
    await beta.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));

    // 1.2 mint some tokens to user
    await alpha.mint(user.address, utils.parseEther("100"));

    // 1.3 verify user token Alpha and Beta balances
    expect(await alpha.balanceOf(user.address)).to.eq(utils.parseEther("100"));
    expect(await beta.balanceOf(user.address)).to.eq(utils.parseEther("0"));

    // 1.4 check amount of tokens retrievable
    const [, betaAmountOut] = await uniswapV2Router02
      .connect(user)
      .getAmountsOut(utils.parseEther("100"), [alpha.address, beta.address]);
    expect(betaAmountOut).to.eq(BigNumber.from("22162943203289985550"));

    // 1.5 run callstatic for swap
    const res = await uniswapV2Router02
      .connect(user)
      .callStatic.swapExactTokensForTokens(
        utils.parseEther("100"),
        betaAmountOut,
        [alpha.address, beta.address],
        user.address,
        ethers.constants.MaxUint256,
      );
    console.log("Result from uniswap call: ", res);

    // 1.6 run the function on blockchain
    const callRes: ContractTransaction = await uniswapV2Router02
      .connect(user)
      .swapExactTokensForTokens(
        utils.parseEther("100"),
        betaAmountOut,
        [alpha.address, beta.address],
        user.address,
        ethers.constants.MaxUint256,
      );

    const receipt = await callRes.wait();

    //console.log("uni events: ", receipt?.events);
    //console.log("uni logs: ", receipt?.logs);
    //console.log("uni event: ", (receipt?.events as any)[0].event);
    //console.log("uni event: ", receipt.events[0].getTransactionReceipt());

    // 1.7 verify user token Alpha and Beta balances
    expect(await alpha.balanceOf(user.address)).to.eq(utils.parseEther("0"));
    expect(await beta.balanceOf(user.address)).to.eq(BigNumber.from("22162943203289985550"));

    // slippage = (25 - 22.162) / 25
    const slippageLoss = utils.parseEther("25").sub(BigNumber.from("22162943203289985550"));
    const slippageLossDecimal = +utils.formatEther(slippageLoss);
    const slippageLossPercent = (slippageLossDecimal / 25) * 100;
    expect(slippageLossDecimal).to.be.eq(2.8370567967100144);
    expect(slippageLossPercent.toFixed(3)).to.be.eq("11.348"); // 11.348% lost in slippage

    // 2. swap via precompile

    // 2.1 mint some tokens to bob
    await new Promise<void>((resolve, reject) => {
      api.tx.assets
        .mint(TOKEN_ID_SUBALPHA, bobSigner.address, utils.parseEther("100").toString())
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) {
            console.log(`setup block hash: ${status.asInBlock}`);
            resolve();
          }
        })
        .catch((err) => reject(err));
    });

    // 2.2 verify user token Alpha and Beta balances
    expect(((await api.query.assets.account(TOKEN_ID_SUBALPHA, bobSigner.address)).toJSON() as any).balance).to.eq(
      utils.parseEther("100"),
    );
    expect((await api.query.assets.account(TOKEN_ID_SUBBETA, bobSigner.address)).toJSON() as any).to.eq(null);

    // 2.3 check amount of tokens retrievable
    const [, betaAmountOutSub] = await dexPrecompile
      .connect(bobSigner)
      .getAmountsOut(utils.parseEther("100"), [
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA),
      ]);
    expect(betaAmountOutSub).to.eq(BigNumber.from("22162943203289985550"));

    // 2.4 run function call static
    const resPrecompile = await dexPrecompile
      .connect(bobSigner)
      .callStatic.swapExactTokensForTokens(
        utils.parseEther("100"),
        betaAmountOutSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA), assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA)],
        bobSigner.address,
        20000,
      );

    console.log("Result from precompile: ", resPrecompile);

    // 2.5 run and finalize it on chain
    const callResSub = await dexPrecompile
      .connect(bobSigner)
      .swapExactTokensForTokens(
        utils.parseEther("100"),
        betaAmountOutSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA), assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA)],
        bobSigner.address,
        20000,
      );

    const receiptSub = await callResSub.wait();

    //console.log("uni events: ", receiptSub?.events);
    //console.log("uni logs: ", receiptSub?.logs);
    //console.log("uni event: ", (receiptSub?.events as any)[0].event);

    // 2.6 verify user token Alpha and Beta balances
    expect((await api.query.assets.account(TOKEN_ID_SUBALPHA, bobSigner.address)).toJSON() as any).to.eq(null);
    expect(((await api.query.assets.account(TOKEN_ID_SUBBETA, bobSigner.address)).toJSON() as any).balance).to.eq(
      BigNumber.from("22162943203289985550"),
    );

    // slippage = (25 - 22.162) / 25
    const slippageLossSub = utils.parseEther("25").sub(BigNumber.from("22162943203289985550"));
    const slippageLossDecimalSub = +utils.formatEther(slippageLossSub);
    const slippageLossPercentSub = (slippageLossDecimal / 25) * 100;
    expect(slippageLossDecimalSub).to.be.eq(2.8370567967100144);
    expect(slippageLossPercentSub.toFixed(3)).to.be.eq("11.348"); // 11.348% lost in slippage

    // 3. the two results should be equaled
    expect(res[0]).to.eq(resPrecompile[0]);
    expect(res[1]).to.eq(resPrecompile[1]);
  });

  it("swap exact token for eth", async () => {
    // 1. swap in uniswap

    // 1.1 get lp token
    const pairAddress = await uniswapV2Factory.getPair(alpha.address, weth.address);
    const lpToken: CustomERC20 = await ethers.getContractAt("CustomERC20", pairAddress);

    // 1.2 user approves router to spend tokens
    await alpha.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));
    await weth.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));

    // 1.3 mint some tokens to user
    await alpha.mint(user.address, utils.parseEther("100"));

    // 1.4 verify user token Alpha and Eth balances
    expect(await alpha.balanceOf(user.address)).to.eq(utils.parseEther("100"));
    const userEthBalanceBefore = await etherProvider.getBalance(user.address);

    // 1.5 check amount of tokens retrievable
    const [, ethAmountOut] = await uniswapV2Router02
      .connect(user)
      .getAmountsOut(utils.parseEther("100"), [alpha.address, weth.address]);
    expect(ethAmountOut).to.eq(BigNumber.from("22162943203289985550"));

    // 1.6 run callstatic for swap
    const res = await uniswapV2Router02
      .connect(user)
      .callStatic.swapExactTokensForETH(
        utils.parseEther("100"),
        ethAmountOut,
        [alpha.address, weth.address],
        user.address,
        ethers.constants.MaxUint256,
      );
    console.log("The result from uniswap call: ", res);

    console.log("user lptoken uniswap are: ", user.address, lpToken.address, uniswapV2Router02.address);

    // 1.7 run the function on blockchain
    await expect(
      uniswapV2Router02
        .connect(user)
        .swapExactTokensForETH(
          utils.parseEther("100"),
          ethAmountOut,
          [alpha.address, weth.address],
          user.address,
          ethers.constants.MaxUint256,
        ),
    )
      .to.emit(alpha, "Transfer")
      .withArgs(user.address, lpToken.address, utils.parseEther("100"))
      .to.emit(weth, "Transfer")
      .withArgs(lpToken.address, uniswapV2Router02.address, ethAmountOut);

    //const receipt = await uniAddLiquidity.wait();

    //console.log("uni events: ", receipt?.events);

    // 1.8 verify user token balances
    expect(await alpha.balanceOf(user.address)).to.eq(utils.parseEther("0"));
    // FIXME error occurs here
    const userEthBalanceAfter = await etherProvider.getBalance(user.address);
    expect(userEthBalanceAfter.sub(userEthBalanceBefore)).to.approximately(ethAmountOut, 100000000000000000n);

    // 2. swap via precompile

    // 2.1 mint some tokens to bob
    await new Promise<void>((resolve, reject) => {
      api.tx.assets
        .mint(TOKEN_ID_SUBALPHA, bobSigner.address, utils.parseEther("100").toString())
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) {
            console.log(`setup block hash: ${status.asInBlock}`);
            resolve();
          }
        })
        .catch((err) => reject(err));
    });

    // 2.2 verify user token balances
    expect(((await api.query.assets.account(TOKEN_ID_SUBALPHA, bobSigner.address)).toJSON() as any).balance).to.eq(
      utils.parseEther("100"),
    );
    const balanceEthBefore = ((await api.query.assets.account(GAS_TOKEN_ID, alithSigner.address)).toJSON() as any)
      .balance;

    // 2.3 check amount of tokens retrievable
    const [, ethAmountOutSub] = await dexPrecompile
      .connect(bobSigner)
      .getAmountsOut(utils.parseEther("100"), [
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        assetIdToERC20ContractAddress(GAS_TOKEN_ID),
      ]);
    expect(ethAmountOutSub).to.eq(BigNumber.from("22162943203289985550"));

    // 2.4 run function call static
    const resPrecompile = await dexPrecompile
      .connect(bobSigner)
      .callStatic.swapExactTokensForETH(
        utils.parseEther("100"),
        ethAmountOutSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA), assetIdToERC20ContractAddress(GAS_TOKEN_ID)],
        alithSigner.address,
        20000,
      );

    console.log("Add liquidity result from precompile: ", resPrecompile);

    // 2.5 run and finalize it on chain
    const addLiquidity = await dexPrecompile
      .connect(bobSigner)
      .swapExactTokensForETH(
        utils.parseEther("100"),
        ethAmountOutSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA), assetIdToERC20ContractAddress(GAS_TOKEN_ID)],
        alithSigner.address,
        20000,
      );

    const receiptSub = await addLiquidity.wait();

    //console.log("dex events: ", receiptSub?.events);

    // 2.6 verify user token Alpha and Beta balances
    expect((await api.query.assets.account(TOKEN_ID_SUBALPHA, bobSigner.address)).toJSON() as any).to.eq(null);
    // FIXME AssertionError: expected 22162943202060927000 to equal '22162943203289985550'
    expect(
      ((await api.query.assets.account(GAS_TOKEN_ID, alithSigner.address)).toJSON() as any).balance - balanceEthBefore,
    ).to.approximately(22162943203289985550, 100000000000);

    // 3. the two results should be equaled
    expect(res[0]).to.eq(resPrecompile[0]);
    expect(res[1]).to.eq(resPrecompile[1]);
  });

  it("swap exact eth for token", async () => {
    // 1. swap in uniswap

    // 1.1 user approves router to spend tokens
    await alpha.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));
    await weth.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));

    // 1.2 get the current user eth balance
    const userEthBalanceBefore = await etherProvider.getBalance(user.address);

    // 1.3 verify user token balances
    expect(await alpha.balanceOf(user.address)).to.eq(utils.parseEther("0"));

    // 1.4 check amount of tokens retrievable
    const [, alphaAmountOut] = await uniswapV2Router02
      .connect(user)
      .getAmountsOut(utils.parseEther("100"), [weth.address, alpha.address]);

    expect(alphaAmountOut).to.eq(BigNumber.from("323308177421962483072"));

    // 1.5 run callstatic for swap
    const res = await uniswapV2Router02
      .connect(user)
      .callStatic.swapExactETHForTokens(
        alphaAmountOut,
        [weth.address, alpha.address],
        user.address,
        ethers.constants.MaxUint256,
        {
          value: utils.parseEther("100"),
        },
      );
    console.log("Result from uniswap call: ", res);

    // 1.6 run the function on blockchain
    await uniswapV2Router02
      .connect(user)
      .swapExactETHForTokens(alphaAmountOut, [weth.address, alpha.address], user.address, ethers.constants.MaxUint256, {
        value: utils.parseEther("100"),
      });

    // verify user token balances
    expect(await alpha.balanceOf(user.address)).to.eq(alphaAmountOut);
    const userEthBalanceAfter = await etherProvider.getBalance(user.address);
    expect(userEthBalanceBefore.sub(userEthBalanceAfter)).to.approximately(
      utils.parseEther("100"),
      1000000000000000000n,
    );

    // 2. swap via precompile

    // 2.1 mint some tokens to bob
    await new Promise<void>((resolve, reject) => {
      api.tx.assets
        .mint(GAS_TOKEN_ID, bobSigner.address, utils.parseEther("100").toString())
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) {
            console.log(`setup block hash: ${status.asInBlock}`);
            resolve();
          }
        })
        .catch((err) => reject(err));
    });

    // 2.2 verify user token balances
    expect((await api.query.assets.account(TOKEN_ID_SUBALPHA, bobSigner.address)).toJSON() as any).to.eq(null);
    const xrpBalanceBefore = ((await api.query.assets.account(GAS_TOKEN_ID, bobSigner.address)).toJSON() as any)
      .balance;

    console.log(`: get amount out`);
    // 2.3 check amount of tokens retrievable
    const [, alphaAmountOutSub] = await dexPrecompile
      .connect(bobSigner)
      .getAmountsOut(utils.parseEther("100"), [
        assetIdToERC20ContractAddress(GAS_TOKEN_ID),
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
      ]);
    expect(alphaAmountOutSub).to.eq(BigNumber.from("323308177421962483072"));
    console.log(`: get amount out ${alphaAmountOutSub}`);

    // 2.4 run function call static
    const resPrecompile = await dexPrecompile
      .connect(bobSigner)
      .callStatic.swapExactETHForTokens(
        alphaAmountOutSub,
        [assetIdToERC20ContractAddress(GAS_TOKEN_ID), assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA)],
        bobSigner.address,
        20000,
        {
          value: utils.parseEther("100"),
        },
      );

    console.log("Result from precompile: ", resPrecompile);

    // 2.5 run and finalize it on chain
    const callRes = await dexPrecompile
      .connect(bobSigner)
      .swapExactETHForTokens(
        alphaAmountOutSub,
        [assetIdToERC20ContractAddress(GAS_TOKEN_ID), assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA)],
        bobSigner.address,
        20000,
        {
          value: utils.parseEther("100"),
        },
      );

    await callRes.wait();

    // 2.6 verify user token balances
    const xrpBalanceAfter = ((await api.query.assets.account(GAS_TOKEN_ID, bobSigner.address)).toJSON() as any).balance;
    // Approximately equal due to gas fee usage
    expect(BigInt(xrpBalanceBefore - xrpBalanceAfter)).to.approximately(utils.parseEther("100"), 1000000000n);
    expect(((await api.query.assets.account(TOKEN_ID_SUBALPHA, bobSigner.address)).toJSON() as any).balance).to.eq(
      BigNumber.from("323308177421962483072"),
    );

    // 3. the two results should be equaled
    expect(res[0]).to.eq(resPrecompile[0]);
    expect(res[1]).to.eq(resPrecompile[1]);
  });

  it("swap token for exact token", async () => {
    // 1. swap in uniswap

    // 1.1 user approves router to spend tokens
    await alpha.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));
    await beta.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));

    // 1.2 mint some tokens to user
    await alpha.mint(user.address, utils.parseEther("100"));

    // 1.3 get current token Alpha and Beta balances
    const alphaBalanceBefore = await alpha.balanceOf(user.address);
    const betaBalanceBefore = await beta.balanceOf(user.address);

    // 1.4 check amount of tokens retrievable
    const [alphaIn] = await uniswapV2Router02
      .connect(user)
      .getAmountsIn(utils.parseEther("25"), [alpha.address, beta.address]);
    expect(alphaIn).to.eq(BigNumber.from("147658582167971835438"));

    // 1.5 run callstatic for swap
    const res = await uniswapV2Router02
      .connect(user)
      .callStatic.swapTokensForExactTokens(
        utils.parseEther("25"),
        alphaIn,
        [alpha.address, beta.address],
        user.address,
        ethers.constants.MaxUint256,
      );
    console.log("Result from uniswap call: ", res);

    // 1.6 run the function on blockchain
    const callRes: ContractTransaction = await uniswapV2Router02
      .connect(user)
      .swapTokensForExactTokens(
        utils.parseEther("25"),
        alphaIn,
        [alpha.address, beta.address],
        user.address,
        ethers.constants.MaxUint256,
      );

    const receipt = await callRes.wait();

    //console.log("uni events: ", receipt?.events);
    //console.log("uni logs: ", receipt?.logs);
    //console.log("uni event: ", (receipt?.events as any)[0].event);
    //console.log("uni event: ", receipt.events[0].getTransactionReceipt());

    // 1.7 verify user token Alpha and Beta balances
    const alphaBalanceAfter = await alpha.balanceOf(user.address);
    const betaBalanceAfter = await beta.balanceOf(user.address);
    expect(alphaBalanceBefore.sub(alphaBalanceAfter)).to.eq(alphaIn);
    expect(betaBalanceAfter.sub(betaBalanceBefore)).to.eq(utils.parseEther("25"));

    // 2. swap via precompile

    // 2.1 mint some tokens to bob
    await new Promise<void>((resolve, reject) => {
      api.tx.assets
        .mint(TOKEN_ID_SUBALPHA, bobSigner.address, utils.parseEther("100").toString())
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) {
            console.log(`setup block hash: ${status.asInBlock}`);
            resolve();
          }
        })
        .catch((err) => reject(err));
    });

    // 2.2 get token Alpha and Beta balances
    const subalphaBalanceBefore = (
      (await api.query.assets.account(TOKEN_ID_SUBALPHA, bobSigner.address)).toJSON() as any
    ).balance;
    const subbetaBalanceBefore = ((await api.query.assets.account(TOKEN_ID_SUBBETA, bobSigner.address)).toJSON() as any)
      .balance;

    // 2.3 check amount of tokens retrievable
    const [alphaInSub] = await dexPrecompile
      .connect(bobSigner)
      .getAmountsIn(utils.parseEther("25"), [
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA),
      ]);
    expect(alphaInSub).to.eq(BigNumber.from("147658582167971835438"));

    // 2.4 run function call static
    const resPrecompile = await dexPrecompile
      .connect(bobSigner)
      .callStatic.swapTokensForExactTokens(
        utils.parseEther("25"),
        alphaInSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA), assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA)],
        bobSigner.address,
        20000,
      );

    console.log("Result from precompile: ", resPrecompile);

    // 2.5 run and finalize it on chain
    const callResSub = await dexPrecompile
      .connect(bobSigner)
      .swapTokensForExactTokens(
        utils.parseEther("25"),
        alphaInSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA), assetIdToERC20ContractAddress(TOKEN_ID_SUBBETA)],
        bobSigner.address,
        20000,
      );

    const receiptSub = await callResSub.wait();

    //console.log("uni events: ", receiptSub?.events);
    //console.log("uni logs: ", receiptSub?.logs);
    //console.log("uni event: ", (receiptSub?.events as any)[0].event);

    // 2.6 verify user token Alpha and Beta balances
    const subalphaBalanceAfter = (
      (await api.query.assets.account(TOKEN_ID_SUBALPHA, bobSigner.address)).toJSON() as any
    ).balance;
    const subbetaBalanceAfter = ((await api.query.assets.account(TOKEN_ID_SUBBETA, bobSigner.address)).toJSON() as any)
      .balance;
    // FIXME there are some slight differences here. Needs to investigate it deeper
    expect(BigInt(subalphaBalanceBefore - subalphaBalanceAfter)).to.approximately(alphaInSub, 100000n);
    expect(BigInt(subbetaBalanceAfter - subbetaBalanceBefore)).to.approximately(utils.parseEther("25"), 100000n);

    // 3. the two results should be equaled
    expect(res[0]).to.eq(resPrecompile[0]);
    expect(res[1]).to.eq(resPrecompile[1]);
  });

  it("swap token for exact eth", async () => {
    // 1. swap in uniswap

    // 1.1 get lp token
    const pairAddress = await uniswapV2Factory.getPair(alpha.address, weth.address);
    const lpToken: CustomERC20 = await ethers.getContractAt("CustomERC20", pairAddress);

    // 1.2 user approves router to spend tokens
    await alpha.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));
    await weth.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));

    // 1.3 mint some tokens to user
    await alpha.mint(user.address, utils.parseEther("100"));

    // 1.4 get current token Alpha and Beta balances
    const alphaBalanceBefore = await alpha.balanceOf(user.address);
    const userEthBalanceBefore = await etherProvider.getBalance(user.address);

    // 1.5 check amount of tokens retrievable
    const [alphaIn] = await uniswapV2Router02
      .connect(user)
      .getAmountsIn(utils.parseEther("25"), [alpha.address, weth.address]);
    expect(alphaIn).to.eq(BigNumber.from("57193663699659203786"));

    // 1.6 run callstatic for swap
    const res = await uniswapV2Router02
      .connect(user)
      .callStatic.swapTokensForExactETH(
        utils.parseEther("25"),
        alphaIn,
        [alpha.address, weth.address],
        user.address,
        ethers.constants.MaxUint256,
      );
    console.log("The result from uniswap call: ", res);

    // 1.7 run the function on blockchain
    const uniRes = await uniswapV2Router02
      .connect(user)
      .swapTokensForExactETH(
        utils.parseEther("25"),
        alphaIn,
        [alpha.address, weth.address],
        user.address,
        ethers.constants.MaxUint256,
      );
    const receipt = await uniRes.wait();

    //console.log("uni events: ", receipt?.events);

    // 1.8 verify user token balances
    const alphaBalanceAfter = await alpha.balanceOf(user.address);
    expect(alphaBalanceBefore.sub(alphaBalanceAfter)).to.eq(alphaIn);
    const userEthBalanceAfter = await etherProvider.getBalance(user.address);
    expect(userEthBalanceAfter.sub(userEthBalanceBefore)).to.approximately(utils.parseEther("25"), 100000000000000000n);

    // 2. swap via precompile

    // 2.1 mint some tokens to bob
    await new Promise<void>((resolve, reject) => {
      api.tx.assets
        .mint(TOKEN_ID_SUBALPHA, bobSigner.address, utils.parseEther("100").toString())
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) {
            console.log(`setup block hash: ${status.asInBlock}`);
            resolve();
          }
        })
        .catch((err) => reject(err));
    });

    // 2.2 verify user token balances
    const subalphaBalanceBefore = (
      (await api.query.assets.account(TOKEN_ID_SUBALPHA, bobSigner.address)).toJSON() as any
    ).balance;
    const subEthBalanceBefore = ((await api.query.assets.account(GAS_TOKEN_ID, bobSigner.address)).toJSON() as any)
      .balance;

    // 2.3 check amount of tokens retrievable
    const [alphaInSub] = await dexPrecompile
      .connect(bobSigner)
      .getAmountsIn(utils.parseEther("25"), [
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
        assetIdToERC20ContractAddress(GAS_TOKEN_ID),
      ]);
    expect(alphaInSub).to.eq(BigNumber.from("57193663699659203786"));

    // 2.4 run function call static
    const resPrecompile = await dexPrecompile
      .connect(bobSigner)
      .callStatic.swapTokensForExactETH(
        utils.parseEther("25"),
        alphaInSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA), assetIdToERC20ContractAddress(GAS_TOKEN_ID)],
        bobSigner.address,
        20000,
      );

    console.log("Add liquidity result from precompile: ", resPrecompile);

    // 2.5 run and finalize it on chain
    const addLiquidity = await dexPrecompile
      .connect(bobSigner)
      .swapTokensForExactETH(
        utils.parseEther("25"),
        alphaInSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA), assetIdToERC20ContractAddress(GAS_TOKEN_ID)],
        bobSigner.address,
        20000,
      );

    const receiptSub = await addLiquidity.wait();

    //console.log("dex events: ", receiptSub?.events);

    // 2.6 verify user token Alpha and Beta balances
    const subalphaBalanceAfter = (
      (await api.query.assets.account(TOKEN_ID_SUBALPHA, bobSigner.address)).toJSON() as any
    ).balance;
    const subEthBalanceAfter = ((await api.query.assets.account(GAS_TOKEN_ID, bobSigner.address)).toJSON() as any)
      .balance;
    // FIXME there are some slight differences here. Needs to investigate it deeper
    expect(BigInt(subalphaBalanceBefore - subalphaBalanceAfter)).to.approximately(alphaInSub, 100000n);
    expect(BigInt(subEthBalanceAfter - subEthBalanceBefore)).to.approximately(utils.parseEther("25"), 10000000n);

    // 3. the two results should be equaled
    expect(res[0]).to.eq(resPrecompile[0]);
    expect(res[1]).to.eq(resPrecompile[1]);
  });

  it("swap eth for exact token", async () => {
    // 1. swap in uniswap

    // 1.1 user approves router to spend tokens
    await alpha.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));
    await weth.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));

    // 1.2 get the current user eth balance
    const userEthBalanceBefore = await etherProvider.getBalance(user.address);
    const alphaBalanceBefore = await alpha.balanceOf(user.address);

    // 1.3 check amount of tokens retrievable
    const [ethIn] = await uniswapV2Router02
      .connect(user)
      .getAmountsIn(utils.parseEther("25"), [weth.address, alpha.address]);

    expect(ethIn).to.eq(BigNumber.from("10412378684622396208"));

    // 1.4 callstatic for swap
    const res = await uniswapV2Router02
      .connect(user)
      .callStatic.swapETHForExactTokens(
        utils.parseEther("25"),
        [weth.address, alpha.address],
        user.address,
        ethers.constants.MaxUint256,
        {
          value: ethIn,
        },
      );
    console.log("Result from uniswap call: ", res);

    // 1.5 run the function on blockchain
    await uniswapV2Router02
      .connect(user)
      .swapETHForExactTokens(
        utils.parseEther("25"),
        [weth.address, alpha.address],
        user.address,
        ethers.constants.MaxUint256,
        {
          value: ethIn,
        },
      );

    // 1.6 verify user token balances
    const alphaBalanceAfter = await alpha.balanceOf(user.address);
    expect(alphaBalanceAfter.sub(alphaBalanceBefore)).to.eq(utils.parseEther("25"));
    const userEthBalanceAfter = await etherProvider.getBalance(user.address);
    expect(userEthBalanceBefore.sub(userEthBalanceAfter)).to.approximately(ethIn, 100000000000000000n);

    // 2. swap via precompile

    // 2.1 mint some tokens to bob
    await new Promise<void>((resolve, reject) => {
      api.tx.assets
        .mint(GAS_TOKEN_ID, bobSigner.address, utils.parseEther("100").toString())
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) {
            console.log(`setup block hash: ${status.asInBlock}`);
            resolve();
          }
        })
        .catch((err) => reject(err));
    });

    // 2.2 verify user token balances
    const subalphaBalanceBefore = (
      (await api.query.assets.account(TOKEN_ID_SUBALPHA, bobSigner.address)).toJSON() as any
    ).balance;
    const subEthBalanceBefore = ((await api.query.assets.account(GAS_TOKEN_ID, bobSigner.address)).toJSON() as any)
      .balance;

    // 2.3 check amount of tokens retrievable
    const [ethInSub] = await dexPrecompile
      .connect(bobSigner)
      .getAmountsIn(utils.parseEther("25"), [
        assetIdToERC20ContractAddress(GAS_TOKEN_ID),
        assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA),
      ]);
    expect(ethInSub).to.eq(BigNumber.from("10412378684622396208"));

    // 2.4 run function call static
    const resPrecompile = await dexPrecompile
      .connect(bobSigner)
      .callStatic.swapETHForExactTokens(
        utils.parseEther("25"),
        [assetIdToERC20ContractAddress(GAS_TOKEN_ID), assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA)],
        bobSigner.address,
        20000,
        {
          value: ethInSub,
        },
      );

    console.log("Result from precompile: ", resPrecompile);

    // 2.5 run and finalize it on chain
    const callRes = await dexPrecompile
      .connect(bobSigner)
      .swapETHForExactTokens(
        utils.parseEther("25"),
        [assetIdToERC20ContractAddress(GAS_TOKEN_ID), assetIdToERC20ContractAddress(TOKEN_ID_SUBALPHA)],
        bobSigner.address,
        20000,
        {
          value: ethInSub,
        },
      );

    await callRes.wait();

    // 2.6 verify user token balances
    const subalphaBalanceAfter = (
      (await api.query.assets.account(TOKEN_ID_SUBALPHA, bobSigner.address)).toJSON() as any
    ).balance;
    const subEthBalanceAfter = ((await api.query.assets.account(GAS_TOKEN_ID, bobSigner.address)).toJSON() as any)
      .balance;
    // Approximately equal due to gas fee usage
    expect(BigInt(subEthBalanceBefore - subEthBalanceAfter)).to.approximately(ethInSub, 1000000000n);
    expect(BigInt(subalphaBalanceAfter - subalphaBalanceBefore)).to.approximately(utils.parseEther("25"), 100000n);

    // 3. the two results should be equaled
    expect(res[0]).to.eq(resPrecompile[0]);
    expect(res[1]).to.eq(resPrecompile[1]);
  });

  it("quote", async () => {
    // 1. quote in uniswap
    const res = await uniswapV2Router02
      .connect(user)
      .quote(utils.parseEther("5"), utils.parseEther("200"), utils.parseEther("120"));

    // 2. quote in precompile
    const resSub = await dexPrecompile
      .connect(bobSigner)
      .quote(utils.parseEther("5"), utils.parseEther("200"), utils.parseEther("120"));

    // 3. verify the result
    expect(res).to.eq(resSub);
  });
});
