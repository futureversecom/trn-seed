import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { hexToU8a, u8aToString } from "@polkadot/util";
import axios from "axios";
import { expect } from "chai";

import { ALITH_PRIVATE_KEY, NodeProcess, startNode, typedefs } from "../common";

export const rpc = {
  sft: {
    tokenUri: {
      description: "Returns the constructed tokenURI for an SFT token",
      params: [
        {
          name: "tokenId",
          type: "(u32,u32)",
        },
      ],
      type: "Vec<u8>",
    },
  },
};

describe("SftRPC", () => {
  let node: NodeProcess;
  let api: ApiPromise;
  let collectionId: any;

  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    api = await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
      rpc,
    });

    const keyring = new Keyring({ type: "ethereum" });
    const alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    collectionId = await api.query.nft.nextCollectionId();

    await new Promise<void>((resolve, reject) => {
      api.tx.sft
        .createCollection("test-collection", null, "https://test/api/", null)
        .signAndSend(alith, ({ status, events }) => {
          if (status.isInBlock) {
            events.forEach(({ event: { data, method } }) => {
              if (method == "CollectionCreate") {
                collectionId = (data.toJSON() as any)[0];
                console.log(`Collection UUID: ${collectionId}`);
                resolve();
              }
            });
          }
        })
        .catch((err) => reject(err));
    });

    await new Promise<void>((resolve, reject) => {
      api.tx.sft
        .createToken(collectionId, "test-token", 100, null, null)
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) {
            resolve();
          }
        })
        .catch((err) => reject(err));
    });
  });

  after(async () => node.stop());

  it("token_uri rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://localhost:${node.httpPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "sft_tokenUri",
      params: [[collectionId, 0]],
    });

    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.statusText).to.eql("OK");
    const res = u8aToString(new Uint8Array(httpResult.data.result));
    expect(res).to.eql("https://test/api/0");
  });

  it("token_uri rpc works [library]", async () => {
    const result = await (api.rpc as any).sft.tokenUri([collectionId, 0]);
    expect(u8aToString(result)).to.eql("https://test/api/0");
  });
});
