import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Contract, Wallet } from "ethers";
import { ethers } from "hardhat";
import web3 from "web3";

import { ALICE_PRIVATE_KEY, ERC20_ABI, typedefs } from "../common";
import type { TestCall } from "../typechain-types";

const FIRST_ASSET_ID = 1124;

// Note: Tests must be run in order, synchronously
describe("TxFeePot fees accruel", () => {
	let api: ApiPromise;
	let alice: KeyringPair;
	let aliceSigner: Wallet;
	let test: TestCall;
	let accruedFees: number = 0;

	before(async () => {
		// Substrate variables
		const wsProvider = new WsProvider(`ws://localhost:9944`);
		api = await ApiPromise.create({
			provider: wsProvider,
			types: typedefs,
		});
		const keyring = new Keyring({ type: "ethereum" });
		alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));

		// EVM variables
		const provider = new JsonRpcProvider(`http://localhost:9933`);
		aliceSigner = new Wallet(ALICE_PRIVATE_KEY).connect(provider);
	});

	it("TxFeePot has 0 accrued fees", async () => {
		const currentAccruedFees = +(
			await api.query.txFeePot.eraTxFees()
		).toString();
		expect(currentAccruedFees).to.equal(0);

		accruedFees = currentAccruedFees;
	});

	it("Contract creation transaction accrues base fee in TxFeePot", async () => {
		const TestFactory = await ethers.getContractFactory("TestCall");
		test = await TestFactory.connect(aliceSigner).deploy();
		await test.deployed();
		console.log("TestCall deployed to:", test.address);

		const feesFromCreateContract = 6_472_828;
		const currentAccruedFees = +(
			await api.query.txFeePot.eraTxFees()
		).toString();
		expect(currentAccruedFees - accruedFees).to.equal(feesFromCreateContract);

		accruedFees = currentAccruedFees;
	});

	it("Contract call transaction accrues base fee in TxFeePot", async () => {
		const tx = await test.set(1);
		await tx.wait();

		const feesFromContractCall = 135_323;
		const currentAccruedFees = +(
			await api.query.txFeePot.eraTxFees()
		).toString();
		expect(currentAccruedFees - accruedFees).to.equal(feesFromContractCall);

		accruedFees = currentAccruedFees;
	});

	it("Extrinsic transactions accrue base fee in TxFeePot", async () => {
		const txs = [
			api.tx.assetsExt.createAsset(), // create asset
			api.tx.assets.mint(
				// mint 1M tokens (18 decimals) to alice
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

		const feesFromExtrinsic = 340_787;
		const currentAccruedFees = +(
			await api.query.txFeePot.eraTxFees()
		).toString();
		expect(currentAccruedFees - accruedFees).to.equal(feesFromExtrinsic);

		accruedFees = currentAccruedFees;
	});

	it("Pre-compile contract transaction accrues base fee in TxFeePot", async () => {
		const erc20Token = FIRST_ASSET_ID.toString(16).padStart(8, "0");
		const erc20TokenAddress = web3.utils.toChecksumAddress(
			`0xCCCCCCCC${erc20Token}000000000000000000000000`
		);
		const erc20 = new Contract(erc20TokenAddress, ERC20_ABI, aliceSigner);
		const tx = await erc20.transfer(
			"0x0000000000000000000000000000000000000000",
			1
		);
		await tx.wait();

		const feesFromPrecompile = 69_662;
		const currentAccruedFees = +(
			await api.query.txFeePot.eraTxFees()
		).toString();
		expect(currentAccruedFees - accruedFees).to.equal(feesFromPrecompile);

		accruedFees = currentAccruedFees;
	});
});
