import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { blake256 } from "codechain-primitives";
import { BigNumber, Wallet } from "ethers";
import { computePublicKey } from "ethers/lib/utils";
import { encode, encodeForSigning } from "ripple-binary-codec";
import { deriveAddress, sign } from "ripple-keypairs";

import { ALITH_PRIVATE_KEY, GAS_TOKEN_ID, NodeProcess, finalizeTx, startNode, stringToHex, typedefs } from "../common";

describe("XRPL pallet", () => {
  let node: NodeProcess;

  let api: ApiPromise;
  let alith: KeyringPair;
  let chainId: number;

  before(async () => {
    node = await startNode();

    // substrate variables
    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.wsPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    alith = new Keyring({ type: "ethereum" }).addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    chainId = +(await api.query.evmChainId.chainId());
  });

  after(async () => await node.stop());

  // NOTE: use this test to generate a valid xaman tx (msg + signature) for integration tests
  it.skip("debug tx message and signature", async () => {
    // const user = Wallet.createRandom();
    const publicKey = computePublicKey(alith.publicKey, true);
    // console.log(hexToU8a(publicKey));

    // fund the user account to pay for tx fees
    // await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 1_000_000));

    const extrinsic = api.tx.system.remark("Mischief Managed");
    const hashedExtrinsicWithoutPrefix = blake256(extrinsic.toHex().slice(6)).toString();

    const xamanJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: publicKey.slice(2),
      Account: deriveAddress(publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`0:0:5:0:${hashedExtrinsicWithoutPrefix}`),
          },
        },
      ],
    };

    // sign xaman tx
    const message = encode(xamanJsonTx);
    const encodedSigningMessage = encodeForSigning(xamanJsonTx);
    const signature = sign(encodedSigningMessage, ALITH_PRIVATE_KEY.slice(2));

    console.log("message", message);
    console.log("signature", signature);
  });

  it("can submit system remark extrinsic", async () => {
    const user = Wallet.createRandom();
    const publicKey = computePublicKey(user.publicKey, true);

    // fund the user account to pay for tx fees
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 1_000_000));

    const extrinsic = api.tx.system.remark("hello world");
    const hashedExtrinsicWithoutPrefix = blake256(extrinsic.toHex().slice(6)).toString();
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xrpBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;

    const xamanJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: publicKey.slice(2),
      Account: deriveAddress(publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`${chainId}:0:${maxBlockNumber}:0:${hashedExtrinsicWithoutPrefix}`),
          },
        },
      ],
    };

    // sign xaman tx
    const message = encode(xamanJsonTx);
    const encodedSigningMessage = encodeForSigning(xamanJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    const cost = await api.tx.xrpl
      .transact(`0x${message}`, `0x${signature}`, extrinsic)
      .paymentInfo(user.address);
    expect(cost.partialFee.toNumber()).to.be.greaterThan(905_000).and.lessThan(920_000);

    // execute xaman tx extrinsic
    const events = await new Promise<any[]>(async (resolve) => {
      await api.tx.xrpl
        .transact(`0x${message}`, `0x${signature}`, extrinsic)
        .send(({ events = [], status }) => {
          if (status.isInBlock) resolve(events);
        });
    });

    // events.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    // assert events
    expect(events.length).to.equal(5);
    let index = 0;

    // assetsExt InternalWithdraw [2,"0x8800043D76AFd08b019F3db2016b9573041C1B59",560011]
    expect(events[index].event.section).to.equal("assetsExt");
    expect(events[index].event.method).to.equal("InternalWithdraw");
    expect(events[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(events[index].event.data[1].toString()).to.equal(user.address);

    // xrpl XRPLExtrinsicExecuted ["0x023b7f0df4d92da1ebe88be92fd59b2becfa4a60875b97c295c7a2524b03c487fc", "0x27Fd5891543A45aB8a0B7A387285bdd4A6562B51",0,{"callIndex":"0x0001","args":{"remark":"0x68656c6c6f20776f726c64"}}]
    index += 1;
    expect(events[index].event.section).to.equal("xrpl");
    expect(events[index].event.method).to.equal("XRPLExtrinsicExecuted");
    expect(events[index].event.data[0].toString()).to.equal(publicKey);
    expect(events[index].event.data[1].toString()).to.equal(user.address);
    expect(events[index].event.data[2].toString()).to.equal(xamanJsonTx.Account);

    // assetsExt InternalDeposit [2,"0x6D6F646c7478666565706F740000000000000000",557511]
    index += 1;
    expect(events[index].event.section).to.equal("assetsExt");
    expect(events[index].event.method).to.equal("InternalDeposit");

    // transactionPayment TransactionFeePaid ["0xe8d9B65B4D1daA328b4980405393a9563FecC592",557511,0]
    index += 1;
    expect(events[index].event.section).to.equal("transactionPayment");
    expect(events[index].event.method).to.equal("TransactionFeePaid");

    // system ExtrinsicSuccess [{"weight":86298000,"class":"Normal","paysFee":"Yes"}]
    index += 1;
    expect(events[index].event.section).to.equal("system");
    expect(events[index].event.method).to.equal("ExtrinsicSuccess");

    // assert balance after < balance before (tx fee must be paid)
    const xrpBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    expect(xrpBalanceAfter).to.be.lessThan(xrpBalanceBefore);
    expect(xrpBalanceBefore - xrpBalanceAfter)
      .to.greaterThan(685_000)
      .and.lessThan(700_000);

    // assert user nonce is updated (1 tx)
    const nonce = ((await api.query.system.account(user.address)).toJSON() as any)?.nonce;
    expect(nonce).to.equal(1);
  });

  it("can submit system remark extrinsic with tip", async () => {
    const user = Wallet.createRandom();
    const publicKey = computePublicKey(user.publicKey, true);

    // fund the user account to pay for tx fees
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 2_000_000));

    const extrinsic = api.tx.system.remark("hello world");
    const hashedExtrinsicWithoutPrefix = blake256(extrinsic.toHex().slice(6)).toString();
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xrpBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;

    const xamanJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: publicKey.slice(2),
      Account: deriveAddress(publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`${chainId}:0:${maxBlockNumber}:1000000:${hashedExtrinsicWithoutPrefix}`),
          },
        },
      ],
    };

    // sign xaman tx
    const message = encode(xamanJsonTx);
    const encodedSigningMessage = encodeForSigning(xamanJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    const cost = await api.tx.xrpl
      .transact(`0x${message}`, `0x${signature}`, extrinsic)
      .paymentInfo(user.address);
    expect(cost.partialFee.toNumber()).to.be.greaterThan(920_000).and.lessThan(935_000);

    // execute xaman tx extrinsic
    const events = await new Promise<any[]>(async (resolve) => {
      await api.tx.xrpl
        .transact(`0x${message}`, `0x${signature}`, extrinsic)
        .send(({ events = [], status }) => {
          if (status.isInBlock) resolve(events);
        });
    });

    // events.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    // assert events
    expect(events.length).to.equal(5);
    let index = 0;

    // assetsExt InternalWithdraw [2,"0x8800043D76AFd08b019F3db2016b9573041C1B59",560011]
    expect(events[index].event.section).to.equal("assetsExt");
    expect(events[index].event.method).to.equal("InternalWithdraw");
    expect(events[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(events[index].event.data[1].toString()).to.equal(user.address);

    // xrpl XRPLExtrinsicExecuted ["0x023b7f0df4d92da1ebe88be92fd59b2becfa4a60875b97c295c7a2524b03c487fc", "0x27Fd5891543A45aB8a0B7A387285bdd4A6562B51",0,{"callIndex":"0x0001","args":{"remark":"0x68656c6c6f20776f726c64"}}]
    index += 1;
    expect(events[index].event.section).to.equal("xrpl");
    expect(events[index].event.method).to.equal("XRPLExtrinsicExecuted");
    expect(events[index].event.data[0].toString()).to.equal(publicKey);
    expect(events[index].event.data[1].toString()).to.equal(user.address);
    expect(events[index].event.data[2].toString()).to.equal(xamanJsonTx.Account);

    // assetsExt InternalDeposit [2,"0x6D6F646c7478666565706F740000000000000000",557511]
    index += 1;
    expect(events[index].event.section).to.equal("assetsExt");
    expect(events[index].event.method).to.equal("InternalDeposit");

    // transactionPayment TransactionFeePaid ["0xe8d9B65B4D1daA328b4980405393a9563FecC592",557511,0]
    index += 1;
    expect(events[index].event.section).to.equal("transactionPayment");
    expect(events[index].event.method).to.equal("TransactionFeePaid");

    // system ExtrinsicSuccess [{"weight":86298000,"class":"Normal","paysFee":"Yes"}]
    index += 1;
    expect(events[index].event.section).to.equal("system");
    expect(events[index].event.method).to.equal("ExtrinsicSuccess");

    // assert balance after < balance before (tx fee must be paid)
    const xrpBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    expect(xrpBalanceAfter).to.be.lessThan(xrpBalanceBefore);
    expect(xrpBalanceBefore - xrpBalanceAfter)
      .to.greaterThan(1_700_000)
      .and.lessThan(1_715_000);

    // assert user nonce is updated (1 tx)
    const nonce = ((await api.query.system.account(user.address)).toJSON() as any)?.nonce;
    expect(nonce).to.equal(1);
  });

  it("can submit assets transfer extrinsic", async () => {
    const user = Wallet.createRandom();
    const publicKey = computePublicKey(user.publicKey, true);

    // fund the user account first (so it can transfer back to alice)
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 10_000_000));

    const extrinsic = api.tx.assets.transfer(GAS_TOKEN_ID, alith.address, 1000);
    const hashedExtrinsicWithoutPrefix = blake256(extrinsic.toHex().slice(6)).toString();
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xrpBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;

    const xamanJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: publicKey.slice(2),
      Account: deriveAddress(publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`${chainId}:0:${maxBlockNumber}:0:${hashedExtrinsicWithoutPrefix}`),
          },
        },
      ],
    };

    // sign xaman tx
    const message = encode(xamanJsonTx);
    const encodedSigningMessage = encodeForSigning(xamanJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    const cost = await api.tx.xrpl
      .transact(`0x${message}`, `0x${signature}`, extrinsic)
      .paymentInfo(user.address);
    expect(cost.partialFee.toNumber()).to.be.greaterThan(935_000).and.lessThan(950_000);

    // execute xaman tx extrinsic
    const events = await new Promise<any[]>(async (resolve) => {
      await api.tx.xrpl
        .transact(`0x${message}`, `0x${signature}`, extrinsic)
        .send(({ events = [], status }) => {
          if (status.isInBlock) resolve(events);
        });
    });

    // events.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    // assert events
    expect(events.length).to.equal(6);
    let index = 0;

    // assetsExt InternalWithdraw [2,"0x2CE29C6BaB687b05EEcC49AF5fc12730c91C229E",615011]
    expect(events[index].event.section).to.equal("assetsExt");
    expect(events[index].event.method).to.equal("InternalWithdraw");
    expect(events[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(events[index].event.data[1].toString()).to.equal(user.address);

    // assets Transferred [2,"0x582F0E877a678ab8Ddb13a9ebBECf86614f3916E","0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",1000]
    index += 1;
    expect(events[index].event.section).to.equal("assets");
    expect(events[index].event.method).to.equal("Transferred");
    expect(events[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(events[index].event.data[1].toString()).to.equal(user.address);
    expect(events[index].event.data[2].toString()).to.equal(alith.address);

    // xrpl XRPLExtrinsicExecuted ["0x023b7f0df4d92da1ebe88be92fd59b2becfa4a60875b97c295c7a2524b03c487fc", "0x582F0E877a678ab8Ddb13a9ebBECf86614f3916E",0,{"callIndex":"0x0605","args":{"id":2,"target":"0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac","amount":1000}}]
    index += 1;
    expect(events[index].event.section).to.equal("xrpl");
    expect(events[index].event.method).to.equal("XRPLExtrinsicExecuted");
    expect(events[index].event.data[0].toString()).to.equal(publicKey);
    expect(events[index].event.data[1].toString()).to.equal(user.address);
    expect(events[index].event.data[2].toString()).to.equal(xamanJsonTx.Account);

    // assetsExt InternalDeposit [2,"0x6D6F646c7478666565706F740000000000000000",615011]
    index += 1;
    expect(events[index].event.section).to.equal("assetsExt");
    expect(events[index].event.method).to.equal("InternalDeposit");
    expect(events[index].event.data[0]).to.equal(GAS_TOKEN_ID);

    // transactionPayment TransactionFeePaid ["0x2CE29C6BaB687b05EEcC49AF5fc12730c91C229E",615011,0]
    index += 1;
    expect(events[index].event.section).to.equal("transactionPayment");
    expect(events[index].event.method).to.equal("TransactionFeePaid");
    expect(events[index].event.data[0].toString()).to.equal(user.address);

    // system ExtrinsicSuccess [{"weight":86298000,"class":"Normal","paysFee":"Yes"}]
    index += 1;
    expect(events[index].event.section).to.equal("system");
    expect(events[index].event.method).to.equal("ExtrinsicSuccess");

    // assert balance after < balance before (tx fee must be paid)
    const xrpBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    expect(xrpBalanceAfter).to.be.lessThan(xrpBalanceBefore);
    expect(xrpBalanceBefore - xrpBalanceAfter)
      .to.be.greaterThan(710_000)
      .and.lessThan(725_000);
  });

  it("can submit fee-proxy extrinsic", async () => {
    const user = Wallet.createRandom();
    const publicKey = computePublicKey(user.publicKey, true);

    // add liquidity for XRP<->token; fund user account with tokens
    const FEE_TOKEN_ASSET_ID = 1124;
    const txs = [
      api.tx.assetsExt.createAsset("test", "TEST", 18, 1, alith.address),
      api.tx.assets.mint(FEE_TOKEN_ASSET_ID, alith.address, 2_000_000_000_000_000),
      api.tx.assets.mint(FEE_TOKEN_ASSET_ID, user.address, 2_000_000_000_000_000),
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
    console.log("liquidity setup complete...");

    const innerCall = api.tx.system.remark("sup");
    const maxTokenPayment = 2_000_000;
    const extrinsic = api.tx.feeProxy.callWithFeePreferences(FEE_TOKEN_ASSET_ID, maxTokenPayment, innerCall);
    const hashedExtrinsicWithoutPrefix = blake256(extrinsic.toHex().slice(6)).toString();
    const nonce = ((await api.query.system.account(user.address)).toJSON() as any)?.nonce;
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xrpUserBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const assetUserBalanceBefore = BigNumber.from(
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, user.address)).toJSON() as any)?.balance ?? 0,
    );

    const xamanJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: publicKey.slice(2),
      Account: deriveAddress(publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`${chainId}:${nonce}:${maxBlockNumber}:0:${hashedExtrinsicWithoutPrefix}`),
          },
        },
      ],
    };

    // sign xaman tx
    const message = encode(xamanJsonTx);
    const encodedSigningMessage = encodeForSigning(xamanJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    const cost = await api.tx.xrpl
      .transact(`0x${message}`, `0x${signature}`, extrinsic)
      .paymentInfo(user.address);
    expect(cost.partialFee.toNumber()).to.be.greaterThan(945_000).and.lessThan(965_000);

    // execute xaman tx extrinsic
    const events = await new Promise<any[]>(async (resolve) => {
      await api.tx.xrpl
        .transact(`0x${message}`, `0x${signature}`, extrinsic)
        .send(({ events = [], status }) => {
          if (status.isInBlock) resolve(events);
        });
    });

    // events.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    // assert events
    expect(events.length).to.equal(10);
    let index = 0;

    // assets Transferred [1124,"0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac","0xDDDDDDdD00000002000004640000000000000000",727237]
    expect(events[index].event.section).to.equal("assets");
    expect(events[index].event.method).to.equal("Transferred");
    expect(events[index].event.data[0]).to.equal(FEE_TOKEN_ASSET_ID);
    expect(events[index].event.data[1].toString()).to.equal(user.address);

    // assets Transferred [2,"0xDDDDDDdD00000002000004640000000000000000","0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",725039]
    index += 1;
    expect(events[index].event.section).to.equal("assets");
    expect(events[index].event.method).to.equal("Transferred");
    expect(events[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(events[index].event.data[1].toString()).to.equal("0xDDDDDDdD00000002000004640000000000000000");
    expect(events[index].event.data[2].toString()).to.equal(user.address);

    // assets Issued [2148,"0x6D6F646c7478666565706F740000000000000000",181]
    index += 1;
    expect(events[index].event.section).to.equal("assets");
    expect(events[index].event.method).to.equal("Issued");
    expect(events[index].event.data[0]).to.equal(2148);

    // dex Swap ["0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",[1124,2],727237,725039,"0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac"]
    index += 1;
    expect(events[index].event.section).to.equal("dex");
    expect(events[index].event.method).to.equal("Swap");
    expect(events[index].event.data[0].toString()).to.equal(user.address);
    expect(events[index].event.data[1].toString()).to.equal(`[${FEE_TOKEN_ASSET_ID}, ${GAS_TOKEN_ID}]`);

    // assetsExt InternalWithdraw [2,"0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",725039]
    index += 1;
    expect(events[index].event.section).to.equal("assetsExt");
    expect(events[index].event.method).to.equal("InternalWithdraw");
    expect(events[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(events[index].event.data[1].toString()).to.equal(user.address);

    // feeProxy CallWithFeePreferences ["0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",1124,2000000]
    index += 1;
    expect(events[index].event.section).to.equal("feeProxy");
    expect(events[index].event.method).to.equal("CallWithFeePreferences");
    expect(events[index].event.data[0].toString()).to.equal(user.address);
    expect(events[index].event.data[1]).to.equal(FEE_TOKEN_ASSET_ID);
    expect(events[index].event.data[2]).to.equal(maxTokenPayment);

    // xrpl XRPLExtrinsicExecuted ["0x02509540919faacf9ab52146c9aa40db68172d83777250b28e4679176e49ccdd9f","0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac","rDyqBotBNJeXv8PBHY18ABjyw6FQuWXQnu",{"callIndex":"0x1f00","args":{"payment_asset":1124,"max_payment":2000000,"call":{"callIndex":"0x0001","args":{"remark":"0x737570"}}}}]
    index += 1;
    expect(events[index].event.section).to.equal("xrpl");
    expect(events[index].event.method).to.equal("XRPLExtrinsicExecuted");
    expect(events[index].event.data[0].toString()).to.equal(publicKey);
    expect(events[index].event.data[1].toString()).to.equal(user.address);
    expect(events[index].event.data[2].toString()).to.equal(xamanJsonTx.Account);

    // assetsExt InternalDeposit [2,"0x6D6F646c7478666565706F740000000000000000",725039]
    index += 1;
    expect(events[index].event.section).to.equal("assetsExt");
    expect(events[index].event.method).to.equal("InternalDeposit");
    expect(events[index].event.data[0]).to.equal(GAS_TOKEN_ID);

    // transactionPayment TransactionFeePaid ["0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",725039,0]
    index += 1;
    expect(events[index].event.section).to.equal("transactionPayment");
    expect(events[index].event.method).to.equal("TransactionFeePaid");
    expect(events[index].event.data[0].toString()).to.equal(user.address);

    // system ExtrinsicSuccess [{"weight":86298000,"class":"Normal","paysFee":"Yes"}]
    index += 1;
    expect(events[index].event.section).to.equal("system");
    expect(events[index].event.method).to.equal("ExtrinsicSuccess");

    // user xrp balance should not change since tx fees paid in asset
    const xrpUserBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    expect(xrpUserBalanceAfter).to.be.eq(xrpUserBalanceBefore);

    // assert token balance after < balance before (tx fee must be paid in asset)
    const assetUserBalanceAfter = BigNumber.from(
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, user.address)).toJSON() as any)?.balance ?? 0,
    );
    expect(assetUserBalanceAfter).to.be.lessThan(assetUserBalanceBefore);
  });

  it("can submit futurepass proxy-extrinsic", async () => {
    // create futurepass for random user
    const user = Wallet.createRandom();
    const publicKey = computePublicKey(user.publicKey, true);

    // create a futurepass for user
    await finalizeTx(alith, api.tx.futurepass.create(user.address));

    // fund the futurepass account
    const futurepassAddress = (await api.query.futurepass.holders(user.address)).toString();
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, futurepassAddress, 10_000_000));

    // futurepass balance transfer back to alice - in xaman encoded extrinsic
    const innerCall = api.tx.assets.transfer(GAS_TOKEN_ID, alith.address, 1000);
    const extrinsic = api.tx.futurepass.proxyExtrinsic(futurepassAddress, innerCall);
    const hashedExtrinsicWithoutPrefix = blake256(extrinsic.toHex().slice(6)).toString();
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xrpUserBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const xrpFPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;

    const xamanJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: publicKey.slice(2),
      Account: deriveAddress(publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`${chainId}:0:${maxBlockNumber}:0:${hashedExtrinsicWithoutPrefix}`),
          },
        },
      ],
    };

    // sign xaman tx
    const message = encode(xamanJsonTx);
    const encodedSigningMessage = encodeForSigning(xamanJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    const cost = await api.tx.xrpl
      .transact(`0x${message}`, `0x${signature}`, extrinsic)
      .paymentInfo(user.address);
    expect(cost.partialFee.toNumber()).to.be.greaterThan(990_000).and.lessThan(1_005_000);

    // execute xaman tx extrinsic
    const events = await new Promise<any[]>(async (resolve) => {
      await api.tx.xrpl
        .transact(`0x${message}`, `0x${signature}`, extrinsic)
        .send(({ events = [], status }) => {
          if (status.isInBlock) resolve(events);
        });
    });

    // events.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    // assert events
    expect(events.length).to.equal(8);
    let index = 0;

    // assetsExt InternalWithdraw [2,"0xFFFFFfff00000000000000000000000000000001",722511]
    expect(events[index].event.section).to.equal("assetsExt");
    expect(events[index].event.method).to.equal("InternalWithdraw");
    expect(events[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(events[index].event.data[1].toString()).to.equal(futurepassAddress);

    // assets Transferred [2,"0xFFFFFfff00000000000000000000000000000001","0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",1000]
    index += 1;
    expect(events[index].event.section).to.equal("assets");
    expect(events[index].event.method).to.equal("Transferred");
    expect(events[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(events[index].event.data[1].toString()).to.equal(futurepassAddress);
    expect(events[index].event.data[2].toString()).to.equal(alith.address);
    expect(events[index].event.data[3]).to.equal(1000);

    // proxy ProxyExecuted [{"ok":null}]
    index += 1;
    expect(events[index].event.section).to.equal("proxy");
    expect(events[index].event.method).to.equal("ProxyExecuted");

    // futurepass ProxyExecuted ["0x557fce5210eaaE26893404Bf14A1423F8f22EAD9",{"ok":null}]
    index += 1;
    expect(events[index].event.section).to.equal("futurepass");
    expect(events[index].event.method).to.equal("ProxyExecuted");
    expect(events[index].event.data[0].toString()).to.equal(user.address);

    // xrpl XRPLExtrinsicExecuted ["0x023b7f0df4d92da1ebe88be92fd59b2becfa4a60875b97c295c7a2524b03c487fc", "0x557fce5210eaaE26893404Bf14A1423F8f22EAD9",0,{"callIndex":"0x2204","args":{"futurepass":"0xfFFFFfff00000000000000000000000000000008","call":{"callIndex":"0x0605","args":{"id":2,"target":"0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac","amount":1000}}}}]
    index += 1;
    expect(events[index].event.section).to.equal("xrpl");
    expect(events[index].event.method).to.equal("XRPLExtrinsicExecuted");
    expect(events[index].event.data[0].toString()).to.equal(publicKey);
    expect(events[index].event.data[1].toString()).to.equal(user.address);
    expect(events[index].event.data[2].toString()).to.equal(xamanJsonTx.Account);

    // assetsExt InternalDeposit [2,"0x6D6F646c7478666565706F740000000000000000",730011]
    index += 1;
    expect(events[index].event.section).to.equal("assetsExt");
    expect(events[index].event.method).to.equal("InternalDeposit");

    // transactionPayment TransactionFeePaid ["0xFFFFFfff00000000000000000000000000000001",730011,0]
    index += 1;
    expect(events[index].event.section).to.equal("transactionPayment");
    expect(events[index].event.method).to.equal("TransactionFeePaid");
    expect(events[index].event.data[0].toString()).to.equal(futurepassAddress);

    // system ExtrinsicSuccess [{"weight":86298000,"class":"Normal","paysFee":"Yes"}]
    index += 1;
    expect(events[index].event.section).to.equal("system");
    expect(events[index].event.method).to.equal("ExtrinsicSuccess");

    // user xrp balance should be the same before and after since futurepass must be paying tx fees
    const xrpUserBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    expect(xrpUserBalanceAfter).to.be.eq(xrpUserBalanceBefore);

    // assert futurepass balance after < balance before (tx fee must be paid)
    const xrpFPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
    expect(xrpFPBalanceAfter).to.be.lessThan(xrpFPBalanceBefore);
    expect(xrpFPBalanceBefore - xrpFPBalanceAfter)
      .to.be.greaterThan(765_000)
      .and.lessThan(780_000);
  });

  it("can submit futurepass fee-proxy proxy-extrinsic", async () => {
    const user = Wallet.createRandom();
    const publicKey = computePublicKey(user.publicKey, true);

    // create a futurepass for user
    await finalizeTx(alith, api.tx.futurepass.create(user.address));

    const futurepassAddress = (await api.query.futurepass.holders(user.address)).toString();

    // add liquidity for XRP<->token; fund the futurepass account with tokens
    const FEE_TOKEN_ASSET_ID = 1124;
    const txs = [
      api.tx.assetsExt.createAsset("test", "TEST", 18, 1, alith.address),
      api.tx.assets.mint(FEE_TOKEN_ASSET_ID, alith.address, 2_000_000_000_000_000),
      api.tx.assets.mint(FEE_TOKEN_ASSET_ID, futurepassAddress, 2_000_000_000_000_000),
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
    console.log("liquidity setup complete...");

    const innerCall = api.tx.system.remark("sup");
    const futurepassCall = api.tx.futurepass.proxyExtrinsic(futurepassAddress, innerCall);
    const maxTokenPayment = 5_000_000;
    const extrinsic = api.tx.feeProxy.callWithFeePreferences(FEE_TOKEN_ASSET_ID, maxTokenPayment, futurepassCall);
    const hashedExtrinsicWithoutPrefix = blake256(extrinsic.toHex().slice(6)).toString();
    const nonce = ((await api.query.system.account(futurepassAddress)).toJSON() as any)?.nonce;
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xrpUserBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const xrpFPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
    const assetUserBalanceBefore = BigNumber.from(
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, user.address)).toJSON() as any)?.balance ?? 0,
    );
    const assetFPBalanceBefore = BigNumber.from(
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0,
    );

    const xamanJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: publicKey.slice(2),
      Account: deriveAddress(publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`${chainId}:${nonce}:${maxBlockNumber}:0:${hashedExtrinsicWithoutPrefix}`),
          },
        },
      ],
    };

    // sign xaman tx
    const message = encode(xamanJsonTx);
    const encodedSigningMessage = encodeForSigning(xamanJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    const cost = await api.tx.xrpl
      .transact(`0x${message}`, `0x${signature}`, extrinsic)
      .paymentInfo(futurepassAddress);
    expect(cost.partialFee.toNumber()).to.be.greaterThan(1_000_000).and.lessThan(1_015_000);

    // execute xaman tx extrinsic
    const events = await new Promise<any[]>(async (resolve) => {
      await api.tx.xrpl
        .transact(`0x${message}`, `0x${signature}`, extrinsic)
        .send(({ events = [], status }) => {
          if (status.isInBlock) resolve(events);
        });
    });

    // events.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    // assert events
    expect(events.length).to.equal(12);
    let index = 0;

    // assets Transferred [1124,"0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac","0xDDDDDDdD00000002000004640000000000000000",727237]
    expect(events[index].event.section).to.equal("assets");
    expect(events[index].event.method).to.equal("Transferred");
    expect(events[index].event.data[0]).to.equal(FEE_TOKEN_ASSET_ID);
    expect(events[index].event.data[1].toString()).to.equal(futurepassAddress);

    // assets Transferred [2,"0xDDDDDDdD00000002000004640000000000000000","0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",725039]
    index += 1;
    expect(events[index].event.section).to.equal("assets");
    expect(events[index].event.method).to.equal("Transferred");
    expect(events[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(events[index].event.data[1].toString()).to.equal("0xDDDDDDdD00000002000004640000000000000000");
    expect(events[index].event.data[2].toString()).to.equal(futurepassAddress);

    // assets Issued [2148,"0x6D6F646c7478666565706F740000000000000000",181]
    index += 1;
    expect(events[index].event.section).to.equal("assets");
    expect(events[index].event.method).to.equal("Issued");
    expect(events[index].event.data[0]).to.equal(2148);

    // dex Swap ["0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",[1124,2],727237,725039,"0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac"]
    index += 1;
    expect(events[index].event.section).to.equal("dex");
    expect(events[index].event.method).to.equal("Swap");
    expect(events[index].event.data[0].toString()).to.equal(futurepassAddress);
    expect(events[index].event.data[1].toString()).to.equal(`[${FEE_TOKEN_ASSET_ID}, ${GAS_TOKEN_ID}]`);

    // assetsExt InternalWithdraw [2,"0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",725039]
    index += 1;
    expect(events[index].event.section).to.equal("assetsExt");
    expect(events[index].event.method).to.equal("InternalWithdraw");
    expect(events[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(events[index].event.data[1].toString()).to.equal(futurepassAddress);

    // proxy ProxyExecuted [{"ok":null}]
    index += 1;
    expect(events[index].event.section).to.equal("proxy");
    expect(events[index].event.method).to.equal("ProxyExecuted");

    // futurepass ProxyExecuted ["0x557fce5210eaaE26893404Bf14A1423F8f22EAD9",{"ok":null}]
    index += 1;
    expect(events[index].event.section).to.equal("futurepass");
    expect(events[index].event.method).to.equal("ProxyExecuted");
    expect(events[index].event.data[0].toString()).to.equal(user.address);

    // feeProxy CallWithFeePreferences ["0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",1124,2000000]
    index += 1;
    expect(events[index].event.section).to.equal("feeProxy");
    expect(events[index].event.method).to.equal("CallWithFeePreferences");
    expect(events[index].event.data[0].toString()).to.equal(user.address); // TODO: should be futurepass address
    expect(events[index].event.data[1]).to.equal(FEE_TOKEN_ASSET_ID);
    expect(events[index].event.data[2]).to.equal(maxTokenPayment);

    // xrpl XRPLExtrinsicExecuted ["0x02509540919faacf9ab52146c9aa40db68172d83777250b28e4679176e49ccdd9f","0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac","rDyqBotBNJeXv8PBHY18ABjyw6FQuWXQnu",{"callIndex":"0x1f00","args":{"payment_asset":1124,"max_payment":2000000,"call":{"callIndex":"0x0001","args":{"remark":"0x737570"}}}}]
    index += 1;
    expect(events[index].event.section).to.equal("xrpl");
    expect(events[index].event.method).to.equal("XRPLExtrinsicExecuted");
    expect(events[index].event.data[0].toString()).to.equal(publicKey);
    expect(events[index].event.data[1].toString()).to.equal(user.address);
    expect(events[index].event.data[2].toString()).to.equal(xamanJsonTx.Account);

    // assetsExt InternalDeposit [2,"0x6D6F646c7478666565706F740000000000000000",725039]
    index += 1;
    expect(events[index].event.section).to.equal("assetsExt");
    expect(events[index].event.method).to.equal("InternalDeposit");
    expect(events[index].event.data[0]).to.equal(GAS_TOKEN_ID);

    // transactionPayment TransactionFeePaid ["0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",725039,0]
    index += 1;
    expect(events[index].event.section).to.equal("transactionPayment");
    expect(events[index].event.method).to.equal("TransactionFeePaid");
    expect(events[index].event.data[0].toString()).to.equal(user.address);

    // system ExtrinsicSuccess [{"weight":86298000,"class":"Normal","paysFee":"Yes"}]
    index += 1;
    expect(events[index].event.section).to.equal("system");
    expect(events[index].event.method).to.equal("ExtrinsicSuccess");

    // user xrp balance should not change since tx fees paid by futurepass in asset
    const xrpUserBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    expect(xrpUserBalanceAfter).to.be.eq(xrpUserBalanceBefore);

    // user asset balance should not change since tx fees paid by futurepass in asset
    const assetUserBalanceAfter = BigNumber.from(
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, user.address)).toJSON() as any)?.balance ?? 0,
    );
    expect(assetUserBalanceAfter).to.be.eq(assetUserBalanceBefore);

    // futurepass xrp balance should not change since tx fees paid by futurepass in asset
    const xrpFPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
    expect(xrpFPBalanceAfter).to.be.eq(xrpFPBalanceBefore);

    // assert futurepass token balance after < balance before (tx fee must be paid in asset by futurepass)
    const assetFPBalanceAfter = BigNumber.from(
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0,
    );
    expect(assetFPBalanceAfter).to.be.lessThan(assetFPBalanceBefore);
  });

  it("fails futurepass proxy-extrinsic if user does not have futurepass", async () => {
    const user = Wallet.createRandom();
    const publicKey = computePublicKey(user.publicKey, true);

    const innerCall = api.tx.system.remark("hello world");
    const extrinsic = api.tx.futurepass.proxyExtrinsic(user.address, innerCall);
    const hashedExtrinsicWithoutPrefix = blake256(extrinsic.toHex().slice(6)).toString();
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xamanJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: publicKey.slice(2),
      Account: deriveAddress(publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`${chainId}:0:${maxBlockNumber}:0:${hashedExtrinsicWithoutPrefix}`),
          },
        },
      ],
    };

    // sign xaman tx
    const message = encode(xamanJsonTx);
    const encodedSigningMessage = encodeForSigning(xamanJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    let errorFound = false;
    await Promise.race([
      new Promise<any[]>(async (resolve) => {
        await api.tx.xrpl
          .transact(`0x${message}`, `0x${signature}`, extrinsic)
          .send(({ events = [], status }) => {
            if (status.isInBlock) resolve(events);
          });
      }),
      new Promise<any[]>((_, reject) => setTimeout(() => reject(new Error("timeout error")), 4000)),
    ]).catch((err: any) => {
      errorFound = true;
      expect(err.message).eq("timeout error");
    });
    expect(errorFound).to.be.true;
  });

  it("fails if encoded call is nested transact extrinsic", async () => {
    const user = Wallet.createRandom();
    const publicKey = computePublicKey(user.publicKey, true);

    const innerCall = api.tx.system.remark("hello world");
    const extrinsic = api.tx.xrpl.transact(`0x00000000`, `0x00000000`, innerCall);
    const hashedExtrinsicWithoutPrefix = blake256(extrinsic.toHex().slice(6)).toString();
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xamanJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: publicKey.slice(2),
      Account: deriveAddress(publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`${chainId}:0:${maxBlockNumber}:0:${hashedExtrinsicWithoutPrefix}`),
          },
        },
      ],
    };

    // sign xaman tx
    const message = encode(xamanJsonTx);
    const encodedSigningMessage = encodeForSigning(xamanJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    let errorFound = false;
    await Promise.race([
      new Promise<any[]>(async (resolve) => {
        await api.tx.xrpl
          .transact(`0x${message}`, `0x${signature}`, extrinsic)
          .send(({ events = [], status }) => {
            if (status.isInBlock) resolve(events);
          });
      }),
      new Promise<any[]>((_, reject) => setTimeout(() => reject(new Error("timeout error")), 4000)),
    ]).catch((err: any) => {
      errorFound = true;
      expect(err.message).eq("timeout error");
    });
    expect(errorFound).to.be.true;
  });
});
