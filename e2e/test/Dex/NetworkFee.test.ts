import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { utils } from "ethers";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  GAS_TOKEN_ID,
  NodeProcess,
  finalizeTx,
  getNextAssetId,
  rpcs,
  startNode,
  typedefs,
} from "../../common";

describe("NetworkFee", () => {
  let TOKEN_ID: number;
  let LP_TOKEN_ID: number;
  let node: NodeProcess;
  let api: ApiPromise;
  let alith: KeyringPair;
  let bob: KeyringPair;

  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    api = await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
      rpc: rpcs,
    });

    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));

    TOKEN_ID = await getNextAssetId(api);

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

    await finalizeTx(alith, api.tx.utility.batch(txs));

    LP_TOKEN_ID = (await api.query.dex.tradingPairLPToken([GAS_TOKEN_ID, TOKEN_ID])).toJSON() as number;
  });

  after(async () => node.stop());

  it("test network fees", async () => {
    // set FeeTo to bob
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.dex.setFeeTo(bob.address)));

    // get bob's lp balance
    const bobLPBalanceBefore =
      ((await api.query.assets.account(LP_TOKEN_ID, bob.address)).toJSON() as any)?.balance ?? 0;

    // get the total supply before swapping
    const totalSupply = ((await api.query.assets.asset(LP_TOKEN_ID)).toJSON() as any)?.supply;

    // get the last k value before swapping
    const lastK = (await api.query.dex.liquidityPoolLastK(LP_TOKEN_ID)).toJSON() as any;

    // alith makes a swap
    await finalizeTx(
      alith,
      api.tx.dex.swapWithExactSupply(utils.parseEther("100").toString(), 0, [TOKEN_ID, GAS_TOKEN_ID], null, null),
    );

    // get the reserves after swapping
    const reserves = (await api.query.dex.liquidityPool([GAS_TOKEN_ID, TOKEN_ID])).toJSON() as any;
    const kSqrtLast = Math.sqrt(lastK);
    const kSqrt = Math.sqrt(reserves[0] * reserves[1]);

    // get bob's lp balance after swapping
    const bobLPBalanceAfter = ((await api.query.assets.account(LP_TOKEN_ID, bob.address)).toJSON() as any).balance;

    // calculate the expected network fee
    const networkFeeAmountExpected = totalSupply * ((kSqrt - kSqrtLast) / (5 * kSqrt + kSqrtLast));
    const networkFeeAmountActual = bobLPBalanceAfter - bobLPBalanceBefore;
    expect(networkFeeAmountActual).to.eq(Math.floor(networkFeeAmountExpected));

    // alith adds some LP tokens
    await finalizeTx(alith, api.tx.dex.removeLiquidity(TOKEN_ID, GAS_TOKEN_ID, 3000000, 0, 0, null, null));

    // check if the last k value has been updated
    const lastKAfterAddingLiquidity: number = (await api.query.dex.liquidityPoolLastK(LP_TOKEN_ID)).toJSON() as any;
    const reservesAfterAddingLiquidity = (await api.query.dex.liquidityPool([GAS_TOKEN_ID, TOKEN_ID])).toJSON() as any;
    expect(BigInt(lastKAfterAddingLiquidity)).to.eq(
      BigInt(reservesAfterAddingLiquidity[0]) * BigInt(reservesAfterAddingLiquidity[1]),
    );

    // alith withdraws some LP tokens
    await finalizeTx(alith, api.tx.dex.removeLiquidity(TOKEN_ID, GAS_TOKEN_ID, 3000000, 0, 0, null, null));

    // check if the last k value has been updated
    const lastKAfterRemovingLiquidity = (await api.query.dex.liquidityPoolLastK(LP_TOKEN_ID)).toJSON() as any;
    const reservesAfterRemovingLiquidity = (
      await api.query.dex.liquidityPool([GAS_TOKEN_ID, TOKEN_ID])
    ).toJSON() as any;
    expect(BigInt(lastKAfterRemovingLiquidity)).to.eq(
      BigInt(reservesAfterRemovingLiquidity[0]) * BigInt(reservesAfterRemovingLiquidity[1]),
    );
  });
});
