import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";

import {
  ALITH_PRIVATE_KEY,
  GAS_TOKEN_ID,
  NodeProcess,
  finalizeTx,
  getNextAssetId,
  loadTestUsers,
  rpcs,
  sleep,
  startNode,
  typedefs,
} from "../../common";

describe("Reward", () => {
  let TOKEN_ID: number;
  let node: NodeProcess;
  let api: ApiPromise;
  let alith: KeyringPair;

  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    // const wsProvider = new WsProvider(`wss://archive.morel.micklelab.xyz/ws`);
    api = await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
      rpc: rpcs,
    });

    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    TOKEN_ID = await getNextAssetId(api);

    const txs = [
      api.tx.assetsExt.createAsset("testincentive", "TESTINCENTIVE", 6, 1, alith.address), // create asset
    ];

    await finalizeTx(alith, api.tx.utility.batch(txs));
  });

  after(async () => node.stop());

  it("load test for rollover", async () => {
    const batchSize = Number(api.consts.incentive.rolloverBatchSize.toHuman() as number) + 1;
    // load test users
    const testUsers = loadTestUsers();

    const amount = 1000;
    const joinPoolAmount = 100;

    // fund test users
    let txs = [];
    for (let i = 0; i < testUsers.length; i++) {
      const user = testUsers[i];
      // transfer ROOT && GAS assets to test users
      txs.push(api.tx.assets.mint(TOKEN_ID, user.address, amount));
      txs.push(api.tx.assets.mint(GAS_TOKEN_ID, user.address, 100_000_000));
      txs.push(api.tx.balances.transfer(user.address, amount));

      if (txs.length >= batchSize || i === testUsers.length - 1) {
        console.log(`funding ${txs.length} test users`);
        await finalizeTx(alith, api.tx.utility.batch(txs));
        txs = [];
      }
    }
    console.log(`${testUsers.length} test users funded`);

    const incentiveVaultForPool1 = process.env.INCENTIVE_VAULT_ADDRESS_POOL_1;
    // fund vault account
    await finalizeTx(
      alith,
      api.tx.utility.batch([api.tx.balances.transfer(incentiveVaultForPool1, testUsers.length * joinPoolAmount)]),
    );
    console.log(`vault account ${incentiveVaultForPool1} funded`);

    const incentiveVaultForPool2 = process.env.INCENTIVE_VAULT_ADDRESS_POOL_2;
    // fund vault account
    await finalizeTx(
      alith,
      api.tx.utility.batch([api.tx.balances.transfer(incentiveVaultForPool2, testUsers.length * joinPoolAmount)]),
    );
    console.log(`vault account ${incentiveVaultForPool2} funded`);

    // pool 1
    const pool1 = await api.query.incentive.nextPoolId();
    const interestRate = 1;
    const maxTokens = 1000_000;
    const blockDuration = 4000;
    const intervalBlock = 10;
    let startBlock = intervalBlock + Number((await api.rpc.chain.getHeader()).number);
    let rewardPeriod = Math.ceil((testUsers.length * 1.5) / batchSize) + 20;
    let endBlock = startBlock + rewardPeriod;

    // create pool
    await finalizeTx(
      alith,
      api.tx.sudo.sudo(api.tx.incentive.createPool(TOKEN_ID, interestRate, maxTokens, startBlock, endBlock)),
    );
    console.log(`pool ${pool1} created`);

    await sleep(intervalBlock * blockDuration);
    console.log(`waited for start block ${startBlock}`);

    // join pool
    for (let i = 0; i < testUsers.length; i++) {
      let user = testUsers[i];
      api.tx.incentive.joinPool(pool1, joinPoolAmount).signAndSend(user);

      if (i % batchSize === 0 || i === testUsers.length - 1) {
        await sleep(2 * blockDuration);
      }
    }
    console.log(`${testUsers.length} test users joined pool ${pool1}`);

    // verify pool info's lock amount
    // @ts-ignore
    const pool1Info = (await api.query.incentive.pools(pool1)).unwrap();
    expect(pool1Info.lockedAmount.toNumber()).to.equal(testUsers.length * joinPoolAmount);
    console.log(`pool ${pool1} locked amount verified`);

    // get next pool id
    const pool2 = await api.query.incentive.nextPoolId();

    startBlock = intervalBlock + Number((await api.rpc.chain.getHeader()).number);
    endBlock = startBlock + rewardPeriod;
    // create pool
    await finalizeTx(
      alith,
      api.tx.sudo.sudo(api.tx.incentive.createPool(TOKEN_ID, interestRate, maxTokens, startBlock, endBlock)),
    );
    console.log(`pool ${pool2} created`);

    // set successor pool
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.incentive.setIncentivePoolSuccession(pool1, pool2)));
    console.log(`pool ${pool2} set as successor of pool ${pool1}`);

    await sleep(intervalBlock * blockDuration);
    console.log(`waited for start block ${startBlock}`);

    // wait for reward period
    await sleep(rewardPeriod * blockDuration);
    console.log(`waited for reward period ${rewardPeriod} blocks`);
    // wait for rollover
    const blockLength =
      (Math.ceil(testUsers.length / batchSize) + 1) * 2 * (api.consts.incentive.unsignedInterval.toHuman() as number);
    await sleep(blockLength * 4000);
    console.log(`waited for rollover ${blockLength} blocks`);

    // verify successor pool's lock amount
    // @ts-ignore
    const pool2Info = (await api.query.incentive.pools(pool2)).unwrap();
    expect(pool2Info.lockedAmount.toNumber()).to.equal(testUsers.length * joinPoolAmount);
    console.log(`pool ${pool2} locked amount verified`);

    // record user balances before claiming reward
    let userBalancesBefore: { [key: string]: number } = {};
    for (const user of testUsers) {
      const userBalance: any = (await api.query.system.account(user.address)).toJSON();
      userBalancesBefore[user.address] = userBalance?.data?.free;
    }

    // claim reward for pool 1 & 2
    for (let i = 0; i < testUsers.length; i++) {
      let user = testUsers[i];
      api.tx.incentive.claimReward(pool1).signAndSend(user);

      if (i % batchSize === 0 || i === testUsers.length - 1) {
        await sleep(blockDuration);
      }
    }
    console.log(`${testUsers.length} test users claimed reward for pool ${pool1}`);
    for (let i = 0; i < testUsers.length; i++) {
      let user = testUsers[i];
      api.tx.incentive.claimReward(pool2).signAndSend(user);

      if (i % batchSize === 0 || i === testUsers.length - 1) {
        await sleep(blockDuration);
      }
    }
    console.log(`${testUsers.length} test users claimed reward for pool ${pool2}`);

    for (const user of testUsers) {
      // verify reward
      const userBalance: any = (await api.query.system.account(user.address)).toJSON();
      expect(userBalance?.data?.free).to.equal(joinPoolAmount * 2 + userBalancesBefore[user.address]);
    }
    console.log(`test users' reward verified`);
  });
});
