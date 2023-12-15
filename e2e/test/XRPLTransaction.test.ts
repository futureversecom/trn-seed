import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Wallet } from "ethers";
import { encode, encodeForSigning } from "ripple-binary-codec";
import { deriveAddress, sign } from "ripple-keypairs";

import { ALITH_PRIVATE_KEY, GAS_TOKEN_ID, NodeProcess, finalizeTx, startNode, typedefs } from "../common";

const stringToHex = (str: string) => str.split("").map(c => c.charCodeAt(0).toString(16)).join("");

describe("XRPL transaction pallet", () => {
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

  it("can submit system remark extrinsic", async () => {
    const user = Wallet.createRandom();

    // fund the user account to pay for tx fees
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 1_000_000));

    const extrinsic = api.tx.system.remark("hello world");
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xrpBalanceBefore = ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;

    const xummJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: user.publicKey.slice(2),
      Account: deriveAddress(user.publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`0:${maxBlockNumber}:${extrinsic.toHex().slice(2)}`),
          }
        }
      ]
    };

    // sign xumm tx
    const message = encode(xummJsonTx);
    const encodedSigningMessage = encodeForSigning(xummJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    // execute xumm tx extrinsic
    const events = await new Promise<any[]>(async (resolve) => {
      await api.tx.xrplTransaction.submitEncodedXummTransaction(`0x${message}`, `0x${signature}`).send(({ events = [], status }) => {
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

    // xrplTransaction XUMMExtrinsicExecuted ["0x023b7f0df4d92da1ebe88be92fd59b2becfa4a60875b97c295c7a2524b03c487fc", "0x27Fd5891543A45aB8a0B7A387285bdd4A6562B51",0,{"callIndex":"0x0001","args":{"remark":"0x68656c6c6f20776f726c64"}}]
    index += 1;
    expect(events[index].event.section).to.equal("xrplTransaction");
    expect(events[index].event.method).to.equal("XUMMExtrinsicExecuted");
    expect(events[index].event.data[0].toString()).to.equal(user.publicKey);
    expect(events[index].event.data[1].toString()).to.equal(user.address);

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
    const xrpBalanceAfter = ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    expect(xrpBalanceAfter).to.be.lessThan(xrpBalanceBefore);

    // assert user nonce is updated (1 tx)
    const nonce = ((await api.query.system.account(user.address)).toJSON() as any)?.nonce;
    expect(nonce).to.equal(1);
  });

  it("can submit assets transfer extrinsic", async () => {
    const user = Wallet.createRandom();

    // fund the user account first (so it can transfer back to alice)
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 10_000_000));

    const extrinsic = api.tx.assets.transfer(GAS_TOKEN_ID, alith.address, 1000);
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xrpBalanceBefore = ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;

    const xummJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: user.publicKey.slice(2),
      Account: deriveAddress(user.publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`0:${maxBlockNumber}:${extrinsic.toHex().slice(2)}`),
          }
        }
      ]
    };

    // sign xumm tx
    const message = encode(xummJsonTx);
    const encodedSigningMessage = encodeForSigning(xummJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    // execute xumm tx extrinsic
    const events = await new Promise<any[]>(async (resolve) => {
      await api.tx.xrplTransaction.submitEncodedXummTransaction(`0x${message}`, `0x${signature}`).send(({ events = [], status }) => {
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

    // xrplTransaction XUMMExtrinsicExecuted ["0x023b7f0df4d92da1ebe88be92fd59b2becfa4a60875b97c295c7a2524b03c487fc", "0x582F0E877a678ab8Ddb13a9ebBECf86614f3916E",0,{"callIndex":"0x0605","args":{"id":2,"target":"0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac","amount":1000}}]
    index += 1;
    expect(events[index].event.section).to.equal("xrplTransaction");
    expect(events[index].event.method).to.equal("XUMMExtrinsicExecuted");
    expect(events[index].event.data[0].toString()).to.equal(user.publicKey);
    expect(events[index].event.data[1].toString()).to.equal(user.address);

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
    const xrpBalanceAfter = ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    expect(xrpBalanceAfter).to.be.lessThan(xrpBalanceBefore);
  });

  it("can proxy futurepass extrinsic", async () => {
    // create futurepass for random user
    const user = Wallet.createRandom();

    // create a futurepass for user
    await finalizeTx(alith, api.tx.futurepass.create(user.address));

    // fund the futurepass account
    const futurepassAddress = (await api.query.futurepass.holders(user.address)).toString();
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, futurepassAddress, 10_000_000));

    // futurepass balance transfer back to alice - in xumm encoded extrinsic
    const innerCall = api.tx.assets.transfer(GAS_TOKEN_ID, alith.address, 1000)
    const extrinsic = api.tx.futurepass.proxyExtrinsic(futurepassAddress, innerCall);
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xrpUserBalanceBefore = ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;

    const xummJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: user.publicKey.slice(2),
      Account: deriveAddress(user.publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`0:${maxBlockNumber}:${extrinsic.toHex().slice(2)}`),
          }
        }
      ]
    };

    // sign xumm tx
    const message = encode(xummJsonTx);
    const encodedSigningMessage = encodeForSigning(xummJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    // execute xumm tx extrinsic
    const events = await new Promise<any[]>(async (resolve) => {
      await api.tx.xrplTransaction.submitEncodedXummTransaction(`0x${message}`, `0x${signature}`).send(({ events = [], status }) => {
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

    // xrplTransaction XUMMExtrinsicExecuted ["0x023b7f0df4d92da1ebe88be92fd59b2becfa4a60875b97c295c7a2524b03c487fc", "0x557fce5210eaaE26893404Bf14A1423F8f22EAD9",0,{"callIndex":"0x2204","args":{"futurepass":"0xfFFFFfff00000000000000000000000000000008","call":{"callIndex":"0x0605","args":{"id":2,"target":"0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac","amount":1000}}}}]
    index += 1;
    expect(events[index].event.section).to.equal("xrplTransaction");
    expect(events[index].event.method).to.equal("XUMMExtrinsicExecuted");
    expect(events[index].event.data[0].toString()).to.equal(user.publicKey);
    expect(events[index].event.data[1].toString()).to.equal(user.address);

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
    const xrpUserBalanceAfter = ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    expect(xrpUserBalanceAfter).to.be.eq(xrpUserBalanceBefore);
  });

  it("fails proxy futurepass extrinsic if user does not have futurepass", async () => {
    const user = Wallet.createRandom();

    const innerCall = api.tx.system.remark("hello world");
    const extrinsic = api.tx.futurepass.proxyExtrinsic(user.address, innerCall);
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xummJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: user.publicKey.slice(2),
      Account: deriveAddress(user.publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`0:${maxBlockNumber}:${extrinsic.toHex().slice(2)}`),
          }
        }
      ]
    };

    // sign xumm tx
    const message = encode(xummJsonTx);
    const encodedSigningMessage = encodeForSigning(xummJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    let errorFound = false;
    await Promise.race([
      new Promise<any[]>(async (resolve) => {
        await api.tx.xrplTransaction.submitEncodedXummTransaction(`0x${message}`, `0x${signature}`).send(({ events = [], status }) => {
          if (status.isInBlock) resolve(events);
        });
      }),
      new Promise<any[]>((_, reject) => setTimeout(() => reject(new Error('timeout error')), 4000))
    ])
    .catch((err: any) => {
      errorFound = true;
      expect(err.message).eq("timeout error");
    });
    expect(errorFound).to.be.true;
  });

  it("fails if encoded call is nested submitEncodedXummTransaction extrinsic", async () => {
    const user = Wallet.createRandom();

    const extrinsic = api.tx.xrplTransaction.submitEncodedXummTransaction(`0x00000000`, `0x00000000`);
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xummJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: user.publicKey.slice(2),
      Account: deriveAddress(user.publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`0:${maxBlockNumber}:${extrinsic.toHex().slice(2)}`),
          }
        }
      ]
    };

    // sign xumm tx
    const message = encode(xummJsonTx);
    const encodedSigningMessage = encodeForSigning(xummJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    let errorFound = false;
    await Promise.race([
      new Promise<any[]>(async (resolve) => {
        await api.tx.xrplTransaction.submitEncodedXummTransaction(`0x${message}`, `0x${signature}`).send(({ events = [], status }) => {
          if (status.isInBlock) resolve(events);
        });
      }),
      new Promise<any[]>((_, reject) => setTimeout(() => reject(new Error('timeout error')), 4000))
    ])
    .catch((err: any) => {
      errorFound = true;
      expect(err.message).eq("timeout error");
    });
    expect(errorFound).to.be.true;
  });
});
