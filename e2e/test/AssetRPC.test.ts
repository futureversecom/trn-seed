import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToBn, hexToU8a } from "@polkadot/util";
import { expect } from "chai";

import { ALITH_PRIVATE_KEY, NodeProcess, rpcs, startNode, typedefs } from "../common";

describe("RPC", () => {
  let node: NodeProcess;

  let api: ApiPromise;
  let alith: KeyringPair;
  const mintAmount = "140282367920947470662629348422000000";

  before(async () => {
    node = await startNode();

    await node.wait(); // wait for the node to be ready
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs, rpc: rpcs });
    alith = new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    await new Promise<void>((resolve) => {
      api.tx.assets
        .mint(2, "0x6D1eFDE1BbF146EF88c360AF255D9d54A5D39408", mintAmount)
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) {
            resolve();
          }
        });
    });
  });

  it("RPC call to fetch alith's balance", async () => {
    const currentBalance = await (api.rpc as any)["assets-ext"].assetBalance(
      2,
      "0x6D1eFDE1BbF146EF88c360AF255D9d54A5D39408",
    );
    console.log("currentBalance::::", hexToBn(`${currentBalance}`).toString());
    expect(hexToBn(`${currentBalance}`).toString()).to.eq(mintAmount);
  });
});
