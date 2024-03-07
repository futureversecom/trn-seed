import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a, u8aToHex, u8aToString } from "@polkadot/util";
import axios from "axios";
import { expect } from "chai";

import { ALITH_PRIVATE_KEY, NodeProcess, startNode, typedefs } from "../common";

export const rpc = {
  nft: {
    collectionDetails: {
      description: "Returns the constructed tokenURI for an SFT token",
      params: [
        {
          name: "collectionId",
          type: "u32",
        },
      ],
      type: "(AccountId, Vec<u8>, Vec<u8>, Option<Vec<(T::AccountId, Permill)>>,Option<u32>, u32, u32, CrossChainCompatibility, Text)",
    },
  },
};

describe("NftRPC", () => {
  let node: NodeProcess;
  let api: ApiPromise;
  let collectionId: any;
  let alith: KeyringPair;

  before(async () => {
    node = await startNode();

    await node.wait(); // wait for the node to be ready
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    api = await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
      rpc,
    });

    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    collectionId = await api.query.nft.nextCollectionId();
    const royaltiesSchedule = {
      entitlements: [[alith.address, 10000 /* one percent */]],
    };
    await new Promise<void>((resolve, reject) => {
      api.tx.nft
        .createCollection("test-collection", 0, null, null, "https://test/api/", royaltiesSchedule, { xrpl: false })
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
  });

  after(async () => node.stop());

  it("collection info rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://localhost:${node.httpPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "nft_collectionDetails",
      params: [collectionId],
    });

    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.statusText).to.eql("OK");
    const { result } = httpResult.data;
    const owner = result[0];
    const name = u8aToString(new Uint8Array(result[1]));
    const metadata = u8aToString(new Uint8Array(result[2]));
    const royalitySchedule = result[3];
    const maxIssuance = result[4];
    const nextSerialNumber = result[5];
    const collectionIssuance = result[6];
    const crossChainCompatibility = result[7];
    const chainOrigin = result[8];
    expect(owner).to.eql("0xf24ff3a9cf04c71dbc94d0b566f7a27b94566cac");
    expect(name).to.eql("test-collection");
    expect(metadata).to.eql("https://test/api/0");
    expect(royalitySchedule).to.eql([["0xf24ff3a9cf04c71dbc94d0b566f7a27b94566cac", 10000]]);
    expect(maxIssuance).to.eql(null);
    expect(nextSerialNumber).to.eql(0);
    expect(collectionIssuance).to.eql(0);
    expect(crossChainCompatibility).to.eql({ xrpl: false });
    expect(chainOrigin).to.eql("Root");
  });

  it("collection info rpc works [library]", async () => {
    const result = await (api.rpc as any).nft.collectionDetails(collectionId);
    const owner = u8aToHex(result[0]);
    const name = u8aToString(result[1]);
    const metadata = u8aToString(result[2]);
    const royalitySchedule = result[3];
    const maxIssuance = result[4];
    const nextSerialNumber = result[5];
    const collectionIssuance = result[6];
    const crossChainCompatibility = result[7];
    const chainOrigin = result[8];
    expect(owner).to.eql("0xf24ff3a9cf04c71dbc94d0b566f7a27b94566cac");
    expect(name).to.eql("test-collection");
    expect(metadata).to.eql("https://test/api/0");
    expect(royalitySchedule.toJSON()).to.eql([[alith.address, 10000]]);
    expect(maxIssuance.toJSON()).to.eql(null);
    expect(nextSerialNumber.toNumber()).to.eql(0);
    expect(collectionIssuance.toNumber()).to.eql(0);
    expect(crossChainCompatibility.toJSON()).to.eql({ xrpl: false });
    expect(chainOrigin.toString()).to.eql("Root");
  });

  it("collection info rpc works [library] that does not exist", async () => {
    const result = await (api.rpc as any).nft.collectionDetails(1);
    const owner = u8aToHex(result[0]);
    const name = u8aToString(result[1]);
    const metadata = u8aToString(result[2]);
    const royalitySchedule = result[3];
    const maxIssuance = result[4];
    const nextSerialNumber = result[5];
    const collectionIssuance = result[6];
    const crossChainCompatibility = result[7];
    const chainOrigin = result[8];
    expect(owner).to.eql("0x0000000000000000000000000000000000000000");
    expect(name).to.eql("");
    expect(metadata).to.eql("");
    expect(royalitySchedule.toJSON()).to.eql(null);
    expect(maxIssuance.toJSON()).to.eql(null);
    expect(nextSerialNumber.toNumber()).to.eql(0);
    expect(collectionIssuance.toNumber()).to.eql(0);
    expect(crossChainCompatibility.toJSON()).to.eql({ xrpl: false });
    expect(chainOrigin.toString()).to.eql("Root");
  });
});
