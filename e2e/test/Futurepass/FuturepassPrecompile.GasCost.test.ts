import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { BigNumber, Contract, Wallet } from "ethers";
import { ethers } from "hardhat";
import web3 from "web3";

import MockCreateContract from "../../artifacts/contracts/FuturepassIntegrator.sol/CreateTester.json";
import MockCreatePayableContract from "../../artifacts/contracts/FuturepassIntegrator.sol/CreateTesterPayable.json";
import {
    ALITH_PRIVATE_KEY,
    ERC20_ABI,
    FP_DELEGATE_RESERVE,
    FUTUREPASS_PRECOMPILE_ABI,
    FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
    FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
    GAS_TOKEN_ID,
    NodeProcess,
    startNode,
    saveTxCosts,
    typedefs, txCosts,
} from "../../common";

const XRP_PRECOMPILE_ADDRESS = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000");

const CALL_TYPE = {
    StaticCall: 0,
    Call: 1,
    DelegateCall: 2,
    Create: 3,
    Create2: 4,
};

const PROXY_TYPE = {
    NoPermission: 0,
    Any: 1,
    NonTransfer: 2,
};

describe("Futurepass Precompile", function () {
    let node: NodeProcess;

    let api: ApiPromise;
    let provider: JsonRpcProvider;
    let alithKeyring: KeyringPair;
    let alithSigner: Wallet;
    let futurepassRegistrar: Contract;
    let xrpERC20Precompile: Contract;
    const keyring = new Keyring({type: "ethereum"});

    const allTxCosts: { [key: string]: txCosts } = {};

    beforeEach(async () => {
        node = await startNode();

        // Substrate variables
        const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
        api = await ApiPromise.create({
            provider: wsProvider,
            types: typedefs,
        });

        alithKeyring = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

        // Ethereum variables
        provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
        alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider);

        futurepassRegistrar = new Contract(
            FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
            FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
            alithSigner,
        );
        xrpERC20Precompile = new Contract(XRP_PRECOMPILE_ADDRESS, ERC20_ABI, alithSigner);
    });

    afterEach(async () => await node.stop());

    after(async () => {
        saveTxCosts(allTxCosts, "Futurepass/TxCosts.md", "Futurepass Precompiles");
    });

    async function createFuturepass(caller: Wallet, address: string) {
        // fund caller to pay for futurepass creation
        await fundAccount(api, alithKeyring, address);

        const tx = await futurepassRegistrar.connect(caller).create(address);
        const receipt = await tx.wait();

        const futurepass: string = (receipt?.events as any)[0].args.futurepass;
        return new Contract(futurepass, FUTUREPASS_PRECOMPILE_ABI, caller);
    }

    it("gas estimate and actual fee tallies", async () => {
        const owner = Wallet.createRandom().connect(provider);
        // fund caller to pay for futurepass creation
        await fundAccount(api, alithKeyring, owner.address);
        const precompileGasEstimate = await futurepassRegistrar.estimateGas.create(owner.address);

        const balanceBefore = await owner.getBalance();
        const tx = await futurepassRegistrar.connect(owner).create(owner.address);
        await tx.wait();
        const balanceAfter = await owner.getBalance();
        const actualCost = balanceBefore.sub(balanceAfter);
        const fees = await provider.getFeeData();

        // Note: we charge the maxFeePerGas * gaslimit upfront and do not refund the extra atm. Hence, users are charged extra
        console.log("actual cost: ", actualCost.toString());
        console.log("calculated cost: ", precompileGasEstimate.mul(fees.maxFeePerGas!).toString());
    });

    it("create futurepass tx costs", async () => {
        const owner = Wallet.createRandom().connect(provider);
        // fund caller to pay for futurepass creation
        await fundAccount(api, alithKeyring, owner.address);

        // precompile
        let balanceBefore = await owner.getBalance();
        const tx = await futurepassRegistrar.connect(owner).create(owner.address);
        await tx.wait();
        let balanceAfter = await owner.getBalance();
        const precompileCost = balanceBefore.sub(balanceAfter);

        // extrinsic
        const owner2 = Wallet.createRandom().connect(provider);
        balanceBefore = await alithSigner.getBalance();
        await new Promise<void>((resolve) => {
            api.tx.futurepass.create(owner2.address).signAndSend(alithKeyring, ({ status }) => {
                if (status.isInBlock) resolve();
            });
        });
        balanceAfter = await alithSigner.getBalance();
        const extrinsicCost = balanceBefore.sub(balanceAfter);

        expect(extrinsicCost).to.be.lessThan(precompileCost);
        // Update all costs with allTxCosts
        allTxCosts["create"] = {
            Contract: BigNumber.from(0), // no contract
            Precompile: precompileCost.div(1000000000000n),// convert to XRP Drops(6)
            Extrinsic: extrinsicCost.div(1000000000000n), // convert to XRP Drops(6)
        };
    });

    it("register delegate tx costs", async () => {
        const owner1 = Wallet.createRandom().connect(provider);
        const owner2 = Wallet.createRandom().connect(provider);
        const delegate = Wallet.createRandom().connect(provider);
        // create FPs for owner1 and owner2
        const fp1 = await createFuturepass(owner1, owner1.address);
        const fp2 = await createFuturepass(owner2, owner2.address);

        // precompile approach
        let balanceBefore = await owner1.getBalance();
        const tx = await fp1.connect(owner1).registerDelegate(delegate.address, PROXY_TYPE.Any);
        await tx.wait();
        let balanceAfter = await owner1.getBalance();
        // assert delegate is registered
        expect(await fp1.delegateType(delegate.address)).to.equal(PROXY_TYPE.Any);
        const precompileCost = balanceBefore.sub(balanceAfter);

        // extrinsic approach
        const owner2KeyRing = keyring.addFromSeed(hexToU8a(owner2.privateKey));
        balanceBefore = await owner2.getBalance();
        await new Promise<void>((resolve) => {
            api.tx.futurepass.registerDelegate(fp2.address, delegate.address, PROXY_TYPE.Any).signAndSend(owner2KeyRing, ({ status }) => {
                if (status.isInBlock) resolve();
            });
        });
        balanceAfter = await owner2.getBalance();
        // assert delegate is registered
        expect(await fp2.delegateType(delegate.address)).to.equal(PROXY_TYPE.Any);

        const extrinsicCost = balanceBefore.sub(balanceAfter);
        expect(extrinsicCost).to.be.lessThan(precompileCost);
        // Update all costs with allTxCosts
        allTxCosts["registerDelegate"] = {
            Contract: BigNumber.from(0), // no contract
            Precompile: precompileCost.div(1000000000000n),// convert to XRP Drops(6)
            Extrinsic: extrinsicCost.div(1000000000000n), // convert to XRP Drops(6)
        };
    });
});

async function fundAccount(
    api: ApiPromise,
    keyring: KeyringPair,
    address: string,
    amount: string | number = 10_000_000,
): Promise<void> {
    return new Promise<void>((resolve) => {
        api.tx.utility
            .batch([
                api.tx.assets.transfer(GAS_TOKEN_ID, address, amount), // 10 XRP
                api.tx.balances.transfer(address, amount), // 10 ROOT
            ])
            .signAndSend(keyring, ({ status }) => {
                if (status.isInBlock) resolve();
            });
    });
}

async function fundEOA(signer: Wallet, address: string, value: string = "10000") {
    const tx = await signer.sendTransaction({ to: address, value: ethers.utils.parseEther(value) });
    await tx.wait();
}

function parseEther(amount: number): BigNumber {
    return ethers.utils.parseEther(amount.toString());
}