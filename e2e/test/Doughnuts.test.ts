import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { SubmittableResultValue } from "@polkadot/api/types";
import { KeyringPair } from "@polkadot/keyring/types";
import { DispatchError } from "@polkadot/types/interfaces";
import { hexToU8a, u8aToHex } from "@polkadot/util";
import { blake2AsHex } from "@polkadot/util-crypto";
import { Doughnut, PayloadVersion, SignatureVersion, Topping } from "@therootnetwork/doughnut-nodejs";
import { OpCodeComparator, OpComp, OpLoad, Pact } from "@therootnetwork/pact-nodejs";
import { expect } from "chai";
import { Wallet } from "ethers";

import {
  ALICE_PRIVATE_KEY,
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  GAS_TOKEN_ID,
  NodeProcess,
  finalizeTx,
  sleep,
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

    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });

    keyring = new Keyring({ type: "ethereum" });
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));
    alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    // get rid of fee pot account creation event, i.e
    // system	NewAccount	["0x6D6F646c7478666565706F740000000000000000"]
    await finalizeTx(alith, api.tx.balances.transfer("0x6D6F646c7478666565706F740000000000000000", 10));
  });

  after(async () => node.stop());

  it("doughnut works - alice issued doughnut for Balances::transfer with constraints amount = 10", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const transferAmount = 10;
    const nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const genesis_hash = await api.rpc.chain.getBlockHash(0);
    const tip = 0;
    const call = api.tx.balances.transfer(receiverAddress, transferAmount);
    const version = 1;
    const issuerPubkey = alice.publicKey;
    const holderPubkey = holder.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    const doughnut = new Doughnut(version, issuerPubkey, holderPubkey, feeMode, expiry, notBefore);
    // Create the permission topping object. Balances::transfer with a constraint for amount = 10
    const dataTable = ["10"];
    const comp = new OpCodeComparator(OpLoad.InputVsUser, OpComp.EQ, 1, 0, false); // RHS is the data table
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

    const topping = new Topping(module);

    // Add to trn topping
    doughnut.addTopping(TRN_PERMISSION_DOMAIN, topping.encode());

    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await aliceWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // Create a call with empty signature to be signed by the holder (Bob)
    const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
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
      api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig).send(({ events, status }) => {
        if (status.isInBlock) {
          resolve(events);
        }
      });
    });
    expect(eventData).to.exist;
    // eventData.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    const balance = await api.query.system.account(receiverAddress);
    const freeBalance = balance.toJSON()?.data.free;
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
    const genesis_hash = await api.rpc.chain.getBlockHash(0);
    const tip = 0;
    const call = api.tx.balances.transfer(receiverAddress, transferAmount - 1);
    const version = 1;
    const issuerPubkey = alice.publicKey;
    const holderPubkey = holder.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    const doughnut = new Doughnut(version, issuerPubkey, holderPubkey, feeMode, expiry, notBefore);
    // Create the permission topping object. Balances::transfer with a constraint for amount = 10
    const dataTable = ["10"];
    const comp = new OpCodeComparator(OpLoad.InputVsUser, OpComp.EQ, 1, 0, false); // RHS is the data table
    const bytecode = new Uint8Array([...comp.encode()]);
    const pactContract = new Pact(dataTable, bytecode);
    const pactEncoded = pactContract.encode();

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

    const topping = new Topping(module);

    // Add to trn topping
    doughnut.addTopping(TRN_PERMISSION_DOMAIN, topping.encode());

    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await aliceWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // Create a call with empty signature to be signed by the holder (Bob)
    const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
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
      .transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig)
      .send()
      .catch((err: any) => {
        console.log(err);
      });

    const balance = await api.query.system.account(receiverAddress);
    const freeBalance = balance.toJSON()?.data.free;
    expect(freeBalance).to.be.equal(0);

    const aliceBalanceAfter = await api.query.system.account(alice.address);
    // alice should bear transferAmount in Root + fees in XRP
    expect(alice_balance_before.toJSON()?.data.free - aliceBalanceAfter.toJSON()?.data.free).equal(0);
  });

  it("doughnut without trn permission object fails", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const transferAmount = 1234;
    const nonce = ((await api.query.system.account(bob.address)).toJSON() as any)?.nonce;
    const genesis_hash = await api.rpc.chain.getBlockHash(0);
    const tip = 0;
    const call = api.tx.balances.transfer(receiverAddress, transferAmount);
    const version = 1;
    const issuer = alice.publicKey;
    const holder = bob.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    const doughnut = new Doughnut(version, issuer, holder, feeMode, expiry, notBefore);
    // Add a random topping to allow the doughnut to be encoded
    doughnut.addTopping("Test", new Uint8Array(12));

    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await aliceWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holder, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // Create a call with empty signature to be signed by the holder (Bob)
    const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
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
      .transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig)
      .send()
      .catch((err: any) => {
        console.log("DOUGHNUT ERR", err);
      });

    // console.log(events);
    const balance = await api.query.system.account(receiverAddress);
    const freeBalance = balance.toJSON()?.data.free;
    expect(freeBalance).to.be.equal(0);

    const aliceBalanceAfter = await api.query.system.account(alice.address);
    // alice should bear transferAmount in Root + fees in XRP
    expect(alice_balance_before.toJSON()?.data.free - aliceBalanceAfter.toJSON()?.data.free).equal(0);
  });

  it("doughnut works - incorrect genesis hash fails", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const transferAmount = 10;
    const nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const genesis_hash = await api.rpc.chain.getBlockHash(1);
    const tip = 0;
    const call = api.tx.balances.transfer(receiverAddress, transferAmount);
    const version = 1;
    const issuerPubkey = alice.publicKey;
    const holderPubkey = holder.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    const doughnut = new Doughnut(version, issuerPubkey, holderPubkey, feeMode, expiry, notBefore);
    // Create the permission topping object. Balances::transfer with a constraint for amount = 10
    const dataTable = ["10"];
    const comp = new OpCodeComparator(OpLoad.InputVsUser, OpComp.EQ, 1, 0, false); // RHS is the data table
    const bytecode = new Uint8Array([...comp.encode()]);
    const pactContract = new Pact(dataTable, bytecode);
    const pactEncoded = pactContract.encode();

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

    const topping = new Topping(module);

    // Add to trn topping
    doughnut.addTopping(TRN_PERMISSION_DOMAIN, topping.encode());

    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await aliceWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // Create a call with empty signature to be signed by the holder (Bob)
    const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
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
      .transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig)
      .send()
      .catch((_err: any) => {
        // console.log(err);
      });

    // console.log(events);
    const balance = await api.query.system.account(receiverAddress);
    const freeBalance = balance.toJSON()?.data.free;
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
    const genesis_hash = await api.rpc.chain.getBlockHash(0);
    const tip = 0;
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

    const topping = new Topping(module);

    // Add to trn topping
    doughnut.addTopping(TRN_PERMISSION_DOMAIN, topping.encode());

    // Sign the doughnut
    const userAWallet = await new Wallet(userAPrivateKey);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await userAWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // Create a call with empty signature to be signed by the holder
    const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
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
      api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig).send(({ events, status }) => {
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

  it("can submit futurepass fee-proxy proxy-extrinsic", async () => {
    // create a random user A
    const userAPrivateKey = Wallet.createRandom().privateKey;
    const userA: KeyringPair = keyring.addFromSeed(hexToU8a(userAPrivateKey));

    // create a futurepass for userA
    await finalizeTx(alice, api.tx.futurepass.create(userA.address));
    const futurepassAddress = (await api.query.futurepass.holders(userA.address)).toString();

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

    // call setup
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const innerCall = api.tx.system.remark("sup");
    const futurepassCall = api.tx.futurepass.proxyExtrinsic(futurepassAddress, innerCall);
    const maxTokenPayment = 5_000_000;
    const call = api.tx.feeProxy.callWithFeePreferences(FEE_TOKEN_ASSET_ID, maxTokenPayment, futurepassCall);
    const nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const genesis_hash = await api.rpc.chain.getBlockHash(0);
    const tip = 0;
    const issuerPubkey = userA.publicKey;
    const holderPubkey = holder.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    // create a doughnut
    const doughnut = new Doughnut(PayloadVersion.V1, issuerPubkey, holderPubkey, feeMode, expiry, notBefore);

    const module = [
      {
        name: "System",
        block_cooldown: 0,
        methods: [
          {
            name: "remark",
            block_cooldown: 0,
            constraints: null,
          },
        ],
      },
    ];

    const topping = new Topping(module);

    // Add to trn topping
    doughnut.addTopping(TRN_PERMISSION_DOMAIN, topping.encode());

    // Sign the doughnut
    const userAWallet = await new Wallet(userAPrivateKey);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await userAWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // Create a call with empty signature to be signed by the holder
    const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
    // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
    const txU8a = tx.toU8a(true).slice(2);
    const txHex = u8aToHex(txU8a);
    const holderWallet = await new Wallet(holderPrivateKey);
    const txHash = blake2AsHex(txHex);
    const txSlice = Buffer.from(txHash.slice(2), "hex");
    const holderSig = await holderWallet.signMessage(txSlice);

    // balances before
    const userAAssetBalanceBefore =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, userA.address)).toJSON() as any)?.balance ?? 0;
    const futurepassAssetBalanceBefore =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
    const holderAssetBalanceBefore =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, holder.address)).toJSON() as any)?.balance ?? 0;
    const userAXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, userA.address)).toJSON() as any)?.balance ?? 0;
    const futurepassXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
    const holderXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, holder.address)).toJSON() as any)?.balance ?? 0;

    // whitelist the holder.
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(holder.address, true)));

    // Execute the transact call with.send
    const eventData = await new Promise<any[]>((resolve, _reject) => {
      api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig).send(({ events, status }) => {
        if (status.isInBlock) {
          resolve(events);
        }
      });
    });
    expect(eventData).to.exist;
    // eventData.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    // assert events
    expect(eventData.length).to.equal(12);
    let index = 0;

    // assets	Transferred	[1124,"0xFfFFFFff00000000000000000000000000000001","0xDDDDDDdD00000002000004640000000000000000",907864]
    expect(eventData[index].event.section).to.equal("assets");
    expect(eventData[index].event.method).to.equal("Transferred");
    expect(eventData[index].event.data[0]).to.equal(FEE_TOKEN_ASSET_ID);
    expect(eventData[index].event.data[1].toString()).to.equal(futurepassAddress);
    expect(eventData[index].event.data[2].toString()).to.equal("0xDDDDDDdD00000002000004640000000000000000");

    // assets	Transferred	[2,"0xDDDDDDdD00000002000004640000000000000000","0xFfFFFFff00000000000000000000000000000001",905132]
    index += 1;
    expect(eventData[index].event.section).to.equal("assets");
    expect(eventData[index].event.method).to.equal("Transferred");
    expect(eventData[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(eventData[index].event.data[1].toString()).to.equal("0xDDDDDDdD00000002000004640000000000000000");
    expect(eventData[index].event.data[2].toString()).to.equal(futurepassAddress);

    // assets	Issued	[7268,"0x6D6F646c7478666565706F740000000000000000",226]
    index += 1;
    expect(eventData[index].event.section).to.equal("assets");
    expect(eventData[index].event.method).to.equal("Issued");
    expect(eventData[index].event.data[0]).to.equal(7268);

    // dex	Swap	["0xFfFFFFff00000000000000000000000000000001",[1124,2],907864,905132,"0xFfFFFFff00000000000000000000000000000001"]
    index += 1;
    expect(eventData[index].event.section).to.equal("dex");
    expect(eventData[index].event.method).to.equal("Swap");
    expect(eventData[index].event.data[0].toString()).to.equal(futurepassAddress);
    expect(eventData[index].event.data[1].toString()).to.equal(`[${FEE_TOKEN_ASSET_ID}, ${GAS_TOKEN_ID}]`);

    // assetsExt	InternalWithdraw	[2,"0xFfFFFFff00000000000000000000000000000001",905132]
    index += 1;
    expect(eventData[index].event.section).to.equal("assetsExt");
    expect(eventData[index].event.method).to.equal("InternalWithdraw");
    expect(eventData[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(eventData[index].event.data[1].toString()).to.equal(futurepassAddress);

    // proxy ProxyExecuted [{"ok":null}]
    index += 1;
    expect(eventData[index].event.section).to.equal("proxy");
    expect(eventData[index].event.method).to.equal("ProxyExecuted");

    // futurepass	ProxyExecuted	["0xAD20F2B98D9004213b301Cc62b5B838e090bB11d",{"ok":null}]
    index += 1;
    expect(eventData[index].event.section).to.equal("futurepass");
    expect(eventData[index].event.method).to.equal("ProxyExecuted");
    expect(eventData[index].event.data[0].toString()).to.equal(userA.address);

    // feeProxy	CallWithFeePreferences	["0xAD20F2B98D9004213b301Cc62b5B838e090bB11d",1124,5000000]
    index += 1;
    expect(eventData[index].event.section).to.equal("feeProxy");
    expect(eventData[index].event.method).to.equal("CallWithFeePreferences");
    expect(eventData[index].event.data[0].toString()).to.equal(userA.address);
    expect(eventData[index].event.data[1]).to.equal(FEE_TOKEN_ASSET_ID);
    expect(eventData[index].event.data[2]).to.equal(maxTokenPayment);

    // doughnut	DoughnutCallExecuted	[{"ok":null}]
    index += 1;
    expect(eventData[index].event.section).to.equal("doughnut");
    expect(eventData[index].event.method).to.equal("DoughnutCallExecuted");

    // assetsExt	InternalDeposit	[2,"0x6D6F646c7478666565706F740000000000000000",905132]
    index += 1;
    expect(eventData[index].event.section).to.equal("assetsExt");
    expect(eventData[index].event.method).to.equal("InternalDeposit");
    expect(eventData[index].event.data[0]).to.equal(GAS_TOKEN_ID);

    // transactionPayment	TransactionFeePaid	["0xFfFFFFff00000000000000000000000000000001",905132,0]
    index += 1;
    expect(eventData[index].event.section).to.equal("transactionPayment");
    expect(eventData[index].event.method).to.equal("TransactionFeePaid");
    expect(eventData[index].event.data[0].toString()).to.equal(futurepassAddress);

    // system	ExtrinsicSuccess	[{"weight":1060235970,"class":"Normal","paysFee":"Yes"}]
    index += 1;
    expect(eventData[index].event.section).to.equal("system");
    expect(eventData[index].event.method).to.equal("ExtrinsicSuccess");

    // balances after
    const userAAssetBalanceAfter =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, userA.address)).toJSON() as any)?.balance ?? 0;
    const futurepassAssetBalanceAfter =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
    const holderAssetBalanceAfter =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, holder.address)).toJSON() as any)?.balance ?? 0;
    const userAXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, userA.address)).toJSON() as any)?.balance ?? 0;
    const futurepassXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
    const holderXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, holder.address)).toJSON() as any)?.balance ?? 0;

    // userA xrp balance should be unchanged
    expect(userAXRPBalanceAfter).to.be.eq(userAXRPBalanceBefore);
    // userA asset balance should be unchanged
    expect(userAAssetBalanceAfter).to.be.eq(userAAssetBalanceBefore);

    // futurepass xrp balance should not be changed since gas was paid using another asset
    expect(futurepassXRPBalanceAfter).to.be.eq(futurepassXRPBalanceBefore + 1); // 1 existential deposit
    // futurepass asset balance should be lesser since gas is paid
    expect(futurepassAssetBalanceAfter).to.be.lessThan(futurepassAssetBalanceBefore);

    // doughnut holder should not get affected at all
    expect(holderXRPBalanceAfter).to.be.eq(holderXRPBalanceBefore);
    expect(holderAssetBalanceAfter).to.be.eq(holderAssetBalanceBefore);
  });

  it("can submit with fee-proxy", async () => {
    // create a random user A
    const userAPrivateKey = Wallet.createRandom().privateKey;
    const userA: KeyringPair = keyring.addFromSeed(hexToU8a(userAPrivateKey));

    // add liquidity for XRP<->token; fund the userA account with tokens
    const FEE_TOKEN_ASSET_ID = 1124;
    const txs = [
      api.tx.assetsExt.createAsset("test", "TEST", 18, 1, alith.address),
      api.tx.assets.mint(FEE_TOKEN_ASSET_ID, alith.address, 2_000_000_000_000_000),
      api.tx.assets.mint(FEE_TOKEN_ASSET_ID, userA.address, 2_000_000_000_000_000),
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

    // call setup
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const innerCall = api.tx.system.remark("sup");
    const maxTokenPayment = 5_000_000;
    const call = api.tx.feeProxy.callWithFeePreferences(FEE_TOKEN_ASSET_ID, maxTokenPayment, innerCall);
    const nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const genesis_hash = await api.rpc.chain.getBlockHash(0);
    const tip = 0;
    const issuerPubkey = userA.publicKey;
    const holderPubkey = holder.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    // create a doughnut
    const doughnut = new Doughnut(PayloadVersion.V1, issuerPubkey, holderPubkey, feeMode, expiry, notBefore);

    const module = [
      {
        name: "System",
        block_cooldown: 0,
        methods: [
          {
            name: "remark",
            block_cooldown: 0,
            constraints: null,
          },
        ],
      },
    ];

    const topping = new Topping(module);

    // Add to trn topping
    doughnut.addTopping(TRN_PERMISSION_DOMAIN, topping.encode());

    // Sign the doughnut
    const userAWallet = await new Wallet(userAPrivateKey);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await userAWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // Create a call with empty signature to be signed by the holder
    const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
    // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
    const txU8a = tx.toU8a(true).slice(2);
    const txHex = u8aToHex(txU8a);
    const holderWallet = await new Wallet(holderPrivateKey);
    const txHash = blake2AsHex(txHex);
    const txSlice = Buffer.from(txHash.slice(2), "hex");
    const holderSig = await holderWallet.signMessage(txSlice);

    // balances before
    const userAAssetBalanceBefore =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, userA.address)).toJSON() as any)?.balance ?? 0;
    const holderAssetBalanceBefore =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, holder.address)).toJSON() as any)?.balance ?? 0;
    const userAXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, userA.address)).toJSON() as any)?.balance ?? 0;
    const holderXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, holder.address)).toJSON() as any)?.balance ?? 0;

    // whitelist the holder.
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(holder.address, true)));

    // Execute the transact call with.send
    const eventData = await new Promise<any[]>((resolve, _reject) => {
      api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig).send(({ events, status }) => {
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

    // assets	Transferred	[1124,"0x3fAc4E88185Add21c375653b3204c1EF7fff9dE9","0xDDDDDDdD00000002000004640000000000000000",852669]
    expect(eventData[index].event.section).to.equal("assets");
    expect(eventData[index].event.method).to.equal("Transferred");
    expect(eventData[index].event.data[0]).to.equal(FEE_TOKEN_ASSET_ID);
    expect(eventData[index].event.data[1].toString()).to.equal(userA.address);
    expect(eventData[index].event.data[2].toString()).to.equal("0xDDDDDDdD00000002000004640000000000000000");

    // assets	Transferred	[2,"0xDDDDDDdD00000002000004640000000000000000","0x3fAc4E88185Add21c375653b3204c1EF7fff9dE9",850089]
    index += 1;
    expect(eventData[index].event.section).to.equal("assets");
    expect(eventData[index].event.method).to.equal("Transferred");
    expect(eventData[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(eventData[index].event.data[1].toString()).to.equal("0xDDDDDDdD00000002000004640000000000000000");
    expect(eventData[index].event.data[2].toString()).to.equal(userA.address);

    // assets	Issued	[7268,"0x6D6F646c7478666565706F740000000000000000",213]
    index += 1;
    expect(eventData[index].event.section).to.equal("assets");
    expect(eventData[index].event.method).to.equal("Issued");
    expect(eventData[index].event.data[0]).to.equal(7268);

    // dex	Swap	["0x3fAc4E88185Add21c375653b3204c1EF7fff9dE9",[1124,2],852669,850089,"0x3fAc4E88185Add21c375653b3204c1EF7fff9dE9"]
    index += 1;
    expect(eventData[index].event.section).to.equal("dex");
    expect(eventData[index].event.method).to.equal("Swap");
    expect(eventData[index].event.data[0].toString()).to.equal(userA.address);
    expect(eventData[index].event.data[1].toString()).to.equal(`[${FEE_TOKEN_ASSET_ID}, ${GAS_TOKEN_ID}]`);

    // assetsExt	InternalWithdraw	[2,"0x3fAc4E88185Add21c375653b3204c1EF7fff9dE9",850089]
    index += 1;
    expect(eventData[index].event.section).to.equal("assetsExt");
    expect(eventData[index].event.method).to.equal("InternalWithdraw");
    expect(eventData[index].event.data[0]).to.equal(GAS_TOKEN_ID);
    expect(eventData[index].event.data[1].toString()).to.equal(userA.address);

    // feeProxy	CallWithFeePreferences	["0x3fAc4E88185Add21c375653b3204c1EF7fff9dE9",1124,5000000]
    index += 1;
    expect(eventData[index].event.section).to.equal("feeProxy");
    expect(eventData[index].event.method).to.equal("CallWithFeePreferences");
    expect(eventData[index].event.data[0].toString()).to.equal(userA.address);
    expect(eventData[index].event.data[1]).to.equal(FEE_TOKEN_ASSET_ID);
    expect(eventData[index].event.data[2]).to.equal(maxTokenPayment);

    // doughnut	DoughnutCallExecuted	[{"ok":null}]
    index += 1;
    expect(eventData[index].event.section).to.equal("doughnut");
    expect(eventData[index].event.method).to.equal("DoughnutCallExecuted");

    // assetsExt	InternalDeposit	[2,"0x6D6F646c7478666565706F740000000000000000",905132]
    index += 1;
    expect(eventData[index].event.section).to.equal("assetsExt");
    expect(eventData[index].event.method).to.equal("InternalDeposit");
    expect(eventData[index].event.data[0]).to.equal(GAS_TOKEN_ID);

    // transactionPayment	TransactionFeePaid	["0x3fAc4E88185Add21c375653b3204c1EF7fff9dE9",850089,0]
    index += 1;
    expect(eventData[index].event.section).to.equal("transactionPayment");
    expect(eventData[index].event.method).to.equal("TransactionFeePaid");
    expect(eventData[index].event.data[0].toString()).to.equal(userA.address);

    // system	ExtrinsicSuccess	[{"weight":717381872,"class":"Normal","paysFee":"Yes"}]
    index += 1;
    expect(eventData[index].event.section).to.equal("system");
    expect(eventData[index].event.method).to.equal("ExtrinsicSuccess");

    // balances after
    const userAAssetBalanceAfter =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, userA.address)).toJSON() as any)?.balance ?? 0;
    const holderAssetBalanceAfter =
      ((await api.query.assets.account(FEE_TOKEN_ASSET_ID, holder.address)).toJSON() as any)?.balance ?? 0;
    const userAXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, userA.address)).toJSON() as any)?.balance ?? 0;
    const holderXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, holder.address)).toJSON() as any)?.balance ?? 0;

    // userA xrp balance should be unchanged since the gas was paid using a different asset
    expect(userAXRPBalanceAfter).to.be.eq(userAXRPBalanceBefore + 1); // 1 existential deposit
    // userA asset balance should be lesser since gas is paid
    expect(userAAssetBalanceAfter).to.be.lessThan(userAAssetBalanceBefore);

    // doughnut holder should not get affected at all
    expect(holderXRPBalanceAfter).to.be.eq(holderXRPBalanceBefore);
    expect(holderAssetBalanceAfter).to.be.eq(holderAssetBalanceBefore);
  });

  it("doughnut - Balances::transfer with constraints amount < 10", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const transferAmountLimit = 10;
    let nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const genesis_hash = await api.rpc.chain.getBlockHash(0);
    const tip = 0;
    const version = 1;
    const issuerPubkey = alice.publicKey;
    const holderPubkey = holder.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    const doughnut = new Doughnut(version, issuerPubkey, holderPubkey, feeMode, expiry, notBefore);
    // Create the permission topping object. Balances::transfer with a constraint for amount = 10
    const dataTable = [transferAmountLimit.toString()];
    const comp = new OpCodeComparator(OpLoad.InputVsUser, OpComp.GTE, 1, 0, true); // RHS is the data table
    const bytecode = new Uint8Array([...comp.encode()]);
    const pactContract = new Pact(dataTable, bytecode);
    const pactEncoded = pactContract.encode();

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

    const topping = new Topping(module);

    // Add to trn topping
    doughnut.addTopping(TRN_PERMISSION_DOMAIN, topping.encode());

    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await aliceWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // try to transfer < transferAmountLimit
    {
      const call = api.tx.balances.transfer(receiverAddress, transferAmountLimit - 1);
      // Create a call with empty signature to be signed by the holder (Bob)
      const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
      // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
      const txU8a = tx.toU8a(true).slice(2);
      const txHex = u8aToHex(txU8a);
      const holderWallet = await new Wallet(holderPrivateKey);
      const txHash = blake2AsHex(txHex);
      const txSlice = Buffer.from(txHash.slice(2), "hex");
      const holderSig = await holderWallet.signMessage(txSlice);

      // balances before
      const alice_balance_before = ((await api.query.system.account(alice.address)).toJSON() as any)?.data.free ?? 0;
      const receiver_balance_before =
        ((await api.query.system.account(receiverAddress)).toJSON() as any)?.data.free ?? 0;

      // whitelist the holder. i.e bob
      await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(holder.address, true)));

      // Execute the transact call with.send
      const eventData = await new Promise<any[]>((resolve, _reject) => {
        api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig).send(({ events, status }) => {
          if (status.isInBlock) {
            resolve(events);
          }
        });
      });
      expect(eventData).to.exist;
      // eventData.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

      // balances after
      const alice_balance_after = ((await api.query.system.account(alice.address)).toJSON() as any)?.data.free ?? 0;
      const receiver_balance_after =
        ((await api.query.system.account(receiverAddress)).toJSON() as any)?.data.free ?? 0;

      expect(Math.abs(receiver_balance_before - receiver_balance_after)).to.be.equal(transferAmountLimit - 1);
      expect(alice_balance_before - alice_balance_after).equal(transferAmountLimit - 1);
    }

    // try to transfer = transferAmountLimit, should fail
    {
      const call = api.tx.balances.transfer(receiverAddress, transferAmountLimit);
      // Create a call with empty signature to be signed by the holder (Bob)
      nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
      const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
      // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
      const txU8a = tx.toU8a(true).slice(2);
      const txHex = u8aToHex(txU8a);
      const holderWallet = await new Wallet(holderPrivateKey);
      const txHash = blake2AsHex(txHex);
      const txSlice = Buffer.from(txHash.slice(2), "hex");
      const holderSig = await holderWallet.signMessage(txSlice);

      // balance before
      const alice_balance_before = ((await api.query.system.account(alice.address)).toJSON() as any)?.data.free ?? 0;
      const receiver_balance_before =
        ((await api.query.system.account(receiverAddress)).toJSON() as any)?.data.free ?? 0;

      // Execute the transact call with.send
      await api.tx.doughnut
        .transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig)
        .send()
        .catch((err: any) => {
          console.log(err);
        });

      // balances after
      const alice_balance_after = ((await api.query.system.account(alice.address)).toJSON() as any)?.data.free ?? 0;
      const receiver_balance_after =
        ((await api.query.system.account(receiverAddress)).toJSON() as any)?.data.free ?? 0;

      expect(Math.abs(receiver_balance_before - receiver_balance_after)).to.be.equal(0);
      expect(alice_balance_before - alice_balance_after).equal(0);
    }

    await sleep(4000); // TODO: remove this
    // try to transfer > transferAmountLimit, should fail
    {
      const call = api.tx.balances.transfer(receiverAddress, transferAmountLimit + 1);
      // Create a call with empty signature to be signed by the holder (Bob)
      nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
      const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
      // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
      const txU8a = tx.toU8a(true).slice(2);
      const txHex = u8aToHex(txU8a);
      const holderWallet = await new Wallet(holderPrivateKey);
      const txHash = blake2AsHex(txHex);
      const txSlice = Buffer.from(txHash.slice(2), "hex");
      const holderSig = await holderWallet.signMessage(txSlice);

      // balance before
      const alice_balance_before = ((await api.query.system.account(alice.address)).toJSON() as any)?.data.free ?? 0;
      const receiver_balance_before =
        ((await api.query.system.account(receiverAddress)).toJSON() as any)?.data.free ?? 0;

      // Execute the transact call with.send
      await api.tx.doughnut
        .transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig)
        .send()
        .catch((err: any) => {
          console.log(err);
        });

      // balances after
      const alice_balance_after = ((await api.query.system.account(alice.address)).toJSON() as any)?.data.free ?? 0;
      const receiver_balance_after =
        ((await api.query.system.account(receiverAddress)).toJSON() as any)?.data.free ?? 0;

      expect(Math.abs(receiver_balance_before - receiver_balance_after)).to.be.equal(0);
      expect(alice_balance_before - alice_balance_after).equal(0);
    }
  });

  it("doughnut - Balances::transfer with constraints amount <= 10", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const transferAmountLimit = 10;
    let nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const genesis_hash = await api.rpc.chain.getBlockHash(0);
    const tip = 0;
    const version = 1;
    const issuerPubkey = alice.publicKey;
    const holderPubkey = holder.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    const doughnut = new Doughnut(version, issuerPubkey, holderPubkey, feeMode, expiry, notBefore);
    // Create the permission topping object. Balances::transfer with a constraint for amount = 10
    const dataTable = [transferAmountLimit.toString()];
    const comp = new OpCodeComparator(OpLoad.InputVsUser, OpComp.GT, 1, 0, true); // RHS is the data table
    const bytecode = new Uint8Array([...comp.encode()]);
    const pactContract = new Pact(dataTable, bytecode);
    const pactEncoded = pactContract.encode();

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

    const topping = new Topping(module);

    // Add to trn topping
    doughnut.addTopping(TRN_PERMISSION_DOMAIN, topping.encode());

    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await aliceWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // try to transfer < transferAmountLimit
    {
      const call = api.tx.balances.transfer(receiverAddress, transferAmountLimit - 1);
      // Create a call with empty signature to be signed by the holder (Bob)
      const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
      // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
      const txU8a = tx.toU8a(true).slice(2);
      const txHex = u8aToHex(txU8a);
      const holderWallet = await new Wallet(holderPrivateKey);
      const txHash = blake2AsHex(txHex);
      const txSlice = Buffer.from(txHash.slice(2), "hex");
      const holderSig = await holderWallet.signMessage(txSlice);

      // balances before
      const alice_balance_before = ((await api.query.system.account(alice.address)).toJSON() as any)?.data.free ?? 0;
      const receiver_balance_before =
        ((await api.query.system.account(receiverAddress)).toJSON() as any)?.data.free ?? 0;

      // whitelist the holder. i.e bob
      await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(holder.address, true)));

      // Execute the transact call with.send
      const eventData = await new Promise<any[]>((resolve, _reject) => {
        api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig).send(({ events, status }) => {
          if (status.isInBlock) {
            resolve(events);
          }
        });
      });
      expect(eventData).to.exist;
      // eventData.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

      // balances after
      const alice_balance_after = ((await api.query.system.account(alice.address)).toJSON() as any)?.data.free ?? 0;
      const receiver_balance_after =
        ((await api.query.system.account(receiverAddress)).toJSON() as any)?.data.free ?? 0;

      expect(Math.abs(receiver_balance_before - receiver_balance_after)).to.be.equal(transferAmountLimit - 1);
      expect(alice_balance_before - alice_balance_after).equal(transferAmountLimit - 1);
    }

    // try to transfer = transferAmountLimit, should pass
    {
      const call = api.tx.balances.transfer(receiverAddress, transferAmountLimit);
      // Create a call with empty signature to be signed by the holder (Bob)
      nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
      const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
      // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
      const txU8a = tx.toU8a(true).slice(2);
      const txHex = u8aToHex(txU8a);
      const holderWallet = await new Wallet(holderPrivateKey);
      const txHash = blake2AsHex(txHex);
      const txSlice = Buffer.from(txHash.slice(2), "hex");
      const holderSig = await holderWallet.signMessage(txSlice);

      // balance before
      const alice_balance_before = ((await api.query.system.account(alice.address)).toJSON() as any)?.data.free ?? 0;
      const receiver_balance_before =
        ((await api.query.system.account(receiverAddress)).toJSON() as any)?.data.free ?? 0;

      // Execute the transact call with.send
      const eventData = await new Promise<any[]>((resolve, _reject) => {
        api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig).send(({ events, status }) => {
          if (status.isInBlock) {
            resolve(events);
          }
        });
      });
      expect(eventData).to.exist;
      // eventData.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

      // balances after
      const alice_balance_after = ((await api.query.system.account(alice.address)).toJSON() as any)?.data.free ?? 0;
      const receiver_balance_after =
        ((await api.query.system.account(receiverAddress)).toJSON() as any)?.data.free ?? 0;

      expect(Math.abs(receiver_balance_before - receiver_balance_after)).to.be.equal(transferAmountLimit);
      expect(alice_balance_before - alice_balance_after).equal(transferAmountLimit);
    }

    await sleep(4000); // TODO: remove this
    // try to transfer > transferAmountLimit, should fail
    {
      const call = api.tx.balances.transfer(receiverAddress, transferAmountLimit + 1);
      // Create a call with empty signature to be signed by the holder (Bob)
      nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
      const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
      // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
      const txU8a = tx.toU8a(true).slice(2);
      const txHex = u8aToHex(txU8a);
      const holderWallet = await new Wallet(holderPrivateKey);
      const txHash = blake2AsHex(txHex);
      const txSlice = Buffer.from(txHash.slice(2), "hex");
      const holderSig = await holderWallet.signMessage(txSlice);

      // balance before
      const alice_balance_before = ((await api.query.system.account(alice.address)).toJSON() as any)?.data.free ?? 0;
      const receiver_balance_before =
        ((await api.query.system.account(receiverAddress)).toJSON() as any)?.data.free ?? 0;

      // Execute the transact call with.send
      await api.tx.doughnut
        .transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig)
        .send()
        .catch((err: any) => {
          console.log(err);
        });

      // balances after
      const alice_balance_after = ((await api.query.system.account(alice.address)).toJSON() as any)?.data.free ?? 0;
      const receiver_balance_after =
        ((await api.query.system.account(receiverAddress)).toJSON() as any)?.data.free ?? 0;

      expect(Math.abs(receiver_balance_before - receiver_balance_after)).to.be.equal(0);
      expect(alice_balance_before - alice_balance_after).equal(0);
    }
  });

  it("doughnut - System::remark with constraint string comparison == boo", async () => {
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const stringConstraint = "boo";
    let nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const genesis_hash = await api.rpc.chain.getBlockHash(0);
    const tip = 0;
    const version = 1;
    const issuerPubkey = alice.publicKey;
    const holderPubkey = holder.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    const doughnut = new Doughnut(version, issuerPubkey, holderPubkey, feeMode, expiry, notBefore);
    // Create the permission topping object. Balances::transfer with a constraint for amount = 10
    const dataTable = [stringConstraint];
    const comp = new OpCodeComparator(OpLoad.InputVsUser, OpComp.EQ, 0, 0, false); // RHS is the data table
    const bytecode = new Uint8Array([...comp.encode()]);
    const pactContract = new Pact(dataTable, bytecode);
    const pactEncoded = pactContract.encode();

    const module = [
      {
        name: "System",
        block_cooldown: 0,
        methods: [
          {
            name: "remark",
            block_cooldown: 0,
            constraints: [...pactEncoded],
          },
        ],
      },
    ];

    const topping = new Topping(module);

    // Add to trn topping
    doughnut.addTopping(TRN_PERMISSION_DOMAIN, topping.encode());

    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await aliceWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // try remark with stringConstraint
    {
      const call = api.tx.system.remark(stringConstraint);
      // Create a call with empty signature to be signed by the holder
      const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
      // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
      const txU8a = tx.toU8a(true).slice(2);
      const txHex = u8aToHex(txU8a);
      const holderWallet = await new Wallet(holderPrivateKey);
      const txHash = blake2AsHex(txHex);
      const txSlice = Buffer.from(txHash.slice(2), "hex");
      const holderSig = await holderWallet.signMessage(txSlice);

      // whitelist the holder. i.e bob
      await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(holder.address, true)));

      // Execute the transact call with.send
      const eventData = await new Promise<any[]>((resolve, _reject) => {
        api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig).send(({ events, status }) => {
          if (status.isInBlock) {
            resolve(events);
          }
        });
      });
      expect(eventData).to.exist;
      // eventData.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

      // assert events
      expect(eventData.length).to.equal(5);

      // doughnut	DoughnutCallExecuted	[{"ok":null}]
      expect(eventData[1].event.section).to.equal("doughnut");
      expect(eventData[1].event.method).to.equal("DoughnutCallExecuted");

      // system	ExtrinsicSuccess	[{"weight":434518872,"class":"Normal","paysFee":"Yes"}]
      expect(eventData[4].event.section).to.equal("system");
      expect(eventData[4].event.method).to.equal("ExtrinsicSuccess");
    }

    // try remark with "baa"
    {
      const call = api.tx.system.remark("baa");
      // Create a call with empty signature to be signed by the holder
      nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
      const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
      // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
      const txU8a = tx.toU8a(true).slice(2);
      const txHex = u8aToHex(txU8a);
      const holderWallet = await new Wallet(holderPrivateKey);
      const txHash = blake2AsHex(txHex);
      const txSlice = Buffer.from(txHash.slice(2), "hex");
      const holderSig = await holderWallet.signMessage(txSlice);

      // Execute the transact call with.send
      await api.tx.doughnut
        .transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig)
        .send()
        .catch((err: any) => {
          console.log(err);
        });
    }

    await sleep(4000); // TODO: remove this
    // try remark with "boobaa"
    {
      const call = api.tx.system.remark("boobaa");
      // Create a call with empty signature to be signed by the holder
      nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
      const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
      // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
      const txU8a = tx.toU8a(true).slice(2);
      const txHex = u8aToHex(txU8a);
      const holderWallet = await new Wallet(holderPrivateKey);
      const txHash = blake2AsHex(txHex);
      const txSlice = Buffer.from(txHash.slice(2), "hex");
      const holderSig = await holderWallet.signMessage(txSlice);

      // Execute the transact call with.send
      await api.tx.doughnut
        .transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig)
        .send()
        .catch((err: any) => {
          console.log(err);
        });
    }
  });

  it("doughnut - System::remark with maintenance mode call filter", async () => {
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const stringConstraint = "boo";
    let nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const genesis_hash = await api.rpc.chain.getBlockHash(0);
    const tip = 0;
    const version = 1;
    const issuerPubkey = alice.publicKey;
    const holderPubkey = holder.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    // Whitelist holder
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(holder.address, true)));

    // Create doughnut
    const doughnut = new Doughnut(version, issuerPubkey, holderPubkey, feeMode, expiry, notBefore);
    // Create the permission domain object. Balances::transfer with a constraint for amount = 10
    const dataTable = [stringConstraint];
    const comp = new OpCodeComparator(OpLoad.InputVsUser, OpComp.EQ, 0, 0, false); // RHS is the data table
    const bytecode = new Uint8Array([...comp.encode()]);
    const pactContract = new Pact(dataTable, bytecode);
    const pactEncoded = pactContract.encode();

    const module = [
      {
        name: "System",
        block_cooldown: 0,
        methods: [
          {
            name: "remark",
            block_cooldown: 0,
            constraints: [...pactEncoded],
          },
        ],
      },
    ];

    const topping = new Topping(module);

    // Add to trn domain
    doughnut.addTopping(TRN_PERMISSION_DOMAIN, topping.encode());

    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await aliceWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // try remark with maintenance mode pallet blocked
    {
      await finalizeTx(alith, api.tx.sudo.sudo(api.tx.maintenanceMode.blockPallet("System", true)));

      const call = api.tx.system.remark(stringConstraint);
      // Create a call with empty signature to be signed by the holder
      nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
      const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
      // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
      const txU8a = tx.toU8a(true).slice(2);
      const txHex = u8aToHex(txU8a);
      const holderWallet = await new Wallet(holderPrivateKey);
      const txHash = blake2AsHex(txHex);
      const txSlice = Buffer.from(txHash.slice(2), "hex");
      const holderSig = await holderWallet.signMessage(txSlice);

      // Execute the transact call with.send
      const dispatchError = await new Promise<DispatchError>((resolve, _reject) => {
        api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig).send((result) => {
          const { status, dispatchError } = result as SubmittableResultValue;
          if (!status.isFinalized) return;
          if (dispatchError === undefined) return;
          resolve(dispatchError);
        });
      });
      const { section, name } = dispatchError.registry.findMetaError(dispatchError.asModule);

      expect(section).to.equal("system");
      expect(name).to.equal("CallFiltered");
    }

    // try remark with maintenance mode call blocked
    {
      await finalizeTx(
        alith,
        api.tx.utility.batch([
          api.tx.sudo.sudo(api.tx.maintenanceMode.blockPallet("System", false)),
          api.tx.sudo.sudo(api.tx.maintenanceMode.blockCall("System", "remark", true)),
        ]),
      );

      const call = api.tx.system.remark(stringConstraint);
      // Create a call with empty signature to be signed by the holder
      nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
      const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
      // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
      const txU8a = tx.toU8a(true).slice(2);
      const txHex = u8aToHex(txU8a);
      const holderWallet = await new Wallet(holderPrivateKey);
      const txHash = blake2AsHex(txHex);
      const txSlice = Buffer.from(txHash.slice(2), "hex");
      const holderSig = await holderWallet.signMessage(txSlice);

      // Execute the transact call with.send
      const dispatchError = await new Promise<DispatchError>((resolve, _reject) => {
        api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig).send((result) => {
          const { status, dispatchError } = result as SubmittableResultValue;
          if (!status.isFinalized) return;
          if (dispatchError === undefined) return;
          resolve(dispatchError);
        });
      });
      const { section, name } = dispatchError.registry.findMetaError(dispatchError.asModule);

      expect(section).to.equal("system");
      expect(name).to.equal("CallFiltered");
    }

    // Disable maintenance mode
    await finalizeTx(
      alith,
      api.tx.utility.batch([
        api.tx.sudo.sudo(api.tx.maintenanceMode.blockPallet("System", false)),
        api.tx.sudo.sudo(api.tx.maintenanceMode.blockCall("System", "remark", false)),
      ]),
    );
  });

  it("doughnut works - alice issued doughnut for AssetsExt::transfer with constraints assetId = 2, amount = 10, keepAlive = true", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const transferAmount = 10;
    const keepAlive = true;
    const nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const genesis_hash = await api.rpc.chain.getBlockHash(0);
    const tip = 0;
    const call = api.tx.assetsExt.transfer(GAS_TOKEN_ID, receiverAddress, transferAmount, keepAlive);
    const version = 1;
    const issuerPubkey = alice.publicKey;
    const holderPubkey = holder.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    const doughnut = new Doughnut(version, issuerPubkey, holderPubkey, feeMode, expiry, notBefore);
    // Create the permission topping object. assetsExt::transfer with a constraint for amount = 10, keepAlive = true(1)
    const dataTable = ["2", "10", "1"]; // assetID, amount, keepAlive
    const comp1 = new OpCodeComparator(OpLoad.InputVsUser, OpComp.EQ, 0, 0, false); // RHS is the data table
    const comp2 = new OpCodeComparator(OpLoad.InputVsUser, OpComp.EQ, 2, 1, false); // RHS is the data table
    const comp3 = new OpCodeComparator(OpLoad.InputVsUser, OpComp.EQ, 3, 2, false); // RHS is the data table
    const bytecode = new Uint8Array([...comp1.encode(), ...comp2.encode(), ...comp3.encode()]);
    const pactContract = new Pact(dataTable, bytecode);
    const pactEncoded = pactContract.encode();
    // console.log(pactEncoded);

    const module = [
      {
        name: "AssetsExt",
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

    const topping = new Topping(module);

    // Add to trn topping
    doughnut.addTopping(TRN_PERMISSION_DOMAIN, topping.encode());

    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await aliceWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // Create a call with empty signature to be signed by the holder (Bob)
    const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
    // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
    const txU8a = tx.toU8a(true).slice(2);
    const txHex = u8aToHex(txU8a);
    const holderWallet = await new Wallet(holderPrivateKey);
    const txHash = blake2AsHex(txHex);
    const txSlice = Buffer.from(txHash.slice(2), "hex");
    const holderSig = await holderWallet.signMessage(txSlice);

    // balances before
    const aliceXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, alice.address)).toJSON() as any)?.balance ?? 0;
    const receiverXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, receiverAddress)).toJSON() as any)?.balance ?? 0;

    // whitelist the holder. i.e bob
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(holder.address, true)));

    // Execute the transact call with.send
    const eventData = await new Promise<any[]>((resolve, _reject) => {
      api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig).send(({ events, status }) => {
        if (status.isInBlock) {
          resolve(events);
        }
      });
    });
    expect(eventData).to.exist;
    // eventData.forEach(({ event: { data, method, section } }) => console.log(`${section}\t${method}\t${data}`));

    // balances after
    const aliceXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, alice.address)).toJSON() as any)?.balance ?? 0;
    const receiverXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, receiverAddress)).toJSON() as any)?.balance ?? 0;
    // alice should bear transferAmount + fees in XRP
    expect(aliceXRPBalanceBefore - aliceXRPBalanceAfter).to.be.greaterThan(transferAmount);
    // receiverAddress should have transferAmount
    expect(Math.abs(receiverXRPBalanceBefore - receiverXRPBalanceAfter)).to.equal(transferAmount);
  });
});
