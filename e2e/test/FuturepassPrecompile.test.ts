import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { expect } from "chai";
import { Contract, Wallet } from "ethers";
import { ethers } from "hardhat";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  FUTUREPASS_PRECOMPILE_ABI,
  FUTUREPASS_PRECOMPILE_ADDRESS,
  NodeProcess,
  startNode,
  typedefs,
} from "../common";

describe("Futurepass Precompile", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let alithSigner: Wallet;
  let bobSigner: Wallet;
  let futurpassProxy: Contract;
  // Setup api instance
  before(async () => {
    node = await startNode();

    // Substrate variables
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    api = await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
    });

    // Ethereum variables
    const provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);

    futurpassProxy = new Contract(FUTUREPASS_PRECOMPILE_ADDRESS, FUTUREPASS_PRECOMPILE_ABI, bobSigner);
  });

  after(async () => await node.stop());

  it("create futurepass success", async () => {
    const owner = alithSigner.address;

    const createTx = await futurpassProxy
      .connect(bobSigner)
      .create(owner);
    const receipt = await createTx.wait();

    console.log(JSON.stringify(receipt))

    expect((receipt?.events as any)[0].event).to.equal("FuturepassCreated");
    expect((receipt?.events as any)[0].args.owner).to.equal(alithSigner.address);
  });
});
