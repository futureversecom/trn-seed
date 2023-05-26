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
  GAS_TOKEN_ID,
  NodeProcess,
  assetIdToERC20ContractAddress,
  startNode,
  typedefs,
} from "../../common";

const feeTokenAssetId = 1124;
const EMPTY_ACCT_PRIVATE_KEY = "0xf8d74108dbe199c4a6e4ef457046db37c325ba3f709b14cabfa1885663e4c589";

describe("Fee Preferences under low token pair liquidity", function () {
  let node: NodeProcess;

  let bob: KeyringPair;
  let emptyAccountSigner: Wallet;
  let feeToken: Contract;
  let alithSigner: Wallet;

  before(async () => {
    node = await startNode();

    // Setup PolkadotJS rpc provider
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    const api = await ApiPromise.create({ provider: wsProvider, types: typedefs });

    const keyring = new Keyring({ type: "ethereum" });
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));
    const alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    const emptyAcct = keyring.addFromSeed(hexToU8a(EMPTY_ACCT_PRIVATE_KEY));

    const txes = [
      api.tx.assetsExt.createAsset("test", "TEST", 18, 1, alith.address),
      api.tx.assets.mint(feeTokenAssetId, alith.address, 2_000_000_000_000_000),
      api.tx.assets.mint(feeTokenAssetId, emptyAcct.address, 2_000_000_000_000_000),
      api.tx.dex.addLiquidity(feeTokenAssetId, GAS_TOKEN_ID, 100_000, 100_000, 100_000, 100_000, null, null),
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
    const jsonProvider = new JsonRpcProvider(`http://localhost:${node.httpPort}`);
    emptyAccountSigner = new Wallet(EMPTY_ACCT_PRIVATE_KEY).connect(jsonProvider);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(jsonProvider);
    feeToken = new Contract(assetIdToERC20ContractAddress(feeTokenAssetId), ERC20_ABI, alithSigner);
  });

  after(async () => await node.stop());

  it("Fails to pay fees in non-native token if insufficient liquidity", async () => {
    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 10_000_000_000;
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const nonce = await emptyAccountSigner.getTransactionCount();
    const chainId = 7672;
    const maxPriorityFeePerGas = 0; // 1_500_000_000 = '0x59682f00'
    const gasLimit = 23316; // '0x5b14' = 23316;
    const maxFeePerGas = 30_001_500_000_0000; // 30_001_500_000_000 = '0x1b4944c00f00'
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
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      chainId,
    };

    // assert error from tx
    const error = await emptyAccountSigner.sendTransaction(unsignedTx).catch((e) => e);
    expect(error.code).to.be.eq("INSUFFICIENT_FUNDS");
    expect(error.reason).to.be.eq("insufficient funds for intrinsic transaction cost");
  });
});
