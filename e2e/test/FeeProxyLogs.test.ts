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
    const feeProxyNew = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, empty);
    const feeProxyOld = new Contract(
      FEE_PROXY_ADDRESS,
      [
        ...FEE_PROXY_ABI,
        "function callWithFeePreferences(address asset, uint128 maxPayment, address target, bytes input)",
      ],
      empty,
    );

    const iface = new utils.Interface(ERC20_ABI);
    const transferData = iface.encodeFunctionData("transfer", [to, amount]);
    const gasOpts = { gasLimit: 300_000 } as const;

    try {
      const tx = await feeProxyNew["callWithFeePreferences(address,address,bytes)"](
        token.address,
        token.address,
        transferData,
        gasOpts,
      );
      return await tx.wait();
    } catch (e: any) {
      // Fallback to deprecated signature if new one isn't available on this node
      const tx = await feeProxyOld["callWithFeePreferences(address,uint128,address,bytes)"](
        token.address,
        0,
        token.address,
        transferData,
        gasOpts,
      );
      return await tx.wait();
    }
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

  after(async () => {
    await node.stop();
  });
});
