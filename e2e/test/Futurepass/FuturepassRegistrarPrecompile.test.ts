import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { expect } from "chai";
import { Contract, Wallet } from "ethers";
import { ethers } from "hardhat";

import {
  ALITH_PRIVATE_KEY,
  FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
  FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
  NodeProcess,
  startNode,
  typedefs,
} from "../../common";

describe("Futurepass Precompile", function () {
  let node: NodeProcess;

  let provider: JsonRpcProvider;
  let alithSigner: Wallet;
  let futurepassRegistrarProxy: Contract;

  beforeEach(async () => {
    node = await startNode();

    // Substrate variables
    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
    });

    // Ethereum variables
    provider = new JsonRpcProvider(`http://127.0.0.1:${node.rpcPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider);

    futurepassRegistrarProxy = new Contract(
      FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
      FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
      alithSigner,
    );
  });

  afterEach(async () => await node.stop());

  it("create futurepass succeeds for account with balance", async () => {
    const owner = Wallet.createRandom().address;

    // fund owner to pay for futurepass creation
    await fundEOA(alithSigner, owner);

    const tx = await futurepassRegistrarProxy.connect(alithSigner).create(owner);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("FuturepassCreated");
    expect((receipt?.events as any)[0].args.owner).to.equal(owner);

    expect(await futurepassRegistrarProxy.futurepassOf(owner)).to.equal((receipt?.events as any)[0].args.futurepass);

    // should fail upon creation of FP for same owner again
    await futurepassRegistrarProxy
      .connect(alithSigner)
      .create(owner)
      .catch((err: any) => {
        expect(err.message).contains("AccountAlreadyRegistered");
      });
  });

  it("futurepassOf works", async () => {
    const owner = Wallet.createRandom().address;

    // fund owner to pay for futurepass creation
    await fundEOA(alithSigner, owner);

    const tx = await futurepassRegistrarProxy.connect(alithSigner).create(owner);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("FuturepassCreated");
    expect((receipt?.events as any)[0].args.owner).to.equal(owner);

    // check futurepassOf(owner)
    expect(await futurepassRegistrarProxy.futurepassOf(owner)).to.equal((receipt?.events as any)[0].args.futurepass);
    // check futurepassOf of a random address. shoud return 0 address
    expect(await futurepassRegistrarProxy.futurepassOf(Wallet.createRandom().address)).to.equal(
      ethers.constants.AddressZero,
    );
  });
});

async function fundEOA(signer: Wallet, address: string, value: string = "10000") {
  const tx = await signer.sendTransaction({ to: address, value: ethers.utils.parseEther(value) });
  await tx.wait();
}
