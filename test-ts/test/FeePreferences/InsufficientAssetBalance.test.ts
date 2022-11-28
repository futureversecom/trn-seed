// Call an EVM transaction with fee preferences for an account that has zero native token balance, ensuring that the preferred asset with liquidity is spent instead
import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { ChildProcess } from "child_process";
import { Contract, utils, Wallet } from "ethers";

import {
	ALICE_PRIVATE_KEY,
	assetIdToERC20ContractAddress,
	BOB_PRIVATE_KEY,
	ERC20_ABI,
	EVM_PALLET_INDEX,
	executeForPreviousEvent,
	FEE_PROXY_ABI,
	FEE_PROXY_ADDRESS,
	NATIVE_TOKEN_ID,
	sleep,
	startStandaloneNode,
	typedefs,
	WITHDRAW_FAILED_ERROR_INDEX,
} from "../../common";

const EMPTY_ACCT_PRIVATE_KEY =
	"0xf8d74108dbe199c4a6e4ef457046db37c325ba3f709b14cabfa1885663e4c589";
const feeTokenAssetId = 1124;

describe("Fee Preferences in low asset balance scenario", function () {
	let api: ApiPromise;
	let bob: KeyringPair;
	let insufficientAccount: KeyringPair;
	let insufficientAccountSigner: Wallet;
	let feeToken: Contract;
	let aliceNode: ChildProcess;

	// Setup api instance and keyring wallet addresses for alice and bob
	before(async () => {
		aliceNode = startStandaloneNode("alice", { tmp: true, printLogs: false });

		await sleep(10000);
		// Setup providers for jsonRPCs and WS
		const jsonProvider = new JsonRpcProvider(`http://localhost:9933`);
		const wsProvider = new WsProvider(`ws://localhost:9944`);

		api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
		const keyring = new Keyring({ type: "ethereum" });
		const alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));
		bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));

		insufficientAccount = keyring.addFromSeed(hexToU8a(EMPTY_ACCT_PRIVATE_KEY));
		insufficientAccountSigner = new Wallet(EMPTY_ACCT_PRIVATE_KEY).connect(
			jsonProvider
		); // 'development' seed

		feeToken = new Contract(
			assetIdToERC20ContractAddress(feeTokenAssetId),
			ERC20_ABI,
			insufficientAccountSigner
		);

		const txs = [
			api.tx.assetsExt.createAsset(),
			api.tx.assets.mint(feeTokenAssetId, alice.address, 2_000_000_000_000_000),
			api.tx.assets.mint(feeTokenAssetId, insufficientAccount.address, 2_000),
			api.tx.dex.addLiquidity(
				feeTokenAssetId,
				NATIVE_TOKEN_ID,
				100_000_000_000,
				100_000_000_000,
				100_000_000_000,
				100_000_000_000,
				0
			),
		];

		await new Promise<void>((resolve) => {
			api.tx.utility.batch(txs).signAndSend(alice, ({ status }) => {
				if (status.isInBlock) {
					console.log(`setup block hash: ${status.asInBlock}`);
					resolve();
				}
			});
		});
	});

	after(async () => {
		await api?.disconnect();
		aliceNode?.kill("SIGINT");
		await sleep(4000);
	});

	it("Cannot pay fees with non-native, preferred token if low asset balance", async () => {
		const transferAmount = 1;
		let iface = new utils.Interface(ERC20_ABI);
		const transferInput = iface.encodeFunctionData("transfer", [
			bob.address,
			transferAmount,
		]);

		const maxFeePaymentInToken = 10_000_000_000;
		const feeProxy = new Contract(
			FEE_PROXY_ADDRESS,
			FEE_PROXY_ABI,
			insufficientAccountSigner
		);

		const nonce = await insufficientAccountSigner.getTransactionCount();
		const chainId = 3999;
		const maxPriorityFeePerGas = 0; // 1_500_000_000 = '0x59682f00'
		const gasLimit = 23316; // '0x5b14' = 23316;
		const maxFeePerGas = 30_001_500_000_0000; // 30_001_500_000_000 = '0x1b4944c00f00'
		const unsignedTx = {
			// eip1559 tx
			type: 2,
			from: insufficientAccount.address,
			to: FEE_PROXY_ADDRESS,
			nonce,
			data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
				feeToken.address,
				maxFeePaymentInToken,
				feeToken.address,
				transferInput,
			]),
			gasLimit,
			maxFeePerGas,
			maxPriorityFeePerGas,
			chainId,
		};

		try {
			const tx = await insufficientAccountSigner.sendTransaction(unsignedTx);
			await tx.wait();
		} catch (err: any) {
			// See expected behavior for gasLimit === 0 https://github.com/futureversecom/frontier/blob/polkadot-v0.9.27-TRN/ts-tests/tests/test-transaction-cost.ts
			expect(err.code).to.be.eq("INSUFFICIENT_FUNDS");
			expect(err.reason).to.be.eq("insufficient funds for intrinsic transaction cost");
		}
	});
});
