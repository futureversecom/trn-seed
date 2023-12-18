import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { hexToU8a, u8aToString } from "@polkadot/util";
import axios from "axios";
import { expect } from "chai";

import {
    ALITH_PRIVATE_KEY,
    ALICE_PRIVATE_KEY,
    BOB_PRIVATE_KEY,
    GAS_TOKEN_ID,
    NodeProcess,
    startNode,
    typedefs,
    ALICE_PRIVATE_KEY
} from "../common";


describe("Doughnuts", () => {
    // let node: NodeProcess;
    let api: ApiPromise;
    let alith: KeyringPair;
    let alice: KeyringPair;
    let bob: KeyringPair;

    before(async () => {
        // node = await startNode();

        const wsProvider = new WsProvider(`ws://localhost:9944`);
        api = await ApiPromise.create({ provider: wsProvider, types: typedefs });

        const keyring = new Keyring({ type: "ethereum" });
        alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
        alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));
        bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));

    });

    // after(async () => node.stop());

    it("doughnuts test", async () => {
        console.log("Hi, I'm a doughnut");

        const doughnut = "0x000000e8f26a0596337e99a869bb7fc3a03f67a31293a87f9ba5201dedea3264cc1cc9ba78382580ffedae9464627d5f6fa8d4ba13c84bc87eb044fecdc51fc9219a290000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        const transferAmount = 1234;
        const assetId = 2;
        const nonce = ((await api.query.system.account(alith.address)).toJSON() as any)?.nonce;
        const call = api.tx.assets.transfer(assetId, bob.address, transferAmount);
        // const res = await call.signAndSend(alith);
        // console.log(res);
        const tx = await api.tx.doughnut.transact(call, doughnut, nonce);
        const res = await tx.send();
        console.log(res);
        // const events = await new Promise<any[]>(async (resolve) => {
        //     await api.tx.doughnut.transact(call, doughnut, nonce).send(({ events = [], status }) => {
        //         if (status.isInBlock) {
        //             console.log("Hello, I'm in the block");
        //             resolve(events);
        //         }
        //     });
        // });
        // console.log(events);


        console.log("Doughnut done");
    });

});