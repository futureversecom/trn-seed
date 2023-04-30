import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { expect } from "chai";
import { Contract, Wallet } from "ethers";
import { ethers } from "hardhat";

import {
  ALITH_PRIVATE_KEY,
  ERC20_ABI,
  FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
  FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
  GAS_TOKEN_ID,
  NodeProcess,
  startNode,
  typedefs,
} from "../common";

describe("Futurepass Precompile", function () {
  let node: NodeProcess;

  let provider: JsonRpcProvider;
  let alithSigner: Wallet;
  let futurepassRegistrarProxy: Contract;

  beforeEach(async () => {
    node = await startNode();

    // Substrate variables
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
    });

    // Ethereum variables
    provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
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
  });

  // This testcase is included in futurepass substrate tests
  it.skip("create futurepass succeeds for account with no balance", async () => {
    const owner = Wallet.createRandom().address;
    const tx = await futurepassRegistrarProxy.connect(alithSigner).create(owner);
    const receipt = await tx.wait();

    expect((receipt?.events as any)[0].event).to.equal("FuturepassCreated");
    expect((receipt?.events as any)[0].args.owner).to.equal(owner);

    expect(await futurepassRegistrarProxy.futurepassOf(owner)).to.equal((receipt?.events as any)[0].args.futurepass);
  });

  // This testcase is included in futurepass substrate tests
  it.skip("create futurepass fails - already existing account", async () => {
    const owner = Wallet.createRandom().address;
    const tx = await futurepassRegistrarProxy.connect(alithSigner).create(owner);
    await tx.wait();

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
      "0x0000000000000000000000000000000000000000",
    );
  });
});

function fundAccount(
  api: ApiPromise,
  keyring: KeyringPair,
  address: string,
  amount: string | number = 10_000_000,
): Promise<void> {
  return new Promise<void>((resolve) => {
    api.tx.utility
      .batch([
        api.tx.assets.transfer(GAS_TOKEN_ID, address, amount), // 10 XRP
        api.tx.balances.transfer(address, amount), // 10 XRP
      ])
      .signAndSend(keyring, ({ status }) => {
        if (status.isInBlock) resolve();
      });
  });
}

async function fundEOA(signer: Wallet, address: string, value: string = "10000") {
  const tx = await signer.sendTransaction({ to: address, value: ethers.utils.parseEther(value) });
  await tx.wait();
}
