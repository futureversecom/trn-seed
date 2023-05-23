import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { BigNumber, Contract, Wallet, utils } from "ethers";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC20_ABI,
  FEE_PROXY_ABI,
  FEE_PROXY_ADDRESS,
  GAS_TOKEN_ID,
  NATIVE_TOKEN_ID,
  NodeProcess,
  assetIdToERC20ContractAddress,
  startNode,
  typedefs,
} from "../../common";

const FEE_TOKEN_ASSET_ID = 1124;

// Call an EVM transaction with fee preferences for an account that has zero native token balance,
// ensuring that the preferred asset with liquidity is spent instead
describe("Fee Preferences", function () {
  const EMPTY_ACCT_PRIVATE_KEY = "0xf8d74108dbe199c4a6e4ef457046db37c325ba3f709b14cabfa1885663e4c589";

  let node: NodeProcess;

  let provider: JsonRpcProvider;
  let bob: KeyringPair;
  let emptyAccount: KeyringPair;
  let emptyAccountSigner: Wallet;
  let xrpToken: Contract;
  let feeToken: Contract;

  before(async () => {
    node = await startNode();

    // Setup PolkadotJS rpc provider
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    const api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    const keyring = new Keyring({ type: "ethereum" });
    const alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
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
        0,
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

  it("Pays fees in non-native token via legacy type 0 tx object", async () => {
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
    const nonce = await emptyAccountSigner.getTransactionCount();

    const unsignedTx = {
      // legacy tx
      type: 0,
      from: emptyAccount.address,
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

    const nonce = await emptyAccountSigner.getTransactionCount();
    const unsignedTx = {
      // eip1559 tx
      type: 2,
      from: emptyAccount.address,
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
      xrpToken.balanceOf(emptyAccount.address),
      feeToken.balanceOf(emptyAccount.address),
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
      from: emptyAccount.address,
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
      xrpToken.balanceOf(emptyAccount.address),
      feeToken.balanceOf(emptyAccount.address),
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
      from: emptyAccount.address,
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
      from: emptyAccount.address,
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
      from: emptyAccount.address,
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
