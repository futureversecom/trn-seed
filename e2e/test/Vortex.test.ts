import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { SubmittableExtrinsic } from "@polkadot/api/types";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import fs from "fs";
import path from "path";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  GAS_TOKEN_ID,
  NodeProcess,
  finalizeTx,
  getNextAssetId,
  loadTestUsers,
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

    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.wsPort}`);
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

    // simulate vortex distribution for load test with 2 parellel distributions
    // each distribution has 5000 users
    // and would redeem afterwards.
    // There will be 2 same vortex distributions happening right after.
    let txs = [api.tx.assets.mint(VORTEX_ID, alith.address, mintAmount)];
    await finalizeTx(alith, api.tx.utility.batch(txs));
    txs = [
      api.tx.assets.mint(TOKEN_ID_1, alith.address, mintAmount),
      api.tx.assets.mint(TOKEN_ID_2, alith.address, mintAmount),
      api.tx.assets.mint(TOKEN_ID_3, alith.address, mintAmount),
      api.tx.assets.mint(TOKEN_ID_4, alith.address, mintAmount),
    ];
    await finalizeTx(alith, api.tx.utility.batch(txs));

    txs = [api.tx.assets.mint(VORTEX_ID, bob.address, mintAmount)];
    await finalizeTx(alith, api.tx.utility.batch(txs));
    txs = [
      api.tx.assets.mint(TOKEN_ID_1, bob.address, mintAmount),
      api.tx.assets.mint(TOKEN_ID_2, bob.address, mintAmount),
      api.tx.assets.mint(TOKEN_ID_3, bob.address, mintAmount),
      api.tx.assets.mint(TOKEN_ID_4, bob.address, mintAmount),
    ];
    await finalizeTx(alith, api.tx.utility.batch(txs));

    // create vortex distribution
    const vortexDistributionId1 = await api.query.vortexDistribution.nextVortexId();
    console.log(`vortexDistributionId1: ${vortexDistributionId1}`);
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.vortexDistribution.createVtxDist(1_000_000)));
    const vortexDistributionId2 = await api.query.vortexDistribution.nextVortexId();
    console.log(`vortexDistributionId2: ${vortexDistributionId2}`);
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.vortexDistribution.createVtxDist(1_000_000)));

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
          ],
          vortexDistributionId1,
        ),
      ),
    );
    await finalizeTx(
      alith,
      api.tx.sudo.sudo(
        api.tx.vortexDistribution.setAssetPrices(
          [
            [TOKEN_ID_1, 100],
            [TOKEN_ID_2, 200],
            [TOKEN_ID_3, 200],
            [TOKEN_ID_4, 200],
          ],
          vortexDistributionId2,
        ),
      ),
    );

    // load test users
    const users = loadTestUsers();

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
    rewardPairs = [];
    for (let i = 0; i < users.length; i++) {
      rewardPairs.push([users[i].address, 100]);
      if (txs.length >= batchSize || i === users.length - 1) {
        await finalizeTx(
          alith,
          api.tx.sudo.sudo(api.tx.vortexDistribution.registerRewards(vortexDistributionId2, rewardPairs)),
        );
        rewardPairs = [];
      }
    }
    console.log(`registered rewards for ${users.length} users`);

    // trigger distribution
    await finalizeTx(
      alith,
      api.tx.sudo.sudo(
        api.tx.vortexDistribution.triggerVtxDistribution(1, 1, alith.address, alith.address, vortexDistributionId1),
      ),
    );
    console.log(`triggered distribution for ${users.length} users ${alith.address}`);
    await finalizeTx(
      alith,
      api.tx.sudo.sudo(
        api.tx.vortexDistribution.triggerVtxDistribution(1, 1, bob.address, bob.address, vortexDistributionId2),
      ),
    );
    console.log(`triggered distribution for ${users.length} users ${bob.address}`);

    // kick off distribution
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.vortexDistribution.startVtxDist(vortexDistributionId1)));
    console.log(`started distribution for ${users.length} users`);
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.vortexDistribution.startVtxDist(vortexDistributionId2)));
    console.log(`started distribution for ${users.length} users`);

    const blockLength =
      (Math.ceil(users.length / (api.consts.vortexDistribution.payoutBatchSize.toHuman() as number)) + 1) *
      (api.consts.vortexDistribution.unsignedInterval.toHuman() as number) *
      3;
    await sleep(blockLength * 4000);
    console.log(`waited ${blockLength} blocks`);

    // verify vortex balance
    for (const user of users) {
      const vortexBalance = (await api.query.assets.account(VORTEX_ID, user.address)).toJSON() as {
        balance: number;
      };
      expect(vortexBalance.balance).to.equal(200);
    }
    console.log(`verified vortex balance for ${users.length} users`);

    // There will be 2 same vortex distributions happening right after.
    txs = [api.tx.assets.mint(VORTEX_ID, alith.address, mintAmount)];
    await finalizeTx(alith, api.tx.utility.batch(txs));
    txs = [
      api.tx.assets.mint(TOKEN_ID_1, alith.address, mintAmount),
      api.tx.assets.mint(TOKEN_ID_2, alith.address, mintAmount),
      api.tx.assets.mint(TOKEN_ID_3, alith.address, mintAmount),
      api.tx.assets.mint(TOKEN_ID_4, alith.address, mintAmount),
    ];
    await finalizeTx(alith, api.tx.utility.batch(txs));

    txs = [api.tx.assets.mint(VORTEX_ID, bob.address, mintAmount)];
    await finalizeTx(alith, api.tx.utility.batch(txs));
    txs = [
      api.tx.assets.mint(TOKEN_ID_1, bob.address, mintAmount),
      api.tx.assets.mint(TOKEN_ID_2, bob.address, mintAmount),
      api.tx.assets.mint(TOKEN_ID_3, bob.address, mintAmount),
      api.tx.assets.mint(TOKEN_ID_4, bob.address, mintAmount),
    ];
    await finalizeTx(alith, api.tx.utility.batch(txs));

    // create vortex distribution
    const vortexDistributionId3 = await api.query.vortexDistribution.nextVortexId();
    console.log(`vortexDistributionId3: ${vortexDistributionId3}`);
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.vortexDistribution.createVtxDist(1_000_000)));
    const vortexDistributionId4 = await api.query.vortexDistribution.nextVortexId();
    console.log(`vortexDistributionId4: ${vortexDistributionId4}`);
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.vortexDistribution.createVtxDist(1_000_000)));

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
          ],
          vortexDistributionId3,
        ),
      ),
    );
    await finalizeTx(
      alith,
      api.tx.sudo.sudo(
        api.tx.vortexDistribution.setAssetPrices(
          [
            [TOKEN_ID_1, 100],
            [TOKEN_ID_2, 200],
            [TOKEN_ID_3, 200],
            [TOKEN_ID_4, 200],
          ],
          vortexDistributionId4,
        ),
      ),
    );

    // register rewards
    rewardPairs = [];
    for (let i = 0; i < users.length; i++) {
      rewardPairs.push([users[i].address, 100]);
      if (txs.length >= batchSize || i === users.length - 1) {
        await finalizeTx(
          alith,
          api.tx.sudo.sudo(api.tx.vortexDistribution.registerRewards(vortexDistributionId3, rewardPairs)),
        );
        rewardPairs = [];
      }
    }
    console.log(`registered rewards for ${users.length} users`);
    rewardPairs = [];
    for (let i = 0; i < users.length; i++) {
      rewardPairs.push([users[i].address, 100]);
      if (txs.length >= batchSize || i === users.length - 1) {
        await finalizeTx(
          alith,
          api.tx.sudo.sudo(api.tx.vortexDistribution.registerRewards(vortexDistributionId4, rewardPairs)),
        );
        rewardPairs = [];
      }
    }
    console.log(`registered rewards for ${users.length} users`);

    // trigger distribution
    await finalizeTx(
      alith,
      api.tx.sudo.sudo(
        api.tx.vortexDistribution.triggerVtxDistribution(1, 1, alith.address, alith.address, vortexDistributionId3),
      ),
    );
    console.log(`triggered distribution for ${users.length} users ${alith.address}`);
    await finalizeTx(
      alith,
      api.tx.sudo.sudo(
        api.tx.vortexDistribution.triggerVtxDistribution(1, 1, bob.address, bob.address, vortexDistributionId4),
      ),
    );
    console.log(`triggered distribution for ${users.length} users ${bob.address}`);

    // kick off distribution
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.vortexDistribution.startVtxDist(vortexDistributionId3)));
    console.log(`started distribution for ${users.length} users`);
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.vortexDistribution.startVtxDist(vortexDistributionId4)));
    console.log(`started distribution for ${users.length} users`);

    await sleep(blockLength * 4000);
    console.log(`waited ${blockLength} blocks`);

    // verify vortex balance
    for (const user of users) {
      const vortexBalance = (await api.query.assets.account(VORTEX_ID, user.address)).toJSON() as {
        balance: number;
      };
      expect(vortexBalance.balance).to.equal(400);
    }
    console.log(`verified vortex balance for ${users.length} users`);

    const blockDuration = 4000;
    for (let i = 0; i < users.length; i++) {
      const user = users[i];
      api.tx.vortexDistribution.redeemTokensFromVault(vortexDistributionId1, 100).signAndSend(user);
      api.tx.vortexDistribution.redeemTokensFromVault(vortexDistributionId2, 100).signAndSend(user);
      api.tx.vortexDistribution.redeemTokensFromVault(vortexDistributionId3, 100).signAndSend(user);
      api.tx.vortexDistribution.redeemTokensFromVault(vortexDistributionId4, 100).signAndSend(user);

      if (i % batchSize === 0 || i === users.length - 1) {
        await sleep(blockDuration * 4);
      }
    }

    console.log(`redeemed tokens for ${users.length} users`);

    const token1Asset: any = (await api.query.assets.asset(TOKEN_ID_1)).toJSON();
    const token2Asset: any = (await api.query.assets.asset(TOKEN_ID_2)).toJSON();
    const token3Asset: any = (await api.query.assets.asset(TOKEN_ID_3)).toJSON();
    const token4Asset: any = (await api.query.assets.asset(TOKEN_ID_4)).toJSON();

    console.log(
      `token1Asset.supply: ${token1Asset.supply}, token2Asset.supply: ${token2Asset.supply}, token3Asset.supply: ${token3Asset.supply}, token4Asset.supply: ${token4Asset.supply}`,
    );

    for (let i = 0; i < users.length; i++) {
      const user = users[i];

      const expectedToken1RedeemedAmount = (100 * mintAmount * 2) / token1Asset.supply;
      const expectedToken2RedeemedAmount = (100 * mintAmount * 2) / token2Asset.supply;
      const expectedToken3RedeemedAmount = (100 * mintAmount * 2) / token3Asset.supply;
      const expectedToken4RedeemedAmount = (100 * mintAmount * 2) / token4Asset.supply;
      // potential friction lost
      const potential_friction_lost = 10;

      // check withdraw balance
      const token1BalanceAfter = (await api.query.assets.account(TOKEN_ID_1, user.address)).toJSON() as {
        balance: number;
      };
      expect(token1BalanceAfter.balance).to.closeTo(expectedToken1RedeemedAmount, potential_friction_lost);

      const token2BalanceAfter = (await api.query.assets.account(TOKEN_ID_2, user.address)).toJSON() as {
        balance: number;
      };
      expect(token2BalanceAfter.balance).to.closeTo(expectedToken2RedeemedAmount, potential_friction_lost);

      const token3BalanceAfter = (await api.query.assets.account(TOKEN_ID_3, user.address)).toJSON() as {
        balance: number;
      };
      expect(token3BalanceAfter.balance).to.closeTo(expectedToken3RedeemedAmount, potential_friction_lost);

      const token4BalanceAfter = (await api.query.assets.account(TOKEN_ID_4, user.address)).toJSON() as {
        balance: number;
      };
      expect(token4BalanceAfter.balance).to.closeTo(expectedToken4RedeemedAmount, potential_friction_lost);
    }
    console.log(`redeemed tokens and verified for ${users.length} users for 4 distributions`);
  });
});
