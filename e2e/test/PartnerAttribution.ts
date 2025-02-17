import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Wallet } from "ethers";

import {
  ALITH_PRIVATE_KEY,
  GAS_TOKEN_ID,
  NodeProcess,
  finalizeTx,
  futurepassAddress,
  startNode,
  typedefs,
} from "../common";

describe("Partner Attribution", () => {
  let node: NodeProcess;

  let api: ApiPromise;
  let alith: KeyringPair;

  before(async () => {
    node = await startNode();

    // substrate variables
    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    alith = new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
  });

  after(async () => await node.stop());

  it("futurepass attribution success", async () => {
    // create and fund partner account
    const partnerAccount = new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(Wallet.createRandom().privateKey));
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, partnerAccount.address, 10_000_000));

    // get next partner id (to be created by registerPartnerAccount)
    const partnerId = +(await api.query.partnerAttribution.nextPartnerId());

    // create futurepass for random user
    const user = new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(Wallet.createRandom().privateKey));
    await finalizeTx(alith, api.tx.futurepass.create(user.address));

    // fund the futurepass account
    const futurepassAddress = (await api.query.futurepass.holders(user.address)).toString();
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, futurepassAddress, 10_000_000));

    // create a partner
    await finalizeTx(partnerAccount, api.tx.partnerAttribution.registerPartnerAccount(partnerAccount.address));

    // ensure partner is created
    const partner = await api.query.partnerAttribution.partners(partnerId);
    expect(partner.toJSON()).to.deep.equal({
      owner: partnerAccount.address,
      account: partnerAccount.address,
      feePercentage: null,
      accumulatedFees: 0,
    });

    // attribute futurepass to partner
    const innerCall = api.tx.partnerAttribution.attributeAccount(partnerId);
    await finalizeTx(user, api.tx.futurepass.proxyExtrinsic(futurepassAddress, innerCall));

    // validate futurepass attribution
    const attributedPartnerId = await api.query.partnerAttribution.attributions(futurepassAddress);
    expect(+attributedPartnerId.toString()).to.equal(partnerId);
  });

  it("create and attribute futurepass with partner", async () => {
    // create and fund partner account
    const partnerAccount = new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(Wallet.createRandom().privateKey));
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, partnerAccount.address, 10_000_000));

    // get next partner id (to be created by registerPartnerAccount)
    const partnerId = +(await api.query.partnerAttribution.nextPartnerId());

    // create a partner
    await finalizeTx(partnerAccount, api.tx.partnerAttribution.registerPartnerAccount(partnerAccount.address));

    // ensure partner is created
    const partner = await api.query.partnerAttribution.partners(partnerId);
    expect(partner.toJSON()).to.deep.equal({
      owner: partnerAccount.address,
      account: partnerAccount.address,
      feePercentage: null,
      accumulatedFees: 0,
    });

    const nextFuturepassId = +(await api.query.futurepass.nextFuturepassId());
    const expectedFuturepassAddress = futurepassAddress(nextFuturepassId);

    // create and attribute futurepass to partner
    const user = new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(Wallet.createRandom().privateKey));
    await finalizeTx(alith, api.tx.partnerAttribution.createFuturepassWithPartner(partnerId, user.address));

    // validate futurepass attribution
    const attributedPartnerId = await api.query.partnerAttribution.attributions(expectedFuturepassAddress);
    expect(+attributedPartnerId.toString()).to.equal(partnerId);
  });

  it("batch create futurepass accounts and attribute to partner", async () => {
    // create and fund partner account
    const partnerAccount = new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(Wallet.createRandom().privateKey));
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, partnerAccount.address, 10_000_000));

    // get next partner id (to be created by registerPartnerAccount)
    const partnerId = +(await api.query.partnerAttribution.nextPartnerId());

    // create a partner
    await finalizeTx(partnerAccount, api.tx.partnerAttribution.registerPartnerAccount(partnerAccount.address));

    // ensure partner is created
    const partner = await api.query.partnerAttribution.partners(partnerId);
    expect(partner.toJSON()).to.deep.equal({
      owner: partnerAccount.address,
      account: partnerAccount.address,
      feePercentage: null,
      accumulatedFees: 0,
    });

    // create 100 accounts
    const users = Array.from({ length: 100 }, () =>
      new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(Wallet.createRandom().privateKey)),
    );
    const nextFuturepassId = +(await api.query.futurepass.nextFuturepassId());
    const futurepassAddresses = users.map((_, i) => futurepassAddress(nextFuturepassId + i));

    // create futurepass for each user
    const txs = users.map((user) => api.tx.partnerAttribution.createFuturepassWithPartner(partnerId, user.address));
    await finalizeTx(alith, api.tx.utility.batch(txs));

    // validate multiple futurepass attribution
    const firstAttributedPartnerId = await api.query.partnerAttribution.attributions(futurepassAddresses[0]);
    expect(+firstAttributedPartnerId.toString()).to.equal(partnerId);

    const lastAttributedPartnerId = await api.query.partnerAttribution.attributions(
      futurepassAddresses[futurepassAddresses.length - 1],
    );
    expect(+lastAttributedPartnerId.toString()).to.equal(partnerId);
  });
});
