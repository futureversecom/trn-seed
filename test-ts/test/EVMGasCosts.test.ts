import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { utils, Wallet } from "ethers";
import { ethers } from "hardhat";
import web3 from "web3";

import { ALICE_PRIVATE_KEY, BOB_PRIVATE_KEY, typedefs } from "../common";
import type { MockERC20 } from "../typechain-types";

const FIRST_ASSET_ID = 1124;

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
				"1000000000000000000000000"
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
		expect(fees.lastBaseFeePerGas?.toNumber()).to.eql(15_000_000_000_000); // base fee = 15000 gwei
		expect(fees.maxFeePerGas?.toNumber()).to.eql(30_001_500_000_000);
		expect(fees.maxPriorityFeePerGas?.toNumber()).to.eql(1_500_000_000);
		expect(fees.gasPrice?.toNumber()).to.eql(15_000_000_000_000);
	});

	it("gas cost for evm call", async () => {
		const callCost = 21_000;
		const fees = await provider.getFeeData();
		const nonce = await aliceSigner.getTransactionCount();
		const unsignedTx = {
			// eip1559 tx
			type: 2,
			from: aliceSigner.address,
			to: bobSigner.address,
			nonce,
			data: "",
			gasLimit: callCost,
			maxFeePerGas: fees.lastBaseFeePerGas!,
			maxPriorityFeePerGas: 0,
			chainId: 3999,
		};
		const signedTx = await aliceSigner.signTransaction(unsignedTx);
		const tx = await provider.sendTransaction(signedTx);
		const receipt = await tx.wait();

		// assert gas used
		expect(receipt.gasUsed?.toNumber()).to.eql(callCost);
		expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(callCost);
		expect(receipt.effectiveGasPrice?.toNumber()).to.eql(15_000_000_000_000);

		// assert XRP used
		const xrpGasCost = receipt.gasUsed.mul(receipt.effectiveGasPrice);
		const xrpCost6DP = +xrpGasCost.div(10 ** 12).toString();
  	const xrpCostScaled = +utils.formatEther(xrpGasCost).toString();
		expect(xrpCost6DP).to.eql(315000);
		expect(xrpCostScaled).to.eql(0.315);
	});

	it("gas cost for XRP transfer", async () => {
		const sendEthGasCost = 21_000;
		const fees = await provider.getFeeData();
		const aliceBalanceBefore = await aliceSigner.getBalance();

		const tx = await aliceSigner.sendTransaction({
			to: "0x0000000000000000000000000000000000000000",
			value: utils.parseEther("1"),
			gasLimit: sendEthGasCost,
			maxFeePerGas: fees.lastBaseFeePerGas!,
			maxPriorityFeePerGas: 0, // no miner tip
		});
		const receipt = await tx.wait();
		expect(receipt.gasUsed?.toNumber()).to.eql(sendEthGasCost);
		expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(sendEthGasCost);

		// assert gas used
		const totalPaid = receipt.effectiveGasPrice
			?.mul(sendEthGasCost)
			.add(utils.parseEther("1"));
		const aliceBalanceAfter = await aliceSigner.getBalance();
		expect(aliceBalanceBefore.sub(aliceBalanceAfter).toString()).to.eql(
			totalPaid.toString()
		);

		// assert XRP used
		const oneXRP6DP = 1_000_000, oneXRPScaled = 1;
		const xrpCost6DP = +totalPaid.div(10 ** 12).toString() - oneXRP6DP; // subtract XRP sent
  	const xrpCostScaled = +utils.formatEther(totalPaid).toString() - oneXRPScaled; // subtract XRP sent
		expect(xrpCost6DP).to.eql(315000);
		expect(+xrpCostScaled.toFixed(3)).to.eql(0.315);
	});

	it("gas cost for deploying erc20 contract", async () => {
		const fees = await provider.getFeeData();
		const aliceBalanceBefore = await aliceSigner.getBalance();

		const ERC20Factory = await ethers.getContractFactory("MockERC20");
		erc20Contract = await ERC20Factory.connect(aliceSigner).deploy();
		await erc20Contract.deployed();
		console.log("erc20Contract deployed to:", erc20Contract.address);

		const aliceBalanceAfter = await aliceSigner.getBalance();
		const balanceDiff = aliceBalanceBefore.sub(aliceBalanceAfter);

		// assert gas used
		const lowerbound = 7_050_000,
			upperbound = 7_055_000;
		expect(balanceDiff.div(fees.lastBaseFeePerGas!).toNumber())
			.is.greaterThan(lowerbound)
			.and.is.lessThan(upperbound);

		// assert XRP used
		const xrpCost6DP = +balanceDiff.div(10 ** 12).toString();
  	const xrpCostScaled = +utils.formatEther(balanceDiff).toString();
		expect(xrpCost6DP).to.eql(105_769_959);
		expect(xrpCostScaled).to.eql(105.769959);
	});

	it("gas cost for token mint", async () => {
		const fees = await provider.getFeeData();
		const aliceBalanceBefore = await aliceSigner.getBalance();

		const wantGasEstimate = 75_339;
		const gasEstimate = await erc20Contract
			.connect(aliceSigner)
			.estimateGas.mint(aliceSigner.address, 1000, {
				maxFeePerGas: fees.lastBaseFeePerGas!,
				maxPriorityFeePerGas: 0,
			});
		expect(gasEstimate.toNumber()).to.eql(wantGasEstimate);

		const tx = await erc20Contract
			.connect(aliceSigner)
			.mint(aliceSigner.address, 1000, {
				gasLimit: gasEstimate,
				maxFeePerGas: fees.lastBaseFeePerGas!,
				maxPriorityFeePerGas: 0,
			});
		const receipt = await tx.wait();

		// assert gas used
		const wantActualGasUsed = 71_403;
		expect(receipt.gasUsed?.toNumber()).to.eql(wantActualGasUsed);
		expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(wantActualGasUsed);

		const totalPaid = receipt.effectiveGasPrice?.mul(gasEstimate);
		const aliceBalanceAfter = await aliceSigner.getBalance();
		expect(aliceBalanceBefore.sub(aliceBalanceAfter).toString()).to.eql(
			totalPaid.toString()
		);

		// assert XRP used
		const xrpCost6DP = +totalPaid.div(10 ** 12).toString();
  	const xrpCostScaled = +utils.formatEther(totalPaid).toString();
		expect(xrpCost6DP).to.eql(1_130_085);
		expect(xrpCostScaled).to.eql(1.130085);
	});

	it("gas cost for token transfer", async () => {
		const fees = await provider.getFeeData();
		const aliceBalanceBefore = await aliceSigner.getBalance();

		const wantGasEstimate = 50_870;
		const gasEstimate = await erc20Contract
			.connect(aliceSigner)
			.estimateGas.transfer(bobSigner.address, 500, {
				maxFeePerGas: fees.lastBaseFeePerGas!,
				maxPriorityFeePerGas: 0,
			});
		expect(gasEstimate.toNumber()).to.eql(wantGasEstimate);

		const tx = await erc20Contract
			.connect(aliceSigner)
			.transfer(bobSigner.address, 500, {
				gasLimit: gasEstimate,
				maxFeePerGas: fees.lastBaseFeePerGas!,
				maxPriorityFeePerGas: 0,
			});
		const receipt = await tx.wait();

		// assert gas used
		const wantActualGasUsed = 49_483;
		expect(receipt.gasUsed?.toNumber()).to.eql(wantActualGasUsed);
		expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(wantActualGasUsed);

		const totalPaid = receipt.effectiveGasPrice?.mul(gasEstimate);
		const aliceBalanceAfter = await aliceSigner.getBalance();
		expect(aliceBalanceBefore.sub(aliceBalanceAfter).toString()).to.eql(
			totalPaid.toString()
		);

		// assert XRP used
		const xrpCost6DP = +totalPaid.div(10 ** 12).toString();
  	const xrpCostScaled = +utils.formatEther(totalPaid).toString();
		expect(xrpCost6DP).to.eql(763_050);
		expect(xrpCostScaled).to.eql(0.763050);
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
		const gasEstimate = await erc20PrecompileContract
			.connect(aliceSigner)
			.estimateGas.transfer(bobSigner.address, 500, {
				maxFeePerGas: fees.lastBaseFeePerGas!,
				maxPriorityFeePerGas: 0,
			});
		expect(gasEstimate.toNumber()).to.eql(wantGasEstimate);

		const tx = await erc20PrecompileContract
			.connect(aliceSigner)
			.transfer(bobSigner.address, 500, {
				gasLimit: gasEstimate,
				maxFeePerGas: fees.lastBaseFeePerGas!,
				maxPriorityFeePerGas: 0,
			});
		const receipt = await tx.wait();

		// assert gas used
		const wantActualGasUsed = 22_953;
		expect(receipt.gasUsed?.toNumber()).to.eql(wantActualGasUsed);
		expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(wantActualGasUsed);

		const totalPaid = receipt.effectiveGasPrice?.mul(gasEstimate);
		const aliceBalanceAfter = await aliceSigner.getBalance();
		expect(aliceBalanceBefore.sub(aliceBalanceAfter).toString()).to.eql(
			totalPaid.toString()
		);

		// assert XRP used
		const xrpCost6DP = +totalPaid.div(10 ** 12).toString();
  	const xrpCostScaled = +utils.formatEther(totalPaid).toString();
		expect(xrpCost6DP).to.eql(348_645);
		expect(xrpCostScaled).to.eql(0.348645);
	});
});
