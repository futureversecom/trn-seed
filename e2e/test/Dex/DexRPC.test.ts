import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { hexToU8a } from "@polkadot/util";
import axios from "axios";
import { expect } from "chai";
import { utils } from "ethers";

import {
  ALITH_PRIVATE_KEY,
  GAS_TOKEN_ID,
  NodeProcess,
  finalizeTx,
  getNextAssetId,
  rpcs,
  startNode,
  typedefs,
} from "../../common";

describe("DexRPC", () => {
  let node: NodeProcess;
  let api: ApiPromise;
  let tokenId: number;

  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs, rpc: rpcs });

    const keyring = new Keyring({ type: "ethereum" });
    const alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    tokenId = await getNextAssetId(api);
    const txs = [
      api.tx.assetsExt.createAsset("test", "TEST", 18, 1, alith.address), // create asset
      api.tx.assets.mint(tokenId, alith.address, utils.parseEther("1000000").toString()),
      api.tx.dex.addLiquidity(
        // provide liquidity
        tokenId,
        GAS_TOKEN_ID,
        utils.parseEther("1000").toString(),
        250_000_000,
        utils.parseEther("1000").toString(),
        250_000_000,
        null,
        null,
      ),
    ];

    await finalizeTx(alith, api.tx.utility.batch(txs));
  });

  after(async () => node.stop());

  it("quote rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://127.0.0.1:${node.rpcPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_quote",
      params: [1, 5, 10],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.haveOwnProperty("Ok");
    expect(httpResult.data.result.Ok).to.eql(2);
  });

  it("quote rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.quote(1, 5, 10);
    expect(result).to.haveOwnProperty("Ok");
    expect(result.Ok).to.eql(2);
  });

  it("getAmountsOut rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://127.0.0.1:${node.rpcPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getAmountsOut",
      params: [100, [GAS_TOKEN_ID, tokenId]],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.haveOwnProperty("Ok");
    expect(httpResult.data.result.Ok).to.eqls([100, 398799840958623]);
  });

  it("getAmountsOut rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getAmountsOut(100, [GAS_TOKEN_ID, tokenId]);
    expect(result).to.haveOwnProperty("Ok");
    expect(result.Ok).to.eqls([100, 398799840958623]);
  });

  it("getAmountsIn rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://127.0.0.1:${node.rpcPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getAmountsIn",
      params: [100, [tokenId, GAS_TOKEN_ID]],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.haveOwnProperty("Ok");
    expect(httpResult.data.result.Ok).to.eqls([401203771314007, 100]);
  });

  it("getAmountsIn rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getAmountsIn(100, [tokenId, GAS_TOKEN_ID]);
    expect(result).to.haveOwnProperty("Ok");
    expect(result.Ok).to.eqls([401203771314007, 100]);
  });

  it("getLPTokenID rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://127.0.0.1:${node.rpcPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getLPTokenID",
      params: [tokenId, GAS_TOKEN_ID],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.haveOwnProperty("Ok");
  });

  it("getLPTokenID rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getLPTokenID(tokenId, GAS_TOKEN_ID);
    expect(result).to.haveOwnProperty("Ok");
  });

  it("getLPTokenID with reversed trading pair rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://127.0.0.1:${node.rpcPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getLPTokenID",
      params: [GAS_TOKEN_ID, tokenId],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.haveOwnProperty("Ok");
  });

  it("getLPTokenID with reversed trading pair rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getLPTokenID(GAS_TOKEN_ID, tokenId);
    expect(result).to.haveOwnProperty("Ok");
  });

  it("getLiquidity rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://127.0.0.1:${node.rpcPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getLiquidity",
      params: [tokenId, GAS_TOKEN_ID],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.eqls([1000000000000000000000, 250000000]);
  });

  it("getLiquidity rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getLiquidity(tokenId, GAS_TOKEN_ID);
    expect(result).to.eqls(
      new Map<string, number>([
        ["0", 1000000000000000000000],
        ["1", 250000000],
      ]),
    );
  });

  it("getLiquidity with reversed trading pair rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://127.0.0.1:${node.rpcPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getLiquidity",
      params: [GAS_TOKEN_ID, tokenId],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.eqls([250000000, 1000000000000000000000]);
  });

  it("getLiquidity with reversed trading pair rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getLiquidity(GAS_TOKEN_ID, tokenId);
    expect(result).to.eqls(
      new Map<string, number>([
        ["0", 250000000],
        ["1", 1000000000000000000000],
      ]),
    );
  });

  it("getTradingPairStatus rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://127.0.0.1:${node.rpcPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getTradingPairStatus",
      params: [tokenId, GAS_TOKEN_ID],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.eqls("Enabled");
  });

  it("getTradingPairStatus rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getTradingPairStatus(tokenId, GAS_TOKEN_ID);
    expect(result.toString()).to.eql("Enabled");
  });

  it("getTradingPairStatus with reversed trading pair rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://127.0.0.1:${node.rpcPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getTradingPairStatus",
      params: [GAS_TOKEN_ID, tokenId],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.eqls("Enabled");
  });

  it("getTradingPairStatus with reversed trading pair rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getTradingPairStatus(GAS_TOKEN_ID, tokenId);
    expect(result.toString()).to.eql("Enabled");
  });
});
