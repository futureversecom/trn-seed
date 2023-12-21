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

    it("doughnuts test", async () => {
        const receiverAddress = await Wallet.createRandom().getAddress();

        // Doughnut from Alice to Bob
        const doughnut = "0x001000020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a10390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f2700000000310000000000000000000000000000000000dda0d39981584d153eb79568a9296673efbf8149cc985d3e10ac083827134f0e181c553e40ed265df82587d1de00d619cbc2a93e03bf0df753f780b6e7e0fbbc";
        // Bob's signature of the outer doughnut
        const signature = "0x21cf55e24342d613ed506f24fc7dba4644c03414bbd7b4bfa802ba5c9db2b308739f6ce9d19933a400c4bc7376e380a4bc37fd285bb3e744f91bee360ba45db0";
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