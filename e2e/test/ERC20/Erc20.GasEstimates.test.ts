import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import {Contract, Wallet} from "ethers";
import { ethers } from "hardhat";

import ERC20Data from "../../artifacts/contracts/MockERC20.sol/MockERC20.json";
import {
    ALITH_PRIVATE_KEY, assetIdToERC20ContractAddress,
    BOB_PRIVATE_KEY,
    ERC20_ABI,
    GasCosts,
    NodeProcess,
    saveGasCosts,
    startNode,
    typedefs,
} from "../../common";
import web3 from "web3";

describe("ERC20 Gas Estimates", function () {
    let node: NodeProcess;

    let provider: JsonRpcProvider;
    let api: ApiPromise;
    let bobSigner: Wallet;
    let alithSigner: Wallet;
    let erc20Precompile: Contract;
    let erc20Contract: Contract;
    let alith: KeyringPair;

    const allCosts: { [key: string]: GasCosts } = {};

    // Setup api instance
    before(async () => {
        node = await startNode();

        const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);

        // Setup Root api instance and keyring
        api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
        const keyring = new Keyring({ type: "ethereum" });
        alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

        provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
        alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
        bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);

        // Create ERC20 token
        const erc20PrecompileAddress = web3.utils.toChecksumAddress(assetIdToERC20ContractAddress(2));


        // Create precompiles contract
        erc20Precompile = new Contract(erc20PrecompileAddress, ERC20_ABI, alithSigner);


        // Deploy OpenZeppelin ERC20 contract
        const factory = new ethers.ContractFactory(ERC20Data.abi, ERC20Data.bytecode, alithSigner);
        erc20Contract = await factory.connect(alithSigner).deploy();
        const tokenAmount = 10000;
        // Estimate contract call
        await erc20Contract
            .connect(alithSigner)
            .mint(alithSigner.address, tokenAmount);
    });

    after(async () => {
        saveGasCosts(allCosts, "ERC20/GasCosts.md", "ERC20 Precompiles");

        await node.stop();
    });

    it("name gas estimates", async () => {
        // Estimate contract call
        const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.name();
        // Estimate precompile call
        const precompileGasEstimate = await erc20Precompile.connect(alithSigner).estimateGas.name();

        expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

        // Update all costs with gas info
        allCosts["name"] = {
            Contract: contractGasEstimate.toNumber(),
            Precompile: precompileGasEstimate.toNumber(),
            Extrinsic: 0, // No extrinsic
        };
    });

    it("balanceOf gas estimates", async () => {
        // Estimate contract call
        const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.balanceOf(bobSigner.address);
        // Estimate precompile call
        const precompileGasEstimate = await erc20Precompile
            .connect(alithSigner)
            .estimateGas.balanceOf(bobSigner.address);

        expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

        // Update all costs with gas info
        allCosts["balanceOf"] = {
            Contract: contractGasEstimate.toNumber(),
            Precompile: precompileGasEstimate.toNumber(),
            Extrinsic: 0, // No extrinsic
        };
    });

    it("setApproval gas estimates", async () => {
        const amount = 1000;
        // Estimate contract call
        const contractGasEstimate = await erc20Contract
            .connect(alithSigner)
            .estimateGas.approve(bobSigner.address, amount);
        // Estimate precompile call
        const precompileGasEstimate = await erc20Precompile
            .connect(alithSigner)
            .estimateGas.approve(bobSigner.address, amount);

        expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

        // Update all costs with gas info
        allCosts["Approval"] = {
            Contract: contractGasEstimate.toNumber(),
            Precompile: precompileGasEstimate.toNumber(),
            Extrinsic: 0, // No extrinsic
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

        // Update all costs with gas info
        allCosts["safeTransferFrom"] = {
            Contract: contractGasEstimate.toNumber(),
            Precompile: precompileGasEstimate.toNumber(),
            Extrinsic: 0,
        };
    });

    it("transferFrom gas estimates", async () => {
        const amount = 100;

        // Estimate contract call
        const contractGasEstimate = await erc20Contract
            .connect(bobSigner)
            .estimateGas.transferFrom(alithSigner.address, amount);
        // Estimate precompile call
        const precompileGasEstimate = await erc20Precompile
            .connect(bobSigner)
            .estimateGas.transferFrom(alithSigner.address, amount);

        expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

        // Update all costs with gas info
        allCosts["transferFrom"] = {
            Contract: contractGasEstimate.toNumber(),
            Precompile: precompileGasEstimate.toNumber(),
            Extrinsic: 0,
        };
    });
});
