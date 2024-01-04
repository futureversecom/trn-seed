import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";

import {
    ALITH_PRIVATE_KEY,
    BOB_PRIVATE_KEY,
    GAS_TOKEN_ID,
    // NodeProcess,
    // startNode,
    typedefs,
} from "../common";
import {Wallet} from "ethers";


describe("Doughnuts", () => {
    // let node: NodeProcess;
    let api: ApiPromise;
    let bob: KeyringPair;

    before(async () => {
        // node = await startNode();

        const wsProvider = new WsProvider(`ws://localhost:9944`);
        api = await ApiPromise.create({ provider: wsProvider, types: typedefs });

        const keyring = new Keyring({ type: "ethereum" });
        bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));

    });

    // after(async () => node.stop());

    it("doughnut works", async () => {
        const receiverAddress = "0x000000000000000000000000000000000000000c";

        // Doughnut from Alice to Bob
        const doughnut = "0x011000020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a10390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f270000000000310000000000000000000000000000000000d35aef05a023e2e75f038abba9d1357671c3044e2664c6cc099115adceb492a2677fe2233735f31442d5c33921e9d49fd305af28b8ffa6bd4ec86076ad026b8d";
        // Bob's signature of the outer call
        const signature = "0xbb7f00599d3b7fe0177d4478949c57151510a48bc80ac5b9868d22743cb413dd7ca06351fede444df13f4db83265eae627b54910476edb112dd8c4a5460a8309";
        const transferAmount = 1234;
        const nonce = ((await api.query.system.account(bob.address)).toJSON() as any)?.nonce;
        const call = api.tx.balances.transfer(receiverAddress, transferAmount);

        // Execute the transact call with.send
        const events = await new Promise<any[]>(async (resolve) => {
            await api.tx.doughnut.transact(call, doughnut, nonce, signature).send(({ events = [], status }) => {
                if (status.isInBlock) {
                    resolve(events);
                }
            });
        });

        // console.log(events);
        const balance = await api.query.system.account(receiverAddress);
        const freeBalance = balance.toJSON()?.data.free;
        console.log(`Free balance after doughnut transsact: ${freeBalance}`);
        expect(freeBalance).to.be.equal(transferAmount);
    });

});