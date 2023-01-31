import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Contract, Wallet, utils } from "ethers";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC20_ABI,
  FEE_PROXY_ABI,
  FEE_PROXY_ADDRESS,
  NATIVE_TOKEN_ID,
  NodeProcess,
  assetIdToERC20ContractAddress,
  startNode,
  typedefs,
} from "../../common";

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
    const xrpTokenId = 2;
    const feeTokenAssetId = 1124;
    const txes = [
      api.tx.assetsExt.createAsset("test", "TEST", 18, 1, alith.address),
      api.tx.assets.mint(feeTokenAssetId, alith.address, 2_000_000_000_000_000),
      api.tx.assets.mint(feeTokenAssetId, emptyAcct.address, 2_000_000_000_000_000),
      api.tx.dex.addLiquidity(
        feeTokenAssetId,
        xrpTokenId,
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
    feeToken = new Contract(assetIdToERC20ContractAddress(feeTokenAssetId), ERC20_ABI, emptyAccountSigner);
  });

  after(async () => await node.stop());

  it("Pays fees in non-native token", async () => {
    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpToken.balanceOf(emptyAccount.address),
      feeToken.balanceOf(emptyAccount.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 10_000_000_000;
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const nonce = await emptyAccountSigner.getTransactionCount();
    const chainId = 3999;
    const maxPriorityFeePerGas = 0; // 1_500_000_000 = '0x59682f00'
    const gasLimit = 23316; // '0x5b14' = 23316;
    const maxFeePerGas = 30_001_500_000_0000; // 30_001_500_000_000 = '0x1b4944c00f00'
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
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      chainId,
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
  });

  it("Pays fees in non-native token via legacy type 1 tx", async () => {
    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpToken.balanceOf(emptyAccount.address),
      feeToken.balanceOf(emptyAccount.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 10_000_000_000;
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const nonce = await emptyAccountSigner.getTransactionCount();
    const gasLimit = 23316; // '0x5b14' = 23316;
    const gasPrice = 15_000_000_000_000;

    const unsignedTx = {
      // legacy tx
      type: 1,
      from: emptyAccount.address,
      to: FEE_PROXY_ADDRESS,
      nonce,
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        maxFeePaymentInToken,
        feeToken.address,
        transferInput,
      ]),
      gasLimit,
      gasPrice,
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
  });

  it("Pays fees in non-native token via eip1559 type 2 tx", async () => {
    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpToken.balanceOf(emptyAccount.address),
      feeToken.balanceOf(emptyAccount.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 10_000_000_000;
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const nonce = await emptyAccountSigner.getTransactionCount();
    const chainId = 3999;
    const maxPriorityFeePerGas = 1_500_000_000; // 1_500_000_000 = '0x59682f00'
    const gasLimit = 23316; // '0x5b14' = 23316;
    const maxFeePerGas = 30_001_500_000_0000; // 30_001_500_000_000 = '0x1b4944c00f00'
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
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      chainId,
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
  });

  it("Does not pay in non-native token if max fee payment is insufficient", async () => {
    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 1; // <-- insufficient payment
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const nonce = await emptyAccountSigner.getTransactionCount();
    const chainId = 3999;
    const maxPriorityFeePerGas = 1_500_000_000; // 1_500_000_000 = '0x59682f00'
    const gasLimit = 23316; // '0x5b14' = 23316;
    const maxFeePerGas = 30_001_500_000_0000; // 30_001_500_000_000 = '0x1b4944c00f00'
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
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      chainId,
    };

    const error = await emptyAccountSigner.sendTransaction(unsignedTx).catch((e) => e);
    expect(error.code).to.be.eq("INSUFFICIENT_FUNDS");
    expect(error.reason).to.be.eq("insufficient funds for intrinsic transaction cost");
  });

  it("Does not pay in non-native token with gasLimit 0", async () => {
    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 10_000_000_000;
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const nonce = await emptyAccountSigner.getTransactionCount();
    const chainId = 3999;
    const maxPriorityFeePerGas = 0; // 1_500_000_000 = '0x59682f00'
    const gasLimit = 0;
    const maxFeePerGas = 0; // 30_001_500_000_000 = '0x1b4944c00f00'

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
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      chainId,
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

  it("Does not pay fees in non-native token with gasLimit 0 - legacy tx", async () => {
    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 10_000_000_000;
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const nonce = await emptyAccountSigner.getTransactionCount();
    const gasLimit = 0;
    const gasPrice = 15_000_000_000_000;

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
      gasLimit,
      gasPrice,
    };

    const error = await emptyAccountSigner.sendTransaction(unsignedTx).catch((e) => e);
    // See expected behavior for gasLimit === 0 https://github.com/futureversecom/frontier/blob/polkadot-v0.9.27-TRN/ts-tests/tests/test-transaction-cost.ts
    expect(error.code).to.be.eq("SERVER_ERROR");
    const body = JSON.parse(error.body);
    expect(body.error.message).to.be.eq(
      "submit transaction to pool failed: InvalidTransaction(InvalidTransaction::Custom(3))",
    );
  });

  it("Does not pay in non-native token if user does not have non-native token balance", async () => {
    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const newAccount = Wallet.createRandom().connect(provider);

    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 10_000_000_000;
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, newAccount);
    const nonce = await newAccount.getTransactionCount();
    const gasLimit = 23316; // '0x5b14' = 23316;
    const maxFeePerGas = 30_001_500_000_0000; // 30_001_500_000_000 = '0x1b4944c00f00'
    const unsignedTx = {
      type: 2,
      from: newAccount.address,
      to: FEE_PROXY_ADDRESS,
      nonce,
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        maxFeePaymentInToken,
        feeToken.address,
        transferInput,
      ]),
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas: 0,
      chainId: 3999,
    };

    const error = await newAccount.sendTransaction(unsignedTx).catch((e) => e);
    expect(error.code).to.be.eq("INSUFFICIENT_FUNDS");
    expect(error.reason).to.be.eq("insufficient funds for intrinsic transaction cost");
  });
});
