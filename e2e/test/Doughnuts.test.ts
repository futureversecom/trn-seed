import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a, u8aToHex } from "@polkadot/util";
import { expect } from "chai";
import { Wallet } from "ethers";
import { blake2AsHex } from "@polkadot/util-crypto";
import { OpCodeComparator, OpComp, OpLoad, Pact } from "../../../trn-pact/js";
// const Doughnut = require('@trn/doughnut-wasm').default;
import { Doughnut } from "../../../trn-doughnut-rs/js";
import { TRNNut } from "../../../trn-trnnut-rs/js";
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
  let bob_ecdsa: KeyringPair;
  let alice: KeyringPair;
  let keyring: Keyring;
  let keyring_ecdsa: Keyring;
  let alith: KeyringPair;

  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });

    keyring = new Keyring({ type: "ethereum" });
    keyring_ecdsa = new Keyring({ type: "ecdsa" });
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));
    bob_ecdsa = keyring_ecdsa.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));
    alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
  });

  after(async () => node.stop());

  it("doughnut works - alice issued doughnut for Balances::transfer with constraints amount = 10", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const holder_ecdsa: KeyringPair = keyring_ecdsa.addFromSeed(hexToU8a(holderPrivateKey));
    const transferAmount = 10;
    const nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const call = api.tx.balances.transfer(receiverAddress, transferAmount);
    const version = 1;
    const issuerPubkey = alice.publicKey;
    const holderPubkey = holder.publicKey;
    const fee_mode = 0;
    const expiry = 100000;
    const notBefore = 0;

    console.log("\n====  Creating Doughnut");
    const doughnut = new Doughnut(version, issuerPubkey, holderPubkey, fee_mode, expiry, notBefore);
    // Create the permission domain object. Balances::transfer with a constraint for amount = 10
    const data_table = ["10"];
    const comp = new OpCodeComparator(OpLoad.INPUT_VS_USER, OpComp.EQ, 1, 0, false); // RHS is data table
    const bytecode = new Uint8Array([...comp.encode()]);
    const pactContract = new Pact(data_table, bytecode);
    const pactEncoded = pactContract.encode();
    // console.log(pactEncoded);

    const module = [
      [
        "Balances",
        {
          name: "Balances",
          block_cooldown: 0,
          methods: [
            [
              "transfer",
              {
                name: "transfer",
                block_cooldown: 0,
                constraints: [...pactEncoded],
              },
            ],
          ],
        },
      ],
    ];
    const contract = [
      [
        [
          27, 137, 65, 29, 182, 25, 157, 61, 226, 13, 230, 14, 111, 6, 25, 186, 227, 117, 177, 244, 172, 147, 40, 119,
          209, 78, 13, 109, 236, 119, 205, 202,
        ],
        {
          address: [
            27, 137, 65, 29, 182, 25, 157, 61, 226, 13, 230, 14, 111, 6, 25, 186, 227, 117, 177, 244, 172, 147, 40, 119,
            209, 78, 13, 109, 236, 119, 205, 202,
          ],
          block_cooldown: 270549120,
        },
      ],
    ];
    const trnnut = new TRNNut(module, contract);

    // Add to trn domain
    doughnut.addDomain(TRN_PERMISSION_DOMAIN, trnnut.encode());
    console.log(`Domain    : ${doughnut.domain(TRN_PERMISSION_DOMAIN)}`);

    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuer_sig = await aliceWallet.signMessage(ethSlice);
    const sig_uint8 = Buffer.from(issuer_sig.slice(2), "hex");
    doughnut.addSignature(sig_uint8);

    console.log(`Signature : ${doughnut.signature()}`);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encoded_doughnut = doughnut.encode();
    const doughnut_hex = u8aToHex(encoded_doughnut);

    // Create a call with empty signature to be signed by the holder (Bob)
    const tx = await api.tx.doughnut.transact(call, doughnut_hex, nonce, "");
    // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
    const tx_u8a = tx.toU8a(true).slice(2);
    const tx_hex = u8aToHex(tx_u8a);
    const signature = holder_ecdsa.sign(tx_hex);
    const sig_hex = u8aToHex(signature);

    // alice balance before
    const alice_balance_before = await api.query.system.account(alice.address);

    // whitelist the holder. i.e bob
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(holder.address, true)));

    // Execute the transact call with.send
    const eventData = await new Promise<any[]>((resolve, reject) => {
      api.tx.doughnut.transact(call, doughnut_hex, nonce, sig_hex).send(({ events, status }) => {
        if (status.isInBlock) {
          for (const { event } of events) {
            if (event.section === "doughnut" && event.method === "DoughnutCallExecuted") {
              resolve(event.data);
            }
          }
          reject(null);
        }
      });
    });
    expect(eventData).to.exist;

    // console.log(events);
    const balance = await api.query.system.account(receiverAddress);
    const freeBalance = balance.toJSON()?.data.free;
    console.log(`Free balance after doughnut transact: ${freeBalance}`);
    expect(freeBalance).to.be.equal(transferAmount);

    const alice_balance_after = await api.query.system.account(alice.address);
    // alice should bear transferAmount in Root + fees in XRP
    expect(alice_balance_before.toJSON()?.data.free - alice_balance_after.toJSON()?.data.free).equal(transferAmount);

    // check the events
  });

  it("doughnut fails - alice issued doughnut for Balances::transfer with constraints amount = 10 can not be used to transfer amount != 10", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const holder_ecdsa: KeyringPair = keyring_ecdsa.addFromSeed(hexToU8a(holderPrivateKey));
    const transferAmount = 10;
    const nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const call = api.tx.balances.transfer(receiverAddress, transferAmount - 1);
    const version = 1;
    const issuerPubkey = alice.publicKey;
    const holderPubkey = holder.publicKey;
    const fee_mode = 0;
    const expiry = 100000;
    const notBefore = 0;

    console.log("\n====  Creating Doughnut");
    const doughnut = new Doughnut(version, issuerPubkey, holderPubkey, fee_mode, expiry, notBefore);
    // Create the permission domain object. Balances::transfer with a constraint for amount = 10
    const data_table = ["10"];
    const comp = new OpCodeComparator(OpLoad.INPUT_VS_USER, OpComp.EQ, 1, 0, false); // RHS is data table
    const bytecode = new Uint8Array([...comp.encode()]);
    const pactContract = new Pact(data_table, bytecode);
    const pactEncoded = pactContract.encode();
    // console.log(pactEncoded);

    const module = [
      [
        "Balances",
        {
          name: "Balances",
          block_cooldown: 0,
          methods: [
            [
              "transfer",
              {
                name: "transfer",
                block_cooldown: 0,
                constraints: [...pactEncoded],
              },
            ],
          ],
        },
      ],
    ];
    const contract = [
      [
        [
          27, 137, 65, 29, 182, 25, 157, 61, 226, 13, 230, 14, 111, 6, 25, 186, 227, 117, 177, 244, 172, 147, 40, 119,
          209, 78, 13, 109, 236, 119, 205, 202,
        ],
        {
          address: [
            27, 137, 65, 29, 182, 25, 157, 61, 226, 13, 230, 14, 111, 6, 25, 186, 227, 117, 177, 244, 172, 147, 40, 119,
            209, 78, 13, 109, 236, 119, 205, 202,
          ],
          block_cooldown: 270549120,
        },
      ],
    ];
    const trnnut = new TRNNut(module, contract);

    // Add to trn domain
    doughnut.addDomain(TRN_PERMISSION_DOMAIN, trnnut.encode());
    console.log(`Domain    : ${doughnut.domain(TRN_PERMISSION_DOMAIN)}`);

    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuer_sig = await aliceWallet.signMessage(ethSlice);
    const sig_uint8 = Buffer.from(issuer_sig.slice(2), "hex");
    doughnut.addSignature(sig_uint8);
    console.log(`Signature : ${doughnut.signature()}`);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encoded_doughnut = doughnut.encode();
    const doughnut_hex = u8aToHex(encoded_doughnut);

    // Create a call with empty signature to be signed by the holder (Bob)
    const tx = await api.tx.doughnut.transact(call, doughnut_hex, nonce, "");
    // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
    const tx_u8a = tx.toU8a(true).slice(2);
    const tx_hex = u8aToHex(tx_u8a);
    const signature = holder_ecdsa.sign(tx_hex);
    // console.log(signature);
    // console.log(holder.sign(tx_hex))

    const sig_hex = u8aToHex(signature);

    // alice balance before
    const alice_balance_before = await api.query.system.account(alice.address);

    // whitelist the holder. i.e bob
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(holder.address, true)));

    // Execute the transact call with.send
    await api.tx.doughnut
      .transact(call, doughnut_hex, nonce, sig_hex)
      .send()
      .catch((err: any) => {
        console.log(err);
      });

    // console.log(events);
    const balance = await api.query.system.account(receiverAddress);
    const freeBalance = balance.toJSON()?.data.free;
    console.log(`Free balance after doughnut transact: ${freeBalance}`);
    expect(freeBalance).to.be.equal(0);

    const alice_balance_after = await api.query.system.account(alice.address);
    // alice should bear transferAmount in Root + fees in XRP
    expect(alice_balance_before.toJSON()?.data.free - alice_balance_after.toJSON()?.data.free).equal(0);

    // check the events
  });

  it("doughnut without trn permission object fails", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const transferAmount = 1234;
    const nonce = ((await api.query.system.account(bob.address)).toJSON() as any)?.nonce;
    const call = api.tx.balances.transfer(receiverAddress, transferAmount);
    const version = 1;
    const issuer = alice.publicKey;
    const holder = bob.publicKey;
    const fee_mode = 0;
    const expiry = 100000;
    const notBefore = 0;

    console.log("\n====  Creating Doughnut");
    const doughnut = new Doughnut(version, issuer, holder, fee_mode, expiry, notBefore);
    // Add a random domain to allow the doughnut to be encoded
    doughnut.addDomain("Test", new Uint8Array(12));
    console.log(`Domain    : ${doughnut.domain("Test")}`);


    // Sign the doughnut
    const aliceWallet = await new Wallet(ALICE_PRIVATE_KEY);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuer_sig = await aliceWallet.signMessage(ethSlice);
    const sig_uint8 = Buffer.from(issuer_sig.slice(2), "hex");
    doughnut.addSignature(sig_uint8);
    console.log(`Signature : ${doughnut.signature()}`);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holder, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encoded_doughnut = doughnut.encode();
    const doughnut_hex = u8aToHex(encoded_doughnut);

    // Create a call with empty signature to be signed by the holder (Bob)
    const tx = await api.tx.doughnut.transact(call, doughnut_hex, nonce, "");
    // Convert tx to u8Array and remove the first 2 bytes (Not sure why. It's to do with length)
    const tx_u8a = tx.toU8a(true).slice(2);
    const tx_hex = u8aToHex(tx_u8a);
    const signature = bob_ecdsa.sign(tx_hex);
    const sig_hex = u8aToHex(signature);

    // alice balance before
    const alice_balance_before = await api.query.system.account(alice.address);

    // whitelist the holder. i.e bob
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(bob.address, true)));

    // Execute the transact call with.send
    await api.tx.doughnut
      .transact(call, doughnut_hex, nonce, sig_hex)
      .send()
      .catch((err: any) => {
        console.log(err);
      });

    // console.log(events);
    const balance = await api.query.system.account(receiverAddress);
    const freeBalance = balance.toJSON()?.data.free;
    console.log(`Free balance after doughnut transact: ${freeBalance}`);
    expect(freeBalance).to.be.equal(0);

    const alice_balance_after = await api.query.system.account(alice.address);
    // alice should bear transferAmount in Root + fees in XRP
    expect(alice_balance_before.toJSON()?.data.free - alice_balance_after.toJSON()?.data.free).equal(0);
  });
});
