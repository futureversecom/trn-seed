import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { Codec, IEventData } from "@polkadot/types/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { BigNumber, BigNumberish, Contract, Wallet, utils } from "ethers";
import { ethers } from "hardhat";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC20_ABI,
  FEE_PROXY_ABI,
  FEE_PROXY_ABI_DEPRECATED,
  FEE_PROXY_ADDRESS,
  FUTUREPASS_PRECOMPILE_ABI,
  FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
  FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
  GAS_TOKEN_ID,
  NodeProcess,
  XRP_PRECOMPILE_ADDRESS,
  assetIdToERC20ContractAddress,
  finalizeTx,
  rpcs,
  startNode,
  typedefs,
} from "../../common";
import { ERC20 } from "../../typechain-types";

const FEE_TOKEN_ASSET_ID = 1124;

// Call an EVM transaction with fee preferences for an account that has zero native token balance,
// ensuring that the preferred asset with liquidity is spent instead
describe("Fee Preferences", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let alith: KeyringPair;
  let bob: KeyringPair;
  let provider: JsonRpcProvider;
  let emptyAccountSigner: Wallet;
  let xrpERC20Precompile: Contract;
  let feeToken: Contract;

  before(async () => {
    node = await startNode();

    // Setup PolkadotJS rpc provider
    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.wsPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs, rpc: rpcs });
    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));

    // Setup JSON RPC provider
    provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
    emptyAccountSigner = Wallet.createRandom().connect(provider);
    xrpERC20Precompile = new Contract(XRP_PRECOMPILE_ADDRESS, ERC20_ABI, emptyAccountSigner);
    feeToken = new Contract(assetIdToERC20ContractAddress(FEE_TOKEN_ASSET_ID), ERC20_ABI, emptyAccountSigner) as ERC20;

    // add liquidity for XRP<->token
    const txes = [
      api.tx.assetsExt.createAsset("test", "TEST", 18, 1, alith.address),
      api.tx.assets.mint(FEE_TOKEN_ASSET_ID, alith.address, 2_000_000_000_000_000),
      api.tx.assets.mint(FEE_TOKEN_ASSET_ID, emptyAccountSigner.address, 2_000_000_000_000_000),
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
    await finalizeTx(alith, api.tx.utility.batch(txes));
  });

  after(async () => await node.stop());

  it("Legacy tx type 0 not supported", async () => {
    const fees = await provider.getFeeData();
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const unsignedTx = {
      type: 0,
      from: emptyAccountSigner.address,
      to: FEE_PROXY_ADDRESS,
      nonce: await emptyAccountSigner.getTransactionCount(),
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        feeToken.address,
        transferInput,
      ]),
      gasLimit: 50000,
      gasPrice: fees.lastBaseFeePerGas!,
    };
    const error = await emptyAccountSigner.sendTransaction(unsignedTx).catch((e) => e);
    expect(error.code).to.be.eq("SERVER_ERROR");
    expect(error.reason).to.be.eq("processing response error");
    expect(error.message).contains("unknown error");
  });

  it("Legacy tx type 1 not supported", async () => {
    const fees = await provider.getFeeData();
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const unsignedTx = {
      type: 1,
      from: emptyAccountSigner.address,
      to: FEE_PROXY_ADDRESS,
      nonce: await emptyAccountSigner.getTransactionCount(),
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        feeToken.address,
        transferInput,
      ]),
      gasLimit: 50000,
      gasPrice: fees.lastBaseFeePerGas!,
    };
    const error = await emptyAccountSigner.sendTransaction(unsignedTx).catch((e) => e);
    expect(error.code).to.be.eq("SERVER_ERROR");
    expect(error.reason).to.be.eq("processing response error");
    expect(error.message).contains("unknown error");
  });

  it("[DEPRECATED] Pays fees in non-native token - maxFeePerGas (MIN)", async () => {
    const fees = await provider.getFeeData();

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI_DEPRECATED, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const { tokenCost, gasOverrides } = await calcPaymentAmounts(provider, FEE_TOKEN_ASSET_ID, gasEstimate); // default to min payment
    const tx = await feeProxy
      .connect(emptyAccountSigner)
      .callWithFeePreferences(feeToken.address, 0, feeToken.address, transferInput, gasOverrides);
    const receipt = await tx.wait();

    // calculate refunded XRP amount - based on actual cost of TX
    const refundAmountXRP = calcRefundedXRP(gasOverrides, fees.lastBaseFeePerGas!, gasEstimate, receipt.gasUsed);

    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // verify XRP balance updated (payment/refund made in native token)
    expect(xrpBalanceUpdated.sub(xrpBalance)).to.equal(refundAmountXRP);
    expect(tokenBalance.sub(tokenBalanceUpdated)).to.equal(tokenCost + transferAmount);
  });

  it("[DEPRECATED] Pays fees in non-native token - maxFeePerGas (MAX)", async () => {
    const fees = await provider.getFeeData();

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI_DEPRECATED, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const { tokenCost, gasOverrides } = await calcPaymentAmounts(
      provider,
      FEE_TOKEN_ASSET_ID,
      gasEstimate,
      fees.maxFeePerGas!, // adding priority fee to maxFeePerGas
    );
    const tx = await feeProxy
      .connect(emptyAccountSigner)
      .callWithFeePreferences(feeToken.address, 0, feeToken.address, transferInput, gasOverrides);
    const receipt = await tx.wait();

    // calculate refunded XRP amount - based on actual cost of TX
    const refundAmountXRP = calcRefundedXRP(gasOverrides, fees.lastBaseFeePerGas!, gasEstimate, receipt.gasUsed);

    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // verify XRP balance updated (payment/refund made in native token)
    expect(xrpBalanceUpdated.sub(xrpBalance)).to.equal(refundAmountXRP);
    expect(tokenBalance.sub(tokenBalanceUpdated)).to.equal(tokenCost + transferAmount);
  });

  it("[DEPRECATED] Pays fees in non-native token - maxFeePerGas (CUSTOM)", async () => {
    const fees = await provider.getFeeData();

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI_DEPRECATED, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const { tokenCost, gasOverrides } = await calcPaymentAmounts(
      provider,
      FEE_TOKEN_ASSET_ID,
      gasEstimate,
      fees.lastBaseFeePerGas!.add(1), // adding priority fee to maxFeePerGas
    );

    const tx = await feeProxy
      .connect(emptyAccountSigner)
      .callWithFeePreferences(feeToken.address, 0, feeToken.address, transferInput, gasOverrides);
    const receipt = await tx.wait();

    // calculate refunded XRP amount - based on actual cost of TX
    const refundAmountXRP = calcRefundedXRP(gasOverrides, fees.lastBaseFeePerGas!, gasEstimate, receipt.gasUsed);

    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // verify XRP balance updated (payment/refund made in native token)
    expect(xrpBalanceUpdated.sub(xrpBalance)).to.equal(refundAmountXRP);
    expect(tokenBalance.sub(tokenBalanceUpdated)).to.equal(tokenCost + transferAmount);
  });

  it("Pays fees in non-native token - maxFeePerGas (MIN)", async () => {
    const fees = await provider.getFeeData();

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const { tokenCost, gasOverrides } = await calcPaymentAmounts(provider, FEE_TOKEN_ASSET_ID, gasEstimate); // default to min payment
    const tx = await feeProxy
      .connect(emptyAccountSigner)
      .callWithFeePreferences(feeToken.address, feeToken.address, transferInput, gasOverrides);
    const receipt = await tx.wait();

    // calculate refunded XRP amount - based on actual cost of TX
    const refundAmountXRP = calcRefundedXRP(gasOverrides, fees.lastBaseFeePerGas!, gasEstimate, receipt.gasUsed);

    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // verify XRP balance updated (payment/refund made in native token)
    expect(xrpBalanceUpdated.sub(xrpBalance)).to.equal(refundAmountXRP);
    expect(tokenBalance.sub(tokenBalanceUpdated)).to.equal(tokenCost + transferAmount);
  });

  it("Pays fees in non-native token - maxFeePerGas (MAX)", async () => {
    const fees = await provider.getFeeData();

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const { tokenCost, gasOverrides } = await calcPaymentAmounts(
      provider,
      FEE_TOKEN_ASSET_ID,
      gasEstimate,
      fees.maxFeePerGas!, // adding priority fee to maxFeePerGas
    );
    const tx = await feeProxy
      .connect(emptyAccountSigner)
      .callWithFeePreferences(feeToken.address, feeToken.address, transferInput, gasOverrides);
    const receipt = await tx.wait();

    // calculate refunded XRP amount - based on actual cost of TX
    const refundAmountXRP = calcRefundedXRP(gasOverrides, fees.lastBaseFeePerGas!, gasEstimate, receipt.gasUsed);

    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // verify XRP balance updated (payment/refund made in native token)
    expect(xrpBalanceUpdated.sub(xrpBalance)).to.equal(refundAmountXRP);
    expect(tokenBalance.sub(tokenBalanceUpdated)).to.equal(tokenCost + transferAmount);
  });

  it("Pays fees in non-native token - maxFeePerGas (CUSTOM)", async () => {
    const fees = await provider.getFeeData();

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const { tokenCost, gasOverrides } = await calcPaymentAmounts(
      provider,
      FEE_TOKEN_ASSET_ID,
      gasEstimate,
      fees.lastBaseFeePerGas!.add(1), // adding priority fee to maxFeePerGas
    );

    const tx = await feeProxy
      .connect(emptyAccountSigner)
      .callWithFeePreferences(feeToken.address, feeToken.address, transferInput, gasOverrides);
    const receipt = await tx.wait();

    // calculate refunded XRP amount - based on actual cost of TX
    const refundAmountXRP = calcRefundedXRP(gasOverrides, fees.lastBaseFeePerGas!, gasEstimate, receipt.gasUsed);

    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // verify XRP balance updated (payment/refund made in native token)
    expect(xrpBalanceUpdated.sub(xrpBalance)).to.equal(refundAmountXRP);
    expect(tokenBalance.sub(tokenBalanceUpdated)).to.equal(tokenCost + transferAmount);
  });

  it("Pays fees in non-native token - maxPriorityFeePerGas (CUSTOM)", async () => {
    const fees = await provider.getFeeData();

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const { tokenCost, gasOverrides } = await calcPaymentAmounts(
      provider,
      FEE_TOKEN_ASSET_ID,
      gasEstimate,
      fees.lastBaseFeePerGas!.add(500_000), // adding priority fee to maxFeePerGas
      BigNumber.from(500_000),
    );

    const tx = await feeProxy
      .connect(emptyAccountSigner)
      .callWithFeePreferences(feeToken.address, feeToken.address, transferInput, gasOverrides);
    const receipt = await tx.wait();

    // calculate refunded XRP amount - based on actual cost of TX
    const refundAmountXRP = calcRefundedXRP(gasOverrides, fees.lastBaseFeePerGas!, gasEstimate, receipt.gasUsed);

    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // verify XRP balance updated (payment/refund made in native token)
    expect(xrpBalanceUpdated.sub(xrpBalance)).to.equal(refundAmountXRP);
    expect(tokenBalance.sub(tokenBalanceUpdated)).to.equal(tokenCost + transferAmount);
  });

  it("Fails to pay fees in non-native token - token payment conversion exceeds maxFeePerGas", async () => {
    const fees = await provider.getFeeData();

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);

    const { gasOverrides } = await calcPaymentAmounts(
      provider,
      FEE_TOKEN_ASSET_ID,
      gasEstimate,
      fees.lastBaseFeePerGas!.add(500_000), // adding priority fee to maxFeePerGas
      BigNumber.from(600_000), // (base + priority fee) exceeds maxFeePerGas
    );

    const unsignedTx = {
      type: 2,
      from: emptyAccountSigner.address,
      to: FEE_PROXY_ADDRESS,
      nonce: await emptyAccountSigner.getTransactionCount(),
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        feeToken.address,
        transferInput,
      ]),
      ...gasOverrides,
    };
    const error = await emptyAccountSigner.sendTransaction(unsignedTx).catch((e) => e);
    expect(error.code).to.be.eq("INSUFFICIENT_FUNDS");
    expect(error.reason).to.be.eq("insufficient funds for intrinsic transaction cost");
  });

  it("Fails to pay fees in non-native token if user does not have non-native token balance", async () => {
    // this is a new account which has no token balance
    const newAccount = Wallet.createRandom().connect(provider);

    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, newAccount);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const { gasOverrides } = await calcPaymentAmounts(provider, FEE_TOKEN_ASSET_ID, gasEstimate); // default to min payment
    const unsignedTx = {
      type: 2,
      from: newAccount.address,
      to: FEE_PROXY_ADDRESS,
      nonce: await newAccount.getTransactionCount(),
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        feeToken.address,
        transferInput,
      ]),
      ...gasOverrides,
    };

    const error = await newAccount.sendTransaction(unsignedTx).catch((e) => e);
    expect(error.code).to.be.eq("INSUFFICIENT_FUNDS");
    expect(error.reason).to.be.eq("insufficient funds for intrinsic transaction cost");
  });

  it("Fails to pay fees in non-native token with gasLimit 0", async () => {
    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const unsignedTx = {
      // eip1559 tx
      type: 2,
      from: emptyAccountSigner.address,
      to: FEE_PROXY_ADDRESS,
      nonce: await emptyAccountSigner.getTransactionCount(),
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        feeToken.address,
        transferInput,
      ]),
      gasLimit: 0,
      maxFeePerGas: 0,
      maxPriorityFeePerGas: 0,
    };

    const error = await emptyAccountSigner.sendTransaction(unsignedTx).catch((e) => e);
    expect(error.code).to.be.eq("INSUFFICIENT_FUNDS");
    expect(error.reason).to.be.eq("insufficient funds for intrinsic transaction cost");
  });

  it("Futurepass caller pays fees in non-native token", async () => {
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
    await finalizeTx(
      alith,
      api.tx.utility.batch([
        api.tx.assets.mint(FEE_TOKEN_ASSET_ID, owner.address, 2_000_000_000),
        api.tx.assets.mint(FEE_TOKEN_ASSET_ID, futurepass.address, 1),
      ]),
    );

    // get token balances
    const [xrpBalance, tokenBalance, fpXRPBalance, fpTokenBalance] = await Promise.all([
      xrpERC20Precompile.balanceOf(owner.address),
      feeToken.balanceOf(owner.address),
      xrpERC20Precompile.balanceOf(futurepass.address),
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

    const { tokenCost, gasOverrides } = await calcPaymentAmounts(provider, FEE_TOKEN_ASSET_ID, gasEstimate); // default to min payment

    tx = await feeProxy
      .connect(owner)
      .callWithFeePreferences(feeToken.address, futurepass.address, proxyCallInput, gasOverrides);
    receipt = await tx.wait();
    expect((receipt?.events as any).length).to.equal(2);
    expect((receipt?.events as any)[0].address).to.equal(feeToken.address); // transfer event
    expect((receipt?.events as any)[1].address).to.equal(futurepass.address); // futurepass executed event

    // calculate refunded XRP amount - based on actual cost of TX
    const refundAmountXRP = calcRefundedXRP(gasOverrides, fees.lastBaseFeePerGas!, gasEstimate, receipt.gasUsed);

    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated, fpXRPBalanceUpdated, fpTokenBalanceUpdated] = await Promise.all([
      xrpERC20Precompile.balanceOf(owner.address),
      feeToken.balanceOf(owner.address),
      xrpERC20Precompile.balanceOf(futurepass.address),
      feeToken.balanceOf(futurepass.address),
    ]);

    // verify XRP balance refunds
    expect(xrpBalanceUpdated).to.equal(refundAmountXRP);
    // verify fee token was paid for fees
    expect(tokenBalanceUpdated).to.be.lessThan(tokenBalance);
    // verify FP balance has not changed (payment made in non-native token)
    expect(fpXRPBalanceUpdated).to.equal(0);
    // verify minted token was transferred to bob
    expect(fpTokenBalanceUpdated).to.equal(0);

    expect(tokenBalance.sub(tokenBalanceUpdated)).to.equal(tokenCost);
  });

  it("Pays fees in non-native token via eip1559 type 2 tx object", async () => {
    const fees = await provider.getFeeData();

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const gasEstimate = await feeToken.estimateGas.transfer(bob.address, transferAmount);
    const { tokenCost, gasOverrides } = await calcPaymentAmounts(provider, FEE_TOKEN_ASSET_ID, gasEstimate); // default to min payment

    const nonce = await emptyAccountSigner.getTransactionCount();
    const unsignedTx = {
      // eip1559 tx
      type: 2,
      from: emptyAccountSigner.address,
      to: FEE_PROXY_ADDRESS,
      nonce,
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        feeToken.address,
        transferInput,
      ]),
      gasLimit: gasOverrides.gasLimit,
      maxFeePerGas: gasOverrides.maxFeePerGas,
      maxPriorityFeePerGas: gasOverrides.maxPriorityFeePerGas,
    };

    await emptyAccountSigner.signTransaction(unsignedTx);
    const tx = await emptyAccountSigner.sendTransaction(unsignedTx);
    const receipt = await tx.wait();

    const refundAmountXRP = calcRefundedXRP(gasOverrides, fees.lastBaseFeePerGas!, gasEstimate, receipt.gasUsed);

    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpERC20Precompile.balanceOf(emptyAccountSigner.address),
      feeToken.balanceOf(emptyAccountSigner.address),
    ]);
    // verify XRP balance has not changed (payment made in non-native token)
    expect(xrpBalanceUpdated.sub(xrpBalance)).to.equal(refundAmountXRP);
    expect(tokenBalance.sub(tokenBalanceUpdated)).to.equal(tokenCost + transferAmount);
  });

  it("Futurepass account pays fees in non-native token - using extrinsic", async () => {
    // create futurepass for random user
    const user = Wallet.createRandom().connect(provider);
    const userKeyring = new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(user.privateKey));
    await finalizeTx(alith, api.tx.futurepass.create(user.address));
    const futurepassAddress = (await api.query.futurepass.holders(user.address)).toString();

    // mint fee tokens to futurepass
    await finalizeTx(alith, api.tx.assets.mint(FEE_TOKEN_ASSET_ID, futurepassAddress, 2_000_000_000_000));

    const eoaXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const eoaTokenBalanceBefore =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const fpXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
    const fpTokenBalanceBefore =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;

    // console.table({ eoaXRPBalanceBefore, eoaTokenBalanceBefore, fpXRPBalanceBefore, fpTokenBalanceBefore });

    const innerCall = api.tx.system.remark("sup");
    const proxyExtrinsic = api.tx.futurepass.proxyExtrinsic(futurepassAddress, innerCall);
    const feeproxiedCall = api.tx.feeProxy.callWithFeePreferences(FEE_TOKEN_ASSET_ID, 1000000, proxyExtrinsic);
    await finalizeTx(userKeyring, feeproxiedCall);

    const eoaXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const eoaTokenBalanceAfter =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const fpXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
    const fpTokenBalanceAfter =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;

    // futurepass should only fee lose tokens
    expect(eoaXRPBalanceBefore).to.be.eq(eoaXRPBalanceAfter);
    expect(eoaTokenBalanceBefore).to.be.eq(eoaTokenBalanceAfter);
    expect(fpXRPBalanceBefore).to.be.eq(fpXRPBalanceAfter);
    expect(fpTokenBalanceAfter).to.be.lessThan(fpTokenBalanceBefore);
  });

  it("Futurepass account pays fees in non-native token for an evm call using proxy_extrinsic", async () => {
    // create futurepass for random user
    const user = Wallet.createRandom().connect(provider);
    const userKeyring = new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(user.privateKey));
    await finalizeTx(alith, api.tx.futurepass.create(user.address));
    const futurepassAddress = (await api.query.futurepass.holders(user.address)).toString();

    // mint fee tokens to futurepass
    await finalizeTx(alith, api.tx.assets.mint(FEE_TOKEN_ASSET_ID, futurepassAddress, 2_000_000_000_000));

    const eoaXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const eoaTokenBalanceBefore =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const fpXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
    const fpTokenBalanceBefore =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const erc20PrecompileAddress = assetIdToERC20ContractAddress(FEE_TOKEN_ASSET_ID);
    const sender = futurepassAddress;
    const value = 0; //eth
    const gasLimit = 22953;
    const maxFeePerGas = "15000000000000";
    const maxPriorityFeePerGas = null;
    const nonce = null;
    const accessList = null;
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);
    const evmCall = api.tx.evm.call(
      sender,
      erc20PrecompileAddress,
      transferInput,
      value,
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      nonce,
      accessList,
    );

    // record bob's token balance
    const bobTokenBalanceBefore =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, bob.address)).toJSON() as any)?.balance ?? 0;

    const evmCallGasEstimate = await evmCall.paymentInfo(sender);
    const evmCallGasEstimateinXRP = evmCallGasEstimate.partialFee;

    const proxyExtrinsic = api.tx.futurepass.proxyExtrinsic(futurepassAddress, evmCall);

    // Find estimate cost for feeProxy call
    const extrinsicInfo = await api.tx.feeProxy
      .callWithFeePreferences(
        FEE_TOKEN_ASSET_ID,
        utils.parseEther("1").toString(), // 10e18
        proxyExtrinsic,
      )
      .paymentInfo(sender);
    const feeProxyGasEstimateinXRP = extrinsicInfo.partialFee;

    // cost for fee proxy with proxy_extrinsic + cost for evm call
    const estimatedTotalGasCost = evmCallGasEstimateinXRP.toNumber() + feeProxyGasEstimateinXRP.toNumber();

    // convert estimatedTotalGasCost to FEE_TOKEN_ASSET_ID amount
    const {
      Ok: [estimatedTokenTxCost],
    } = await (api.rpc as any).dex.getAmountsIn(estimatedTotalGasCost, [FEE_TOKEN_ASSET_ID, GAS_TOKEN_ID]);

    // Now call the callWithFeePreferences with sufficient max_payment of estimatedTokenTxCost
    const feeproxiedCall = api.tx.feeProxy.callWithFeePreferences(
      FEE_TOKEN_ASSET_ID,
      estimatedTokenTxCost,
      proxyExtrinsic,
    );
    await finalizeTx(userKeyring, feeproxiedCall);

    const eoaXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const eoaTokenBalanceAfter =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const fpXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
    const fpTokenBalanceAfter =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
    const bobTokenBalanceAfter =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, bob.address)).toJSON() as any)?.balance ?? 0;

    // eoa token and XRP balance should remain untouched.
    expect(eoaXRPBalanceBefore).to.be.eq(eoaXRPBalanceAfter);
    expect(eoaTokenBalanceBefore).to.be.eq(eoaTokenBalanceAfter);
    // futurepass should pay for the fees and transfer amount in tokens
    expect(fpTokenBalanceAfter).to.be.lt(fpTokenBalanceBefore);
    // futurepass might have remaining XRP from the dex swap. this is due to the evm call's input gasLimit and the actual gas used having differences.
    expect(fpXRPBalanceAfter).to.be.gte(fpXRPBalanceBefore);
    // check bob received 1 token
    expect(bobTokenBalanceAfter).to.be.eq(bobTokenBalanceBefore + 1);
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

