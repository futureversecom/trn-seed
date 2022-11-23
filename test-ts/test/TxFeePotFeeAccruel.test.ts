import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Contract, utils, Wallet } from "ethers";
import { ethers } from "hardhat";
import web3 from "web3";

import { ALICE_PRIVATE_KEY, ERC20_ABI, typedefs } from "../common";
import TestCallData from '../artifacts/contracts/TestCall.sol/TestCall.json';
import type { TestCall } from "../typechain-types";

const FIRST_ASSET_ID = 1124;

describe("TxFeePot fees accruel", () => {
	let api: ApiPromise;
	let alice: KeyringPair;
	let provider: JsonRpcProvider;
	let aliceSigner: Wallet;
	let test: TestCall;
	let xrpInitialIssuance: number;
	let accruedFees: number;

	before(async () => {
		// Substrate variables
		const wsProvider = new WsProvider(`ws://localhost:9944`);
		api = await ApiPromise.create({
			provider: wsProvider,
			types: typedefs,
		});
		const keyring = new Keyring({ type: "ethereum" });
		alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));

		// create asset
		await new Promise<void>((resolve) => {
			api.tx.assetsExt.createAsset().signAndSend(alice, ({ status }) => {
				if (status.isInBlock) resolve();
			});
		});

		// EVM variables
		provider = new JsonRpcProvider(`http://localhost:9933`);
		aliceSigner = new Wallet(ALICE_PRIVATE_KEY).connect(provider);

		xrpInitialIssuance = +(await api.query.balances.totalIssuance()).toString();
		accruedFees = +(await api.query.txFeePot.eraTxFees()).toString();
	});

	it("Contract creation transaction accrues base fee in TxFeePot", async () => {
		const fees = await provider.getFeeData();

		const factory = new ethers.ContractFactory(TestCallData.abi, TestCallData.bytecode, aliceSigner);
		const estimatedGas = await provider.estimateGas(factory.getDeployTransaction());

		test = await factory.connect(aliceSigner).deploy({
			gasLimit: estimatedGas,
			maxFeePerGas: fees.lastBaseFeePerGas!,
			maxPriorityFeePerGas: 0,
		}) as TestCall;
		const receipt = await test.deployTransaction.wait();
		console.log("TestCall deployed to:", test.address);

		const feesFromContractDeployment = receipt.effectiveGasPrice
			?.mul(estimatedGas)
			.div(10 ** 12)
			.toNumber();
		const currentAccruedFees = +(
			await api.query.txFeePot.eraTxFees()
		).toString();
		expect(currentAccruedFees - accruedFees)
			.to.be.greaterThanOrEqual(feesFromContractDeployment)
			.and.lessThanOrEqual(feesFromContractDeployment + 1); // account for rounding errors

		accruedFees = currentAccruedFees;
	});

	it("Contract call transaction accrues base fee in TxFeePot", async () => {
		const fees = await provider.getFeeData();

		const gasEstimate = await test.estimateGas.set(1, { maxFeePerGas: fees.lastBaseFeePerGas!, maxPriorityFeePerGas: 0 });
		const tx = await test.set(1, { gasLimit: gasEstimate, maxFeePerGas: fees.lastBaseFeePerGas!, maxPriorityFeePerGas: 0 });
		const receipt = await tx.wait();

		const feesFromContractCall = receipt.effectiveGasPrice?.mul(gasEstimate).div(10 ** 12).toNumber();
		const currentAccruedFees = +(
			await api.query.txFeePot.eraTxFees()
		).toString();
		expect(currentAccruedFees - accruedFees)
			.to.be.greaterThanOrEqual(feesFromContractCall)
			.and.lessThanOrEqual(feesFromContractCall + 1); // account for rounding errors

		accruedFees = currentAccruedFees;
	});

	it("Extrinsic transactions accrue base fee in TxFeePot", async () => {
		const tx = api.tx.assets.mint(
			// mint 1M tokens (18 decimals) to alice
			FIRST_ASSET_ID,
			alice.address,
			utils.parseEther("1").toString(),
		);
		await new Promise<void>((resolve) => {
			tx.signAndSend(alice, ({ status }) => {
				if (status.isInBlock) resolve();
			});
		});

		const feesFromExtrinsicLower = 320_000, feesFromExtrinsicUpper = 330_000;
		const currentAccruedFees = +(
			await api.query.txFeePot.eraTxFees()
		).toString();
		expect(currentAccruedFees - accruedFees).to.be.greaterThan(feesFromExtrinsicLower).and.lessThan(feesFromExtrinsicUpper);

		accruedFees = currentAccruedFees;
	});

	it("Pre-compile contract transaction accrues base fee in TxFeePot", async () => {
		const fees = await provider.getFeeData();

		const erc20Token = FIRST_ASSET_ID.toString(16).padStart(8, "0");
		const erc20TokenAddress = web3.utils.toChecksumAddress(
			`0xCCCCCCCC${erc20Token}000000000000000000000000`
		);
		const erc20 = new Contract(erc20TokenAddress, ERC20_ABI, aliceSigner);
		const gasEstimate = await erc20.estimateGas.transfer("0x000000000000000000000000000000000000DEAD", 1,
			{ maxFeePerGas: fees.lastBaseFeePerGas! }
		);
		const tx = await erc20.transfer(
			"0x000000000000000000000000000000000000DEAD",
			1,
			{ gasLimit: gasEstimate, maxFeePerGas: fees.lastBaseFeePerGas! }
		);
		const receipt = await tx.wait();

		const feesFromPrecompile = receipt.effectiveGasPrice?.mul(gasEstimate).div(10 ** 12).toNumber();
		const currentAccruedFees = +(
			await api.query.txFeePot.eraTxFees()
		).toString();
		expect(currentAccruedFees - accruedFees)
			.to.be.greaterThanOrEqual(feesFromPrecompile)
			.and.lessThanOrEqual(feesFromPrecompile + 1); // account for rounding errors

		accruedFees = currentAccruedFees;
	});

	it('XRP total issuance remains unchanged', async () => {
		const totalIssuance = +(await api.query.balances.totalIssuance()).toString();
		expect(totalIssuance).to.equal(xrpInitialIssuance);
	});
});
