import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { hexToU8a } from "@polkadot/util";
import { KeyringPair } from "@polkadot/keyring/types";
import { expect } from "chai";
import { Contract, Wallet } from "ethers";
import web3 from "web3";

import MockERC20Data from "../artifacts/contracts/MockERC20.sol/MockERC20.json";
import MockERC721Data from "../artifacts/contracts/MockERC721.sol/MockERC721.json";
import MockERC1155Data from "../artifacts/contracts/MockERC1155.sol/MockERC1155.json";
import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC20_ABI,
  FUTUREPASS_PRECOMPILE_ABI,
  FUTUREPASS_PRECOMPILE_ADDRESS,
  NodeProcess,
  startNode,
  typedefs,
} from "../common";
import type { MockERC20, MockERC721, MockERC1155 } from "../typechain-types";

describe("Futurepass Precompile", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let alithKeyring: KeyringPair;
  let alithSigner: Wallet;
  let bobSigner: Wallet;
  let futurpassProxy: Contract;

  beforeEach(async () => {
    node = await startNode();

    // Substrate variables
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    api = await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
    });

    const keyring = new Keyring({ type: "ethereum" });
    alithKeyring = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    // Ethereum variables
    const provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider);
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);

    futurpassProxy = new Contract(FUTUREPASS_PRECOMPILE_ADDRESS, FUTUREPASS_PRECOMPILE_ABI, bobSigner);
  });

  afterEach(async () => await node.stop());

  // TODO: migrate to unit test
  it("create futurepass succeeds for account with balance", async () => {
    const owner = await Wallet.createRandom().getAddress();
    await new Promise<void>((resolve) => {
      api.tx.balances.transfer(owner, 10_000_000).signAndSend(alithKeyring, ({ status }) => { // 10 XRP
        if (status.isInBlock) resolve();
      });
    });
    const tx = await futurpassProxy.connect(bobSigner).create(owner);
    const receipt = await tx.wait();

    expect((receipt?.events as any)[0].event).to.equal("FuturepassCreated");
    expect((receipt?.events as any)[0].args.owner).to.equal(owner);

    expect(await futurpassProxy.futurepassOf(owner)).to.equal((receipt?.events as any)[0].args.futurepass);
  });

  // TODO: migrate to unit test
  // TODO: fix
  it("create futurepass succeeds for account with no balance", async () => {
    const owner = await Wallet.createRandom().getAddress();
    const tx = await futurpassProxy.connect(bobSigner).create(owner);
    const receipt = await tx.wait();

    expect((receipt?.events as any)[0].event).to.equal("FuturepassCreated");
    expect((receipt?.events as any)[0].args.owner).to.equal(owner);

    expect(await futurpassProxy.futurepassOf(owner)).to.equal((receipt?.events as any)[0].args.futurepass);
  });

  it("create futurepass fails - already existing account", async () => {
    const owner = await Wallet.createRandom().getAddress();
    const tx = await futurpassProxy.connect(bobSigner).create(owner);
    await tx.wait();

    // should fail upon creation of FP for same owner again
    await futurpassProxy
      .connect(bobSigner)
      .create(owner)
      .catch((err: any) => {
        expect(err.message).contains("AccountAlreadyRegistered");
      });
  });

  it("register delegate works", async () => {
    const owner = bobSigner.address;
    const delegate = alithSigner.address;
    let futurepass;
    {
      // create FP for bob
      const createTx = await futurpassProxy.connect(bobSigner).create(owner);
      const receipt = await createTx.wait();
      expect((receipt?.events as any)[0].event).to.equal("FuturepassCreated");
      expect((receipt?.events as any)[0].args.owner).to.equal(bobSigner.address);
      futurepass = (receipt?.events as any)[0].args.futurepass;
    }
    {
      //check alice is not a delegate of bob's FP
      expect(await futurpassProxy.isDelegate(futurepass, delegate)).to.equal(false);
      // make alice bob's FP's delegate
      const delegateTx = await futurpassProxy.connect(bobSigner).registerDelegate(futurepass, delegate);
      const receipt = await delegateTx.wait();
      expect(await futurpassProxy.isDelegate(futurepass, delegate)).to.equal(true);
      expect((receipt?.events as any)[0].event).to.equal("FuturepassDelegateRegistered");
      expect((receipt?.events as any)[0].args.futurepass).to.equal(futurepass);
      expect((receipt?.events as any)[0].args.delegate).to.equal(delegate);
    }
    // Try to register the same delegate again - should return error
    await futurpassProxy
      .connect(bobSigner)
      .registerDelegate(futurepass, delegate)
      .catch((err: any) => {
        expect(err.message).contains("DelegateAlreadyExists");
      });
  });

  it("unregister delegate works", async () => {
    const owner = bobSigner.address;
    const delegate = alithSigner.address;
    let futurepass;
    {
      // create FP for bob
      const createTx = await futurpassProxy.connect(bobSigner).create(owner);
      const receipt = await createTx.wait();
      expect((receipt?.events as any)[0].event).to.equal("FuturepassCreated");
      expect((receipt?.events as any)[0].args.owner).to.equal(bobSigner.address);
      futurepass = (receipt?.events as any)[0].args.futurepass;
    }
    {
      // make alice bob's FP's delegate
      const delegateTx = await futurpassProxy.connect(bobSigner).registerDelegate(futurepass, delegate);
      const receipt = await delegateTx.wait();
      expect(await futurpassProxy.isDelegate(futurepass, delegate)).to.equal(true);
      expect((receipt?.events as any)[0].event).to.equal("FuturepassDelegateRegistered");
      expect((receipt?.events as any)[0].args.futurepass).to.equal(futurepass);
      expect((receipt?.events as any)[0].args.delegate).to.equal(delegate);
    }
    // unregister alith from delegate
    const delegateTx = await futurpassProxy.connect(bobSigner).unregisterDelegate(futurepass, delegate);
    const receipt = await delegateTx.wait();
    expect(await futurpassProxy.isDelegate(futurepass, delegate)).to.equal(false);
    expect((receipt?.events as any)[0].event).to.equal("FuturepassDelegateUnregistered");
    expect((receipt?.events as any)[0].args.futurepass).to.equal(futurepass);
    expect((receipt?.events as any)[0].args.delegate).to.equal(delegate);
  });

  it("proxy call works", async () => {
    const owner = await Wallet.createRandom().getAddress();
    const delegate = await Wallet.createRandom().getAddress();

    // create FP for owner
    let tx = await futurpassProxy.connect(bobSigner).create(owner);
    let receipt = await tx.wait();
    const futurepass = (receipt?.events as any)[0].args.futurepass;

    // add delegate for owners futurepass
    tx = await futurpassProxy.connect(owner).registerDelegate(futurepass, delegate);
    await tx.wait();
      
    // transfer some funds to futurepass from bob
    expect(await api.rpc.eth.getBalance(futurepass)).to.equal(0);
    const xrpTokenAddress = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000");
    const xrpToken = new Contract(xrpTokenAddress, ERC20_ABI, bobSigner);
    tx = await xrpToken.transfer(futurepass, 1_000_000);
    await tx.wait();
    expect(await api.rpc.eth.getBalance(futurepass)).to.equal(1000000n * 1000000000000n);

    const recipientAddress = await Wallet.createRandom().getAddress();
    // transfer funds from futurepass to recipient - call initiated via delegate
    const callData =
      "0xa9059cbb0000000000000000000000008B9f1582D367dDBB5b2E736671db253F0b602DDa000000000000000000000000000000000000000000000000000000000007a120";
    tx = await futurpassProxy.connect(delegate).proxyCall(futurepass, xrpTokenAddress, callData);
    await tx.wait();
    expect(await api.rpc.eth.getBalance(futurepass)).to.equal(500000n * 1000000000000n);
    expect(await api.rpc.eth.getBalance(recipientAddress)).to.equal(500000n * 1000000000000n);
  });
});
