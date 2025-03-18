import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Wallet } from "ethers";

import {
  ALITH_PRIVATE_KEY,
  GAS_TOKEN_ID,
  NATIVE_TOKEN_ID,
  NodeProcess,
  finalizeTx,
  getNextAssetId,
  rpcs,
  sleep,
  startNode,
  typedefs,
} from "../common";

describe("Reward", () => {
  let node: NodeProcess;
  let api: ApiPromise;
  let alith: KeyringPair;

  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://localhost:${node.rpcPort}`);
    api = await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
      rpc: rpcs,
    });

    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
  });

  after(async () => node.stop());

  it("load test for rollover", async () => {
    const STAKED_TOKEN_ID = await getNextAssetId(api);
    await finalizeTx(
      alith,
      api.tx.assetsExt.createAsset("testliquiditypools", "TESTLIQUIDITYPOOLS", 6, 1, alith.address),
    );

    const BATCH_SIZE = Number(api.consts.liquidityPools.rolloverBatchSize.toHuman() as number) + 1;

    const stake_amount = 1000;
    const joinPoolAmount = 100;

    // 10 users
    const users = Array.from({ length: 10 }, () =>
      new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(Wallet.createRandom().privateKey)),
    );

    // fund test users
    const txs = [
      // mint stake token
      ...users.map((user) => api.tx.assets.mint(STAKED_TOKEN_ID, user.address, stake_amount)),
      // mint gas token
      ...users.map((user) => api.tx.assets.mint(GAS_TOKEN_ID, user.address, 100_000_000)),
      // transfer root to test users
      ...users.map((user) => api.tx.balances.transfer(user.address, stake_amount)),
    ];
    await finalizeTx(alith, api.tx.utility.batch(txs));
    console.log(`${users.length} test users funded`);

    // pool 1
    const pool1 = await api.query.liquidityPools.nextPoolId();
    const interestRate = 1_000_000;
    const maxTokens = 1000_000;
    const blockDuration = 4000;
    const intervalBlock = 5;
    let startBlock = intervalBlock + Number((await api.rpc.chain.getHeader()).number);
    const rewardPeriod = Math.ceil(users.length / BATCH_SIZE) + intervalBlock;
    let endBlock = startBlock + rewardPeriod;

    // create pool
    await finalizeTx(
      alith,
      api.tx.liquidityPools.createPool(NATIVE_TOKEN_ID, STAKED_TOKEN_ID, interestRate, maxTokens, startBlock, endBlock),
    );
    console.log(`pool ${pool1} created`);

    // all users join pool
    await Promise.all(users.map((user) => finalizeTx(user, api.tx.liquidityPools.enterPool(pool1, joinPoolAmount))));
    console.log(`${users.length} test users joined pool ${pool1}`);

    // verify pool info's lock amount
    const pool1Info = ((await api.query.liquidityPools.pools(pool1)) as any).unwrap();
    expect(pool1Info.lockedAmount.toNumber()).to.equal(users.length * joinPoolAmount);
    console.log(`pool ${pool1} locked amount verified`);

    // get next pool id
    const pool2 = await api.query.liquidityPools.nextPoolId();

    startBlock = endBlock + 1;
    endBlock = startBlock + rewardPeriod;
    // create pool
    await finalizeTx(
      alith,
      api.tx.liquidityPools.createPool(NATIVE_TOKEN_ID, STAKED_TOKEN_ID, interestRate, maxTokens, startBlock, endBlock),
    );
    console.log(`pool ${pool2} created`);

    // set successor pool
    await finalizeTx(alith, api.tx.liquidityPools.setPoolSuccession(pool1, pool2));
    console.log(`pool ${pool2} set as successor of pool ${pool1}`);

    // set pool rollover preference for users
    await Promise.all(users.map((user) => finalizeTx(user, api.tx.liquidityPools.setPoolRollover(pool1, true))));
    console.log(`${users.length} test users set pool rollover preference for pool ${pool2}`);

    console.log(`waiting for start block ${startBlock}...`);
    await sleep(intervalBlock * 2 * blockDuration);
    console.log(`waited for start block ${startBlock}`);

    // wait for reward period
    console.log(`waiting for reward period ${rewardPeriod} blocks...`);
    await sleep(rewardPeriod * blockDuration);
    console.log(`waited for reward period ${rewardPeriod} blocks`);

    // verify successor pool's lock amount
    const pool2Info = ((await api.query.liquidityPools.pools(pool2)) as any).unwrap();
    expect(pool2Info.lockedAmount.toNumber()).to.equal(users.length * joinPoolAmount); // TODO: fix
    console.log(`pool ${pool2} locked amount verified`);

    // record user balances before claiming reward
    const userBalancesBefore = await Promise.all(
      users.map(async (user) => {
        const userBalance: any = (await api.query.system.account(user.address)).toJSON();
        return userBalance?.data?.free;
      }),
    );

    // claim reward for pool 1 & 2
    await Promise.all(users.map((user) => finalizeTx(user, api.tx.liquidityPools.claimReward(pool1))));
    console.log(`${users.length} test users claimed reward for pool ${pool1}`);

    await Promise.all(users.map((user) => finalizeTx(user, api.tx.liquidityPools.claimReward(pool2))));
    console.log(`${users.length} test users claimed reward for pool ${pool2}`);

    // verify reward
    const userBalancesAfter = await Promise.all(
      users.map(async (user) => {
        const userBalance: any = (await api.query.system.account(user.address)).toJSON();
        return userBalance?.data?.free;
      }),
    );

    for (let i = 0; i < users.length; i++) {
      expect(userBalancesAfter[i]).to.equal(joinPoolAmount * 2 + userBalancesBefore[i]);
    }
    console.log(`test users reward verified`);
  });
});
