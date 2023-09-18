import {JsonRpcProvider} from "@ethersproject/providers";
import {ApiPromise, Keyring, WsProvider} from "@polkadot/api";
import {KeyringPair} from "@polkadot/keyring/types";
import {hexToU8a} from "@polkadot/util";
import {expect} from "chai";
import {BigNumber, Contract, Wallet} from "ethers";
import {ethers} from "hardhat";
import web3 from "web3";

import {
    ALITH_PRIVATE_KEY,
    ERC20_ABI,
    FP_DELEGATE_RESERVE,
    FUTUREPASS_PRECOMPILE_ABI,
    FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
    FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
    GAS_TOKEN_ID,
    NodeProcess,
    TxCosts,
    getScaledGasForExtrinsicFee,
    saveTxFees,
    saveTxGas,
    startNode,
    typedefs,
    weiTo6DP, EvmEstimates, saveTxEstimates,
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
    // let node: NodeProcess;

    let api: ApiPromise;
    let provider: JsonRpcProvider;
    let alithKeyring: KeyringPair;
    let alithSigner: Wallet;
    let futurepassRegistrar: Contract;
    const keyring = new Keyring({type: "ethereum"});

    const allTxGasCosts: { [key: string]: TxCosts } = {};
    const allTxFeeCosts: { [key: string]: TxCosts } = {};
    const allEstimates: { [key: string]: EvmEstimates } = {};

    before(async () => {
        // node = await startNode();

        // Substrate variables
        const wsProvider = new WsProvider(`ws://localhost:9944`);
        api = await ApiPromise.create({
            provider: wsProvider,
            types: typedefs,
        });

        alithKeyring = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

        // Ethereum variables
        provider = new JsonRpcProvider(`http://127.0.0.1:9933`);
        alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider);

        futurepassRegistrar = new Contract(
            FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
            FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
            alithSigner,
        );
    });

    after(async () => {
        // await node.stop();
        saveTxGas(allTxGasCosts, "Futurepass/TxCosts.md", "Futurepass Precompiles");
        saveTxFees(allTxFeeCosts, "Futurepass/TxCosts.md", "Futurepass Precompiles");
        saveTxEstimates(allEstimates, "Futurepass/TxCosts.md", "Futurepass Precompiles");

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
        const receipt = await tx.wait();
        const balanceAfter = await owner.getBalance();
        const actualCost = balanceBefore.sub(balanceAfter);
        const fees = await provider.getFeeData();

        const effectiveFeePerGas = fees.lastBaseFeePerGas?.add(fees.maxPriorityFeePerGas!);
        const calculatedCostFromGasEstimate = precompileGasEstimate.mul(effectiveFeePerGas!);
        const calculatedCostFromActualGas = receipt.gasUsed.mul(effectiveFeePerGas!);
        expect(weiTo6DP(actualCost)).to.equal(weiTo6DP(calculatedCostFromActualGas).sub(1)); // sub 1 drop to match the rounding
        expect(weiTo6DP(actualCost)).to.lessThan(weiTo6DP(calculatedCostFromGasEstimate));
    });

    it("create futurepass tx costs", async () => {
        const owner = Wallet.createRandom().connect(provider);
        // fund caller to pay for futurepass creation
        await fundAccount(api, alithKeyring, owner.address);

        // precompile
        const precompileGasEstimate = await futurepassRegistrar.estimateGas.create(owner.address);
        let balanceBefore = await owner.getBalance();
        const tx = await futurepassRegistrar.connect(owner).create(owner.address);
        const precompileReceipt = await tx.wait();
        let balanceAfter = await owner.getBalance();
        const precompileFeeCost = balanceBefore.sub(balanceAfter);

        // extrinsic
        const owner2 = Wallet.createRandom().connect(provider);
        balanceBefore = await alithSigner.getBalance();
        await new Promise<void>((resolve) => {
            api.tx.futurepass.create(owner2.address).signAndSend(alithKeyring, ({status}) => {
                if (status.isInBlock) resolve();
            });
        });
        balanceAfter = await alithSigner.getBalance();
        const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
        const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
        // expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

        // Update all costs
        allTxGasCosts["create"] = {
            Contract: BigNumber.from(0), // no contract
            Precompile: precompileGasEstimate, // convert to XRP Drops(6)
            Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
        };
        allTxFeeCosts["create"] = {
            Contract: BigNumber.from(0), // no contract
            Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
            Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
        };
        allEstimates["create"] = {
            Contract: {
                estimate: BigNumber.from(0),
                actual: BigNumber.from(0)
            },
            Precompile: {
                estimate: precompileGasEstimate,
                actual: precompileReceipt.gasUsed
            }
        };
    });

    it("register delegate with signature tx costs", async () => {
        const owner1 = Wallet.createRandom().connect(provider);
        const owner2 = Wallet.createRandom().connect(provider);
        const delegate = Wallet.createRandom().connect(provider);
        // create FPs for owner1 and owner2
        const fp1 = await createFuturepass(owner1, owner1.address);
        const fp2 = await createFuturepass(owner2, owner2.address);

        // precompile approach
        const deadline = (await provider.getBlockNumber()) + 20;
        const message = ethers.utils
            .solidityKeccak256(
                ["address", "address", "uint8", "uint32"],
                [fp1.address, delegate.address, PROXY_TYPE.Any, deadline],
            )
            .substring(2); // remove `0x` prefix

        // eip191 sign message using delegate
        const signature = await delegate.signMessage(message);

        const precompileGasEstimate = await fp1.estimateGas.registerDelegateWithSignature(
            delegate.address,
            PROXY_TYPE.Any,
            deadline,
            signature,
        );
        let balanceBefore = await owner1.getBalance();
        const tx = await fp1
            .connect(owner1)
            .registerDelegateWithSignature(delegate.address, PROXY_TYPE.Any, deadline, signature);
        const precompileReceipt = await tx.wait();
        let balanceAfter = await owner1.getBalance();
        const precompileFeeCost = balanceBefore.sub(balanceAfter);

        // extrinsic approach
        const deadline2 = (await provider.getBlockNumber()) + 20;
        const message2 = ethers.utils
            .solidityKeccak256(
                ["address", "address", "uint8", "uint32"],
                [fp2.address, delegate.address, PROXY_TYPE.Any, deadline2],
            )
            .substring(2); // remove `0x` prefix

        // eip191 sign message using delegate
        const signature2 = await delegate.signMessage(message2);
        const owner2KeyRing = keyring.addFromSeed(hexToU8a(owner2.privateKey));
        balanceBefore = await owner2.getBalance();
        await new Promise<void>((resolve) => {
            api.tx.futurepass
                .registerDelegateWithSignature(fp2.address, delegate.address, PROXY_TYPE.Any, deadline2, signature2)
                .signAndSend(owner2KeyRing, ({status}) => {
                    if (status.isInBlock) resolve();
                });
        });
        balanceAfter = await owner2.getBalance();

        const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
        const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
        // expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);
        // expect(extrinsicGasCost).to.be.lessThan(precompileGasEstimate);

        // Update all costs
        allTxGasCosts["registerDelegateWithSignature"] = {
            Contract: BigNumber.from(0), // no contract
            Precompile: precompileGasEstimate, // convert to XRP Drops(6)
            Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
        };
        allTxFeeCosts["registerDelegateWithSignature"] = {
            Contract: BigNumber.from(0), // no contract
            Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
            Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
        };
        allEstimates["registerDelegateWithSignature"] = {
            Contract: {
                estimate: BigNumber.from(0),
                actual: BigNumber.from(0)
            },
            Precompile: {
                estimate: precompileGasEstimate,
                actual: precompileReceipt.gasUsed
            }
        };
    });

    it("unregister delegate tx costs", async () => {
        const owner1 = Wallet.createRandom().connect(provider);
        const owner2 = Wallet.createRandom().connect(provider);
        const delegate = Wallet.createRandom().connect(provider);
        // create FPs for owner1 and owner2 and register delegate delegate
        const fp1 = await createFuturepass(owner1, owner1.address);
        const fp2 = await createFuturepass(owner2, owner2.address);

        const deadline = (await provider.getBlockNumber()) + 20;
        const message = ethers.utils
            .solidityKeccak256(
                ["address", "address", "uint8", "uint32"],
                [fp1.address, delegate.address, PROXY_TYPE.Any, deadline],
            )
            .substring(2); // remove `0x` prefix

        // eip191 sign message using delegate
        const signature = await delegate.signMessage(message);

        const deadline2 = (await provider.getBlockNumber()) + 20;
        const message2 = ethers.utils
            .solidityKeccak256(
                ["address", "address", "uint8", "uint32"],
                [fp2.address, delegate.address, PROXY_TYPE.Any, deadline2],
            )
            .substring(2); // remove `0x` prefix

        // eip191 sign message using delegate
        const signature2 = await delegate.signMessage(message2);

        let tx = await fp1
            .connect(owner1)
            .registerDelegateWithSignature(delegate.address, PROXY_TYPE.Any, deadline, signature);
        await tx.wait();
        tx = await fp2
            .connect(owner2)
            .registerDelegateWithSignature(delegate.address, PROXY_TYPE.Any, deadline2, signature2);
        await tx.wait();
        expect(await fp1.delegateType(delegate.address)).to.equal(PROXY_TYPE.Any);
        expect(await fp2.delegateType(delegate.address)).to.equal(PROXY_TYPE.Any);

        // precompile approach
        const precompileGasEstimate = await fp1.estimateGas.unregisterDelegate(delegate.address);
        let balanceBefore = await owner1.getBalance();
        tx = await fp1.connect(owner1).unregisterDelegate(delegate.address);
        const precompileReceipt = await tx.wait();
        let balanceAfter = await owner1.getBalance();
        const precompileFeeCost = balanceBefore.sub(balanceAfter);

        // extrinsic approach
        const owner2KeyRing = keyring.addFromSeed(hexToU8a(owner2.privateKey));
        balanceBefore = await owner2.getBalance();
        await new Promise<void>((resolve) => {
            api.tx.futurepass.unregisterDelegate(fp2.address, delegate.address).signAndSend(owner2KeyRing, ({status}) => {
                if (status.isInBlock) resolve();
            });
        });
        balanceAfter = await owner2.getBalance();

        const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
        const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
        // expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);
        // expect(extrinsicGasCost).to.be.lessThan(precompileGasEstimate);

        // Update all costs
        allTxGasCosts["unregisterDelegate"] = {
            Contract: BigNumber.from(0), // no contract
            Precompile: precompileGasEstimate, // convert to XRP Drops(6)
            Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
        };
        allTxFeeCosts["unregisterDelegate"] = {
            Contract: BigNumber.from(0), // no contract
            Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
            Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
        };
        allEstimates["unregisterDelegate"] = {
            Contract: {
                estimate: BigNumber.from(0),
                actual: BigNumber.from(0)
            },
            Precompile: {
                estimate: precompileGasEstimate,
                actual: precompileReceipt.gasUsed
            }
        };
    });

    it("transferOwnership tx costs", async () => {
        const owner1 = Wallet.createRandom().connect(provider);
        const owner2 = Wallet.createRandom().connect(provider);
        const newOwner1 = Wallet.createRandom().connect(provider);
        const newOwner2 = Wallet.createRandom().connect(provider);
        // create FPs for owner1 and owner2
        const fp1 = await createFuturepass(owner1, owner1.address);
        await createFuturepass(owner2, owner2.address);

        // precompile approach
        const precompileGasEstimate = await fp1.estimateGas.transferOwnership(newOwner1.address);
        let balanceBefore = await owner1.getBalance();
        const tx = await fp1.connect(owner1).transferOwnership(newOwner1.address);
        const precompileReceipt = await tx.wait();
        let balanceAfter = await owner1.getBalance();
        const precompileFeeCost = balanceBefore.sub(balanceAfter);

        // extrinsic approach
        const owner2KeyRing = keyring.addFromSeed(hexToU8a(owner2.privateKey));
        balanceBefore = await owner2.getBalance();
        await new Promise<void>((resolve) => {
            api.tx.futurepass
                .transferFuturepass(owner2.address, newOwner2.address)
                .signAndSend(owner2KeyRing, ({status}) => {
                    if (status.isInBlock) resolve();
                });
        });
        balanceAfter = await owner2.getBalance();

        const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
        const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
        // expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);
        // expect(extrinsicGasCost).to.be.lessThan(precompileGasEstimate);

        // Update all costs
        allTxGasCosts["transferOwnership"] = {
            Contract: BigNumber.from(0), // no contract
            Precompile: precompileGasEstimate, // convert to XRP Drops(6)
            Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
        };
        allTxFeeCosts["transferOwnership"] = {
            Contract: BigNumber.from(0), // no contract
            Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
            Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
        };
        allEstimates["transferOwnership"] = {
            Contract: {
                estimate: BigNumber.from(0),
                actual: BigNumber.from(0)
            },
            Precompile: {
                estimate: precompileGasEstimate,
                actual: precompileReceipt.gasUsed
            }
        };
    });

    it("proxy transfer from futurepass tx costs", async () => {
        const owner1 = Wallet.createRandom().connect(provider);
        const owner2 = Wallet.createRandom().connect(provider);
        const recipient = Wallet.createRandom().connect(provider);
        // create FPs for owner1 and owner2
        const fp1 = await createFuturepass(owner1, owner1.address);
        const fp2 = await createFuturepass(owner2, owner2.address);

        const transferAmount = 5; // 1 XRP
        // fund futurepasses
        await fundAccount(api, alithKeyring, fp1.address);
        await fundAccount(api, alithKeyring, fp2.address);

        // precompile approach
        const precompileGasEstimate = await fp1.estimateGas.proxyCall(
            CALL_TYPE.Call,
            recipient.address,
            parseEther(transferAmount),
            "0x",
        );
        let balanceBefore = await owner1.getBalance();
        const tx = await fp1.connect(owner1).proxyCall(CALL_TYPE.Call, recipient.address, parseEther(transferAmount), "0x");
        const precompileReceipt = await tx.wait();
        let balanceAfter = await owner1.getBalance();
        const precompileFeeCost = balanceBefore.sub(balanceAfter);
        let recipientBalance = await provider.getBalance(recipient.address);
        expect(recipientBalance).to.equal(parseEther(transferAmount));

        // extrinsic approach
        const owner2KeyRing = keyring.addFromSeed(hexToU8a(owner2.privateKey));
        balanceBefore = await provider.getBalance(fp2.address); // futurepass pays the tx fee for extrinsic approach
        await new Promise<void>((resolve) => {
            api.tx.futurepass
                .proxyExtrinsic(fp2.address, api.tx.assets.transfer(2, recipient.address, transferAmount * 1000000))
                .signAndSend(owner2KeyRing, ({status}) => {
                    if (status.isInBlock) resolve();
                });
        });
        balanceAfter = await provider.getBalance(fp2.address);
        recipientBalance = await provider.getBalance(recipient.address);
        expect(recipientBalance).to.equal(parseEther(2 * transferAmount));

        const extrinsicFeeCost = balanceBefore.sub(balanceAfter).sub(parseEther(transferAmount));
        const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
        // expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);
        // expect(extrinsicGasCost).to.be.lessThan(precompileGasEstimate);

        // Update all costs
        allTxGasCosts["proxyCall"] = {
            Contract: BigNumber.from(0), // no contract
            Precompile: precompileGasEstimate, // convert to XRP Drops(6)
            Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
        };
        allTxFeeCosts["proxyCall"] = {
            Contract: BigNumber.from(0), // no contract
            Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
            Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
        };
        allEstimates["proxyCall"] = {
            Contract: {
                estimate: BigNumber.from(0),
                actual: BigNumber.from(0)
            },
            Precompile: {
                estimate: precompileGasEstimate,
                actual: precompileReceipt.gasUsed
            }
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
            .signAndSend(keyring, ({status}) => {
                if (status.isInBlock) resolve();
            });
    });
}

function parseEther(amount: number): BigNumber {
    return ethers.utils.parseEther(amount.toString());
}
