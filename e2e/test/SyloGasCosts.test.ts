import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Wallet } from "ethers";
import { xit } from "mocha";

import { ALITH_PRIVATE_KEY, GAS_TOKEN_ID, NodeProcess, finalizeTx, startNode, typedefs } from "../common";

describe("Sylo", () => {
  let node: NodeProcess;
  let api: ApiPromise;
  let keyring: Keyring;
  let alith: KeyringPair;
  let userPrivateKey: string;
  let user: KeyringPair;

  const FEE_TOKEN_ASSET_ID = 1124;

  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });

    keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    userPrivateKey = Wallet.createRandom().privateKey;
    user = keyring.addFromSeed(hexToU8a(userPrivateKey));

    // add liquidity for XRP/SYLO token and set up user funds
    const txs = [
      api.tx.assetsExt.createAsset("sylo", "SYLO", 18, 1, alith.address),
      api.tx.assets.mint(FEE_TOKEN_ASSET_ID, alith.address, 2_000_000_000_000_000),
      api.tx.assets.mint(FEE_TOKEN_ASSET_ID, user.address, 2_000_000_000_000_000),
      api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 1), // avoids xrp balance increase due to preservation rules
      api.tx.dex.addLiquidity(
        FEE_TOKEN_ASSET_ID,
        GAS_TOKEN_ID,
        100_000_000_000,
        100_000_000_000,
        100_000_000_000,
        100_000_000_000,
        null,
        null,
      ),
    ];
    await finalizeTx(alith, api.tx.utility.batch(txs));

    // set payment asset
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.sylo.setPaymentAsset(FEE_TOKEN_ASSET_ID)));

    console.log("liquidity setup complete...");
  });

  after(async () => node.stop());

  // A set of sylo extrinsics to test, where each extrinsic should be paid for
  // using sylo tokens
  const createSyloExtrinsics = (api: ApiPromise) => [
    api.tx.sylo.registerResolver("id", ["endpoint"]),
    api.tx.sylo.updateResolver("id", ["endpoint-2"]),
    api.tx.sylo.deregisterResolver("id"),
    api.tx.sylo.createValidationRecord(
      "data-id",
      [{ method: "sylo-resolver", identifier: "id" }],
      "data-type",
      ["tag"],
      "0x0000000000000000000000000000000000000000000000000000000000000000",
    ),
    api.tx.sylo.addValidationRecordEntry(
      "data-id",
      "0x0000000000000000000000000000000000000000000000000000000000000000",
    ),
    api.tx.sylo.updateValidationRecord("data-id", [{ method: "sylo-resolver", identifier: "id-2" }], "data-type-2", [
      "tag-2",
    ]),
    api.tx.sylo.deleteValidationRecord("data-id"),
  ];

  it("can submit sylo extrinsic and pay with sylo tokens", async () => {
    const calls = createSyloExtrinsics(api);

    for (const call of calls) {
      console.log("testing call", call.meta.name.toString());

      const userXRPBalanceBefore =
        ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
      const userSyloBalanceBefore =
        ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, user.address)).toJSON() as any)?.balance ?? 0;

      await finalizeTx(user, call);

      // verify balances updated
      const userXRPBalanceAfter =
        ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
      const userSyloBalanceAfter =
        ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, user.address)).toJSON() as any)?.balance ?? 0;

      expect(userXRPBalanceAfter).to.be.eq(userXRPBalanceBefore);
      expect(userSyloBalanceAfter).to.be.lessThan(userSyloBalanceBefore);
    }
  });

  it("can submit sylo extrinsic with futurepass", async () => {
    // create a random user A
    const userPrivateKey = Wallet.createRandom().privateKey;
    const user: KeyringPair = keyring.addFromSeed(hexToU8a(userPrivateKey));

    // create a futurepass for user
    await finalizeTx(alith, api.tx.futurepass.create(user.address));

    // fund the futurepass account
    const futurepassAddress = (await api.query.futurepass.holders(user.address)).toString();
    await finalizeTx(alith, api.tx.assets.transfer(FEE_TOKEN_ASSET_ID, futurepassAddress, 100_000_000)); // gas
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, futurepassAddress, 1)); // preservation rules

    const calls = createSyloExtrinsics(api);

    for (const call of calls) {
      console.log("testing call", call.meta.name.toString());

      const userXRPBalanceBefore =
        ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
      const userSyloBalanceBefore =
        ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, user.address)).toJSON() as any)?.balance ?? 0;

      const fpXRPBalanceBefore =
        ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
      const fpSyloBalanceBefore =
        ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;

      const futurepassCall = api.tx.futurepass.proxyExtrinsic(futurepassAddress, call);

      await finalizeTx(user, futurepassCall);

      const userXRPBalanceAfter =
        ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
      const userSyloBalanceAfter =
        ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, user.address)).toJSON() as any)?.balance ?? 0;

      const fpXRPBalanceAfter =
        ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
      const fpSyloBalanceAfter =
        ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;

      // validate the futurepass's token balance has decreased, and the user's asset
      // balance remains the same
      expect(userXRPBalanceAfter).to.be.eq(userXRPBalanceBefore);
      expect(userSyloBalanceAfter).to.be.eq(userSyloBalanceBefore);

      expect(fpXRPBalanceAfter).to.be.eq(fpXRPBalanceBefore);
      expect(fpSyloBalanceAfter).to.be.lt(fpSyloBalanceBefore);
    }
  });

  // Failures to pay for extrinsics will hang, so failures tests are disabled.
  // Enable and run these tests manually to verify fee swap behaviour.
  xit("fails to submit without sylo tokens available", async () => {
    // create a new user
    const userPrivateKey = Wallet.createRandom().privateKey;
    const user = keyring.addFromSeed(hexToU8a(userPrivateKey));

    // ensure user has enough xrp to submit regular extrinsics
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 100_000_000));

    await finalizeTx(user, api.tx.sylo.registerResolver("id", ["endpoint"]));
  });

  xit("fails to submit when wrapping sylo exstrinsic in fee-proxy call", async () => {
    // create a new user
    const userPrivateKey = Wallet.createRandom().privateKey;
    const user = keyring.addFromSeed(hexToU8a(userPrivateKey));

    // ensure user has enough xrp to submit regular extrinsics
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 100_000_000));

    const syloCall = api.tx.sylo.registerResolver("id", ["endpoint"]);

    const next_fee_token_id = 2148;

    // add liquidity for XRP/SYLO token and set up user funds
    const txs = [
      api.tx.assetsExt.createAsset("sylo-new", "SYLO-NEW", 18, 1, alith.address),
      api.tx.assets.mint(next_fee_token_id, user.address, 2_000_000_000_000_000),
      api.tx.dex.addLiquidity(
        next_fee_token_id,
        GAS_TOKEN_ID,
        100_000_000_000,
        100_000_000_000,
        100_000_000_000,
        100_000_000_000,
        null,
        null,
      ),
    ];
    await finalizeTx(alith, api.tx.utility.batch(txs));

    const maxTokenPayment = 5_000_000;

    await finalizeTx(user, api.tx.feeProxy.callWithFeePreferences(next_fee_token_id, maxTokenPayment, syloCall));
  });

  it.only("proxy", async () => {});
});
