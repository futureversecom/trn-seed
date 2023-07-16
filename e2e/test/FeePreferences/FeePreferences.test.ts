import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { Codec, IEventData } from "@polkadot/types/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { BigNumber, Contract, Wallet, utils } from "ethers";
import { ethers } from "hardhat";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC20_ABI,
  FEE_PROXY_ABI,
  FEE_PROXY_ADDRESS,
  FUTUREPASS_PRECOMPILE_ABI,
  FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
  FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
  GAS_TOKEN_ID,
  NATIVE_TOKEN_ID,
  NodeProcess,
  assetIdToERC20ContractAddress,
  rpcs,
  startNode,
  typedefs,
} from "../../common";

const FEE_TOKEN_ASSET_ID = 1124;

// Call an EVM transaction with fee preferences for an account that has zero native token balance,
// ensuring that the preferred asset with liquidity is spent instead
describe("Fee Preferences", function () {
  const EMPTY_ACCT_PRIVATE_KEY = "0xf8d74108dbe199c4a6e4ef457046db37c325ba3f709b14cabfa1885663e4c589";

  let node: NodeProcess;

  let api: ApiPromise;
  let provider: JsonRpcProvider;
  let alith: KeyringPair;
  let bob: KeyringPair;
  let emptyAccount: KeyringPair;
  let emptyAccountSigner: Wallet;
  let xrpToken: Contract;
  let feeToken: Contract;

  before(async () => {
    node = await startNode();

    // Setup PolkadotJS rpc provider
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs, rpc: rpcs });
    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));
    emptyAccount = keyring.addFromSeed(hexToU8a(EMPTY_ACCT_PRIVATE_KEY));

    // Empty with regards to native balance only
    const emptyAcct = keyring.addFromSeed(hexToU8a(EMPTY_ACCT_PRIVATE_KEY));

    // add liquidity for XRP<->token
    const txes = [
      api.tx.assetsExt.createAsset("test", "TEST", 18, 1, alith.address),
      api.tx.assets.mint(FEE_TOKEN_ASSET_ID, alith.address, 2_000_000_000_000_000),
      api.tx.assets.mint(FEE_TOKEN_ASSET_ID, emptyAcct.address, 2_000_000_000_000_000),
      api.tx.dex.addLiquidity(
        FEE_TOKEN_ASSET_ID,
        GAS_TOKEN_ID,
        100_000_000_000,
        100_000_000_000,
        100_000_000_000,
        100_000_000_000,
        null,
        null,
      ),
    ];
    await new Promise<void>((resolve) => {
      api.tx.utility.batch(txes).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) {
          console.log(`setup block hash: ${status.asInBlock}`);
          resolve();
        }
      });
    });

    // Setup JSON RPC provider
    provider = new JsonRpcProvider(`http://localhost:${node.httpPort}`);
    emptyAccountSigner = new Wallet(EMPTY_ACCT_PRIVATE_KEY).connect(provider); // 'development' seed
    xrpToken = new Contract(assetIdToERC20ContractAddress(NATIVE_TOKEN_ID), ERC20_ABI, emptyAccountSigner);
    feeToken = new Contract(assetIdToERC20ContractAddress(FEE_TOKEN_ASSET_ID), ERC20_ABI, emptyAccountSigner);
  });

  after(async () => await node.stop());

  it("Pays fees in non-native token - legacy tx", async () => {
    const fees = await provider.getFeeData();

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpToken.balanceOf(emptyAccount.address),
      feeToken.balanceOf(emptyAccount.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const estimatedTokenTxCost = await getAmountIn(provider, gasEstimate, FEE_TOKEN_ASSET_ID);
    const tx = await feeProxy
      .connect(emptyAccountSigner)
      .callWithFeePreferences(feeToken.address, estimatedTokenTxCost, feeToken.address, transferInput, {
        gasLimit: gasEstimate,
        gasPrice: fees.gasPrice!,
      });
    await tx.wait();

    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpToken.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);
    // verify XRP balance has not changed (payment made in non-native token)
    expect(xrpBalanceUpdated.sub(xrpBalance).toString()).to.equal("0");
    // verify fee token was paid for fees
    expect(tokenBalanceUpdated).to.be.lessThan(tokenBalance);

    expect(tokenBalance.sub(tokenBalanceUpdated)).to.equal(estimatedTokenTxCost + transferAmount);
  });

  it("Pays fees in non-native token - eip1559 tx", async () => {
    const fees = await provider.getFeeData();

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpToken.balanceOf(emptyAccount.address),
      feeToken.balanceOf(emptyAccount.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const estimatedTokenTxCost = await getAmountIn(provider, gasEstimate, FEE_TOKEN_ASSET_ID);
    const tx = await feeProxy
      .connect(emptyAccountSigner)
      .callWithFeePreferences(feeToken.address, estimatedTokenTxCost, feeToken.address, transferInput, {
        gasLimit: gasEstimate,
        maxFeePerGas: fees.lastBaseFeePerGas!,
        maxPriorityFeePerGas: 0,
      });
    await tx.wait();

    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpToken.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);
    // verify XRP balance has not changed (payment made in non-native token)
    expect(xrpBalanceUpdated.sub(xrpBalance).toString()).to.equal("0");
    // verify fee token was paid for fees
    expect(tokenBalanceUpdated).to.be.lessThan(tokenBalance);

    expect(tokenBalance.sub(tokenBalanceUpdated)).to.equal(estimatedTokenTxCost + transferAmount);
  });

  it("Futurepass caller pays fees in non-native token - legacy tx", async () => {
    const fees = await provider.getFeeData();

    // create futurepass
    const alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider);
    const futurepassRegistrar = new Contract(
      FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
      FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
      alithSigner,
    );
    const owner = Wallet.createRandom().connect(provider);
    let tx = await futurepassRegistrar.connect(alithSigner).create(owner.address);
    let receipt = await tx.wait();
    const futurepassAddress: string = (receipt?.events as any)[0].args.futurepass;
    const futurepass = new Contract(futurepassAddress, FUTUREPASS_PRECOMPILE_ABI, owner);

    // mint fee tokens to owner (pay for fees) & FP (transfer tokens to bob)
    await new Promise<void>((resolve) => {
      api.tx.utility
        .batch([
          api.tx.assets.mint(FEE_TOKEN_ASSET_ID, owner.address, 2_000_000_000),
          api.tx.assets.mint(FEE_TOKEN_ASSET_ID, futurepass.address, 1),
        ])
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });

    // get token balances
    const [xrpBalance, tokenBalance, fpXRPBalance, fpTokenBalance] = await Promise.all([
      xrpToken.balanceOf(owner.address),
      feeToken.balanceOf(owner.address),
      xrpToken.balanceOf(futurepass.address),
      feeToken.balanceOf(futurepass.address),
    ]);
    expect(xrpBalance).to.equal(0);
    expect(tokenBalance).to.equal(2_000_000_000);
    expect(fpXRPBalance).to.equal(0);
    expect(fpTokenBalance).to.equal(1);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferCallData = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);

    // estimate gas for futurepass proxy call - which encodes transfer call data
    const callTxType = 1;
    const proxyCallInput = futurepass.interface.encodeFunctionData("proxyCall", [
      callTxType,
      feeToken.address,
      ethers.constants.Zero,
      transferCallData,
    ]);
    const gasEstimate = await futurepass
      .connect(owner)
      .estimateGas.proxyCall(callTxType, feeToken.address, ethers.constants.Zero, transferCallData);

    const estimatedTokenTxCost = await getAmountIn(provider, gasEstimate, FEE_TOKEN_ASSET_ID);
    tx = await feeProxy
      .connect(owner)
      .callWithFeePreferences(feeToken.address, estimatedTokenTxCost, futurepass.address, proxyCallInput, {
        gasLimit: gasEstimate,
        gasPrice: fees.gasPrice!,
      });
    receipt = await tx.wait();
    expect((receipt?.events as any).length).to.equal(2);
    expect((receipt?.events as any)[0].address).to.equal(feeToken.address); // transfer event
    expect((receipt?.events as any)[1].address).to.equal(futurepass.address); // futurepass executed event

    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated, fpXRPBalanceUpdated, fpTokenBalanceUpdated] = await Promise.all([
      xrpToken.balanceOf(owner.address),
      feeToken.balanceOf(owner.address),
      xrpToken.balanceOf(futurepass.address),
      feeToken.balanceOf(futurepass.address),
    ]);
    // verify XRP balance has not changed (payment made in non-native token)
    expect(xrpBalanceUpdated).to.equal(0);
    // verify fee token was paid for fees
    expect(tokenBalanceUpdated).to.be.lessThan(tokenBalance);
    // verify FP balance has not changed (payment made in non-native token)
    expect(fpXRPBalanceUpdated).to.equal(0);
    // verify minted token was transferred to bob
    expect(fpTokenBalanceUpdated).to.equal(0);

    expect(tokenBalance.sub(tokenBalanceUpdated)).to.equal(estimatedTokenTxCost);
  });

  it("Pays fees in non-native token via legacy type 0 tx object", async () => {
    const fees = await provider.getFeeData();

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpToken.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const estimatedTokenTxCost = await getAmountIn(provider, gasEstimate, FEE_TOKEN_ASSET_ID);
    const nonce = await emptyAccountSigner.getTransactionCount();

    const unsignedTx = {
      // legacy tx
      type: 0,
      from: emptyAccountSigner.address,
      to: FEE_PROXY_ADDRESS,
      nonce,
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        estimatedTokenTxCost,
        feeToken.address,
        transferInput,
      ]),
      gasLimit: gasEstimate,
      gasPrice: fees.gasPrice!,
    };

    await emptyAccountSigner.signTransaction(unsignedTx);
    const tx = await emptyAccountSigner.sendTransaction(unsignedTx);
    await tx.wait();
    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpToken.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);
    // verify XRP balance has not changed (payment made in non-native token)
    expect(xrpBalanceUpdated.sub(xrpBalance).toString()).to.equal("0");
    // Verify fee token was paid for fees
    expect(tokenBalanceUpdated).to.be.lessThan(tokenBalance);

    expect(tokenBalance.sub(tokenBalanceUpdated)).to.equal(estimatedTokenTxCost + transferAmount);
  });

  it("Pays fees in non-native token via eip1559 type 2 tx object", async () => {
    const fees = await provider.getFeeData();

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpToken.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const estimatedTokenTxCost = await getAmountIn(provider, gasEstimate, FEE_TOKEN_ASSET_ID);

    const nonce = await emptyAccountSigner.getTransactionCount();
    const unsignedTx = {
      // eip1559 tx
      type: 2,
      from: emptyAccountSigner.address,
      to: FEE_PROXY_ADDRESS,
      nonce,
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        estimatedTokenTxCost,
        feeToken.address,
        transferInput,
      ]),
      gasLimit: gasEstimate,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    };

    await emptyAccountSigner.signTransaction(unsignedTx);
    const tx = await emptyAccountSigner.sendTransaction(unsignedTx);
    await tx.wait();
    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpToken.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);
    // verify XRP balance has not changed (payment made in non-native token)
    expect(xrpBalanceUpdated.sub(xrpBalance).toString()).to.equal("0");
    // Verify fee token was burned for fees
    expect(tokenBalanceUpdated).to.be.lessThan(tokenBalance);

    expect(tokenBalance.sub(tokenBalanceUpdated)).to.equal(estimatedTokenTxCost + transferAmount);
  });

  it("Does not pay in non-native token if max fee payment is insufficient - for legacy type 0 tx", async () => {
    const fees = await provider.getFeeData();

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpToken.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 1; // <-- insufficient payment
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const nonce = await emptyAccountSigner.getTransactionCount();
    const unsignedTx = {
      // legacy tx
      type: 0,
      from: emptyAccountSigner.address,
      to: FEE_PROXY_ADDRESS,
      nonce,
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        maxFeePaymentInToken,
        feeToken.address,
        transferInput,
      ]),
      gasLimit: gasEstimate,
      gasPrice: fees.gasPrice!,
    };

    const error = await emptyAccountSigner.sendTransaction(unsignedTx).catch((e) => e);
    expect(error.code).to.be.eq("INSUFFICIENT_FUNDS");
    expect(error.reason).to.be.eq("insufficient funds for intrinsic transaction cost");

    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpToken.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // verify XRP balance has not changed (payment made in non-native token)
    expect(xrpBalanceUpdated).to.equal(xrpBalance);
    expect(tokenBalanceUpdated).to.equal(tokenBalance);
  });

  it("Does not pay in non-native token if max fee payment is insufficient - for eip1559 type 2 tx", async () => {
    const fees = await provider.getFeeData();

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpToken.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 1; // <-- insufficient payment
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const nonce = await emptyAccountSigner.getTransactionCount();
    const unsignedTx = {
      // eip1559 tx
      type: 2,
      from: emptyAccountSigner.address,
      to: FEE_PROXY_ADDRESS,
      nonce,
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        maxFeePaymentInToken,
        feeToken.address,
        transferInput,
      ]),
      gasLimit: gasEstimate,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    };
    const error = await emptyAccountSigner.sendTransaction(unsignedTx).catch((e) => e);
    expect(error.code).to.be.eq("INSUFFICIENT_FUNDS");
    expect(error.reason).to.be.eq("insufficient funds for intrinsic transaction cost");

    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpToken.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // verify XRP balance has not changed (payment made in non-native token)
    expect(xrpBalanceUpdated).to.equal(xrpBalance);
    expect(tokenBalanceUpdated).to.equal(tokenBalance);
  });

  it("Does not pay fees in non-native token with gasLimit 0 - legacy tx", async () => {
    const fees = await provider.getFeeData();

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 10_000_000_000;
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const nonce = await emptyAccountSigner.getTransactionCount();
    const unsignedTx = {
      // legacy tx
      from: emptyAccountSigner.address,
      to: FEE_PROXY_ADDRESS,
      nonce,
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        maxFeePaymentInToken,
        feeToken.address,
        transferInput,
      ]),
      gasLimit: 0,
      gasPrice: fees.gasPrice!,
    };

    const error = await emptyAccountSigner.sendTransaction(unsignedTx).catch((e) => e);
    // See expected behavior for gasLimit === 0 https://github.com/futureversecom/frontier/blob/polkadot-v0.9.27-TRN/ts-tests/tests/test-transaction-cost.ts
    expect(error.code).to.be.eq("SERVER_ERROR");
    const body = JSON.parse(error.body);
    expect(body.error.message).to.be.eq(
      "submit transaction to pool failed: InvalidTransaction(InvalidTransaction::Custom(3))",
    );
  });

  it("Does not pay in non-native token with gasLimit 0 - eip1559 tx", async () => {
    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 10_000_000_000;
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const nonce = await emptyAccountSigner.getTransactionCount();
    const unsignedTx = {
      // eip1559 tx
      type: 2,
      from: emptyAccountSigner.address,
      to: FEE_PROXY_ADDRESS,
      nonce,
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        maxFeePaymentInToken,
        feeToken.address,
        transferInput,
      ]),
      gasLimit: 0,
      maxFeePerGas: 0,
      maxPriorityFeePerGas: 0,
    };

    const error = await emptyAccountSigner.sendTransaction(unsignedTx).catch((e) => e);
    // See expected behavior for gasLimit === 0 https://github.com/futureversecom/frontier/blob/polkadot-v0.9.27-TRN/ts-tests/tests/test-transaction-cost.ts
    expect(error.code).to.be.eq("SERVER_ERROR");
    const body = JSON.parse(error.body);
    expect(body.error.message).to.be.eq(
      "submit transaction to pool failed: InvalidTransaction(InvalidTransaction::Custom(3))",
    );
    expect(error.reason).to.be.eq("processing response error");
  });

  it("Does not pay in non-native token if user does not have non-native token balance", async () => {
    const fees = await provider.getFeeData();

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const newAccount = Wallet.createRandom().connect(provider);

    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, newAccount);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const estimatedTokenTxCost = await getAmountIn(provider, gasEstimate, FEE_TOKEN_ASSET_ID);
    const nonce = await newAccount.getTransactionCount();
    const unsignedTx = {
      type: 0,
      from: newAccount.address,
      to: FEE_PROXY_ADDRESS,
      nonce,
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        estimatedTokenTxCost,
        feeToken.address,
        transferInput,
      ]),
      gasLimit: gasEstimate,
      gasPrice: fees.gasPrice!,
    };

    const error = await newAccount.sendTransaction(unsignedTx).catch((e) => e);
    expect(error.code).to.be.eq("INSUFFICIENT_FUNDS");
    expect(error.reason).to.be.eq("insufficient funds for intrinsic transaction cost");
  });

  it("Pays fees in non-native token with extrinsic - check maxPayment works fine", async () => {
    const erc20PrecompileAddress = assetIdToERC20ContractAddress(FEE_TOKEN_ASSET_ID);
    const sender = alith.address;
    const value = 0; //eth
    const gasLimit = 22953;
    const maxFeePerGas = "15000000000000";
    const maxPriorityFeePerGas = null;
    const nonce = null;
    const accessList = null;
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const encodedInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const evmCall = api.tx.evm.call(
      sender,
      erc20PrecompileAddress,
      encodedInput,
      value,
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      nonce,
      accessList,
    );

    // Find estimate cost for evm call
    const evmCallGasEstimate = await evmCall.paymentInfo(sender);
    const evmCallGasEstimateinXRP = evmCallGasEstimate.partialFee;

    // Find estimate cost for feeProxy call
    const extrinsicInfo = await api.tx.feeProxy
      .callWithFeePreferences(
        FEE_TOKEN_ASSET_ID,
        utils.parseEther("1").toString(), // 10e18
        api.createType("Call", evmCall).toHex(),
      )
      .paymentInfo(sender);
    const feeProxyGasEstimateinXRP = extrinsicInfo.partialFee;

    // cost for evm call + cost for fee proxy
    const estimatedTotalGasCost = evmCallGasEstimateinXRP.toNumber() + feeProxyGasEstimateinXRP.toNumber();

    const {
      Ok: [estimatedTokenTxCost],
    } = await (api.rpc as any).dex.getAmountsIn(estimatedTotalGasCost, [FEE_TOKEN_ASSET_ID, GAS_TOKEN_ID]);

    const eventData = await new Promise<Codec[] & IEventData>((resolve, reject) => {
      api.tx.feeProxy
        .callWithFeePreferences(
          FEE_TOKEN_ASSET_ID,
          estimatedTokenTxCost.toString(),
          api.createType("Call", evmCall).toHex(),
        )
        .signAndSend(alith, ({ events, status }) => {
          if (status.isInBlock) {
            for (const { event } of events) {
              if (event.section === "feeProxy" && event.method === "CallWithFeePreferences") {
                resolve(event.data);
              }
            }
            reject(null);
          }
        });
    });
    expect(eventData).to.exist;
    const [from, paymentAsset, maxPayment] = eventData;
    expect(paymentAsset.toString()).to.equal(FEE_TOKEN_ASSET_ID.toString());
    expect(from.toString()).to.equal(alith.address.toString());
    expect(maxPayment.toString()).to.equal(estimatedTokenTxCost.toString());
  });
});

async function getAmountIn(provider: JsonRpcProvider, estimate: BigNumber, feeTokenAssetId: number): Promise<number> {
  const fees = await provider.getFeeData();
  const txCostXRP = estimate
    .mul(fees.gasPrice!)
    .div(10 ** 12)
    .toNumber();
  const result = await provider.send("dex_getAmountsIn", [txCostXRP, [feeTokenAssetId, GAS_TOKEN_ID]]);
  return result.Ok![0];
}
