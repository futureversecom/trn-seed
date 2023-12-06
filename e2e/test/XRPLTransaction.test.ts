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

  it.skip("xrpl transaction unsigned extrinsic with signed payload", async () => {
    // Create a payload (e.g., a message) and sign it
    const publicKey = '02A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A';
    const message = '5354580059169587200771100000000000000000004E8E737E560742FFB6566219799A06C5732102A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A7446304402200973DE957E7FF3195AD8220032A7BB9E81D2C50193903131F61E304EDE5B6AA502207E4AE27CB73EDCACA55F0B149BBF1E7929D572DBD77F3E29C7494A0F9DDFDC15811424A53BB5CAAD40A961836FEF648E8424846EC75A';
    const signature = '3045022100D5AF787E3E16876F87149C524E94B02BCC7FA499F6941D22E36650C44CCEF89A02204597361DED30D499A021497DEED1BEC917B6B91B796765D010754525D99287DB';

    // public: Public,
		// encoded_msg: Vec<u8>,
		// signature: Vec<u8>, 
    
    // Create the extrinsic to call a custom runtime method that handles signed payloads
    // Assume the runtime has a method `verifyAndExecute` in a pallet named `customModule`
    const extrinsic = api.tx.xrplTransaction.submitEncodedXummTransactionWithSignedPayload({
      public: publicKey,
      encodedMsg: message,
      signature,
      // (u8aToHex(messageU8a),
      // u8aToHex(signature),
    });
    const unsub = await extrinsic.send(({ events = [], status }) => {
      console.log(`Transaction status: ${status.type}`);
      if (status.isInBlock) {
        console.log(`Included at block hash: ${status.asInBlock.toHex()}`);
        console.log(`Events:`);
  
        events.forEach(({ event: { data, method, section }, phase }) => {
          console.log(`\t${phase}: ${section}.${method} ${data}`);
        });
  
        unsub();
      }
    });

    console.warn('hi');
  });

  it.only("can submit system remark extrinsic", async () => {
    const user = Wallet.createRandom();

    // fund the user account to pay for tx fees
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 10_000_000));

    const extrinsic = api.tx.system.remark("hello world");
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xrpBalanceBefore = ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    console.log("xrp bal before", xrpBalanceBefore)

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

    events.forEach(({ event: { data, method, section }, phase }) => console.log(`${section}\t${method}\t${data}`));

    // assert events
    expect(events.length).to.equal(5);

    // assetsExt InternalWithdraw [2,"0x8800043D76AFd08b019F3db2016b9573041C1B59",560011]
    expect(events[0].event.section).to.equal("assetsExt");
    expect(events[0].event.method).to.equal("InternalWithdraw");

    // xrplTransaction XUMMExtrinsicExecuted ["0x27Fd5891543A45aB8a0B7A387285bdd4A6562B51",0,{"callIndex":"0x0001","args":{"remark":"0x68656c6c6f20776f726c64"}}]
    expect(events[1].event.section).to.equal("xrplTransaction");
    expect(events[1].event.method).to.equal("XUMMExtrinsicExecuted");
    expect(events[1].event.data[0].toString()).to.equal(user.address);

    // assetsExt InternalDeposit [2,"0x6D6F646c7478666565706F740000000000000000",557511]
    expect(events[2].event.section).to.equal("assetsExt");
    expect(events[2].event.method).to.equal("InternalDeposit");

    // transactionPayment TransactionFeePaid ["0xe8d9B65B4D1daA328b4980405393a9563FecC592",557511,0]
    expect(events[3].event.section).to.equal("transactionPayment");
    expect(events[3].event.method).to.equal("TransactionFeePaid");

    // system ExtrinsicSuccess [{"weight":86298000,"class":"Normal","paysFee":"Yes"}]
    expect(events[4].event.section).to.equal("system");
    expect(events[4].event.method).to.equal("ExtrinsicSuccess");

    // assert balance after < balance before (tx fee must be paid)
    const xrpBalanceAfter = ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    console.log("xrp bal after", xrpBalanceBefore)

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

    // events.forEach(({ event: { data, method, section }, phase }) => console.log(`${section}\t${method}\t${data}`));

    // assert events
    expect(events.length).to.equal(3);

    // assets Transferred [2,"0x582F0E877a678ab8Ddb13a9ebBECf86614f3916E","0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",1000]
    expect(events[0].event.section).to.equal("assets");
    expect(events[0].event.method).to.equal("Transferred");
    expect(events[0].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(events[0].event.data[1].toString()).to.equal(user.address);
    expect(events[0].event.data[2].toString()).to.equal(alith.address);

    // xrplTransaction XUMMExtrinsicExecuted ["0x582F0E877a678ab8Ddb13a9ebBECf86614f3916E",0,{"callIndex":"0x0605","args":{"id":2,"target":"0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac","amount":1000}}]
    expect(events[1].event.section).to.equal("xrplTransaction");
    expect(events[1].event.method).to.equal("XUMMExtrinsicExecuted");
    expect(events[1].event.data[0].toString()).to.equal(user.address);

    // system ExtrinsicSuccess [{"weight":86298000,"class":"Normal","paysFee":"Yes"}]
    expect(events[2].event.section).to.equal("system");
    expect(events[2].event.method).to.equal("ExtrinsicSuccess");
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

    // assert events
    expect(events.length).to.equal(5);

    // events.forEach(({ event: { data, method, section }, phase }) => console.log(`${section}\t${method}\t${data}`));
    
    expect(events[0].event.section).to.equal("assets");
    expect(events[0].event.method).to.equal("Transferred");
    expect(events[0].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(events[0].event.data[1].toString()).to.equal(futurepassAddress);
    expect(events[0].event.data[2].toString()).to.equal(alith.address);

    // proxy ProxyExecuted [{"ok":null}]
    expect(events[1].event.section).to.equal("proxy");
    expect(events[1].event.method).to.equal("ProxyExecuted");

    // futurepass ProxyExecuted ["0x557fce5210eaaE26893404Bf14A1423F8f22EAD9",{"ok":null}]
    expect(events[2].event.section).to.equal("futurepass");
    expect(events[2].event.method).to.equal("ProxyExecuted");
    expect(events[2].event.data[0].toString()).to.equal(user.address);

    // xrplTransaction XUMMExtrinsicExecuted ["0x557fce5210eaaE26893404Bf14A1423F8f22EAD9",0,{"callIndex":"0x2204","args":{"futurepass":"0xfFFFFfff00000000000000000000000000000008","call":{"callIndex":"0x0605","args":{"id":2,"target":"0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac","amount":1000}}}}]
    expect(events[3].event.section).to.equal("xrplTransaction");
    expect(events[3].event.method).to.equal("XUMMExtrinsicExecuted");
    expect(events[3].event.data[0].toString()).to.equal(user.address);
    
    // system ExtrinsicSuccess [{"weight":86298000,"class":"Normal","paysFee":"Yes"}]
    expect(events[4].event.section).to.equal("system");
    expect(events[4].event.method).to.equal("ExtrinsicSuccess");
  });
});
