import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { utils, Wallet } from "ethers";
import { ethers } from "hardhat";
import web3 from "web3";

import MockERC20Data from "../artifacts/contracts/MockERC20.sol/MockERC20.json";
import {
	ALICE_PRIVATE_KEY,
	BOB_PRIVATE_KEY,
	DEAD_ADDRESS,
	typedefs,
} from "../common";
import type { MockERC20 } from "../typechain-types";

const FIRST_ASSET_ID = 1124;
const BASE_GAS_COST = 21_000;
const BASE_FEE_PER_GAS = 15_000_000_000_000;
const PRIORITY_FEE_PER_GAS = 1_500_000_000;
const MAX_FEE_PER_GAS = BASE_FEE_PER_GAS * 2 + PRIORITY_FEE_PER_GAS;

// Note: Tests must be run in order, synchronously
describe("EVM gas costs", () => {
	let api: ApiPromise;
	let alice: KeyringPair;

	let provider: JsonRpcProvider;
	let aliceSigner: Wallet;
	let bobSigner: Wallet;
	let erc20Contract: MockERC20;

	before(async () => {
		// Substrate variables
		const wsProvider = new WsProvider(`ws://localhost:9944`);
		api = await ApiPromise.create({
			provider: wsProvider,
			types: typedefs,
		});
		const keyring = new Keyring({ type: "ethereum" });
		alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));

		// create and mint asset - ID 1124
		const txs = [
			api.tx.assetsExt.createAsset(),
			api.tx.assets.mint(
				FIRST_ASSET_ID,
				alice.address,
				utils.parseEther("1000").toString()
			),
		];
		await new Promise<void>((resolve) => {
			api.tx.utility.batch(txs).signAndSend(alice, ({ status }) => {
				if (status.isInBlock) resolve();
			});
		});
		console.log("Created and minted asset:", FIRST_ASSET_ID);

		// EVM variables
		provider = new JsonRpcProvider(`http://localhost:9933`);
		aliceSigner = new Wallet(ALICE_PRIVATE_KEY).connect(provider);
		bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);
	});

	it("default gas fees", async () => {
		const fees = await provider.getFeeData();
		expect(fees.lastBaseFeePerGas?.toNumber()).to.eql(BASE_FEE_PER_GAS); // base fee = 15000 gwei
		expect(fees.maxFeePerGas?.toNumber()).to.eql(MAX_FEE_PER_GAS);
		expect(fees.maxPriorityFeePerGas?.toNumber()).to.eql(PRIORITY_FEE_PER_GAS);
		expect(fees.gasPrice?.toNumber()).to.eql(BASE_FEE_PER_GAS);
	});

	it("gas cost for evm call", async () => {
		const fees = await provider.getFeeData();
		const nonce = await aliceSigner.getTransactionCount();
		const unsignedTx = {
			// eip1559 tx
			type: 2,
			from: aliceSigner.address,
			to: bobSigner.address,
			nonce,
			data: "",
			gasLimit: BASE_GAS_COST,
			maxFeePerGas: fees.lastBaseFeePerGas!,
			maxPriorityFeePerGas: 0,
			chainId: 3999,
		};
		const signedTx = await aliceSigner.signTransaction(unsignedTx);
		const tx = await provider.sendTransaction(signedTx);
		const receipt = await tx.wait();

		// assert gas used
		expect(receipt.gasUsed?.toNumber()).to.eql(BASE_GAS_COST);
		expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(BASE_GAS_COST);
		expect(receipt.effectiveGasPrice?.toNumber()).to.eql(BASE_FEE_PER_GAS);

		// assert XRP used
		const xrpGasCost = receipt.gasUsed.mul(receipt.effectiveGasPrice);
		const xrpCost6DP = xrpGasCost.div(10 ** 12).toNumber();
		const xrpCostScaled = +utils.formatEther(xrpGasCost);
		expect(xrpCost6DP).to.eql(315000);
		expect(xrpCostScaled).to.eql(0.315);
	});

	it("gas cost for XRP transfer", async () => {
		const fees = await provider.getFeeData();
		const aliceBalanceBefore = await aliceSigner.getBalance();

		const tx = await aliceSigner.sendTransaction({
			to: DEAD_ADDRESS,
			value: utils.parseEther("1"),
			gasLimit: BASE_GAS_COST,
			maxFeePerGas: fees.lastBaseFeePerGas!,
			maxPriorityFeePerGas: 0, // no miner tip
		});
		const receipt = await tx.wait();
		expect(receipt.gasUsed?.toNumber()).to.eql(BASE_GAS_COST);
		expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(BASE_GAS_COST);

		// assert gas used
		const totalPaid = receipt.effectiveGasPrice
			?.mul(BASE_GAS_COST)
			.add(utils.parseEther("1"));
		const aliceBalanceAfter = await aliceSigner.getBalance();
		expect(aliceBalanceBefore.sub(aliceBalanceAfter)).to.eql(totalPaid);

		// assert XRP used
		const oneXRP6DP = 1_000_000,
			oneXRPScaled = 1;
		const xrpCost6DP = totalPaid.div(10 ** 12).toNumber() - oneXRP6DP; // subtract XRP sent
		const xrpCostScaled = +utils.formatEther(totalPaid) - oneXRPScaled; // subtract XRP sent
		expect(xrpCost6DP).to.eql(315000);
		expect(+xrpCostScaled.toFixed(3)).to.eql(0.315);
	});

	it("gas cost for deploying erc20 contract", async () => {
		const fees = await provider.getFeeData();
		const aliceBalanceBefore = await aliceSigner.getBalance();

		const factory = new ethers.ContractFactory(
			MockERC20Data.abi,
			MockERC20Data.bytecode,
			aliceSigner
		);
		const actualGasEstimate = await provider.estimateGas(
			factory.getDeployTransaction()
		);
		erc20Contract = (await factory.connect(aliceSigner).deploy({
			gasLimit: actualGasEstimate,
			maxFeePerGas: fees.lastBaseFeePerGas!,
			maxPriorityFeePerGas: 0,
		})) as MockERC20;
		const receipt = await erc20Contract.deployTransaction.wait();
		console.log("erc20Contract deployed to:", erc20Contract.address);

		// assert gas used
		const totalPaid = receipt.effectiveGasPrice?.mul(actualGasEstimate);
		const aliceBalanceAfter = await aliceSigner.getBalance();
		expect(aliceBalanceBefore.sub(aliceBalanceAfter)).to.eql(totalPaid);

		// assert XRP used
		const xrpCost6DP = totalPaid.div(10 ** 12).toNumber();
		const xrpCostScaled = +utils.formatEther(totalPaid);
		expect(xrpCost6DP).to.eql(52_574_490);
		expect(xrpCostScaled).to.eql(52.57449);
	});

	it("gas cost for token mint", async () => {
		const fees = await provider.getFeeData();
		const aliceBalanceBefore = await aliceSigner.getBalance();

		const wantGasEstimate = 75_339;
		const actualGasEstimate = await erc20Contract
			.connect(aliceSigner)
			.estimateGas.mint(aliceSigner.address, 1000, {
				maxFeePerGas: fees.lastBaseFeePerGas!,
				maxPriorityFeePerGas: 0,
			});
		expect(actualGasEstimate.toNumber()).to.eql(wantGasEstimate);

		const tx = await erc20Contract
			.connect(aliceSigner)
			.mint(aliceSigner.address, 1000, {
				gasLimit: actualGasEstimate,
				maxFeePerGas: fees.lastBaseFeePerGas!,
				maxPriorityFeePerGas: 0,
			});
		const receipt = await tx.wait();

		// assert gas used
		const wantGasUsed = 71_403;
		expect(receipt.gasUsed?.toNumber()).to.eql(wantGasUsed);
		expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(wantGasUsed);

		const totalPaid = receipt.effectiveGasPrice?.mul(actualGasEstimate);
		const aliceBalanceAfter = await aliceSigner.getBalance();
		expect(aliceBalanceBefore.sub(aliceBalanceAfter)).to.eql(totalPaid);

		// assert XRP used
		const xrpCost6DP = totalPaid.div(10 ** 12).toNumber();
		const xrpCostScaled = +utils.formatEther(totalPaid);
		expect(xrpCost6DP).to.eql(1_130_085);
		expect(xrpCostScaled).to.eql(1.130085);
	});

	it("gas cost for token transfer", async () => {
		const fees = await provider.getFeeData();
		const aliceBalanceBefore = await aliceSigner.getBalance();

		const wantGasEstimate = 50_870;
		const actualGasEstimate = await erc20Contract
			.connect(aliceSigner)
			.estimateGas.transfer(bobSigner.address, 500, {
				maxFeePerGas: fees.lastBaseFeePerGas!,
				maxPriorityFeePerGas: 0,
			});
		expect(actualGasEstimate.toNumber()).to.eql(wantGasEstimate);

		const tx = await erc20Contract
			.connect(aliceSigner)
			.transfer(bobSigner.address, 500, {
				gasLimit: actualGasEstimate,
				maxFeePerGas: fees.lastBaseFeePerGas!,
				maxPriorityFeePerGas: 0,
			});
		const receipt = await tx.wait();

		// assert gas used
		const wantGasUsed = 49_483;
		expect(receipt.gasUsed?.toNumber()).to.eql(wantGasUsed);
		expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(wantGasUsed);

		const totalPaid = receipt.effectiveGasPrice?.mul(actualGasEstimate);
		const aliceBalanceAfter = await aliceSigner.getBalance();
		expect(aliceBalanceBefore.sub(aliceBalanceAfter)).to.eql(totalPaid);

		// assert XRP used
		const xrpCost6DP = totalPaid.div(10 ** 12).toNumber();
		const xrpCostScaled = +utils.formatEther(totalPaid);
		expect(xrpCost6DP).to.eql(763_050);
		expect(xrpCostScaled).to.eql(0.76305);
	});

	it("gas cost for pre-compile token transfer", async () => {
		// connect to erc20 precompile
		const erc20Token = FIRST_ASSET_ID.toString(16).padStart(8, "0");
		const erc20TokenAddress = web3.utils.toChecksumAddress(
			`0xCCCCCCCC${erc20Token}000000000000000000000000`
		);
		const ERC20Factory = await ethers.getContractFactory("MockERC20");
		const erc20PrecompileContract =
			ERC20Factory.connect(aliceSigner).attach(erc20TokenAddress);

		// transfer token via precompile tests
		const fees = await provider.getFeeData();
		const aliceBalanceBefore = await aliceSigner.getBalance();

		const wantGasEstimate = 23_243;
		const actualGasEstimate = await erc20PrecompileContract
			.connect(aliceSigner)
			.estimateGas.transfer(bobSigner.address, 500, {
				maxFeePerGas: fees.lastBaseFeePerGas!,
				maxPriorityFeePerGas: 0,
			});
		expect(actualGasEstimate.toNumber()).to.eql(wantGasEstimate);

		const tx = await erc20PrecompileContract
			.connect(aliceSigner)
			.transfer(bobSigner.address, 500, {
				gasLimit: actualGasEstimate,
				maxFeePerGas: fees.lastBaseFeePerGas!,
				maxPriorityFeePerGas: 0,
			});
		const receipt = await tx.wait();

		// assert gas used
		const wantGasUsed = 22_953;
		expect(receipt.gasUsed?.toNumber()).to.eql(wantGasUsed);
		expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(wantGasUsed);

		const totalPaid = receipt.effectiveGasPrice?.mul(actualGasEstimate);
		const aliceBalanceAfter = await aliceSigner.getBalance();
		expect(aliceBalanceBefore.sub(aliceBalanceAfter)).to.eql(totalPaid);

		// assert XRP used
		const xrpCost6DP = totalPaid.div(10 ** 12).toNumber();
		const xrpCostScaled = +utils.formatEther(totalPaid);
		expect(xrpCost6DP).to.eql(348_645);
		expect(xrpCostScaled).to.eql(0.348645);
	});
});
