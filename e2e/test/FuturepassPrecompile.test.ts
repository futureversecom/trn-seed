import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { Vec } from "@polkadot/types";
import { hexToU8a } from "@polkadot/util";
import { defaults } from "axios";
import { expect } from "chai";
import { Contract, Wallet } from "ethers";
import { ethers } from "hardhat";
import { address } from "hardhat/internal/core/config/config-validation";
import web3 from "web3";

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

// NOTE(surangap): Each test(it) is independent from each other. If you run the tests against docker, it will spawn new
// container before each test. If you are running against a local service, make sure to reset/restart the service before
// each test
describe("Futurepass Precompile", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let alithSigner: Wallet;
  let bobSigner: Wallet;
  let otherSigner: Wallet;
  let futurpassProxy: Contract;
  // Setup api instance
  beforeEach(async () => {
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
    otherSigner = new Wallet("0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcd").connect(provider);

    futurpassProxy = new Contract(FUTUREPASS_PRECOMPILE_ADDRESS, FUTUREPASS_PRECOMPILE_ABI, bobSigner);
  });

  afterEach(async () => await node.stop());

  it("create futurepass success", async () => {
    const owner = alithSigner.address;
    const createTx = await futurpassProxy.connect(bobSigner).create(owner);
    const receipt = await createTx.wait();

    expect((receipt?.events as any)[0].event).to.equal("FuturepassCreated");
    expect((receipt?.events as any)[0].args.owner).to.equal(alithSigner.address);
  });

  it("create futurepass fail - already existing account", async () => {
    const owner = bobSigner.address;
    {
      const createTx = await futurpassProxy.connect(bobSigner).create(owner);
      const receipt = await createTx.wait();

      expect((receipt?.events as any)[0].event).to.equal("FuturepassCreated");
      expect((receipt?.events as any)[0].args.owner).to.equal(bobSigner.address);
    }
    //try to create an FP for bob again
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
    {
      // transfer some funds to futurepass from bob
      expect(await api.rpc.eth.getBalance(futurepass)).to.equal(0);
      const xrpTokenAddress = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000");
      const xrpToken = new Contract(xrpTokenAddress, ERC20_ABI, bobSigner);
      const tfrTx = await xrpToken.transfer(futurepass, 1000000);
      const receipt = await tfrTx.wait();
      expect(await api.rpc.eth.getBalance(futurepass)).to.equal(1000000n * 1000000000000n);
    }
    {
      const recipientAddress = otherSigner.address;
      // alith is bob's FP's. Hence should be able to transfer the balance out from the futurepass.
      // send 500000 back to recipientAddress(8B9f1582D367dDBB5b2E736671db253F0b602DDa)
      const callData =
        "0xa9059cbb0000000000000000000000008B9f1582D367dDBB5b2E736671db253F0b602DDa000000000000000000000000000000000000000000000000000000000007a120";
      const xrpTokenAddress = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000");
      const tfrTx = await futurpassProxy.connect(alithSigner).proxyCall(futurepass, xrpTokenAddress, callData);
      const receipt = await tfrTx.wait();
      expect(await api.rpc.eth.getBalance(futurepass)).to.equal(500000n * 1000000000000n);
      expect(await api.rpc.eth.getBalance(recipientAddress)).to.equal(500000n * 1000000000000n);
    }
  });

  it("test call", async () => {
    const recipient = otherSigner.address;
    console.log(recipient);
  });
});
