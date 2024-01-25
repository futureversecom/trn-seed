import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
// const Doughnut = require('@trn/doughnut-wasm').default;
import { Doughnut } from "../../../trn-doughnut-rs/js";
import {hexToU8a, u8aToHex} from "@polkadot/util";
import {KeyringPair} from "@polkadot/keyring/types";
import { expect } from "chai";

import {
    ALICE_PRIVATE_KEY,
    BOB_PRIVATE_KEY,
    GAS_TOKEN_ID,
    NodeProcess,
    startNode,
    typedefs,
} from "../common";
import {Wallet} from "ethers";


describe("Doughnuts", () => {
    let node: NodeProcess;
    let api: ApiPromise;
    let bob: KeyringPair;
    let bob_ecdsa: KeyringPair;
    let alice: KeyringPair;

    before(async () => {
        node = await startNode();

        const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
        api = await ApiPromise.create({ provider: wsProvider, types: typedefs });

        const keyring = new Keyring({ type: "ethereum" });
        const keyring_ecdsa = new Keyring({ type: "ecdsa" });
        bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));
        bob_ecdsa = keyring_ecdsa.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));
        alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));
    });

    after(async () => node.stop());

    it("doughnut works", async () => {
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

        // Sign the doughnut with the issuers private key
        doughnut.signECDSA(hexToU8a(ALICE_PRIVATE_KEY));
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
    });

});