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

// Validates that logs emitted by an EVM call executed via FeeProxy are available via eth_getLogs.
// Keeps it minimal and reuses existing helpers.
describe("FeeProxy EVM logs are canonicalized", function () {
  let node: NodeProcess;
  let api: ApiPromise;
  let provider: JsonRpcProvider;
  let alith: any;
  let empty: Wallet;
  let feeTokenAssetId: number;

  before(async () => {
    node = await startNode();
    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs, rpc: rpcs });
    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    provider = new JsonRpcProvider(`http://127.0.0.1:${node.rpcPort}`);
    empty = Wallet.createRandom().connect(provider);

    // Create fee token and liquidity so FeeProxy can pay fees
    feeTokenAssetId = await getNextAssetId(api);
    await finalizeTx(
      alith,
      api.tx.utility.batch([
        api.tx.assetsExt.createAsset("test", "TEST", 18, 1, alith.address),
        api.tx.assets.mint(feeTokenAssetId, alith.address, 1_000_000_000_000n),
        api.tx.assets.mint(feeTokenAssetId, empty.address, 1_000_000_000_000n),
        api.tx.dex.addLiquidity(
          feeTokenAssetId,
          GAS_TOKEN_ID,
          100_000_000_000n,
          100_000_000_000n,
          100_000_000_000n,
          100_000_000_000n,
          null,
          null,
        ),
      ])
    );
  });

  after(async () => {
    await node.stop();
  });

  it("eth_getLogs includes logs from FeeProxy-origin call", async () => {
    const tokenAddr = assetIdToERC20ContractAddress(feeTokenAssetId);
    const token = new Contract(tokenAddr, ERC20_ABI, empty);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, empty);

    // Build ERC20 transfer calldata which will emit a Transfer(address,address,uint256) log
    const iface = new utils.Interface(ERC20_ABI);
    const transferData = iface.encodeFunctionData("transfer", [empty.address, 1]);

    // Execute via FeeProxy
    const tx = await feeProxy.callWithFeePreferences(token.address, token.address, transferData);
    const receipt = await tx.wait();
    expect(receipt.status).to.eq(1);
    const blockNumber = receipt.blockNumber;

    // Query eth_getLogs in the block where the tx was included
    const logs = await provider.getLogs({ fromBlock: blockNumber, toBlock: blockNumber });
    expect(logs.length).to.be.greaterThan(0);

    // Verify at least one log is from the token and has the Transfer topic
    const transferTopic = utils.id("Transfer(address,address,uint256)");
    const matched = logs.find(
      (l) => l.address.toLowerCase() === token.address.toLowerCase() && l.topics[0] === transferTopic
    );
    expect(matched, "missing canonicalized Transfer log").to.not.be.undefined;
  });

  it("orders logs and logIndex across multiple FeeProxy calls in the same block", async () => {
    const tokenAddr = assetIdToERC20ContractAddress(feeTokenAssetId);
    const token = new Contract(tokenAddr, ERC20_ABI, empty);
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, empty);

    const iface = new utils.Interface(ERC20_ABI);
    const transferData1 = iface.encodeFunctionData("transfer", [empty.address, 1]);
    const transferData2 = iface.encodeFunctionData("transfer", [empty.address, 2]);

    // Fire two FeeProxy calls quickly so they land in the same block.
    const tx1Promise = feeProxy.callWithFeePreferences(token.address, token.address, transferData1);
    const tx2Promise = feeProxy.callWithFeePreferences(token.address, token.address, transferData2);

    const [tx1, tx2] = await Promise.all([tx1Promise, tx2Promise]);
    const [r1, r2] = await Promise.all([tx1.wait(), tx2.wait()]);
    expect(r1.status).to.eq(1);
    expect(r2.status).to.eq(1);

    // Expect both in the same block; if not, test is inconclusive but should still assert ordering separately
    // If they differ, skip strict same-block assertions but still verify ordering when possible
    const b1 = r1.blockNumber;
    const b2 = r2.blockNumber;

    const fromBlock = Math.min(b1, b2);
    const toBlock = Math.max(b1, b2);

    // Fetch logs for the token over the relevant block range
    const transferTopic = utils.id("Transfer(address,address,uint256)");
    const allLogs = await provider.getLogs({ address: token.address, topics: [transferTopic], fromBlock, toBlock });

    // Find the two logs corresponding to the two tx hashes
    const l1 = allLogs.find((l) => l.transactionHash.toLowerCase() === r1.transactionHash.toLowerCase());
    const l2 = allLogs.find((l) => l.transactionHash.toLowerCase() === r2.transactionHash.toLowerCase());

    expect(l1, "missing first transfer log").to.not.be.undefined;
    expect(l2, "missing second transfer log").to.not.be.undefined;

    // Compare transactionIndex and logIndex ordering
    // In Ethereum semantics, logs are ordered by transactionIndex, then by index within the receipt.
    // So the log with lower transactionIndex must have lower or equal logIndex; since each tx emits one log here, strictly lower.
    const lowerTx = r1.transactionIndex! < r2.transactionIndex! ? r1 : r2;
    const higherTx = lowerTx.transactionHash === r1.transactionHash ? r2 : r1;
    const lowerLog = lowerTx.transactionHash === l1!.transactionHash ? l1! : l2!;
    const higherLog = higherTx.transactionHash === l1!.transactionHash ? l1! : l2!;

    expect(lowerLog.transactionIndex).to.be.at.most(higherLog.transactionIndex);
    expect(lowerLog.logIndex).to.be.lessThan(higherLog.logIndex);

    // If both in the same block, assert the block-level ordering properties strictly
    if (b1 === b2) {
      expect(lowerLog.blockNumber).to.eq(higherLog.blockNumber);
      // Ensure there are no inversions in address-scoped logs ordering
      const tokenLogsInBlock = allLogs.filter((l) => l.blockNumber === b1);
      const idxLower = tokenLogsInBlock.findIndex((l) => l.transactionHash === lowerTx.transactionHash);
      const idxHigher = tokenLogsInBlock.findIndex((l) => l.transactionHash === higherTx.transactionHash);
      expect(idxLower).to.be.lessThan(idxHigher);
    }
  });
});
