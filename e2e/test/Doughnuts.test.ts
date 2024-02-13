import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a, u8aToHex } from "@polkadot/util";
import { blake2AsHex } from "@polkadot/util-crypto";
import { expect } from "chai";
import { Wallet } from "ethers";

import { OpCodeComparator, OpComp, OpLoad, Pact } from "../../../../pact/js";
import { Doughnut, PayloadVersion, SignatureVersion, TRNNut } from "../../../../trn-doughnut-rs/js";
import {
  ALICE_PRIVATE_KEY,
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  GAS_TOKEN_ID,
  NodeProcess,
  finalizeTx,
  startNode,
  typedefs,
} from "../common";

const TRN_PERMISSION_DOMAIN: string = "trn";

describe("Doughnuts", () => {
  let node: NodeProcess;
  let api: ApiPromise;
  let bob: KeyringPair;
  let alice: KeyringPair;
  let keyring: Keyring;
  let alith: KeyringPair;

  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });

    keyring = new Keyring({ type: "ethereum" });
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));
    alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
  });

  after(async () => node.stop());

  it("doughnut works - alice issued doughnut for Balances::transfer with constraints amount = 10", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const transferAmount = 10;
    const nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const call = api.tx.balances.transfer(receiverAddress, transferAmount);
    const version = 1;
    const issuerPubkey = alice.publicKey;
    const holderPubkey = holder.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    console.log("\n====  Creating Doughnut");
    const doughnut = new Doughnut(version, issuerPubkey, holderPubkey, feeMode, expiry, notBefore);
    // Create the permission domain object. Balances::transfer with a constraint for amount = 10
    const dataTable = ["10"];
    const comp = new OpCodeComparator(OpLoad.INPUT_VS_USER, OpComp.EQ, 1, 0, false); // RHS is data table
    const bytecode = new Uint8Array([...comp.encode()]);
    const pactContract = new Pact(dataTable, bytecode);
    const pactEncoded = pactContract.encode();
    // console.log(pactEncoded);

    const module = [
      {
        name: "Balances",
        block_cooldown: 0,
        methods: [
          {
            name: "transfer",
            block_cooldown: 0,
            constraints: [...pactEncoded],
          },
        ],
      },
    ];

    const trnnut = new TRNNut(module);

    // Add to trn domain
    doughnut.addDomain(TRN_PERMISSION_DOMAIN, trnnut.encode());
    console.log(`Domain    : ${doughnut.domain(TRN_PERMISSION_DOMAIN)}`);

    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await aliceWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    console.log(`Signature : ${doughnut.signature()}`);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // Create a call with empty signature to be signed by the holder (Bob)
    const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, "");
    // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
    const txU8a = tx.toU8a(true).slice(2);
    const txHex = u8aToHex(txU8a);
    const holderWallet = await new Wallet(holderPrivateKey);
    const txHash = blake2AsHex(txHex);
    const txSlice = Buffer.from(txHash.slice(2), "hex");
    const holderSig = await holderWallet.signMessage(txSlice);

    // alice balance before
    const alice_balance_before = await api.query.system.account(alice.address);

    // whitelist the holder. i.e bob
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(holder.address, true)));

    // Execute the transact call with.send
    const eventData = await new Promise<any[]>((resolve, _reject) => {
      api.tx.doughnut.transact(call, doughnutHex, nonce, holderSig).send(({ events, status }) => {
        if (status.isInBlock) {
          resolve(events);
        }
      });
    });
    expect(eventData).to.exist;
    // eventData.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    const balance = await api.query.system.account(receiverAddress);
    const freeBalance = balance.toJSON()?.data.free;
    console.log(`Free balance after doughnut transact: ${freeBalance}`);
    expect(freeBalance).to.be.equal(transferAmount);

    const aliceBalanceAfter = await api.query.system.account(alice.address);
    // alice should bear transferAmount in Root + fees in XRP
    expect(alice_balance_before.toJSON()?.data.free - aliceBalanceAfter.toJSON()?.data.free).equal(transferAmount);

    // check the events
    expect(eventData.length).to.equal(8);
    let index = 0;

    // assetsExt	InternalWithdraw	[2,"0xE04CC55ebEE1cBCE552f250e85c57B70B2E2625b",875115]
    expect(eventData[index].event.section).to.equal("assetsExt");
    expect(eventData[index].event.method).to.equal("InternalWithdraw");
    expect(eventData[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(eventData[index].event.data[1].toString()).to.equal(alice.address); // issuer

    // system	NewAccount	["0x07486b456ca1A0fb92344061278dF1D3504C7FB0"]
    index += 1;
    expect(eventData[index].event.section).to.equal("system");
    expect(eventData[index].event.method).to.equal("NewAccount");
    expect(eventData[index].event.data[0].toString()).to.equal(receiverAddress);

    // balances	Endowed	["0xE0DE516A460Ad64105f5dF4010E58ECA9d23DFFD",10]
    index += 1;
    expect(eventData[index].event.section).to.equal("balances");
    expect(eventData[index].event.method).to.equal("Endowed");
    expect(eventData[index].event.data[0].toString()).to.equal(receiverAddress);
    expect(eventData[index].event.data[1]).to.equal(transferAmount);

    // balances	Transfer	["0xE04CC55ebEE1cBCE552f250e85c57B70B2E2625b","0xE0DE516A460Ad64105f5dF4010E58ECA9d23DFFD",10]
    index += 1;
    expect(eventData[index].event.section).to.equal("balances");
    expect(eventData[index].event.method).to.equal("Transfer");
    expect(eventData[index].event.data[0].toString()).to.equal(alice.address);
    expect(eventData[index].event.data[1].toString()).to.equal(receiverAddress);
    expect(eventData[index].event.data[2]).to.equal(transferAmount);

    // doughnut	DoughnutCallExecuted	[{"ok":null}]
    index += 1;
    expect(eventData[index].event.section).to.equal("doughnut");
    expect(eventData[index].event.method).to.equal("DoughnutCallExecuted");

    // assetsExt	InternalDeposit	[2,"0x6D6F646c7478666565706F740000000000000000",875115]
    index += 1;
    expect(eventData[index].event.section).to.equal("assetsExt");
    expect(eventData[index].event.method).to.equal("InternalDeposit");
    expect(eventData[index].event.data[0]).to.equal(GAS_TOKEN_ID);

    // transactionPayment	TransactionFeePaid	["0xE04CC55ebEE1cBCE552f250e85c57B70B2E2625b",875115,0]
    index += 1;
    expect(eventData[index].event.section).to.equal("transactionPayment");
    expect(eventData[index].event.method).to.equal("TransactionFeePaid");
    expect(eventData[index].event.data[0].toString()).to.equal(alice.address);

    // system	ExtrinsicSuccess	[{"weight":925414000,"class":"Normal","paysFee":"Yes"}]
    index += 1;
    expect(eventData[index].event.section).to.equal("system");
    expect(eventData[index].event.method).to.equal("ExtrinsicSuccess");
  });

  it("doughnut fails - alice issued doughnut for Balances::transfer with constraints amount = 10 can not be used to transfer amount != 10", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const transferAmount = 10;
    const nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const call = api.tx.balances.transfer(receiverAddress, transferAmount - 1);
    const version = 1;
    const issuerPubkey = alice.publicKey;
    const holderPubkey = holder.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    console.log("\n====  Creating Doughnut");
    const doughnut = new Doughnut(version, issuerPubkey, holderPubkey, feeMode, expiry, notBefore);
    // Create the permission domain object. Balances::transfer with a constraint for amount = 10
    const dataTable = ["10"];
    const comp = new OpCodeComparator(OpLoad.INPUT_VS_USER, OpComp.EQ, 1, 0, false); // RHS is data table
    const bytecode = new Uint8Array([...comp.encode()]);
    const pactContract = new Pact(dataTable, bytecode);
    const pactEncoded = pactContract.encode();
    // console.log(pactEncoded);

    const module = [
      {
        name: "Balances",
        block_cooldown: 0,
        methods: [
          {
            name: "transfer",
            block_cooldown: 0,
            constraints: [...pactEncoded],
          },
        ],
      },
    ];

    const trnnut = new TRNNut(module);

    // Add to trn domain
    doughnut.addDomain(TRN_PERMISSION_DOMAIN, trnnut.encode());
    console.log(`Domain    : ${doughnut.domain(TRN_PERMISSION_DOMAIN)}`);

    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await aliceWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);
    console.log(`Signature : ${doughnut.signature()}`);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // Create a call with empty signature to be signed by the holder (Bob)
    const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, "");
    // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
    const txU8a = tx.toU8a(true).slice(2);
    const txHex = u8aToHex(txU8a);
    const holderWallet = await new Wallet(holderPrivateKey);
    const txHash = blake2AsHex(txHex);
    const txSlice = Buffer.from(txHash.slice(2), "hex");
    const holderSig = await holderWallet.signMessage(txSlice);

    // alice balance before
    const alice_balance_before = await api.query.system.account(alice.address);

    // whitelist the holder. i.e bob
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(holder.address, true)));

    // Execute the transact call with.send
    await api.tx.doughnut
      .transact(call, doughnutHex, nonce, holderSig)
      .send()
      .catch((err: any) => {
        console.log(err);
      });

    const balance = await api.query.system.account(receiverAddress);
    const freeBalance = balance.toJSON()?.data.free;
    console.log(`Free balance after doughnut transact: ${freeBalance}`);
    expect(freeBalance).to.be.equal(0);

    const aliceBalanceAfter = await api.query.system.account(alice.address);
    // alice should bear transferAmount in Root + fees in XRP
    expect(alice_balance_before.toJSON()?.data.free - aliceBalanceAfter.toJSON()?.data.free).equal(0);
  });

  it("doughnut without trn permission object fails", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const transferAmount = 1234;
    const nonce = ((await api.query.system.account(bob.address)).toJSON() as any)?.nonce;
    const call = api.tx.balances.transfer(receiverAddress, transferAmount);
    const version = 1;
    const issuer = alice.publicKey;
    const holder = bob.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    console.log("\n====  Creating Doughnut");
    const doughnut = new Doughnut(version, issuer, holder, feeMode, expiry, notBefore);
    // Add a random domain to allow the doughnut to be encoded
    doughnut.addDomain("Test", new Uint8Array(12));
    console.log(`Domain    : ${doughnut.domain("Test")}`);

    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await aliceWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);
    console.log(`Signature : ${doughnut.signature()}`);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holder, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // Create a call with empty signature to be signed by the holder (Bob)
    const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, "");
    // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
    const txU8a = tx.toU8a(true).slice(2);
    const txHex = u8aToHex(txU8a);
    const holderWallet = await new Wallet(BOB_PRIVATE_KEY);
    const txHash = blake2AsHex(txHex);
    const txSlice = Buffer.from(txHash.slice(2), "hex");
    const holderSig = await holderWallet.signMessage(txSlice);

    // alice balance before
    const alice_balance_before = await api.query.system.account(alice.address);

    // whitelist the holder. i.e bob
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(bob.address, true)));

    // Execute the transact call with.send
    await api.tx.doughnut
      .transact(call, doughnutHex, nonce, holderSig)
      .send()
      .catch((err: any) => {
        console.log(err);
      });

    // console.log(events);
    const balance = await api.query.system.account(receiverAddress);
    const freeBalance = balance.toJSON()?.data.free;
    console.log(`Free balance after doughnut transact: ${freeBalance}`);
    expect(freeBalance).to.be.equal(0);

    const aliceBalanceAfter = await api.query.system.account(alice.address);
    // alice should bear transferAmount in Root + fees in XRP
    expect(alice_balance_before.toJSON()?.data.free - aliceBalanceAfter.toJSON()?.data.free).equal(0);
  });

  it("can submit futurepass proxy-extrinsic", async () => {
    // create a random user A
    const userAPrivateKey = Wallet.createRandom().privateKey;
    const userA: KeyringPair = keyring.addFromSeed(hexToU8a(userAPrivateKey));

    // create a futurepass for userA
    await finalizeTx(alice, api.tx.futurepass.create(userA.address));

    // fund the futurepass account
    const futurepassAddress = (await api.query.futurepass.holders(userA.address)).toString();
    const initialTfrAmount = 10_000_000; // 10 Root
    const initialGasAmount = 10_000_000; // 10 XRP
    await finalizeTx(alice, api.tx.balances.transfer(futurepassAddress, initialTfrAmount)); // Root
    await finalizeTx(alice, api.tx.assets.transfer(GAS_TOKEN_ID, futurepassAddress, initialGasAmount)); // gas

    // transfer balance from futurepass to another user receiverAddress
    const receiverAddress = await Wallet.createRandom().getAddress();
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const transferAmount = initialTfrAmount / 2; // transfer half to receiverAddress
    const nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const innerCall = api.tx.balances.transfer(receiverAddress, transferAmount);
    const call = api.tx.futurepass.proxyExtrinsic(futurepassAddress, innerCall);
    const issuerPubkey = userA.publicKey;
    const holderPubkey = holder.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    // create a doughnut
    const doughnut = new Doughnut(PayloadVersion.V1, issuerPubkey, holderPubkey, feeMode, expiry, notBefore);

    const module = [
      {
        name: "Balances",
        block_cooldown: 0,
        methods: [
          {
            name: "transfer",
            block_cooldown: 0,
            constraints: null,
          },
        ],
      },
    ];

    const trnnut = new TRNNut(module);

    // Add to trn domain
    doughnut.addDomain(TRN_PERMISSION_DOMAIN, trnnut.encode());
    // console.log(`Domain    : ${doughnut.domain(TRN_PERMISSION_DOMAIN)}`);

    // Sign the doughnut
    const userAWallet = await new Wallet(userAPrivateKey);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await userAWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    // console.log(`Signature : ${doughnut.signature()}`);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // Create a call with empty signature to be signed by the holder (Bob)
    const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, "");
    // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
    const txU8a = tx.toU8a(true).slice(2);
    const txHex = u8aToHex(txU8a);
    const holderWallet = await new Wallet(holderPrivateKey);
    const txHash = blake2AsHex(txHex);
    const txSlice = Buffer.from(txHash.slice(2), "hex");
    const holderSig = await holderWallet.signMessage(txSlice);

    // balances before
    const userARootBalanceBefore = ((await api.query.system.account(userA.address)).toJSON() as any)?.data.free ?? 0;
    const futurepassRootBalanceBefore =
      ((await api.query.system.account(futurepassAddress)).toJSON() as any)?.data.free ?? 0;
    const holderRootBalanceBefore = ((await api.query.system.account(holder.address)).toJSON() as any)?.data.free ?? 0;
    const receiverRootBalanceBefore =
      ((await api.query.system.account(receiverAddress)).toJSON() as any)?.data.free ?? 0;
    const userAXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, userA.address)).toJSON() as any)?.balance ?? 0;
    const futurepassXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
    const holderXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, holder.address)).toJSON() as any)?.balance ?? 0;
    const receiverXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, receiverAddress)).toJSON() as any)?.balance ?? 0;

    // whitelist the holder.
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(holder.address, true)));

    // Execute the transact call with.send
    const eventData = await new Promise<any[]>((resolve, _reject) => {
      api.tx.doughnut.transact(call, doughnutHex, nonce, holderSig).send(({ events, status }) => {
        if (status.isInBlock) {
          resolve(events);
        }
      });
    });
    expect(eventData).to.exist;
    // eventData.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    // assert events
    expect(eventData.length).to.equal(10);
    let index = 0;

    // assetsExt	InternalWithdraw	[2,"0xFfFFFFff00000000000000000000000000000001",995158]
    expect(eventData[index].event.section).to.equal("assetsExt");
    expect(eventData[index].event.method).to.equal("InternalWithdraw");
    expect(eventData[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(eventData[index].event.data[1].toString()).to.equal(futurepassAddress);

    // system	NewAccount	["0x07486b456ca1A0fb92344061278dF1D3504C7FB0"]
    index += 1;
    expect(eventData[index].event.section).to.equal("system");
    expect(eventData[index].event.method).to.equal("NewAccount");
    expect(eventData[index].event.data[0].toString()).to.equal(receiverAddress);

    // balances	Endowed	["0x07486b456ca1A0fb92344061278dF1D3504C7FB0",5000000]
    index += 1;
    expect(eventData[index].event.section).to.equal("balances");
    expect(eventData[index].event.method).to.equal("Endowed");
    expect(eventData[index].event.data[0].toString()).to.equal(receiverAddress);
    expect(eventData[index].event.data[1]).to.equal(transferAmount);

    // balances	Transfer	["0xFfFFFFff00000000000000000000000000000001","0x07486b456ca1A0fb92344061278dF1D3504C7FB0",5000000]
    index += 1;
    expect(eventData[index].event.section).to.equal("balances");
    expect(eventData[index].event.method).to.equal("Transfer");
    expect(eventData[index].event.data[0].toString()).to.equal(futurepassAddress);
    expect(eventData[index].event.data[1].toString()).to.equal(receiverAddress);
    expect(eventData[index].event.data[2]).to.equal(transferAmount);

    // proxy	ProxyExecuted	[{"ok":null}]
    index += 1;
    expect(eventData[index].event.section).to.equal("proxy");
    expect(eventData[index].event.method).to.equal("ProxyExecuted");

    // futurepass	ProxyExecuted	["0xb5571159C644DB649E698d4D66e1F32A59D8aC52",{"ok":null}]
    index += 1;
    expect(eventData[index].event.section).to.equal("futurepass");
    expect(eventData[index].event.method).to.equal("ProxyExecuted");
    expect(eventData[index].event.data[0].toString()).to.equal(userA.address); // issuer

    // doughnut	DoughnutCallExecuted	[{"ok":null}]
    index += 1;
    expect(eventData[index].event.section).to.equal("doughnut");
    expect(eventData[index].event.method).to.equal("DoughnutCallExecuted");

    // assetsExt	InternalDeposit	[2,"0x6D6F646c7478666565706F740000000000000000",995158]
    index += 1;
    expect(eventData[index].event.section).to.equal("assetsExt");
    expect(eventData[index].event.method).to.equal("InternalDeposit");
    expect(eventData[index].event.data[0]).to.equal(GAS_TOKEN_ID);

    // transactionPayment	TransactionFeePaid	["0xFfFFFFff00000000000000000000000000000001",995158,0]
    index += 1;
    expect(eventData[index].event.section).to.equal("transactionPayment");
    expect(eventData[index].event.method).to.equal("TransactionFeePaid");
    expect(eventData[index].event.data[0].toString()).to.equal(futurepassAddress);

    // system	ExtrinsicSuccess	[{"weight":1268268098,"class":"Normal","paysFee":"Yes"}]
    index += 1;
    expect(eventData[index].event.section).to.equal("system");
    expect(eventData[index].event.method).to.equal("ExtrinsicSuccess");

    // balances after
    const userARootBalanceAfter = ((await api.query.system.account(userA.address)).toJSON() as any)?.data.free ?? 0;
    const futurepassRootBalanceAfter =
      ((await api.query.system.account(futurepassAddress)).toJSON() as any)?.data.free ?? 0;
    const holderRootBalanceAfter = ((await api.query.system.account(holder.address)).toJSON() as any)?.data.free ?? 0;
    const receiverRootBalanceAfter =
      ((await api.query.system.account(receiverAddress)).toJSON() as any)?.data.free ?? 0;
    const userAXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, userA.address)).toJSON() as any)?.balance ?? 0;
    const futurepassXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
    const holderXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, holder.address)).toJSON() as any)?.balance ?? 0;
    const receiverXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, receiverAddress)).toJSON() as any)?.balance ?? 0;

    // userA root balance should be unchanged
    expect(userARootBalanceAfter).to.be.eq(userARootBalanceBefore);
    // userA xrp balance should be unchanged
    expect(userAXRPBalanceAfter).to.be.eq(userAXRPBalanceBefore);

    // futurepass root balance should be lesser by transferAmount
    expect(futurepassRootBalanceBefore - transferAmount).to.be.eq(futurepassRootBalanceAfter);
    // futurepass xrp balance should be lesser than before since gas was paid
    expect(futurepassXRPBalanceAfter).to.be.lessThan(futurepassXRPBalanceBefore);

    // doughnut holder should not get affected at all
    expect(holderRootBalanceAfter).to.be.eq(holderRootBalanceBefore);
    expect(holderXRPBalanceAfter).to.be.eq(holderXRPBalanceBefore);

    // receiver root balance should be increased by transferAmount
    expect(Math.abs(receiverRootBalanceBefore - transferAmount)).to.be.eq(receiverRootBalanceAfter);
    // receiver XRP balance unchanged
    expect(receiverXRPBalanceAfter).to.be.eq(receiverXRPBalanceBefore);
  });
});
