import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { hexToU8a } from '@polkadot/util';
import { expect } from "chai";
import axios from "axios";

export const ALICE_PRIVATE_KEY = '0xcb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854';
export const NATIVE_TOKEN_ID = 2;
export const FEE_TOKEN_ID = 1124;

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
  },
};

describe('DexRPC', () => {

  let api: ApiPromise;

  before(async () => {
    const wsProvider = new WsProvider(`ws://localhost:9944`);

    const keyring = new Keyring({ type: 'ethereum' });
    const alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));

    api = await ApiPromise.create({ provider: wsProvider, rpc });
    console.log('connected to api.')
    
    console.log('setting up dex liquidity...');

    const txs = [
      api.tx.assetsExt.createAsset(), // create asset
      api.tx.assets.mint(FEE_TOKEN_ID, alice.address, "1000000000000000000000000"), // mint 1M tokens (18 decimals) to alice
      api.tx.dex.addLiquidity(        // provide liquidity
        FEE_TOKEN_ID,
        NATIVE_TOKEN_ID,
          "1000000000000000000000",   // 1000 tokens
          250_000_000,
          "1000000000000000000000",   // 1000 tokens
          250_000_000,
        0,
      ),
    ];

    await new Promise<void>((resolve) => {
      api.tx.utility
        .batch(txs)
        .signAndSend(alice, ({ status }) => {
          if (status.isInBlock) {
            console.log(`setup block hash: ${status.asInBlock}`);
            resolve();
          }
        })
      });
    
    console.log('done setting up dex liquidity.');
  });

  it('quote rpc works', async () => {
    // test via http
    const httpResult = await axios.post('http://localhost:9933', {
      id: 1,
      jsonrpc:"2.0",
      method: "dex_quote",
      params: [1, 5, 10],
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty('result');
    expect(httpResult.data.result).to.haveOwnProperty('Ok');
    expect(httpResult.data.result.Ok).to.eql(2);

    // test via polkadotjs substrate library
    const result = await (api.rpc as any).dex.quote(1, 5, 10);
    expect(result).to.haveOwnProperty('Ok');
    expect(result.Ok).to.eql(2);
  });

  it('getAmountsOut rpc works', async () => {
    // test via http
    const httpResult = await axios.post('http://localhost:9933', {
      id: 1,
      jsonrpc:"2.0",
      method: "dex_getAmountsOut",
      params: [100, [2, 1124]]
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty('result');
    expect(httpResult.data.result).to.haveOwnProperty('Ok');
    expect(httpResult.data.result.Ok).to.eqls([100,398799840958623]);
  
    // test via polkadotjs substrate library
    const result = await (api.rpc as any).dex.getAmountsOut(100, [2, 1124]);
    expect(result).to.haveOwnProperty('Ok');
    expect(result.Ok).to.eqls([100,398799840958623]);
  });

  it('getAmountsIn rpc works', async () => {
    // test via http
    const httpResult = await axios.post('http://localhost:9933', {
      id: 1,
      jsonrpc:"2.0",
      method: "dex_getAmountsIn",
      params: [100, [1124, 2]]
    });
    expect(httpResult.status).to.eql(200);
    expect(httpResult.data).to.haveOwnProperty('result');
    expect(httpResult.data.result).to.haveOwnProperty('Ok');
    expect(httpResult.data.result.Ok).to.eqls([401203771314007, 100]);
  
    // test via polkadotjs substrate library
    const result = await (api.rpc as any).dex.getAmountsIn(100, [1124, 2]);
    expect(result).to.haveOwnProperty('Ok');
    expect(result.Ok).to.eqls([401203771314007, 100]);
  });
});