// Call an EVM transaction with fee preferences for an account that has zero native token balance, ensuring that the preferred asset with liquidity is spent instead
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

const feeTokenAssetId = 1124;

describe("Fee Preferences in low asset balance scenario", function () {
  let node: NodeProcess;

  let bob: KeyringPair;
  let insufficientAccountSigner: Wallet;
  let feeToken: Contract;

  // Setup api instance and keyring wallet addresses for alith and bob
  before(async () => {
    node = await startNode();

    // Setup PolkadotJS rpc provider
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    const api = await ApiPromise.create({ provider: wsProvider, types: typedefs });

    const keyring = new Keyring({ type: "ethereum" });
    const alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));

    // Setup JSON RPC provider
    const provider = new JsonRpcProvider(`http://localhost:${node.httpPort}`);
    insufficientAccountSigner = Wallet.createRandom().connect(provider);
    feeToken = new Contract(assetIdToERC20ContractAddress(feeTokenAssetId), ERC20_ABI, insufficientAccountSigner);

    // Mint and Setup liqiudity on dex for fee token
    const txs = [
      api.tx.assetsExt.createAsset("test", "TEST", 18, 1, alith.address),
      api.tx.assets.mint(feeTokenAssetId, alith.address, 2_000_000_000_000_000),
      api.tx.assets.mint(feeTokenAssetId, insufficientAccountSigner.address, 2_000),
      api.tx.dex.addLiquidity(
        feeTokenAssetId,
        NATIVE_TOKEN_ID,
        100_000_000_000,
        100_000_000_000,
        100_000_000_000,
        100_000_000_000,
        0,
      ),
    ];
    await new Promise<void>((resolve) => {
      api.tx.utility.batch(txs).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) {
          console.log(`setup block hash: ${status.asInBlock}`);
          resolve();
        }
      });
    });
  });

  after(async () => await node.stop());

  it("Cannot pay fees with non-native, preferred token if low asset balance", async () => {
    const transferAmount = 1;
    const iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 10_000_000_000;
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, insufficientAccountSigner);

    const nonce = await insufficientAccountSigner.getTransactionCount();
    const chainId = 7672;
    const maxPriorityFeePerGas = 0;
    const gasLimit = 23316;
    const maxFeePerGas = 30_001_500_000_0000;
    const unsignedTx = {
      // eip1559 tx
      type: 2,
      from: insufficientAccountSigner.address,
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

    const error = await insufficientAccountSigner.sendTransaction(unsignedTx).catch((e) => e);
    expect(error.code).to.be.eq("INSUFFICIENT_FUNDS");
    expect(error.reason).to.be.eq("insufficient funds for intrinsic transaction cost");
  });
});
