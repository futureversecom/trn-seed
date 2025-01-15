import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { BN, hexToU8a, u8aToString, u8aToU8a } from "@polkadot/util";
import axios from "axios";
import { expect } from "chai";

import { ALITH_PRIVATE_KEY, NodeProcess, startNode, typedefs } from "../common";

export const rpc = {
  nft: {
    collectionDetails: {
      description: "Returns the collection info for a NFT collection",
      params: [
        {
          name: "collectionId",
          type: "u32",
        },
      ],
      type: "Json",
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
    const httpResult = await axios.post(`http://127.0.0.1:9944`, {
      id: 1,
      jsonrpc: "2.0",
      method: "nft_collectionDetails",
      params: [collectionId],
    });

    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.statusText).to.eql("OK");
    const {
      result: { Ok },
    } = httpResult.data;
    const {
      owner,
      name,
      metadata_scheme,
      royalties_schedule,
      max_issuance,
      origin_chain,
      next_serial_number,
      collection_issuance,
      cross_chain_compatibility,
    } = Ok;
    expect(owner).to.eql("0xf24ff3a9cf04c71dbc94d0b566f7a27b94566cac");
    expect(u8aToString(new Uint8Array(name))).to.eql("test-collection");
    expect(u8aToString(new Uint8Array(metadata_scheme))).to.eql("https://test/api/");
    expect(royalties_schedule).to.eql([["0xf24ff3a9cf04c71dbc94d0b566f7a27b94566cac", 10000]]);
    expect(max_issuance).to.eql(null);
    expect(next_serial_number).to.eql(0);
    expect(collection_issuance).to.eql(0);
    expect(cross_chain_compatibility).to.eql({ xrpl: false });
    expect(origin_chain).to.eql("Root");
  });

  it("collection info rpc works [library]", async () => {
    const result = await (api.rpc as any).nft.collectionDetails(collectionId);
    const data = result.Ok;
    const {
      owner,
      name,
      metadata_scheme,
      royalties_schedule,
      max_issuance,
      origin_chain,
      next_serial_number,
      collection_issuance,
      cross_chain_compatibility,
    } = data;
    expect(owner).to.eql("0xf24ff3a9cf04c71dbc94d0b566f7a27b94566cac");
    expect(u8aToString(new Uint8Array(name))).to.eql("test-collection");
    expect(u8aToString(new Uint8Array(metadata_scheme))).to.eql("https://test/api/");
    expect(royalties_schedule).to.eql([[alith.address.toLowerCase(), 10000]]);
    expect(max_issuance).to.eql(null);
    expect(next_serial_number).to.eql(0);
    expect(collection_issuance).to.eql(0);
    expect(cross_chain_compatibility).to.eql({ xrpl: false });
    expect(origin_chain).to.eql("Root");
  });

  it("collection info rpc works [library] that does not exist", async () => {
    const result = await (api.rpc as any).nft.collectionDetails(100);
    const { Err } = result;
    const { section, name } = api.registry.findMetaError({
      index: new BN(Err.Module.index),
      error: u8aToU8a(Err.Module.error),
    });
    expect(section).to.equal("nft");
    expect(name).to.equal("NoCollectionFound");
  });
});