async function calcPaymentAmounts(
  provider: JsonRpcProvider,
  feeTokenAssetId: number,
  estimate: BigNumber,
  maxFeePerGas?: BigNumber,
  maxPriorityFeePerGas?: BigNumber, // only taken into account if maxFeePerGas is not provided
): Promise<{ tokenCost: number; xrpCost: number; gasOverrides: GasOverrides }> {
  let maxFeePerGasDerived = maxFeePerGas!;
  const maxPriorityFeePerGasDerived = maxPriorityFeePerGas ?? BigNumber.from(0);

  // calc minimum acceptable maxFeePerGas = baseFee + priorityFee
  if (!maxFeePerGas) {
    const fees = await provider.getFeeData();
    maxFeePerGasDerived = fees.lastBaseFeePerGas!.add(maxPriorityFeePerGasDerived); // default to cheapest payable
  }

  // calc tx cost in XRP; account for precision loss in division - always round up (node behaviour)
  const value = estimate.mul(maxFeePerGasDerived);
  const remainder = value.mod(10 ** 12);
  const txCostXRP = value
    .div(10 ** 12)
    .add(remainder.gt(0) ? 1 : 0)
    .toNumber();

  const result = await provider.send("dex_getAmountsIn", [txCostXRP, [feeTokenAssetId, GAS_TOKEN_ID]]);
  const gasOverrides = {
    gasLimit: estimate,
    maxFeePerGas: maxFeePerGasDerived,
    maxPriorityFeePerGas: maxPriorityFeePerGasDerived,
  };
  return { tokenCost: result.Ok![0], xrpCost: txCostXRP, gasOverrides };
}

function calcRefundedXRP(
  gasOverrides: GasOverrides,
  lastBaseFeePerGas: BigNumber,
  estimatedGas: BigNumberish,
  actualGasUsed: BigNumberish,
): number {
  const actualCostEth = gasOverrides.maxPriorityFeePerGas.add(lastBaseFeePerGas!).mul(actualGasUsed);
  const totalCostEthPaid = gasOverrides.maxFeePerGas.mul(estimatedGas);
  const refundAmountEth = totalCostEthPaid.sub(actualCostEth);
  const remainder = refundAmountEth.mod(10 ** 12);
  const refundAmountXRP = refundAmountEth
    .div(10 ** 12)
    .add(remainder.gt(0) ? 1 : 0)
    .toNumber();
  return refundAmountXRP;
}

export interface GasOverrides {
  gasLimit: BigNumber;
  maxFeePerGas: BigNumber;
  maxPriorityFeePerGas: BigNumber;
}
