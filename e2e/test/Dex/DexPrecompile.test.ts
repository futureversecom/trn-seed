import { JsonRpcProvider, Provider } from "@ethersproject/providers";
import { SignerWithAddress } from "@nomiclabs/hardhat-ethers/signers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { BigNumber, Contract, Wallet, utils } from "ethers";
import { ethers } from "hardhat";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  DEX_PRECOMPILE_ABI,
  DEX_PRECOMPILE_ADDRESS,
  GAS_TOKEN_ID,
  NodeProcess,
  assetIdToERC20ContractAddress,
  getNextAssetId,
  startNode,
  typedefs,
} from "../../common";
import { MockERC20, UniswapV2Factory, UniswapV2Router02, WETH9 } from "../../typechain-types";

describe("DEX Precompile", function () {
  let TOKEN_ID_1: number;
  let TOKEN_ID_2: number;

  let node: NodeProcess;
  let api: ApiPromise;

  let alithSigner: Wallet;
  let bobSigner: Wallet;
  let alith: KeyringPair;
  let dexPrecompile: UniswapV2Router02;

  let owner: SignerWithAddress;
  let user: SignerWithAddress;
  let alpha: MockERC20;
  let beta: MockERC20;
  let weth: WETH9;
  let uniswapV2Factory: UniswapV2Factory;
  let uniswapV2Router02: UniswapV2Router02;
  const localJsonProvider: Provider = ethers.provider;

  // Setup api instance
  before(async () => {
    node = await startNode();

    ///
    /// substrate setup
    ///

    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    const jsonProvider = new JsonRpcProvider(`http://localhost:${node.httpPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    alith = new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(jsonProvider);
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(jsonProvider);

    TOKEN_ID_1 = await getNextAssetId(api);
    TOKEN_ID_2 = await getNextAssetId(api, +(await api.query.assetsExt.nextAssetId()).toString() + 1);

    // create alpha and beta assets via Substrate
    const txs = [
      api.tx.assetsExt.createAsset("subAlpha", "SUBALPHA", 18, 1, alith.address), // create asset
      api.tx.assetsExt.createAsset("subBeta", "SUBBETA", 18, 1, alith.address), // create asset
      api.tx.assets.mint(TOKEN_ID_1, alith.address, utils.parseEther("10000000").toString()),
      api.tx.assets.mint(TOKEN_ID_2, alith.address, utils.parseEther("10000000").toString()),
      api.tx.assets.mint(GAS_TOKEN_ID, alith.address, utils.parseEther("10000000").toString()),
    ];
    await new Promise<void>((resolve, reject) => {
      api.tx.utility
        .batch(txs)
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        })
        .catch((err) => reject(err));
    });

    dexPrecompile = new Contract(DEX_PRECOMPILE_ADDRESS, DEX_PRECOMPILE_ABI, alithSigner) as UniswapV2Router02;

    ///
    /// EVM setup (local hardhat chain)
    ///

    // setup owner for re-use and additional accounts
    const [_owner, _user] = await ethers.getSigners();
    owner = _owner;
    user = _user;

    // deploy all contracts - on hardhat local chain (faster tests)
    const WETH9ERC20Factory = await ethers.getContractFactory("WETH9", owner);
    weth = await WETH9ERC20Factory.deploy();
    await weth.deployed();

    const ERC20Factory = await ethers.getContractFactory("MockERC20");
    alpha = await ERC20Factory.deploy(); // deploy the AlphaERC20
    await alpha.deployed();
    beta = await ERC20Factory.deploy(); // deploy the BetaERC20
    await beta.deployed();

    // mint tokens to the owner
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
  });

  after(async () => await node.stop());

  it("addLiquidity", async () => {
    await alpha.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));
    await beta.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));

    let tx;

    const contractAddLiquidityRes = await uniswapV2Router02
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
      );
    await tx.wait();

    // check events
    //expect((receipt?.events as any)[0].event).to.equal("Mint");
    //expect((receipt?.events as any)[0].args.sender).to.equal(alithSigner.address);
    //expect((receipt?.events as any)[0].args.amount0).to.equal(res.amountA);
    //expect((receipt?.events as any)[0].args.amount1).to.equal(res.amountB);

    // add liquidity via precompile
    const precompileAddLiquidityRes = await dexPrecompile
      .connect(alithSigner)
      .callStatic.addLiquidity(
        assetIdToERC20ContractAddress(TOKEN_ID_1),
        assetIdToERC20ContractAddress(TOKEN_ID_2),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        alithSigner.address,
        20000,
      );
    tx = await dexPrecompile
      .connect(alithSigner)
      .addLiquidity(
        assetIdToERC20ContractAddress(TOKEN_ID_1),
        assetIdToERC20ContractAddress(TOKEN_ID_2),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        alithSigner.address,
        20000,
      );
    await tx.wait();

    // check events
    //expect((receiptSub?.events as any)[0].event).to.equal("Mint");
    //expect((receiptSub?.events as any)[0].args.sender).to.equal(alithSigner.address);
    //expect((receiptSub?.events as any)[0].args.amount0).to.equal(contractAddLiquidityRes.amountA);
    //expect((receiptSub?.events as any)[0].args.amount1).to.equal(contractAddLiquidityRes.amountB);

    // verify the results on chain
    const pairAddress = await uniswapV2Factory.getPair(alpha.address, beta.address);
    const lpToken: MockERC20 = await ethers.getContractAt("MockERC20", pairAddress);
    const lpBalance = await lpToken.balanceOf(owner.address);
    const alphaBalance = await alpha.balanceOf(owner.address);
    const betaBalance = await beta.balanceOf(owner.address);
    const lpTokenSubstrate: any = (await api.query.dex.tradingPairLPToken([TOKEN_ID_1, TOKEN_ID_2])).toJSON();
    const lpBalanceSubstrate: any = (
      (await api.query.assets.account(lpTokenSubstrate, alithSigner.address)).toJSON() as any
    ).balance;
    const alphaBalanceSubstrate: any = (
      (await api.query.assets.account(TOKEN_ID_1, alithSigner.address)).toJSON() as any
    ).balance;
    const betaBalanceSubstrate: any = (
      (await api.query.assets.account(TOKEN_ID_2, alithSigner.address)).toJSON() as any
    ).balance;

    expect(contractAddLiquidityRes.amountA).to.eq(precompileAddLiquidityRes.amountA);
    expect(contractAddLiquidityRes.amountB).to.eq(precompileAddLiquidityRes.amountB);
    expect(contractAddLiquidityRes.liquidity).to.eq(precompileAddLiquidityRes.liquidity);
    expect(alphaBalance).to.eq(alphaBalanceSubstrate);
    expect(betaBalance).to.eq(betaBalanceSubstrate);
    expect(lpBalance).to.eq(lpBalanceSubstrate);
  });

  // dependent on 'addLiquidity' test
  it("removeLiquidity", async () => {
    const pairAddress = await uniswapV2Factory.getPair(alpha.address, beta.address);
    const lpToken: MockERC20 = await ethers.getContractAt("MockERC20", pairAddress);

    // allow owner to send funds to router - this is required to burn LP tokens which removes liquidity
    await lpToken.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));

    let tx;

    const contractRemoveLiquidityRes = await uniswapV2Router02
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
    tx = await uniswapV2Router02
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
    await tx.wait();

    const precompileRemoveLiquidityRes = await dexPrecompile
      .connect(alithSigner)
      .callStatic.removeLiquidity(
        assetIdToERC20ContractAddress(TOKEN_ID_1),
        assetIdToERC20ContractAddress(TOKEN_ID_2),
        utils.parseEther("100").toString(),
        0,
        0,
        alithSigner.address,
        20000,
      );
    tx = await dexPrecompile
      .connect(alithSigner)
      .removeLiquidity(
        assetIdToERC20ContractAddress(TOKEN_ID_1),
        assetIdToERC20ContractAddress(TOKEN_ID_2),
        utils.parseEther("100").toString(),
        0,
        0,
        alithSigner.address,
        20000,
      );
    await tx.wait();

    // verify the results on chain
    const lpBalance = await lpToken.balanceOf(owner.address);
    const alphaBalance = await alpha.balanceOf(owner.address);
    const betaBalance = await beta.balanceOf(owner.address);
    const lpTokenSubstrate: any = (await api.query.dex.tradingPairLPToken([TOKEN_ID_1, TOKEN_ID_2])).toJSON();
    const lpBalanceSubstrate: any = (
      (await api.query.assets.account(lpTokenSubstrate, alithSigner.address)).toJSON() as any
    ).balance;
    const alphaBalanceSubstrate: any = (
      (await api.query.assets.account(TOKEN_ID_1, alithSigner.address)).toJSON() as any
    ).balance;
    const betaBalanceSubstrate: any = (
      (await api.query.assets.account(TOKEN_ID_2, alithSigner.address)).toJSON() as any
    ).balance;

    expect(contractRemoveLiquidityRes.amountA).to.eq(precompileRemoveLiquidityRes.amountA);
    expect(contractRemoveLiquidityRes.amountB).to.eq(precompileRemoveLiquidityRes.amountB);
    expect(alphaBalance).to.eq(alphaBalanceSubstrate);
    expect(betaBalance).to.eq(betaBalanceSubstrate);
    expect(lpBalance).to.eq(lpBalanceSubstrate);
  });

  // dependent on 'addLiquidity' test
  // dependent on 'removeLiquidity' test
  it("swapExactTokensForTokens", async () => {
    await alpha.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));
    await beta.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));

    // get current Alpha and Beta token balances
    const alphaBalanceBefore = await alpha.balanceOf(user.address);
    const betaBalanceBefore = await beta.balanceOf(user.address);

    // check amount of tokens retrievable
    const [, betaAmountOut] = await uniswapV2Router02
      .connect(user)
      .getAmountsOut(utils.parseEther("100"), [alpha.address, beta.address]);

    // mint some tokens to user
    await alpha.mint(user.address, utils.parseEther("100"));

    let tx;

    const contractSwapTokensRes = await uniswapV2Router02
      .connect(user)
      .callStatic.swapExactTokensForTokens(
        utils.parseEther("100"),
        betaAmountOut,
        [alpha.address, beta.address],
        user.address,
        ethers.constants.MaxUint256,
      );
    tx = await uniswapV2Router02
      .connect(user)
      .swapExactTokensForTokens(
        utils.parseEther("100"),
        betaAmountOut,
        [alpha.address, beta.address],
        user.address,
        ethers.constants.MaxUint256,
      );
    await tx.wait();

    //console.log("uni events: ", receipt?.events);
    //console.log("uni logs: ", receipt?.logs);
    //console.log("uni event: ", (receipt?.events as any)[0].event);
    //console.log("uni event: ", receipt.events[0].getTransactionReceipt());

    // user token Alpha and Beta balances after swap
    const alphaBalanceAfter = await alpha.balanceOf(user.address);
    const betaBalanceAfter = await beta.balanceOf(user.address);

    // slippage = (25 - 22.162) / 25
    const slippageLoss = utils.parseEther("25").sub(BigNumber.from("22162943203289985550"));
    const slippageLossDecimal = +utils.formatEther(slippageLoss);
    const slippageLossPercent = (slippageLossDecimal / 25) * 100;
    expect(slippageLossDecimal).to.be.eq(2.8370567967100144);
    expect(slippageLossPercent.toFixed(3)).to.be.eq("11.348"); // 11.348% lost in slippage

    // swap via precompile

    const subalphaBalanceBefore =
      ((await api.query.assets.account(TOKEN_ID_1, bobSigner.address)).toJSON() as any)?.balance ?? 0;
    const subbetaBalanceBefore =
      ((await api.query.assets.account(TOKEN_ID_2, bobSigner.address)).toJSON() as any)?.balance ?? 0;

    // check amount of tokens retrievable
    const alphaInSub = utils.parseEther("100"); // amount willing to swap in
    const [, betaAmountOutSub] = await dexPrecompile
      .connect(bobSigner)
      .getAmountsOut(alphaInSub, [
        assetIdToERC20ContractAddress(TOKEN_ID_1),
        assetIdToERC20ContractAddress(TOKEN_ID_2),
      ]);
    expect(betaAmountOutSub).to.be.eq(betaAmountOut);

    // mint some tokens to bob
    await new Promise<void>((resolve, reject) => {
      api.tx.assets
        .mint(TOKEN_ID_1, bobSigner.address, alphaInSub.toString())
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        })
        .catch((err) => reject(err));
    });

    const precompileSwapTokensRes = await dexPrecompile
      .connect(bobSigner)
      .callStatic.swapExactTokensForTokens(
        alphaInSub,
        betaAmountOutSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_1), assetIdToERC20ContractAddress(TOKEN_ID_2)],
        bobSigner.address,
        20000,
      );
    tx = await dexPrecompile
      .connect(bobSigner)
      .swapExactTokensForTokens(
        alphaInSub,
        betaAmountOutSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_1), assetIdToERC20ContractAddress(TOKEN_ID_2)],
        bobSigner.address,
        20000,
      );
    await tx.wait();

    //console.log("uni events: ", receiptSub?.events);
    //console.log("uni logs: ", receiptSub?.logs);
    //console.log("uni event: ", (receiptSub?.events as any)[0].event);

    // user token Alpha and Beta balances after swap
    const subalphaBalanceAfter =
      ((await api.query.assets.account(TOKEN_ID_1, bobSigner.address)).toJSON() as any)?.balance ?? 0;
    const subbetaBalanceAfter =
      ((await api.query.assets.account(TOKEN_ID_2, bobSigner.address)).toJSON() as any)?.balance ?? 0;

    // slippage = (25 - 22.162) / 25
    const slippageLossSub = utils.parseEther("25").sub(BigNumber.from("22162943203289985550"));
    const slippageLossDecimalSub = +utils.formatEther(slippageLossSub);
    const slippageLossPercentSub = (slippageLossDecimal / 25) * 100;
    expect(slippageLossDecimalSub).to.be.eq(2.8370567967100144);
    expect(slippageLossPercentSub.toFixed(3)).to.be.eq("11.348"); // 11.348% lost in slippage

    // validate before and after balances for contract swaps and precompile swaps are equivalent
    expect(alphaBalanceBefore).to.eq(BigInt(subalphaBalanceBefore));
    expect(alphaBalanceAfter).to.eq(BigInt(subalphaBalanceAfter));
    expect(betaBalanceBefore).to.eq(BigInt(subbetaBalanceBefore));
    expect(betaBalanceAfter).to.eq(BigInt(subbetaBalanceAfter));

    expect(contractSwapTokensRes[0]).to.eq(precompileSwapTokensRes[0]);
    expect(contractSwapTokensRes[1]).to.eq(precompileSwapTokensRes[1]);
  });

  // dependent on 'addLiquidity' test
  // dependent on 'removeLiquidity' test
  it("swapTokensForExactTokens", async () => {
    await alpha.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));
    await beta.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));

    // get current Alpha and Beta token balances
    const alphaBalanceBefore = await alpha.balanceOf(user.address);
    const betaBalanceBefore = await beta.balanceOf(user.address);

    // check amount of tokens retrievable
    const [alphaAmountIn] = await uniswapV2Router02
      .connect(user)
      .getAmountsIn(utils.parseEther("25"), [alpha.address, beta.address]);

    // mint input tokens to user
    await alpha.mint(user.address, alphaAmountIn);

    let tx;

    const contractSwapTokensRes = await uniswapV2Router02
      .connect(user)
      .callStatic.swapTokensForExactTokens(
        utils.parseEther("25"),
        alphaAmountIn,
        [alpha.address, beta.address],
        user.address,
        ethers.constants.MaxUint256,
      );
    tx = await uniswapV2Router02
      .connect(user)
      .swapTokensForExactTokens(
        utils.parseEther("25"),
        alphaAmountIn,
        [alpha.address, beta.address],
        user.address,
        ethers.constants.MaxUint256,
      );
    await tx.wait();

    // TODO check events

    // //console.log("uni events: ", receipt?.events);
    // //console.log("uni logs: ", receipt?.logs);
    // //console.log("uni event: ", (receipt?.events as any)[0].event);
    // //console.log("uni event: ", receipt.events[0].getTransactionReceipt());

    // user token Alpha and Beta balances after swap
    const alphaBalanceAfter = await alpha.balanceOf(user.address);
    const betaBalanceAfter = await beta.balanceOf(user.address);

    // swap via precompile

    // get Alpha and Beta token balances before swap
    const subalphaBalanceBefore =
      ((await api.query.assets.account(TOKEN_ID_1, bobSigner.address)).toJSON() as any)?.balance ?? 0;
    const subbetaBalanceBefore =
      ((await api.query.assets.account(TOKEN_ID_2, bobSigner.address)).toJSON() as any)?.balance ?? 0;

    // check amount of tokens retrievable
    const betaAmountOutSub = utils.parseEther("25"); // amount wanting to get out
    const [alphaInSub] = await dexPrecompile
      .connect(bobSigner)
      .getAmountsIn(betaAmountOutSub, [
        assetIdToERC20ContractAddress(TOKEN_ID_1),
        assetIdToERC20ContractAddress(TOKEN_ID_2),
      ]);
    expect(alphaAmountIn).to.eq(alphaInSub);

    // mint some tokens to bob
    await new Promise<void>((resolve, reject) => {
      api.tx.assets
        .mint(TOKEN_ID_1, bobSigner.address, alphaInSub.toString())
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        })
        .catch((err) => reject(err));
    });

    const precompileSwapTokensRes = await dexPrecompile
      .connect(bobSigner)
      .callStatic.swapTokensForExactTokens(
        betaAmountOutSub,
        alphaInSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_1), assetIdToERC20ContractAddress(TOKEN_ID_2)],
        bobSigner.address,
        20000,
      );
    tx = await dexPrecompile
      .connect(bobSigner)
      .swapTokensForExactTokens(
        betaAmountOutSub,
        alphaInSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_1), assetIdToERC20ContractAddress(TOKEN_ID_2)],
        bobSigner.address,
        20000,
      );
    await tx.wait();

    // //console.log("uni events: ", receiptSub?.events);
    // //console.log("uni logs: ", receiptSub?.logs);
    // //console.log("uni event: ", (receiptSub?.events as any)[0].event);

    // user token Alpha and Beta balances after swap
    const subalphaBalanceAfter =
      ((await api.query.assets.account(TOKEN_ID_1, bobSigner.address)).toJSON() as any)?.balance ?? 0;
    const subbetaBalanceAfter =
      ((await api.query.assets.account(TOKEN_ID_2, bobSigner.address)).toJSON() as any)?.balance ?? 0;

    // validate before and after balances for contract swaps and precompile swaps are equivalent
    expect(alphaBalanceBefore).to.eq(BigInt(subalphaBalanceBefore));
    expect(alphaBalanceAfter).to.eq(BigInt(subalphaBalanceAfter));
    expect(betaBalanceBefore).to.eq(BigInt(subbetaBalanceBefore));
    expect(betaBalanceAfter).to.eq(BigInt(subbetaBalanceAfter));

    // the two results should be equal
    expect(contractSwapTokensRes[0]).to.eq(precompileSwapTokensRes[0]);
    expect(contractSwapTokensRes[1]).to.eq(precompileSwapTokensRes[1]);
  });

  it("addLiquidityETH", async () => {
    await alpha.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));
    await beta.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));

    let tx;

    const contractAddLiquidityEthRes = await uniswapV2Router02
      .connect(owner)
      .callStatic.addLiquidityETH(
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
    tx = await uniswapV2Router02
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
    await tx.wait();

    // TODO check events

    // add liquidity via precompile
    const precompileAddLiquidityEthRes = await dexPrecompile
      .connect(alithSigner)
      .callStatic.addLiquidityETH(
        assetIdToERC20ContractAddress(TOKEN_ID_1),
        utils.parseEther("1000").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        alithSigner.address,
        20000,
        {
          value: utils.parseEther("250"),
        },
      );
    tx = await dexPrecompile
      .connect(alithSigner)
      .addLiquidityETH(
        assetIdToERC20ContractAddress(TOKEN_ID_1),
        utils.parseEther("1000").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        alithSigner.address,
        20000,
        {
          value: utils.parseEther("250"),
        },
      );
    await tx.wait();

    // TODO check events
    const pairAddress = await uniswapV2Factory.getPair(alpha.address, weth.address);
    const lpToken: MockERC20 = await ethers.getContractAt("MockERC20", pairAddress);
    const lpBalance = await lpToken.balanceOf(owner.address);
    const alphaBalance = await alpha.balanceOf(owner.address);
    const ethBalance = await owner.getBalance();
    const alphaBalanceSubstrate: any = (
      (await api.query.assets.account(TOKEN_ID_1, alithSigner.address)).toJSON() as any
    ).balance;
    const ethBalanceSubstrate: any = await localJsonProvider.getBalance(owner.address);
    const lpTokenSubstrate: any = (await api.query.dex.tradingPairLPToken([GAS_TOKEN_ID, TOKEN_ID_1])).toJSON();
    const lpBalanceSubstrate: any = (
      (await api.query.assets.account(lpTokenSubstrate, alithSigner.address)).toJSON() as any
    ).balance;

    // validate results
    expect(contractAddLiquidityEthRes.amountToken).to.eq(precompileAddLiquidityEthRes.amountToken);
    expect(contractAddLiquidityEthRes.amountETH).to.eq(precompileAddLiquidityEthRes.amountETH);
    expect(contractAddLiquidityEthRes.liquidity).to.eq(precompileAddLiquidityEthRes.liquidity);
    expect(alphaBalance).to.eq(alphaBalanceSubstrate);
    expect(ethBalance).to.eq(ethBalanceSubstrate);
    expect(lpBalance).to.eq(lpBalanceSubstrate);
  });

  // dependent on 'addLiquidityETH' test
  it("removeLiquidityETH", async () => {
    const pairAddress = await uniswapV2Factory.getPair(alpha.address, weth.address);
    const lpToken: MockERC20 = await ethers.getContractAt("MockERC20", pairAddress);

    // allow user to send funds to router - this is required to burn LP tokens which removes liquidity
    await lpToken.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));

    let tx;

    const contractRemoveLiquidityEthRes = await uniswapV2Router02
      .connect(owner)
      .callStatic.removeLiquidityETH(
        alpha.address,
        utils.parseEther("100").toString(),
        0,
        0,
        owner.address,
        ethers.constants.MaxUint256,
      );
    tx = await uniswapV2Router02
      .connect(owner)
      .removeLiquidityETH(
        alpha.address,
        utils.parseEther("100").toString(),
        0,
        0,
        owner.address,
        ethers.constants.MaxUint256,
      );
    await tx.wait();

    // TODO check events

    const precompileRemoveLiquidityEthRes = await dexPrecompile
      .connect(alithSigner)
      .callStatic.removeLiquidityETH(
        assetIdToERC20ContractAddress(TOKEN_ID_1),
        utils.parseEther("100").toString(),
        0,
        0,
        alithSigner.address,
        20000,
      );
    tx = await dexPrecompile
      .connect(alithSigner)
      .removeLiquidityETH(
        assetIdToERC20ContractAddress(TOKEN_ID_1),
        utils.parseEther("100").toString(),
        0,
        0,
        alithSigner.address,
        20000,
      );
    await tx.wait();

    // TODO check events

    // verify the results on chain
    const lpBalance = await lpToken.balanceOf(owner.address);
    const alphaBalance = await alpha.balanceOf(owner.address);
    const ethBalance = await owner.getBalance();
    const lpTokenSubstrate: any = (await api.query.dex.tradingPairLPToken([GAS_TOKEN_ID, TOKEN_ID_1])).toJSON();
    const lpBalanceSubstrate: any = (
      (await api.query.assets.account(lpTokenSubstrate, alithSigner.address)).toJSON() as any
    ).balance;
    const alphaBalanceSubstrate: any = (
      (await api.query.assets.account(TOKEN_ID_1, alithSigner.address)).toJSON() as any
    ).balance;
    const ethBalanceSubstrate: any = await localJsonProvider.getBalance(owner.address);

    // validate results
    expect(contractRemoveLiquidityEthRes.amountToken).to.eq(precompileRemoveLiquidityEthRes.amountToken);
    expect(contractRemoveLiquidityEthRes.amountETH).to.eq(precompileRemoveLiquidityEthRes.amountETH);
    expect(alphaBalance).to.eq(alphaBalanceSubstrate);
    expect(ethBalance).to.eq(ethBalanceSubstrate);
    expect(lpBalance).to.eq(lpBalanceSubstrate);
  });

  // dependent on 'addLiquidityETH' test
  // dependent on 'removeLiquidityETH' test
  it("swapExactTokensForETH", async () => {
    await alpha.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));

    // get current Alpha and Eth balances
    const alphaBalanceBefore = await alpha.balanceOf(user.address);
    const ethBalanceBefore = await user.getBalance();

    // check amount of tokens retrievable
    const [, ethAmountOut] = await uniswapV2Router02
      .connect(user)
      .getAmountsOut(utils.parseEther("100"), [alpha.address, weth.address]);

    // mint some tokens to user
    await alpha.mint(user.address, utils.parseEther("100"));

    let tx;

    const contractSwapTokensForEthRes = await uniswapV2Router02
      .connect(user)
      .callStatic.swapExactTokensForETH(
        utils.parseEther("100"),
        ethAmountOut,
        [alpha.address, weth.address],
        user.address,
        ethers.constants.MaxUint256,
      );
    tx = await uniswapV2Router02
      .connect(user)
      .swapExactTokensForETH(
        utils.parseEther("100"),
        ethAmountOut,
        [alpha.address, weth.address],
        user.address,
        ethers.constants.MaxUint256,
      );
    await tx.wait();

    // TODO check events
    // .to.emit(alpha, "Transfer")
    // .withArgs(user.address, lpToken.address, utils.parseEther("100"))
    // .to.emit(weth, "Transfer")
    // .withArgs(lpToken.address, uniswapV2Router02.address, ethAmountOut);

    // user Alpha and Eth balances after swap
    const alphaBalanceAfter = await alpha.balanceOf(user.address);
    const ethBalanceAfter = await user.getBalance();

    // swap via precompile

    const subalphaBalanceBefore =
      ((await api.query.assets.account(TOKEN_ID_1, bobSigner.address)).toJSON() as any)?.balance ?? 0; 
    const substrateEthBalanceBefore = await localJsonProvider.getBalance(bobSigner.address);

    // check amount of tokens retrievable
    const alphaInSub = utils.parseEther("100"); // amount willing to swap in
    const [, ethAmountOutSub] = await dexPrecompile
      .connect(bobSigner)
      .getAmountsOut(alphaInSub, [
        assetIdToERC20ContractAddress(TOKEN_ID_1),
        assetIdToERC20ContractAddress(GAS_TOKEN_ID),
      ]);

    // mint some tokens to bob
    await new Promise<void>((resolve, reject) => {
      api.tx.assets
        .mint(TOKEN_ID_1, bobSigner.address, alphaInSub.toString())
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        })
        .catch((err) => reject(err));
    });

    const precompileSwapTokensForEthRes = await dexPrecompile
      .connect(bobSigner)
      .callStatic.swapExactTokensForETH(
        alphaInSub,
        ethAmountOutSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_1), assetIdToERC20ContractAddress(GAS_TOKEN_ID)],
        alithSigner.address,
        20000,
      );
    tx = await dexPrecompile
      .connect(bobSigner)
      .swapExactTokensForETH(
        alphaInSub,
        ethAmountOutSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_1), assetIdToERC20ContractAddress(GAS_TOKEN_ID)],
        alithSigner.address,
        20000,
      );
    await tx.wait();

    // TODO check events

    const subalphaBalanceAfter =
      ((await api.query.assets.account(TOKEN_ID_1, bobSigner.address)).toJSON() as any)?.balance ?? 0;
    const substrateEthBalanceAfter = await localJsonProvider.getBalance(bobSigner.address);

    // validate before and after balances for contract swaps and precompile swaps are equivalent
    expect(alphaBalanceBefore).to.eq(BigInt(subalphaBalanceBefore));
    expect(alphaBalanceAfter).to.eq(BigInt(subalphaBalanceAfter));
    // expect(ethBalanceBefore).to.eq(substrateEthBalanceBefore);
    // expect(ethBalanceAfter).to.eq(substrateEthBalanceAfter);

    expect(contractSwapTokensForEthRes[0]).to.eq(precompileSwapTokensForEthRes[0]);
    expect(contractSwapTokensForEthRes[1]).to.eq(precompileSwapTokensForEthRes[1]);
  });

  // dependent on 'addLiquidityETH' test
  // dependent on 'removeLiquidityETH' test
  it("swapExactETHForTokens", async () => {
    await alpha.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));

    // get current Alpha and Eth balances
    const alphaBalanceBefore = await alpha.balanceOf(user.address);
    const ethBalanceBefore = await user.getBalance();

    // check amount of tokens retrievable
    const [, alphaAmountOut] = await uniswapV2Router02
      .connect(user)
      .getAmountsOut(utils.parseEther("100"), [weth.address, alpha.address]);

    let tx;

    const contractSwapEthForTokensRes = await uniswapV2Router02
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
    tx = await uniswapV2Router02
      .connect(user)
      .swapExactETHForTokens(alphaAmountOut, [weth.address, alpha.address], user.address, ethers.constants.MaxUint256, {
        value: utils.parseEther("100"),
      });
    await tx.wait();
    
    // TODO check events

    // user Alpha and Eth balances after swap
    const alphaBalanceAfter = await alpha.balanceOf(user.address);
    const ethBalanceAfter = await user.getBalance();

    // swap via precompile

    const subalphaBalanceBefore =
      ((await api.query.assets.account(TOKEN_ID_1, bobSigner.address)).toJSON() as any)?.balance ?? 0; 
    const substrateEthBalanceBefore = await localJsonProvider.getBalance(bobSigner.address);

    const ethInSub = utils.parseEther("100"); // amount willing to swap in
    const [, alphaAmountOutSub] = await dexPrecompile
      .connect(bobSigner)
      .getAmountsOut(ethInSub, [
        assetIdToERC20ContractAddress(GAS_TOKEN_ID),
        assetIdToERC20ContractAddress(TOKEN_ID_1),
      ]);

    // mint some eth to bob
    await new Promise<void>((resolve, reject) => {
      api.tx.assets
        .mint(GAS_TOKEN_ID, bobSigner.address, ethInSub.toString())
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        })
        .catch((err) => reject(err));
    });

    const precompileSwapEthForTokensRes = await dexPrecompile
      .connect(bobSigner)
      .callStatic.swapExactETHForTokens(
        alphaAmountOutSub,
        [assetIdToERC20ContractAddress(GAS_TOKEN_ID), assetIdToERC20ContractAddress(TOKEN_ID_1)],
        bobSigner.address,
        20000,
        {
          value: ethInSub,
        },
      );
    tx = await dexPrecompile
      .connect(bobSigner)
      .swapExactETHForTokens(
        alphaAmountOutSub,
        [assetIdToERC20ContractAddress(GAS_TOKEN_ID), assetIdToERC20ContractAddress(TOKEN_ID_1)],
        bobSigner.address,
        20000,
        {
          value: ethInSub,
        },
      );
    await tx.wait();

    // TODO check events

    const subalphaBalanceAfter =
      ((await api.query.assets.account(TOKEN_ID_1, bobSigner.address)).toJSON() as any)?.balance ?? 0;
    const substrateEthBalanceAfter = await localJsonProvider.getBalance(bobSigner.address);

    // validate before and after balances for contract swaps and precompile swaps are equivalent
    expect(alphaBalanceBefore).to.eq(BigInt(subalphaBalanceBefore));
    expect(alphaBalanceAfter).to.eq(BigInt(subalphaBalanceAfter));
    // expect(ethBalanceBefore).to.eq(substrateEthBalanceBefore);
    // expect(ethBalanceAfter).to.eq(substrateEthBalanceAfter);

    expect(contractSwapEthForTokensRes[0]).to.eq(precompileSwapEthForTokensRes[0]);
    expect(contractSwapEthForTokensRes[1]).to.eq(precompileSwapEthForTokensRes[1]);
  });

  // dependent on 'addLiquidityETH' test
  // dependent on 'removeLiquidityETH' test
  it("swapTokensForExactETH", async () => {
    await alpha.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));

    const alphaBalanceBefore = await alpha.balanceOf(user.address);
    const ethBalanceBefore = await user.getBalance();

    // check amount of tokens retrievable
    const [alphaAmountIn] = await uniswapV2Router02
      .connect(user)
      .getAmountsIn(utils.parseEther("25"), [alpha.address, weth.address]);

    await alpha.mint(user.address, alphaAmountIn);

    let tx;

    const contractSwapTokensForEthRes = await uniswapV2Router02
      .connect(user)
      .callStatic.swapTokensForExactETH(
        utils.parseEther("25"),
        alphaAmountIn,
        [alpha.address, weth.address],
        user.address,
        ethers.constants.MaxUint256,
      );
    tx = await uniswapV2Router02
      .connect(user)
      .swapTokensForExactETH(
        utils.parseEther("25"),
        alphaAmountIn,
        [alpha.address, weth.address],
        user.address,
        ethers.constants.MaxUint256,
      );
    await tx.wait();

    // TODO check events

    // user token Alpha and Eth balances after swap
    const alphaBalanceAfter = await alpha.balanceOf(user.address);
    const ethBalanceAfter = await user.getBalance();

    // swap via precompile

    // get Alpha and Eth precompile balances before swap
    const subalphaBalanceBefore = ((await api.query.assets.account(TOKEN_ID_1, bobSigner.address)).toJSON() as any)?.balance ?? 0;
    const subEthBalanceBefore = await localJsonProvider.getBalance(bobSigner.address);

    // check amount of tokens retrievable
    const ethAmountOutSub = utils.parseEther("25"); // amount wanting to get out
    const [alphaInSub] = await dexPrecompile
      .connect(bobSigner)
      .getAmountsIn(ethAmountOutSub, [
        assetIdToERC20ContractAddress(TOKEN_ID_1),
        assetIdToERC20ContractAddress(GAS_TOKEN_ID),
      ]);
    expect(alphaAmountIn).to.eq(alphaInSub);

    // mint some tokens to bob
    await new Promise<void>((resolve, reject) => {
      api.tx.assets
        .mint(TOKEN_ID_1, bobSigner.address, alphaInSub.toString())
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        })
        .catch((err) => reject(err));
    });

    const precompileSwapTokensForEthRes = await dexPrecompile
      .connect(bobSigner)
      .callStatic.swapTokensForExactETH(
        ethAmountOutSub,
        alphaInSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_1), assetIdToERC20ContractAddress(GAS_TOKEN_ID)],
        bobSigner.address,
        20000,
      );
    tx = await dexPrecompile
      .connect(bobSigner)
      .swapTokensForExactETH(
        ethAmountOutSub,
        alphaInSub,
        [assetIdToERC20ContractAddress(TOKEN_ID_1), assetIdToERC20ContractAddress(GAS_TOKEN_ID)],
        bobSigner.address,
        20000,
      );
    await tx.wait();

    // TODO check events

    // verify user token Alpha and Eth balances
    const subalphaBalanceAfter = ((await api.query.assets.account(TOKEN_ID_1, bobSigner.address)).toJSON() as any)?.balance ?? 0;
    const subEthBalanceAfter = await localJsonProvider.getBalance(bobSigner.address);

    // validate before and after balances for contract swaps and precompile swaps are equivalent
    expect(alphaBalanceBefore).to.eq(BigInt(subalphaBalanceBefore));
    expect(alphaBalanceAfter).to.eq(BigInt(subalphaBalanceAfter));
    // expect(ethBalanceBefore).to.eq(subEthBalanceBefore);
    // expect(ethBalanceAfter).to.eq(subEthBalanceAfter);

    expect(contractSwapTokensForEthRes[0]).to.eq(precompileSwapTokensForEthRes[0]);
    expect(contractSwapTokensForEthRes[1]).to.eq(precompileSwapTokensForEthRes[1]);
  });

  // dependent on 'addLiquidityETH' test
  // dependent on 'removeLiquidityETH' test
  it("swapETHForExactTokens", async () => {
    await alpha.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));

    const alphaBalanceBefore = await alpha.balanceOf(user.address);
    const ethBalanceBefore = await user.getBalance();

    // check amount of tokens retrievable
    const [ethIn] = await uniswapV2Router02
      .connect(user)
      .getAmountsIn(utils.parseEther("25"), [weth.address, alpha.address]);

    let tx;

    const contractSwapEthForTokensRes = await uniswapV2Router02
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
    tx = await uniswapV2Router02
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
    await tx.wait();

    // TODO check events

    // user token Alpha and Eth balances after swap
    const alphaBalanceAfter = await alpha.balanceOf(user.address);
    const ethBalanceAfter = await user.getBalance();

    // swap via precompile

    // get Alpha and Eth precompile balances before swap
    const subalphaBalanceBefore = ((await api.query.assets.account(TOKEN_ID_1, bobSigner.address)).toJSON() as any)?.balance ?? 0;
    const subEthBalanceBefore = await localJsonProvider.getBalance(bobSigner.address);

    // check amount of tokens retrievable
    const alphaAmountOutSub = utils.parseEther("25");
    const [ethInSub] = await dexPrecompile
    .connect(bobSigner)
    .getAmountsIn(alphaAmountOutSub, [
      assetIdToERC20ContractAddress(GAS_TOKEN_ID),
      assetIdToERC20ContractAddress(TOKEN_ID_1),
    ]);
    expect(ethIn).to.eq(ethInSub);

    // mint some tokens to bob
    await new Promise<void>((resolve, reject) => {
      api.tx.assets
        .mint(GAS_TOKEN_ID, bobSigner.address, ethInSub.toString())
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        })
        .catch((err) => reject(err));
    });

    const precompileSwapEthForTokensRes = await dexPrecompile
      .connect(bobSigner)
      .callStatic.swapETHForExactTokens(
        alphaAmountOutSub,
        [assetIdToERC20ContractAddress(GAS_TOKEN_ID), assetIdToERC20ContractAddress(TOKEN_ID_1)],
        bobSigner.address,
        20000,
        {
          value: ethInSub,
        },
      );
    tx = await dexPrecompile
      .connect(bobSigner)
      .swapETHForExactTokens(
        alphaAmountOutSub,
        [assetIdToERC20ContractAddress(GAS_TOKEN_ID), assetIdToERC20ContractAddress(TOKEN_ID_1)],
        bobSigner.address,
        20000,
        {
          value: ethInSub,
        },
      );
    await tx.wait();

    // TODO check events

    // verify user token Alpha and Eth balances
    const subalphaBalanceAfter = ((await api.query.assets.account(TOKEN_ID_1, bobSigner.address)).toJSON() as any)?.balance ?? 0;
    const subEthBalanceAfter = await localJsonProvider.getBalance(bobSigner.address);
    
    // validate before and after balances for contract swaps and precompile swaps are equivalent
    expect(alphaBalanceBefore).to.eq(BigInt(subalphaBalanceBefore));
    expect(alphaBalanceAfter).to.eq(BigInt(subalphaBalanceAfter));
    // expect(ethBalanceBefore).to.eq(subEthBalanceBefore);
    // expect(ethBalanceAfter).to.eq(subEthBalanceAfter);

    expect(contractSwapEthForTokensRes[0]).to.eq(precompileSwapEthForTokensRes[0]);
    expect(contractSwapEthForTokensRes[1]).to.eq(precompileSwapEthForTokensRes[1]);
  });

  it("quote", async () => {
    const contractQuote = await uniswapV2Router02
      .connect(user)
      .quote(utils.parseEther("5"), utils.parseEther("200"), utils.parseEther("120"));

    const precompileQuote = await dexPrecompile
      .connect(bobSigner)
      .quote(utils.parseEther("5"), utils.parseEther("200"), utils.parseEther("120"));

    expect(contractQuote).to.eq(precompileQuote);
  });

  it("getAmountOut", async () => {
    const contractAmountOut = await uniswapV2Router02
      .connect(user)
      .getAmountOut(utils.parseEther("5"), utils.parseEther("200"), utils.parseEther("120"));

    const precompileAmountOut = await dexPrecompile
      .connect(bobSigner)
      .getAmountOut(utils.parseEther("5"), utils.parseEther("200"), utils.parseEther("120"));

    expect(contractAmountOut).to.eq(precompileAmountOut);
  });

  // dependent on 'addLiquidity' test
  // dependent on 'removeLiquidity' test
  it("getAmountsOut", async () => {
    const [, betaAmountOut] = await uniswapV2Router02
      .connect(user)
      .getAmountsOut(utils.parseEther("5"), [alpha.address, beta.address]);

    const [, subBetaAmountOut] = await dexPrecompile
      .connect(bobSigner)
      .getAmountsOut(utils.parseEther("5"), [
        assetIdToERC20ContractAddress(TOKEN_ID_1),
        assetIdToERC20ContractAddress(TOKEN_ID_2),
      ]);

    expect(betaAmountOut).to.eq(subBetaAmountOut);
  });

  // dependent on 'addLiquidity' test
  // dependent on 'removeLiquidity' test
  it("getAmountsIn", async () => {
    const [alphaAmountIn] = await uniswapV2Router02
      .connect(user)
      .getAmountsIn(utils.parseEther("5"), [alpha.address, beta.address]);

    const [subAlphaAmountIn] = await dexPrecompile
      .connect(bobSigner)
      .getAmountsIn(utils.parseEther("5"), [
        assetIdToERC20ContractAddress(TOKEN_ID_1),
        assetIdToERC20ContractAddress(TOKEN_ID_2),
      ]);

    expect(alphaAmountIn).to.eq(subAlphaAmountIn);
  });
});
