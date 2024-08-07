import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";

import { ALITH_PRIVATE_KEY, NodeProcess, finalizeTx, rpcs, startNode, typedefs } from "../common";

describe("RPC", () => {
  let node: NodeProcess;

  let api: ApiPromise;
  let alith: KeyringPair;
  const mintAmount = "140282367920947470662629348422000000"; // Using a value which is greater than js number (2 pow 53 -1) - 9007199254740991
  const mintSmallerAmount = "121";
  before(async () => {
    node = await startNode();

    await node.wait(); // wait for the node to be ready
    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs, rpc: rpcs });
    alith = new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    const tx1 = api.tx.assets.mint(2, "0x6D1eFDE1BbF146EF88c360AF255D9d54A5D39408", mintAmount);
    await finalizeTx(alith, tx1);
    const tx2 = api.tx.assets.mint(2, "0x1Fb0E85b7Ba55F0384d0E06D81DF915aeb3baca3", mintSmallerAmount);
    await finalizeTx(alith, tx2);
  });

  after(async () => await node.stop());

  it("RPC call to fetch account1's balance", async () => {
    const currentBalance = await (api.rpc as any).assetsExt.freeBalance(
      2,
      "0x6D1eFDE1BbF146EF88c360AF255D9d54A5D39408",
    );
    expect(currentBalance.toString()).to.eq(mintAmount);
  });

  it("RPC call to fetch account2's balance", async () => {
    const currentBalance = await (api.rpc as any).assetsExt.freeBalance(
      2,
      "0x1Fb0E85b7Ba55F0384d0E06D81DF915aeb3baca3",
    );
    expect(currentBalance.toString()).to.eq(mintSmallerAmount);
  });
});
