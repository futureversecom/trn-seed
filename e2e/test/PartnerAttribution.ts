import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Wallet } from "ethers";

import { ALITH_PRIVATE_KEY, GAS_TOKEN_ID, NodeProcess, finalizeTx, startNode, typedefs } from "../common";

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
});
