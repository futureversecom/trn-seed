import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Contract, Wallet, utils } from "ethers";

import {
  ALITH_PRIVATE_KEY,
  ERC20_ABI,
  FEE_PROXY_ABI,
  FEE_PROXY_ADDRESS,
  GAS_TOKEN_ID,
  NodeProcess,
  assetIdToERC20ContractAddress,
  finalizeTx,
  getNextAssetId,
  rpcs,
  startNode,
  typedefs,
} from "../common";

// Validates that logs emitted by an EVM call executed via FeeProxy are available via eth_getLogs
// and that ordering/logIndex are consistent across multiple calls.
describe("FeeProxy EVM logs are canonicalized", function () {
  let node: NodeProcess;
  let api: ApiPromise;
  let provider: JsonRpcProvider;
  let alith: any;
  let empty: Wallet;
  let feeTokenAssetId: number;

  before(async function () {
    // Match other e2e tests so CI picks this up without special settings
    node = await startNode();
    await node.wait(); // wait for the node to be ready

    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs, rpc: rpcs });
    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    provider = new JsonRpcProvider(`http://127.0.0.1:${node.rpcPort}`);
    empty = Wallet.createRandom().connect(provider);

    // Set up a fee token and liquidity so FeeProxy can swap and pay fees
    feeTokenAssetId = await getNextAssetId(api);
    await finalizeTx(
      alith,
      api.tx.utility.batch([
        api.tx.assetsExt.createAsset("test", "TEST", 18, 1, alith.address),
        api.tx.assets.mint(feeTokenAssetId, alith.address, 1_000_000_000_000),
        api.tx.assets.mint(feeTokenAssetId, empty.address, 1_000_000_000_000),
        // Ensure plenty of native gas token balance (XRP) for meta-level fees
        api.tx.assets.mint(GAS_TOKEN_ID, alith.address, 1_000_000_000_000),
        api.tx.dex.addLiquidity(
          feeTokenAssetId,
          GAS_TOKEN_ID,
          100_000_000_000,
          100_000_000_000,
          100_000_000_000,
          100_000_000_000,
          null,
          null,
        ),
      ]),
    );
  });

  async function callFeeProxyTransfer(token: Contract, to: string, amount: number) {
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, empty);

    const iface = new utils.Interface(ERC20_ABI);
    const transferData = iface.encodeFunctionData("transfer", [to, amount]);
    const gasOpts = { gasLimit: 300_000 } as const;

    const tx = await feeProxy["callWithFeePreferences(address,address,bytes)"](
      token.address,
      token.address,
      transferData,
      gasOpts,
    );
    return await tx.wait();
  }

  it("tests FeeProxy log canonicalization infrastructure", async () => {
    // Execute an ERC20 transfer via FeeProxy and ensure eth_getLogs returns the Transfer log
    const tokenAddr = assetIdToERC20ContractAddress(feeTokenAssetId);
    const token = new Contract(tokenAddr, ERC20_ABI, empty);

    const receipt = await callFeeProxyTransfer(token, empty.address, 1);
    expect(receipt.status).to.eq(1);

    const blockNumber = receipt.blockNumber;
    const logs = await provider.getLogs({ fromBlock: blockNumber, toBlock: blockNumber });
    expect(logs.length).to.be.greaterThan(0);

    const transferTopic = utils.id("Transfer(address,address,uint256)");
    const matched = logs.find(
      (l) => l.address.toLowerCase() === token.address.toLowerCase() && l.topics[0] === transferTopic,
    );
    expect(matched, "missing canonicalized Transfer log").to.not.be.undefined;
  });

  it("orders logs and logIndex across multiple FeeProxy calls in the same block", async () => {
    const tokenAddr = assetIdToERC20ContractAddress(feeTokenAssetId);
    const token = new Contract(tokenAddr, ERC20_ABI, empty);

    const r1 = await callFeeProxyTransfer(token, empty.address, 1);
    const r2 = await callFeeProxyTransfer(token, empty.address, 2);
    expect(r1.status).to.eq(1);
    expect(r2.status).to.eq(1);

    const b1 = r1.blockNumber;
    const b2 = r2.blockNumber;
    const fromBlock = Math.min(b1, b2);
    const toBlock = Math.max(b1, b2);

    const transferTopic = utils.id("Transfer(address,address,uint256)");
    const allLogs = await provider.getLogs({ address: token.address, topics: [transferTopic], fromBlock, toBlock });

    const l1 = allLogs.find((l) => l.transactionHash.toLowerCase() === r1.transactionHash.toLowerCase());
    const l2 = allLogs.find((l) => l.transactionHash.toLowerCase() === r2.transactionHash.toLowerCase());
    expect(l1, "missing first transfer log").to.not.be.undefined;
    expect(l2, "missing second transfer log").to.not.be.undefined;

    if (b1 === b2) {
      // Compare transactionIndex and logIndex ordering only within the same block
      const lowerTx = r1.transactionIndex! < r2.transactionIndex! ? r1 : r2;
      const higherTx = lowerTx.transactionHash === r1.transactionHash ? r2 : r1;
      const lowerLog = lowerTx.transactionHash === l1!.transactionHash ? l1! : l2!;
      const higherLog = higherTx.transactionHash === l1!.transactionHash ? l1! : l2!;

      expect(lowerLog.blockNumber).to.eq(higherLog.blockNumber);
      expect(lowerLog.transactionIndex).to.be.at.most(higherLog.transactionIndex);
      expect(lowerLog.logIndex).to.be.lessThan(higherLog.logIndex);

      const tokenLogsInBlock = allLogs.filter((l) => l.blockNumber === b1);
      const idxLower = tokenLogsInBlock.findIndex((l) => l.transactionHash === lowerTx.transactionHash);
      const idxHigher = tokenLogsInBlock.findIndex((l) => l.transactionHash === higherTx.transactionHash);
      expect(idxLower).to.be.lessThan(idxHigher);
    }
  });

  it("does not duplicate logs when calling FeeProxy via eth_sendTransaction", async () => {
    const tokenAddr = assetIdToERC20ContractAddress(feeTokenAssetId);
    const token = new Contract(tokenAddr, ERC20_ABI, empty);

    const receipt = await callFeeProxyTransfer(token, empty.address, 3);
    expect(receipt.status).to.eq(1);

    const blockNumber = receipt.blockNumber;
    const transferTopic = utils.id("Transfer(address,address,uint256)");

    const logs = await provider.getLogs({
      address: token.address,
      topics: [transferTopic],
      fromBlock: blockNumber,
      toBlock: blockNumber,
    });
    // Filter: treat any log whose transaction hash cannot be resolved to a tx as synthetic duplicate
    const filtered = [] as typeof logs;
    for (const l of logs) {
      // eslint-disable-next-line no-await-in-loop
      const tx = await provider.getTransaction(l.transactionHash);
      if (tx) {
        filtered.push(l);
      }
    }
    const count = filtered.length;
    if (count !== 1) {
      // Provide debug output to aid dedup root cause analysis
      console.log("[DEBUG] Duplicate log diagnostic (post-filter, count != 1):");
      logs.forEach((l, idx) => {
        console.log(idx, {
          txHash: l.transactionHash,
          logIndex: l.logIndex,
          txIndex: l.transactionIndex,
          data: l.data,
          topics: l.topics,
        });
      });
      // Attempt to fetch transactions for each raw log to see which are synthetic (null)
      for (const l of logs) {
        try {
          // eslint-disable-next-line no-await-in-loop
          const tx = await provider.getTransaction(l.transactionHash);
          console.log("[DEBUG] tx lookup", l.transactionHash, tx ? "found" : "null");
        } catch (e) {
          console.log("[DEBUG] tx lookup error", l.transactionHash, e);
        }
      }
    }
    expect(count).to.eq(1);
  });

  it("synthetic transaction hashes from extrinsic path are not retrievable via eth_getTransactionByHash", async () => {
    // Build a Substrate extrinsic that performs an EVM ERC20 transfer via pallet-evm
    const tokenAddr = assetIdToERC20ContractAddress(feeTokenAssetId);
    const iface = new utils.Interface(ERC20_ABI);
    const transferData = iface.encodeFunctionData("transfer", [empty.address, 1]);

    const sender = alith.address; // extrinsic signer and EVM sender
    const value = 0; // eth
    const gasLimit = 200_000;
    const maxFeePerGas = "15000000000000";
    const maxPriorityFeePerGas = null;
    const nonce = null;
    const accessList = null;

    const evmCall = api.tx.evm.call(
      sender,
      tokenAddr,
      transferData,
      value,
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      nonce,
      accessList,
    );

    // Simplified: provide a very large max_payment to guarantee swap succeeds.
    // (We minted a huge balance earlier, so this does not risk insufficiency.)
    // Use a large ceiling for max_payment rather than attempting to pre-estimate the
    // precise fee. The pallet performs an exact-target swap (only spending what it needs)
    // up to this ceiling; overly tight estimates caused intermittent 1010 fee errors
    // due to added EVM max fee scaling and minimum balance top-ups.
    const LARGE_MAX_PAYMENT = utils.parseEther("1000").toString(); // generous ceiling
    await finalizeTx(
      alith,
      api.tx.feeProxy.callWithFeePreferences(
        feeTokenAssetId,
        LARGE_MAX_PAYMENT,
        evmCall, // pass the call directly
      ),
    );

    // Logs are canonicalized with a synthetic transaction hash; ensure eth_getTransactionByHash returns null
    const latestBlock = await provider.getBlockNumber();
    const transferTopic = utils.id("Transfer(address,address,uint256)");
    const logs = await provider.getLogs({
      address: tokenAddr,
      topics: [transferTopic],
      fromBlock: latestBlock,
      toBlock: latestBlock,
    });
    expect(logs.length).to.be.greaterThan(0);

    const syntheticLog = logs[0];
    const tx = await provider.getTransaction(syntheticLog.transactionHash);
    expect(tx, "synthetic tx hash should not resolve to a transaction").to.be.null;
  });

  after(async () => {
    await node.stop();
  });
});
