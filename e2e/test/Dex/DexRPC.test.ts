import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { hexToU8a } from "@polkadot/util";
import axios from "axios";
import { expect } from "chai";
import { utils } from "ethers";

import { ALITH_PRIVATE_KEY, GAS_TOKEN_ID, NodeProcess, startNode, typedefs } from "../../common";

const TOKEN_ID = 1124;

export const rpc = {
  dex: {
    quote: {
      description: "Returns the amount of output token that can be obtained by swapping an amount of input token",
      params: [
        {
          name: "amountIn",
          type: "u128",
        },
        {
          name: "reserveIn",
          type: "u128",
        },
        {
          name: "reserveOut",
          type: "u128",
        },
      ],
      type: "Json",
    },
    getAmountsOut: {
      description: "Returns the amount of output tokens that can be obtained by swapping an amount of inputs token",
      params: [
        {
          name: "amountIn",
          type: "Balance",
        },
        {
          name: "path",
          type: "Vec<AssetId>",
        },
      ],
      type: "Json",
    },
    getAmountsIn: {
      description: "Returns the amount of input tokens that can be obtained by swapping an amount of output token",
      params: [
        {
          name: "amountOut",
          type: "Balance",
        },
        {
          name: "path",
          type: "Vec<AssetId>",
        },
      ],
      type: "Json",
    },
    getLPTokenID: {
      description: "Returns the LP token ID from the given trading pair",
      params: [
        {
          name: "assetIdA",
          type: "AssetId",
        },
        {
          name: "assetIdB",
          type: "AssetId",
        },
      ],
      type: "Json",
    },
    getLiquidity: {
      description: "Returns the liquidity balances of the given trading pair",
      params: [
        {
          name: "assetIdA",
          type: "AssetId",
        },
        {
          name: "assetIdB",
          type: "AssetId",
        },
      ],
      type: "Json",
    },
    getTradingPairStatus: {
      description: "Returns the status of the given trading pair",
      params: [
        {
          name: "assetIdA",
          type: "AssetId",
        },
        {
          name: "assetIdB",
          type: "AssetId",
        },
      ],
      type: "Json",
    },
  },
};

describe("DexRPC", () => {
  let node: NodeProcess;
  let api: ApiPromise;

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

    const txs = [
      api.tx.assetsExt.createAsset("test", "TEST", 18, 1, alith.address), // create asset
      api.tx.assets.mint(TOKEN_ID, alith.address, utils.parseEther("1000000").toString()),
      api.tx.dex.addLiquidity(
        // provide liquidity
        TOKEN_ID,
        GAS_TOKEN_ID,
        utils.parseEther("1000").toString(),
        250_000_000,
        utils.parseEther("1000").toString(),
        250_000_000,
        null,
        null,
      ),
    ];

    await new Promise<void>((resolve, reject) => {
      api.tx.utility
        .batch(txs)
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) {
            console.log(`setup block hash: ${status.asInBlock}`);
            resolve();
          }
        })
        .catch((err) => reject(err));
    });

    console.log("done setting up dex liquidity.");
  });

  after(async () => node.stop());

  it("quote rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://localhost:${node.httpPort}`, {
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
    const httpResult = await axios.post(`http://localhost:${node.httpPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getAmountsOut",
      params: [100, [GAS_TOKEN_ID, TOKEN_ID]],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.haveOwnProperty("Ok");
    expect(httpResult.data.result.Ok).to.eqls([100, 398799840958623]);
  });

  it("getAmountsOut rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getAmountsOut(100, [GAS_TOKEN_ID, TOKEN_ID]);
    expect(result).to.haveOwnProperty("Ok");
    expect(result.Ok).to.eqls([100, 398799840958623]);
  });

  it("getAmountsIn rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://localhost:${node.httpPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getAmountsIn",
      params: [100, [TOKEN_ID, GAS_TOKEN_ID]],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.haveOwnProperty("Ok");
    expect(httpResult.data.result.Ok).to.eqls([401203771314007, 100]);
  });

  it("getAmountsIn rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getAmountsIn(100, [TOKEN_ID, GAS_TOKEN_ID]);
    expect(result).to.haveOwnProperty("Ok");
    expect(result.Ok).to.eqls([401203771314007, 100]);
  });

  it("getLPTokenID rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://localhost:${node.httpPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getLPTokenID",
      params: [TOKEN_ID, GAS_TOKEN_ID],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.haveOwnProperty("Ok");
    expect(httpResult.data.result.Ok).to.eqls(2148);
  });

  it("getLPTokenID rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getLPTokenID(TOKEN_ID, GAS_TOKEN_ID);
    expect(result).to.haveOwnProperty("Ok");
    expect(result.Ok).to.eqls(2148);
  });

  it("getLPTokenID with reversed trading pair rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://localhost:${node.httpPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getLPTokenID",
      params: [GAS_TOKEN_ID, TOKEN_ID],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.haveOwnProperty("Ok");
    expect(httpResult.data.result.Ok).to.eqls(2148);
  });

  it("getLPTokenID with reversed trading pair rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getLPTokenID(GAS_TOKEN_ID, TOKEN_ID);
    expect(result).to.haveOwnProperty("Ok");
    expect(result.Ok).to.eqls(2148);
  });

  it("getLiquidity rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://localhost:${node.httpPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getLiquidity",
      params: [TOKEN_ID, GAS_TOKEN_ID],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.eqls([1000000000000000000000, 250000000]);
  });

  it("getLiquidity rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getLiquidity(TOKEN_ID, GAS_TOKEN_ID);
    expect(result).to.eqls(
      new Map<string, number>([
        ["0", 1000000000000000000000],
        ["1", 250000000],
      ]),
    );
  });

  it("getLiquidity with reversed trading pair rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://localhost:${node.httpPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getLiquidity",
      params: [GAS_TOKEN_ID, TOKEN_ID],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.eqls([250000000, 1000000000000000000000]);
  });

  it("getLiquidity with reversed trading pair rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getLiquidity(GAS_TOKEN_ID, TOKEN_ID);
    expect(result).to.eqls(
      new Map<string, number>([
        ["0", 250000000],
        ["1", 1000000000000000000000],
      ]),
    );
  });

  it("getTradingPairStatus rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://localhost:${node.httpPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getTradingPairStatus",
      params: [TOKEN_ID, GAS_TOKEN_ID],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.eqls("Enabled");
  });

  it("getTradingPairStatus rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getTradingPairStatus(TOKEN_ID, GAS_TOKEN_ID);
    expect(result).to.eqls(
      new Map<string, string>([
        ["0", "E"],
        ["1", "n"],
        ["2", "a"],
        ["3", "b"],
        ["4", "l"],
        ["5", "e"],
        ["6", "d"],
      ]),
    );
  });

  it("getTradingPairStatus with reversed trading pair rpc works [http - axios]", async () => {
    const httpResult = await axios.post(`http://localhost:${node.httpPort}`, {
      id: 1,
      jsonrpc: "2.0",
      method: "dex_getTradingPairStatus",
      params: [GAS_TOKEN_ID, TOKEN_ID],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty("result");
    expect(httpResult.data.result).to.eqls("Enabled");
  });

  it("getTradingPairStatus with reversed trading pair rpc works [library]", async () => {
    const result = await (api.rpc as any).dex.getTradingPairStatus(GAS_TOKEN_ID, TOKEN_ID);
    expect(result).to.eqls(
      new Map<string, string>([
        ["0", "E"],
        ["1", "n"],
        ["2", "a"],
        ["3", "b"],
        ["4", "l"],
        ["5", "e"],
        ["6", "d"],
      ]),
    );
  });
});
