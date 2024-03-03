import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { DispatchError } from "@polkadot/types/interfaces";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { blake256 } from "codechain-primitives";
import { BigNumber, Wallet } from "ethers";
import { computePublicKey } from "ethers/lib/utils";
import { encode, encodeForSigning } from "ripple-binary-codec";
import { deriveAddress, sign } from "ripple-keypairs";

import { ALITH_PRIVATE_KEY, GAS_TOKEN_ID, NATIVE_TOKEN_ID, NodeProcess, finalizeTx, getNextAssetId, nftCollectionIdToCollectionUUID, startNode, stringToHex, typedefs } from "../common";

describe("Crowdsale pallet", () => {
  let node: NodeProcess;

  let api: ApiPromise;
  let alith: KeyringPair;

  before(async () => {
    node = await startNode();

    // substrate variables
    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.wsPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    alith = new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
  });

  after(async () => await node.stop());

  it("crowdsale extrinsic gas fees", async () => {
    // TODO:
  });

  it("distribute crowdsale rewards cannot be called manually", async () => {
    // TODO:

    // try send

    // try signAndSend
  });

  // TODO: use native token (ROOT) instead of creating new token
  it.skip("oversubscribed crowdsale - ROOT", async () => {
    // crowdsale vars

    const paymentAssetId = await getNextAssetId(api);

    const participants = Array.from({ length: 10 }, () => new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(Wallet.createRandom().privateKey))); // crowsale participants (10)
    const maxIssuance = 5; // create nft collection - total supply
    const nextCollectionUuid = nftCollectionIdToCollectionUUID(await api.query.nft.nextCollectionId() as any);
    const nextCrowdsaleId = +(await api.query.crowdsale.nextSaleId());

    let txs = [
      // create new token (crowdsale payment asset)
      api.tx.assetsExt.createAsset("TOKEN1", "T1", 6, 1, alith.address),

      // fund participants - 50 tokens per participant to participate
      ...participants.map((user) => api.tx.assets.mint(paymentAssetId, user.address, 50_000_000)),
      // ...participants.map((user) => api.tx.assets.transfer(paymentAssetId, user.address, 50_000_000)),

      // fund participants - 2 XRP (GAS) per participant
      ...participants.map((user) => api.tx.assets.mint(GAS_TOKEN_ID, user.address, 2_000_000)),

      // create nft collection
      api.tx.nft.createCollection("test", 0, maxIssuance, null, "http://example.com", null, { xrpl: false }),

      // initialize crowdsale as admin
      api.tx.crowdsale.initialize(
        paymentAssetId,
        nextCollectionUuid,
        50_000_000, // 50 root * 5 = 250 root
        2, // 2 blocks ~ 8s
      ),

      // enable crowdsale as admin - will expire in 2 blocks
      api.tx.crowdsale.enable(nextCrowdsaleId),
    ];

    // const extrinsics = await Promise.all([
    //   // crowdsale setup
    //   finalizeTx(alith, api.tx.utility.batch(txs), { tip: 5000 }),

    //   // user participations
    //   // finalizeTx(participants[0], api.tx.crowdsale.participate(nextCrowdsaleId, 50_000_000), { tip: 1000 }),
    //   // ...participants.map((user) => finalizeTx(user, api.tx.crowdsale.participate(nextCrowdsaleId, 50_000_000), { tip: 1000 })),
    // ]);
    // extrinsics.forEach((events) => events.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`)));

    const events = await finalizeTx(alith, api.tx.utility.batch(txs));
    events.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    // TODO: assert events

    let saleInfo: any = (await api.query.crowdsale.saleInfo(nextCrowdsaleId)).toJSON();
    expect(saleInfo.status).to.haveOwnProperty("enabled");

    // user participations in crowdsale - with all root tokens
    const participationEvents = await Promise.all(
      participants.map((user) => finalizeTx(user, api.tx.crowdsale.participate(nextCrowdsaleId, 50_000_000)))
    );
    participationEvents.forEach((pEvents) => {
      pEvents.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`))
      // TODO: assert pEvents
    });

    // assert all participants token balances are 0
    const userTokenBalances = await Promise.all(
      participants.map(async (user) => ((await api.query.assets.account(paymentAssetId, user.address)).toJSON() as any)?.balance ?? 0)
    );
    expect(userTokenBalances).to.deep.equal(Array(participants.length).fill(0));

    saleInfo = (await api.query.crowdsale.saleInfo(nextCrowdsaleId)).toJSON();
    expect(saleInfo.fundsRaised).to.equal(500_000_000); // 10 participants * 50_000_000 tokens each

    // half of participants manually claim vouchers
    const claimEvents = await Promise.all(
      participants.slice(0, participants.length / 2).map((user) => finalizeTx(user, api.tx.crowdsale.claimVoucher(nextCrowdsaleId)))
    );
    claimEvents.forEach((cEvents, index) => {
      // cEvents.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`))
      // crowdsale       CrowdsaleVouchersClaimed        [42,"0x69aC6Da40afD256B96356c220F3B04E4246a4423",500_000]
      expect(cEvents[2].event.section).to.equal("crowdsale");
      expect(cEvents[2].event.method).to.equal("CrowdsaleVouchersClaimed");
      expect(cEvents[2].event.data[0]).to.equal(nextCrowdsaleId);
      expect(cEvents[2].event.data[1].toString()).to.equal(participants[index].address);
      expect(cEvents[2].event.data[2]).to.equal(500_000); // 0.5 vouchers
    });

    // assert half participants have vouchers
    const voucherAssetId = saleInfo.voucherAssetId;
    let userVoucherBalances = await Promise.all(
      participants.map(async (user) => ((await api.query.assets.account(voucherAssetId, user.address)).toJSON() as any)?.balance ?? 0)
    );
    expect(userVoucherBalances).to.deep.equal([ ...Array(participants.length / 2).fill(500_000), ...Array(participants.length / 2).fill(0)]);

    // wait for sale to reach end block
    const saleEndBlock = saleInfo.status.enabled + saleInfo.duration;
    await new Promise<void>((resolve) => setInterval(async () => {
      if ((await api.query.system.number()).toNumber() > saleEndBlock + 1) resolve();
    }, 500));

    saleInfo = (await api.query.crowdsale.saleInfo(nextCrowdsaleId)).toJSON();
    expect(saleInfo.status).to.haveOwnProperty("ended");

    // assert all participants have vouchers
    userVoucherBalances = await Promise.all(
      participants.map(async (user) => ((await api.query.assets.account(voucherAssetId, user.address)).toJSON() as any)?.balance ?? 0)
    );
    expect(userVoucherBalances).to.deep.equal(Array(participants.length).fill(500_000));

    // redeeming single NFT should fail (oversubscribed by 2x)
    const dispatchError = await new Promise<DispatchError>((resolve, reject) => {
      api.tx.crowdsale
        .redeemVoucher(nextCrowdsaleId, 1)
        .signAndSend(participants[0], ({ status, dispatchError }) => {
          if (!status.isFinalized) return;
          if (dispatchError === undefined) return;
          resolve(dispatchError);
        })
        .catch((err) => reject(err));
    });
    const { section, name } = dispatchError.registry.findMetaError(dispatchError.asModule);
    expect(section).to.equal("assets");
    expect(name).to.equal("BalanceLow");

    // transfer vouchers from one user to another (to make whole)
    await finalizeTx(participants[1], api.tx.assets.transfer(voucherAssetId, participants[0].address, 500_000));
    const rEvents = await finalizeTx(participants[0], api.tx.crowdsale.redeemVoucher(nextCrowdsaleId, 1));

    // rEvents.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    // nft Mint [6244,0,0,"0x1497Fa38AbB70b6fF8227E387De60F5600bb97ef"]
    // crowdsale CrowdsaleNFTRedeemed [5,"0x1497Fa38AbB70b6fF8227E387De60F5600bb97ef",6244,1]
    expect(rEvents[2].event.section).to.equal("nft");
    expect(rEvents[2].event.method).to.equal("Mint");
    expect(rEvents[3].event.section).to.equal("crowdsale");
    expect(rEvents[3].event.method).to.equal("CrowdsaleNFTRedeemed");
    expect(rEvents[3].event.data[0]).to.equal(nextCrowdsaleId);
    expect(rEvents[3].event.data[1].toString()).to.equal(participants[0].address);
    expect(rEvents[3].event.data[3]).to.equal(1); // qty redeemed
  });

  it("oversubscribed crowdsale", async () => {
    // crowdsale vars

    const paymentAssetId = await getNextAssetId(api);

    const participants = Array.from({ length: 10 }, () => new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(Wallet.createRandom().privateKey))); // crowsale participants (10)
    const maxIssuance = 5; // create nft collection - total supply
    const nextCollectionUuid = nftCollectionIdToCollectionUUID(await api.query.nft.nextCollectionId() as any);
    const nextCrowdsaleId = +(await api.query.crowdsale.nextSaleId());

    let txs = [
      // create new token (crowdsale payment asset)
      api.tx.assetsExt.createAsset("TOKEN1", "T1", 6, 1, alith.address),

      // fund participants - 50 tokens per participant to participate
      ...participants.map((user) => api.tx.assets.mint(paymentAssetId, user.address, 50_000_000)),
      // ...participants.map((user) => api.tx.assets.transfer(paymentAssetId, user.address, 50_000_000)),

      // fund participants - 2 XRP (GAS) per participant
      ...participants.map((user) => api.tx.assets.mint(GAS_TOKEN_ID, user.address, 2_000_000)),

      // create nft collection
      api.tx.nft.createCollection("test", 0, maxIssuance, null, "http://example.com", null, { xrpl: false }),

      // initialize crowdsale as admin
      api.tx.crowdsale.initialize(
        paymentAssetId,
        nextCollectionUuid,
        50_000_000, // 50 root * 5 = 250 root
        2, // 2 blocks ~ 8s
      ),

      // enable crowdsale as admin - will expire in 2 blocks
      api.tx.crowdsale.enable(nextCrowdsaleId),
    ];

    const events = await finalizeTx(alith, api.tx.utility.batch(txs));
    // events.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    // crowdsale       CrowdsaleCreated        [10,{"status":{"pending":200},"admin":"0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac","vault":"0x224814c8cf1e0dA5618Df8870F8c9F5700820a5E","paymentAssetId":21604,"rewardCollectionId":11364,"softCapPrice":50000000,"fundsRaised":0,"voucherAssetId":22628,"duration":2}]
    expect(events[events.length-8].event.section).to.equal("crowdsale");
    expect(events[events.length-8].event.method).to.equal("CrowdsaleCreated");
    expect(events[events.length-8].event.data[0]).to.equal(nextCrowdsaleId);
    expect(events[events.length-8].event.data[1].toJSON().status).to.haveOwnProperty("pending");
    expect(events[events.length-8].event.data[1].toJSON().admin).to.equal(alith.address);
    expect(events[events.length-8].event.data[1].toJSON().paymentAssetId).to.equal(paymentAssetId);

    // crowdsale       CrowdsaleEnabled        [10,{"status":{"enabled":200},"admin":"0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac","vault":"0x224814c8cf1e0dA5618Df8870F8c9F5700820a5E","paymentAssetId":21604,"rewardCollectionId":11364,"softCapPrice":50000000,"fundsRaised":0,"voucherAssetId":22628,"duration":2},202]
    expect(events[events.length-6].event.section).to.equal("crowdsale");
    expect(events[events.length-6].event.method).to.equal("CrowdsaleEnabled");
    expect(events[events.length-6].event.data[0]).to.equal(nextCrowdsaleId);
    expect(events[events.length-6].event.data[1].toJSON().status).to.haveOwnProperty("enabled");
    expect(events[events.length-6].event.data[1].toJSON().admin).to.equal(alith.address);

    let saleInfo: any = (await api.query.crowdsale.saleInfo(nextCrowdsaleId)).toJSON();
    expect(saleInfo.status).to.haveOwnProperty("enabled");

    // user participations in crowdsale - with all root tokens
    const participationEvents = await Promise.all(
      participants.map((user) => finalizeTx(user, api.tx.crowdsale.participate(nextCrowdsaleId, 50_000_000)))
    );
    participationEvents.forEach((pEvents, i) => {
      // pEvents.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`))

      // assets  Transferred     [15460,"0xf925e299a343864B397CC6f8a00dF6ce01F63933","0xB3Be2B8Edc1B627Cd7fC9C564B625B14DA3d5313",50000000]
      expect(pEvents[1].event.section).to.equal("assets");
      expect(pEvents[1].event.method).to.equal("Transferred");
      expect(pEvents[1].event.data[0]).to.equal(paymentAssetId);
      expect(pEvents[1].event.data[1].toString()).to.equal(participants[i].address);
      expect(pEvents[1].event.data[2].toString()).to.equal(saleInfo.vault);
      expect(pEvents[1].event.data[3]).to.equal(50_000_000); // 50 tokens

      // crowdsale       CrowdsaleParticipated   [7,"0xf925e299a343864B397CC6f8a00dF6ce01F63933",15460,50000000]
      expect(pEvents[2].event.section).to.equal("crowdsale");
      expect(pEvents[2].event.method).to.equal("CrowdsaleParticipated");
      expect(pEvents[2].event.data[0]).to.equal(nextCrowdsaleId);
      expect(pEvents[2].event.data[1].toString()).to.equal(participants[i].address);
      expect(pEvents[2].event.data[2]).to.equal(paymentAssetId);
      expect(pEvents[2].event.data[3]).to.equal(50_000_000); // 50 tokens
    });

    // assert all participants token balances are 0
    const userTokenBalances = await Promise.all(
      participants.map(async (user) => ((await api.query.assets.account(paymentAssetId, user.address)).toJSON() as any)?.balance ?? 0)
    );
    expect(userTokenBalances).to.deep.equal(Array(participants.length).fill(0));

    saleInfo = (await api.query.crowdsale.saleInfo(nextCrowdsaleId)).toJSON();
    expect(saleInfo.fundsRaised).to.equal(500_000_000); // 10 participants * 50_000_000 tokens each

    // wait for sale to reach end block, automatically distribute vouchers, end sale
    await new Promise<void>((resolve) => setInterval(async () => {
      const saleStatus: any = (await api.query.crowdsale.saleInfo(nextCrowdsaleId)).toJSON();
      if (saleStatus?.status?.ended) resolve();
    }, 500));

    // assert all participants have vouchers
    const userVoucherBalances = await Promise.all(
      participants.map(async (user) => ((await api.query.assets.account(saleInfo.voucherAssetId, user.address)).toJSON() as any)?.balance ?? 0)
    );
    expect(userVoucherBalances).to.deep.equal(Array(participants.length).fill(500_000));

    // redeeming single NFT should fail (oversubscribed by 2x)
    const dispatchError = await new Promise<DispatchError>((resolve, reject) => {
      api.tx.crowdsale
        .redeemVoucher(nextCrowdsaleId, 1)
        .signAndSend(participants[0], ({ status, dispatchError }) => {
          if (!status.isFinalized) return;
          if (dispatchError === undefined) return;
          resolve(dispatchError);
        })
        .catch((err) => reject(err));
    });
    const { section, name } = dispatchError.registry.findMetaError(dispatchError.asModule);
    expect(section).to.equal("assets");
    expect(name).to.equal("BalanceLow");

    // transfer vouchers from one user to another (to make whole)
    await finalizeTx(participants[1], api.tx.assets.transfer(saleInfo.voucherAssetId, participants[0].address, 500_000));
    const rEvents = await finalizeTx(participants[0], api.tx.crowdsale.redeemVoucher(nextCrowdsaleId, 1));
    // rEvents.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    // nft Mint [6244,0,0,"0x1497Fa38AbB70b6fF8227E387De60F5600bb97ef"]
    // crowdsale CrowdsaleNFTRedeemed [5,"0x1497Fa38AbB70b6fF8227E387De60F5600bb97ef",6244,1]
    expect(rEvents[2].event.section).to.equal("nft");
    expect(rEvents[2].event.method).to.equal("Mint");
    expect(rEvents[3].event.section).to.equal("crowdsale");
    expect(rEvents[3].event.method).to.equal("CrowdsaleNFTRedeemed");
    expect(rEvents[3].event.data[0]).to.equal(nextCrowdsaleId);
    expect(rEvents[3].event.data[1].toString()).to.equal(participants[0].address);
    expect(rEvents[3].event.data[3]).to.equal(1); // qty redeemed
  });

  it("undersubscribed crowdsale", async () => {
    const paymentAssetId = await getNextAssetId(api);

    const participants = Array.from({ length: 2 }, () => new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(Wallet.createRandom().privateKey))); // crowsale participants (10)
    const maxIssuance = 5; // create nft collection - total supply
    const nextCollectionUuid = nftCollectionIdToCollectionUUID(await api.query.nft.nextCollectionId() as any);
    const nextCrowdsaleId = +(await api.query.crowdsale.nextSaleId());

    let txs = [
      // create new token (crowdsale payment asset)
      api.tx.assetsExt.createAsset("TOKEN2", "T2", 18, 1, alith.address),

      // fund participants - 50 tokens per participant to participate
      ...participants.map((user) => api.tx.assets.mint(paymentAssetId, user.address, 50_000_000)),
      // ...participants.map((user) => api.tx.assets.transfer(paymentAssetId, user.address, 50_000_000)),

      // fund participants - 2 XRP (GAS) per participant
      ...participants.map((user) => api.tx.assets.mint(GAS_TOKEN_ID, user.address, 2_000_000)),

      // create nft collection
      api.tx.nft.createCollection("test", 0, maxIssuance, null, "http://example.com", null, { xrpl: false }),

      // initialize crowdsale as admin
      api.tx.crowdsale.initialize(
        paymentAssetId,
        nextCollectionUuid,
        50_000_000, // 50 root * 5 = 250 root
        2, // 2 blocks ~ 8s
      ),

      // enable crowdsale as admin - will expire in 2 blocks
      api.tx.crowdsale.enable(nextCrowdsaleId),
    ];
    await finalizeTx(alith, api.tx.utility.batch(txs));

    let saleInfo: any = (await api.query.crowdsale.saleInfo(nextCrowdsaleId)).toJSON();
    expect(saleInfo.status).to.haveOwnProperty("enabled");

    // user participations in crowdsale - with all root tokens
    await Promise.all(
      participants.map((user) => finalizeTx(user, api.tx.crowdsale.participate(nextCrowdsaleId, 50_000_000)))
    );

    // assert all participants token balances are 0
    const userTokenBalances = await Promise.all(
      participants.map(async (user) => ((await api.query.assets.account(paymentAssetId, user.address)).toJSON() as any)?.balance ?? 0)
    );
    expect(userTokenBalances).to.deep.equal(Array(participants.length).fill(0));

    saleInfo = (await api.query.crowdsale.saleInfo(nextCrowdsaleId)).toJSON();
    expect(saleInfo.fundsRaised).to.equal(100_000_000); // 2 participants * 50_000_000 tokens each

    // wait for sale to reach end block, automatically distribute vouchers, end sale
    await new Promise<void>((resolve) => setInterval(async () => {
      const saleStatus: any = (await api.query.crowdsale.saleInfo(nextCrowdsaleId)).toJSON();
      if (saleStatus?.status?.ended) resolve();
    }, 500));

    // assert all participants have vouchers
    const userVoucherBalances = await Promise.all(
      participants.map(async (user) => ((await api.query.assets.account(saleInfo.voucherAssetId, user.address)).toJSON() as any)?.balance ?? 0)
    );
    expect(userVoucherBalances).to.deep.equal(Array(participants.length).fill(1_000_000));

    // participant can redeem 1 NFT (price of each NFT is 1_000_000 vouchers)
    const rEvents = await finalizeTx(participants[0], api.tx.crowdsale.redeemVoucher(nextCrowdsaleId, 1));
    // rEvents.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    expect(rEvents[2].event.section).to.equal("nft");
    expect(rEvents[2].event.method).to.equal("Mint");
    expect(rEvents[3].event.section).to.equal("crowdsale");
    expect(rEvents[3].event.method).to.equal("CrowdsaleNFTRedeemed");
    expect(rEvents[3].event.data[0]).to.equal(nextCrowdsaleId);
    expect(rEvents[3].event.data[1].toString()).to.equal(participants[0].address);
    expect(rEvents[3].event.data[3]).to.equal(1); // qty redeemed
  });

  it("crowdsale participation using fee-proxy", async () => {
    
  });

  it("crowdsale participation using futurepass proxy-extrinsic", async () => {
    // initialize crowdsale as admin
    // enable crowdsale
    // participate in crowdsale as user (eoa, futurepass, feeproxy futurepass)
    // claim vouchers
    // redeem NFTs

    // add liquidity on dex by admin
    // swap tokens for vouchers
    // redeem NFTs
  });

  it("crowdsale participation using fee-proxy & futurepass proxy-extrinsic", async () => {
    // initialize crowdsale as admin
    // enable crowdsale
    // participate in crowdsale as user (eoa, futurepass, feeproxy futurepass)
    // claim vouchers
    // redeem NFTs

    // add liquidity on dex by admin
    // swap tokens for vouchers
    // redeem NFTs
  });

  it("multiple crowdsales", async () => {
    // initialize 2 crowdsales from 2 different accounts - using same payment asset
    // start and end at the same time
    // enable crowdsales

    // participate in crowdsale 1 as user (eoa, futurepass, feeproxy futurepass)
    // participate in crowdsale 2 as user (eoa, futurepass, feeproxy futurepass)

    // claim vouchers
    // redeem NFTs
  });

  it("benchmark crowdsale with 10,000 participants", async () => {
    // initialize crowdsale as admin
    // enable crowdsale
    // participate in crowdsale as user (eoa, futurepass, feeproxy futurepass)
    // claim vouchers
    // redeem NFTs
  });

});
