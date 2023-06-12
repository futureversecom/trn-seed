import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";

import {
  ALITH_PRIVATE_KEY,
  NodeProcess,
  sleep,
  startNode,
  typedefs,
} from "../../common";

describe("Fee Preference Extrinsic", function () {
  const EMPTY_ACCT_PRIVATE_KEY = "0xf8d74108dbe199c4a6e4ef457046db37c325ba3f709b14cabfa1885663e4c589";
  const feeTokenAssetId = 1124;

  let node: NodeProcess;
  let api: ApiPromise;

  let alith: KeyringPair;

  before(async () => {
    node = await startNode();

    // Setup PolkadotJS rpc provider
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    // bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));
    // emptyAccount = keyring.addFromSeed(hexToU8a(EMPTY_ACCT_PRIVATE_KEY));

    // Empty with regards to native balance only
    const emptyAcct = keyring.addFromSeed(hexToU8a(EMPTY_ACCT_PRIVATE_KEY));

    // add liquidity for XRP<->token
    const xrpTokenId = 2;
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
  });

  after(async () => await node.stop());

  it("Pays fees in non-native token", async () => {
    const initialBalance = await api.query.assets.account(feeTokenAssetId, alith.address);
    const innerCall = api.tx.system.remark("sup");
    const feeproxiedCall = api.tx.feeProxy.callWithFeePreferences(feeTokenAssetId, 1000000, innerCall);
    await feeproxiedCall.signAndSend(alith);

    await sleep(4000)

    const proxiedTokenBalance = await api.query.assets.account(feeTokenAssetId, alith.address);
    expect(proxiedTokenBalance.toPrimitive().balance).to.be.lessThan(initialBalance.toPrimitive().balance);
  });

});
