import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  GAS_TOKEN_ID,
  NATIVE_TOKEN_ID,
  NodeProcess,
  finalizeTx,
  generateTestUsers,
  getNextAssetId,
  sleep,
  startNode,
  typedefs,
} from "../common";

describe("Vortex Distribution", () => {
  let api: ApiPromise;
  let node: NodeProcess;

  let alith: KeyringPair;
  let bob: KeyringPair;

  let VORTEX_ID: number;
  let TOKEN_ID_1: number;
  let TOKEN_ID_2: number;
  let TOKEN_ID_3: number;
  let TOKEN_ID_4: number;

  let mintAmount: number;

  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    // const wsProvider = new WsProvider(`wss://archive.morel.micklelab.xyz/ws`);
    api = await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
    });

    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));

    mintAmount = 10_000_000;

    VORTEX_ID = api.consts.vortexDistribution.vtxAssetId.toHuman() as number;
    let txs = [
      api.tx.sudo.sudo(api.tx.assets.forceCreate(VORTEX_ID, alith.address, false, 1)),
      api.tx.sudo.sudo(api.tx.assets.forceSetMetadata(VORTEX_ID, "VORTEX", "vortex", 6, false)),
    ];
    await finalizeTx(alith, api.tx.utility.batch(txs));

    TOKEN_ID_1 = await getNextAssetId(api);
    txs = [api.tx.assetsExt.createAsset("test1", "TEST1", 6, 1, alith.address)];
    await finalizeTx(alith, api.tx.utility.batch(txs));

    TOKEN_ID_2 = await getNextAssetId(api);
    txs = [api.tx.assetsExt.createAsset("test2", "TEST2", 6, 1, alith.address)];
    await finalizeTx(alith, api.tx.utility.batch(txs));

    TOKEN_ID_3 = await getNextAssetId(api);
    txs = [api.tx.assetsExt.createAsset("test3", "TEST3", 6, 1, alith.address)];
    await finalizeTx(alith, api.tx.utility.batch(txs));

    TOKEN_ID_4 = await getNextAssetId(api);
    txs = [api.tx.assetsExt.createAsset("test4", "TEST4", 6, 1, alith.address)];
    await finalizeTx(alith, api.tx.utility.batch(txs));
  });

  it("should distribute vortex for load test", async () => {
    const batchSize = Number(api.consts.vortexDistribution.payoutBatchSize.toHuman() as number) + 1;

    let txs = [api.tx.assets.mint(VORTEX_ID, alith.address, mintAmount)];
    await finalizeTx(alith, api.tx.utility.batch(txs));
    const rootVaultAccount = process.env.ROOT_VAULT_ACCOUNT;
    const feeVaultAccount = process.env.FEE_VAULT_ACCOUNT;

    txs = [api.tx.assets.mint(VORTEX_ID, bob.address, mintAmount)];
    await finalizeTx(alith, api.tx.utility.batch(txs));
    txs = [
      api.tx.balances.transfer(rootVaultAccount, mintAmount),
      api.tx.assets.mint(TOKEN_ID_1, feeVaultAccount, mintAmount),
      api.tx.assets.mint(TOKEN_ID_2, feeVaultAccount, mintAmount),
      api.tx.assets.mint(TOKEN_ID_3, feeVaultAccount, mintAmount),
      api.tx.assets.mint(TOKEN_ID_4, feeVaultAccount, mintAmount),
    ];
    await finalizeTx(alith, api.tx.utility.batch(txs));

    // create vortex distribution
    const vortexDistributionId1 = await api.query.vortexDistribution.nextVortexId();
    console.log(`vortexDistributionId1: ${vortexDistributionId1}`);
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.vortexDistribution.createVtxDist()));

    // set asset price
    await finalizeTx(
      alith,
      api.tx.sudo.sudo(
        api.tx.vortexDistribution.setAssetPrices(
          [
            [TOKEN_ID_1, 100],
            [TOKEN_ID_2, 200],
            [TOKEN_ID_3, 200],
            [TOKEN_ID_4, 200],
            [NATIVE_TOKEN_ID, 200],
          ],
          vortexDistributionId1,
        ),
      ),
    );

    // load test users
    const users = generateTestUsers(20);

    // transfer native token to users to create accounts
    txs = [];
    for (let i = 0; i < users.length; i++) {
      txs.push(api.tx.assets.mint(GAS_TOKEN_ID, users[i].address, 100_000_000));
      txs.push(api.tx.balances.transfer(users[i].address, 1000));

      if (txs.length >= batchSize || i === users.length - 1) {
        await finalizeTx(alith, api.tx.utility.batch(txs));
        txs = [];
      }
    }

    // register rewards
    let rewardPairs = [];
    for (let i = 0; i < users.length; i++) {
      rewardPairs.push([users[i].address, 100]);
      if (txs.length >= batchSize || i === users.length - 1) {
        await finalizeTx(
          alith,
          api.tx.sudo.sudo(api.tx.vortexDistribution.registerRewards(vortexDistributionId1, rewardPairs)),
        );
        rewardPairs = [];
      }
    }
    console.log(`registered rewards for ${users.length} users`);

    // trigger distribution
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.vortexDistribution.triggerVtxDistribution(vortexDistributionId1)));
    console.log(`triggered distribution for ${users.length} users ${alith.address}`);

    // kick off distribution
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.vortexDistribution.startVtxDist(vortexDistributionId1)));
    console.log(`started distribution for ${users.length} users`);

    const blockDuration = 4000;
    const blockLength =
      Math.ceil(users.length / (api.consts.vortexDistribution.payoutBatchSize.toHuman() as number)) *
      (api.consts.vortexDistribution.unsignedInterval.toHuman() as number);
    await sleep(blockLength * blockDuration);
    console.log(`waited ${blockLength} blocks`);

    // verify vortex balance
    for (const user of users) {
      const vortexBalance = (await api.query.assets.account(VORTEX_ID, user.address)).toJSON() as {
        balance: number;
      };
      expect(vortexBalance.balance).to.equal(100);
    }
    console.log(`verified vortex balance for ${users.length} users`);

    for (let i = 0; i < users.length; i++) {
      const user = users[i];
      api.tx.vortexDistribution.redeemTokensFromVault(vortexDistributionId1, 100).signAndSend(user);

      if (i % batchSize === 0 || i === users.length - 1) {
        await sleep(blockDuration * 4);
      }
    }
  });
});
