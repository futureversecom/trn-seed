import {JsonRpcProvider} from "@ethersproject/providers";
import {ApiPromise, Keyring, WsProvider} from "@polkadot/api";
import {KeyringPair} from "@polkadot/keyring/types";
import {hexToU8a} from "@polkadot/util";
import {expect} from "chai";
import {BigNumber, Contract, Wallet} from "ethers";
import {ethers} from "hardhat";

import {
    ALITH_PRIVATE_KEY,
    BOB_PRIVATE_KEY,
    ERC20_ABI,
    GAS_TOKEN_ID,
    NATIVE_TOKEN_ID,
    NodeProcess,
    TxCosts,
    EvmEstimates,
    assetIdToERC20ContractAddress,
    getScaledGasForExtrinsicFee,
    saveTxFees,
    saveTxGas,
    saveTxEstimates,
    startNode,
    typedefs,
} from "../../common";
import {MockERC20} from "../../typechain-types";

describe("ERC20 Gas Estimates", function () {
    // let node: NodeProcess;

    let provider: JsonRpcProvider;
    let api: ApiPromise;
    let bobSigner: Wallet;
    let alithSigner: Wallet;
    let erc20Precompile: MockERC20;
    let erc20Contract: MockERC20;
    let alith: KeyringPair;
    let bob: KeyringPair;

    const allCosts: { [key: string]: TxCosts } = {};
    const allTxFeeCosts: { [key: string]: TxCosts } = {};
    const allEstimates: { [key: string]: EvmEstimates } = {};

    // Setup api instance
    before(async () => {
        // node = await startNode();
        // await node.wait(); // wait for the node to be ready

        const wsProvider = new WsProvider(`ws://localhost:9944`);

        // Setup Root api instance and keyring
        api = await ApiPromise.create({provider: wsProvider, types: typedefs});
        provider = new JsonRpcProvider(`http://127.0.0.1:9933`);
        alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
        bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);
        const keyring = new Keyring({type: "ethereum"});
        alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
        bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));

        // Create ERC20 token
        const erc20PrecompileAddress = assetIdToERC20ContractAddress(GAS_TOKEN_ID);

        // Create precompiles contract
        erc20Precompile = new Contract(erc20PrecompileAddress, ERC20_ABI, alithSigner) as MockERC20;

        // Deploy ERC20 contract
        const ERC20Factory = await ethers.getContractFactory("MockERC20");
        erc20Contract = await ERC20Factory.connect(alithSigner).deploy();
        await erc20Contract.deployed();
        console.log("MockERC20 deployed to:", erc20Contract.address);

        // Mint 100 tokens to alith
        const gas = await erc20Contract.connect(alithSigner).estimateGas.mint(alithSigner.address, 1000);
        const tx = await erc20Contract.connect(alithSigner).mint(alithSigner.address, 1000, {gasLimit: gas});
        await tx.wait();
    });

    after(async () => {
        saveTxGas(allCosts, "ERC20/TxCosts.md", "ERC20 Precompiles");
        saveTxFees(allTxFeeCosts, "ERC20/TxCosts.md", "ERC20 Precompiles");
        saveTxEstimates(allEstimates, "ERC20/TxCosts.md", "ERC20 Precompiles");
        // await node.stop();
    });

    // // ERC20 view functions
    // it("totalSupply gas estimates", async () => {
    //   // Estimate contract call
    //   const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.totalSupply();
    //   // Estimate precompile call
    //   const precompileGasEstimate = await erc20Precompile.connect(alithSigner).estimateGas.totalSupply();
    //
    //   // expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    //
    //   // Update all costs with gas info
    //   allCosts["totalSupply"] = {
    //     Contract: contractGasEstimate,
    //     Precompile: precompileGasEstimate,
    //     Extrinsic: BigNumber.from(0), // No extrinsic
    //   };
    // });
    //
    // it("balanceOf gas estimates", async () => {
    //   // Estimate contract call
    //   const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.balanceOf(bobSigner.address);
    //   // Estimate precompile call
    //   const precompileGasEstimate = await erc20Precompile.connect(alithSigner).estimateGas.balanceOf(bobSigner.address);
    //
    //   // expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    //
    //   // Update all costs with gas info
    //   allCosts["balanceOf"] = {
    //     Contract: contractGasEstimate,
    //     Precompile: precompileGasEstimate,
    //     Extrinsic: BigNumber.from(0), // No extrinsic
    //   };
    // });
    //
    // it("allowance gas estimates", async () => {
    //   // Estimate contract call
    //   const contractGasEstimate = await erc20Contract
    //     .connect(alithSigner)
    //     .estimateGas.allowance(alithSigner.address, bobSigner.address);
    //   // Estimate precompile call
    //   const precompileGasEstimate = await erc20Precompile
    //     .connect(alithSigner)
    //     .estimateGas.allowance(alithSigner.address, bobSigner.address);
    //
    //   // expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    //
    //   // Update all costs with gas info
    //   allCosts["allowance"] = {
    //     Contract: contractGasEstimate,
    //     Precompile: precompileGasEstimate,
    //     Extrinsic: BigNumber.from(0), // No extrinsic
    //   };
    // });

    // ERC20 functions
    it("setApproval gas estimates", async () => {
        const amount = 1000;
        // Estimate contract call
        const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.approve(bobSigner.address, amount);

        // Estimate precompile call
        const precompileGasEstimate = await erc20Precompile
            .connect(alithSigner)
            .estimateGas.approve(bobSigner.address, amount);

        // precompile fee cost
        let balanceBefore = await alithSigner.getBalance();
        let tx = await erc20Precompile.connect(alithSigner).approve(bobSigner.address, amount);
        const precompileReceipt = await tx.wait();
        let balanceAfter = await alithSigner.getBalance();
        const precompileFeeCost = balanceBefore.sub(balanceAfter);

        // Contract fee cost
        balanceBefore = await alithSigner.getBalance();
        tx = await erc20Contract.connect(alithSigner).approve(bobSigner.address, amount, {gasLimit: contractGasEstimate});
        const contractReceipt = await tx.wait();
        balanceAfter = await alithSigner.getBalance();
        const contractFeeCost = balanceBefore.sub(balanceAfter);

        // Extrinsic cost
        balanceBefore = await alithSigner.getBalance();
        await new Promise<void>((resolve) => {
            api.tx.assets.approveTransfer(NATIVE_TOKEN_ID, bobSigner.address, amount).signAndSend(alith, ({status}) => {
                if (status.isInBlock) resolve();
            });
        });
        balanceAfter = await alithSigner.getBalance();
        const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
        const extrinsicGasScaled = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);

        // expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
        // expect(extrinsicGasScaled).to.be.lessThan(precompileGasEstimate);
        // expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

        // Update all costs
        allCosts["approval"] = {
            Contract: contractGasEstimate,
            Precompile: precompileGasEstimate,
            Extrinsic: extrinsicGasScaled,
        };
        allTxFeeCosts["approval"] = {
            Contract: contractFeeCost.div(1000000000000n), // convert to XRP Drops(6)
            Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
            Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
        };
        allEstimates["approval"] = {
            Contract: {
                estimate: contractGasEstimate,
                actual: contractReceipt.gasUsed
            },
            Precompile: {
                estimate: precompileGasEstimate,
                actual: precompileReceipt.gasUsed
            }
        };
    });

    it("transfer gas estimates", async () => {
        const amount = 100;

        // Estimate contract call
        const contractGasEstimate = await erc20Contract
            .connect(alithSigner)
            .estimateGas.transfer(bobSigner.address, amount);
        // Estimate precompile call
        const precompileGasEstimate = await erc20Precompile
            .connect(alithSigner)
            .estimateGas.transfer(bobSigner.address, amount);
        // precompile fee cost
        let balanceBefore = await alithSigner.getBalance();
        let tx = await erc20Precompile.connect(alithSigner).transfer(bobSigner.address, amount);
        const precompileReceipt = await tx.wait();
        let balanceAfter = await alithSigner.getBalance();
        const precompileFeeCost = balanceBefore.sub(balanceAfter);
        // contract fee cost
        balanceBefore = await alithSigner.getBalance();
        tx = await erc20Contract
            .connect(alithSigner)
            .transfer(bobSigner.address, amount, {gasLimit: contractGasEstimate});
        const contractReceipt = await tx.wait();
        balanceAfter = await alithSigner.getBalance();
        const contractFeeCost = balanceBefore.sub(balanceAfter);
        // Extrinsic cost
        balanceBefore = await alithSigner.getBalance();
        await new Promise<void>((resolve) => {
            api.tx.assets.transfer(NATIVE_TOKEN_ID, bobSigner.address, amount).signAndSend(alith, ({status}) => {
                if (status.isInBlock) resolve();
            });
        });
        balanceAfter = await alithSigner.getBalance();
        const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
        const extrinsicGasScaled = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);

        // expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
        // expect(extrinsicGasScaled).to.be.lessThan(precompileGasEstimate);
        // expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

        // Update all costs
        allCosts["transfer"] = {
            Contract: contractGasEstimate,
            Precompile: precompileGasEstimate,
            Extrinsic: extrinsicGasScaled,
        };
        allTxFeeCosts["transfer"] = {
            Contract: contractFeeCost.div(1000000000000n), // convert to XRP Drops(6)
            Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
            Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
        };
        allEstimates["transfer"] = {
            Contract: {
                estimate: contractGasEstimate,
                actual: contractReceipt.gasUsed
            },
            Precompile: {
                estimate: precompileGasEstimate,
                actual: precompileReceipt.gasUsed
            }
        };
    });

    it("transferFrom gas estimates", async () => {
        const amount = 100;

        // // set approval to transfer tokens back from bob to alice
        let gas = await erc20Precompile.connect(alithSigner).estimateGas.approve(bobSigner.address, amount);
        let tx = await erc20Precompile.connect(alithSigner).approve(bobSigner.address, amount, {gasLimit: gas});
        await tx.wait();
        gas = await erc20Contract.connect(alithSigner).estimateGas.approve(bobSigner.address, amount);
        tx = await erc20Contract.connect(alithSigner).approve(bobSigner.address, amount, {gasLimit: gas});
        await tx.wait();

        // Estimate contract call
        // Note: if we use the full approved amount, the contract actual gas usage becomes lesser than the precompile.
        const contractGasEstimate = await erc20Contract
            .connect(bobSigner)
            .estimateGas.transferFrom(alithSigner.address, bobSigner.address, amount);
        // Estimate precompile call
        const precompileGasEstimate = await erc20Precompile
            .connect(bobSigner)
            .estimateGas.transferFrom(alithSigner.address, bobSigner.address, amount);
        // precompile fee cost
        let balanceBefore = await bobSigner.getBalance();
        tx = await erc20Precompile.connect(bobSigner).transferFrom(alithSigner.address, bobSigner.address, amount);
        const precompileReceipt = await tx.wait();
        let balanceAfter = await bobSigner.getBalance();
        const precompileFeeCost = balanceBefore.sub(balanceAfter);
        // Contract fee cost
        balanceBefore = await bobSigner.getBalance();
        tx = await erc20Contract
            .connect(bobSigner)
            .transferFrom(alithSigner.address, bobSigner.address, amount, {gasLimit: contractGasEstimate});
        const contractReceipt = await tx.wait();
        balanceAfter = await bobSigner.getBalance();
        const contractFeeCost = balanceBefore.sub(balanceAfter);
        // Extrinsic cost
        balanceBefore = await bobSigner.getBalance();
        await new Promise<void>((resolve) => {
            api.tx.assets
                .transferApproved(NATIVE_TOKEN_ID, alithSigner.address, bobSigner.address, amount)
                .signAndSend(bob, ({status}) => {
                    if (status.isInBlock) resolve();
                });
        });
        balanceAfter = await bobSigner.getBalance();
        const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
        const extrinsicGasScaled = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);

        // expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
        // expect(extrinsicGasScaled).to.be.lessThan(precompileGasEstimate);
        // expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

        // Update all costs
        allCosts["transferFrom"] = {
            Contract: contractGasEstimate,
            Precompile: precompileGasEstimate,
            Extrinsic: extrinsicGasScaled,
        };
        allTxFeeCosts["transferFrom"] = {
            Contract: contractFeeCost.div(1000000000000n), // convert to XRP Drops(6)
            Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
            Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
        };
        allEstimates["transferFrom"] = {
            Contract: {
                estimate: contractGasEstimate,
                actual: contractReceipt.gasUsed
            },
            Precompile: {
                estimate: precompileGasEstimate,
                actual: precompileReceipt.gasUsed
            }
        };
    });
    //
    // // ERC20 metadata view functions
    // it("name gas estimates", async () => {
    //   // Estimate contract call
    //   const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.name();
    //   // Estimate precompile call
    //   const precompileGasEstimate = await erc20Precompile.connect(alithSigner).estimateGas.name();
    //
    //   // expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    //
    //   // Update all costs with gas info
    //   allCosts["name"] = {
    //     Contract: contractGasEstimate,
    //     Precompile: precompileGasEstimate,
    //     Extrinsic: BigNumber.from(0), // No extrinsic
    //   };
    // });
    //
    // it("decimals gas estimates", async () => {
    //   // Estimate contract call
    //   const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.decimals();
    //   // Estimate precompile call
    //   const precompileGasEstimate = await erc20Precompile.connect(alithSigner).estimateGas.decimals();
    //
    //   // expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate.add(50));
    //
    //   // Update all costs with gas info
    //   allCosts["decimals"] = {
    //     Contract: contractGasEstimate,
    //     Precompile: precompileGasEstimate,
    //     Extrinsic: BigNumber.from(0), // No extrinsic
    //   };
    // });
    //
    // it("symbol gas estimates", async () => {
    //   // Estimate contract call
    //   const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.symbol();
    //   // Estimate precompile call
    //   const precompileGasEstimate = await erc20Precompile.connect(alithSigner).estimateGas.symbol();
    //
    //   // expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    //
    //   // Update all costs with gas info
    //   allCosts["symbol"] = {
    //     Contract: contractGasEstimate,
    //     Precompile: precompileGasEstimate,
    //     Extrinsic: BigNumber.from(0), // No extrinsic
    //   };
    // });
});
