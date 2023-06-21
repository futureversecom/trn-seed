import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { BigNumber, Contract, Wallet } from "ethers";
import { ethers } from "hardhat";
import web3 from "web3";

import {
  ALITH_PRIVATE_KEY,
  ERC20_ABI,
  FP_DELEGATE_RESERVE,
  FUTUREPASS_PRECOMPILE_ABI,
  FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
  FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
  GAS_TOKEN_ID,
  NodeProcess,
  saveTxCosts,
  startNode,
  txCosts,
  typedefs,
} from "../../common";

const XRP_PRECOMPILE_ADDRESS = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000");

const CALL_TYPE = {
  StaticCall: 0,
  Call: 1,
  DelegateCall: 2,
  Create: 3,
  Create2: 4,
};

const PROXY_TYPE = {
  NoPermission: 0,
  Any: 1,
  NonTransfer: 2,
};

describe("Futurepass Precompile", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let provider: JsonRpcProvider;
  let alithKeyring: KeyringPair;
  let alithSigner: Wallet;
  let futurepassRegistrar: Contract;
  const keyring = new Keyring({ type: "ethereum" });

  const allTxCosts: { [key: string]: txCosts } = {};

  beforeEach(async () => {
    node = await startNode();

    // Substrate variables
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    api = await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
    });

    alithKeyring = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    // Ethereum variables
    provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider);

    futurepassRegistrar = new Contract(
      FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
      FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
      alithSigner,
    );
  });

  afterEach(async () => await node.stop());

  after(async () => {
    saveTxCosts(allTxCosts, "Futurepass/TxCosts.md", "Futurepass Precompiles");
  });

  async function createFuturepass(caller: Wallet, address: string) {
    // fund caller to pay for futurepass creation
    await fundAccount(api, alithKeyring, address);

    const tx = await futurepassRegistrar.connect(caller).create(address);
    const receipt = await tx.wait();

    const futurepass: string = (receipt?.events as any)[0].args.futurepass;
    return new Contract(futurepass, FUTUREPASS_PRECOMPILE_ABI, caller);
  }

  function weiTo6DP(value: BigNumber) {
    let quotient = value.div(1000000000000n);
    const remainder = value.mod(1000000000000n);

    if (remainder.isZero()) {
      return quotient;
    } else {
      return quotient.add(1n);
    }
  }

  it("gas estimate and actual fee tallies", async () => {
    const owner = Wallet.createRandom().connect(provider);
    // fund caller to pay for futurepass creation
    await fundAccount(api, alithKeyring, owner.address);
    const precompileGasEstimate = await futurepassRegistrar.estimateGas.create(owner.address);

    const balanceBefore = await owner.getBalance();
    const tx = await futurepassRegistrar.connect(owner).create(owner.address);
    await tx.wait();
    const balanceAfter = await owner.getBalance();
    const actualCost = balanceBefore.sub(balanceAfter);
    const fees = await provider.getFeeData();

    // Note: we charge the maxFeePerGas * gaslimit upfront and do not refund the extra atm. Hence, users are charged extra
    const calculatedCost = precompileGasEstimate.mul(fees.maxFeePerGas!);
    expect(weiTo6DP(actualCost)).to.equal(weiTo6DP(calculatedCost));
  });

  it("create futurepass tx costs", async () => {
    const owner = Wallet.createRandom().connect(provider);
    // fund caller to pay for futurepass creation
    await fundAccount(api, alithKeyring, owner.address);

    // precompile
    let balanceBefore = await owner.getBalance();
    const tx = await futurepassRegistrar.connect(owner).create(owner.address);
    await tx.wait();
    let balanceAfter = await owner.getBalance();
    const precompileCost = balanceBefore.sub(balanceAfter);

    // extrinsic
    const owner2 = Wallet.createRandom().connect(provider);
    balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.futurepass.create(owner2.address).signAndSend(alithKeyring, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);

    expect(extrinsicCost).to.be.lessThan(precompileCost);
    // Update all costs with allTxCosts
    allTxCosts["create"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("register delegate tx costs", async () => {
    const owner1 = Wallet.createRandom().connect(provider);
    const owner2 = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);
    // create FPs for owner1 and owner2
    const fp1 = await createFuturepass(owner1, owner1.address);
    const fp2 = await createFuturepass(owner2, owner2.address);

    // precompile approach
    let balanceBefore = await owner1.getBalance();
    const tx = await fp1.connect(owner1).registerDelegate(delegate.address, PROXY_TYPE.Any);
    await tx.wait();
    let balanceAfter = await owner1.getBalance();
    // assert delegate is registered
    expect(await fp1.delegateType(delegate.address)).to.equal(PROXY_TYPE.Any);
    const precompileCost = balanceBefore.sub(balanceAfter);

    // extrinsic approach
    const owner2KeyRing = keyring.addFromSeed(hexToU8a(owner2.privateKey));
    balanceBefore = await owner2.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.futurepass
        .registerDelegate(fp2.address, delegate.address, PROXY_TYPE.Any)
        .signAndSend(owner2KeyRing, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    balanceAfter = await owner2.getBalance();
    // assert delegate is registered
    expect(await fp2.delegateType(delegate.address)).to.equal(PROXY_TYPE.Any);

    const extrinsicCost = balanceBefore.sub(balanceAfter);
    expect(extrinsicCost).to.be.lessThan(precompileCost);
    // Update all costs with allTxCosts
    allTxCosts["registerDelegate"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("unregister delegate tx costs", async () => {
    const owner1 = Wallet.createRandom().connect(provider);
    const owner2 = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);
    // create FPs for owner1 and owner2 and register delegate delegate
    const fp1 = await createFuturepass(owner1, owner1.address);
    const fp2 = await createFuturepass(owner2, owner2.address);
    let tx = await fp1.connect(owner1).registerDelegate(delegate.address, PROXY_TYPE.Any);
    await tx.wait();
    tx = await fp2.connect(owner2).registerDelegate(delegate.address, PROXY_TYPE.Any);
    await tx.wait();
    expect(await fp1.delegateType(delegate.address)).to.equal(PROXY_TYPE.Any);
    expect(await fp2.delegateType(delegate.address)).to.equal(PROXY_TYPE.Any);

    // precompile approach
    let balanceBefore = await owner1.getBalance();
    tx = await fp1.connect(owner1).unregisterDelegate(delegate.address);
    await tx.wait();
    let balanceAfter = await owner1.getBalance();
    // assert delegate is registered
    expect(await fp1.delegateType(delegate.address)).to.equal(PROXY_TYPE.NoPermission);
    const precompileCost = balanceBefore.sub(balanceAfter);

    // extrinsic approach
    const owner2KeyRing = keyring.addFromSeed(hexToU8a(owner2.privateKey));
    balanceBefore = await owner2.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.futurepass.unregisterDelegate(fp2.address, delegate.address).signAndSend(owner2KeyRing, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    balanceAfter = await owner2.getBalance();
    // assert delegate is registered
    expect(await fp2.delegateType(delegate.address)).to.equal(PROXY_TYPE.NoPermission);

    const extrinsicCost = balanceBefore.sub(balanceAfter);
    expect(extrinsicCost).to.be.lessThan(precompileCost);
    // Update all costs with allTxCosts
    allTxCosts["unregisterDelegate"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("transferOwnership tx costs", async () => {
    const owner1 = Wallet.createRandom().connect(provider);
    const owner2 = Wallet.createRandom().connect(provider);
    const newOwner1 = Wallet.createRandom().connect(provider);
    const newOwner2 = Wallet.createRandom().connect(provider);
    // create FPs for owner1 and owner2
    const fp1 = await createFuturepass(owner1, owner1.address);
    await createFuturepass(owner2, owner2.address);

    // precompile approach
    let balanceBefore = await owner1.getBalance();
    const tx = await fp1.connect(owner1).transferOwnership(newOwner1.address);
    await tx.wait();
    let balanceAfter = await owner1.getBalance();
    // assert newOwner1 is owner
    // expect(await fp1.owner()).to.equal(newOwner1.address);
    const precompileCost = balanceBefore.sub(balanceAfter);

    // extrinsic approach
    const owner2KeyRing = keyring.addFromSeed(hexToU8a(owner2.privateKey));
    balanceBefore = await owner2.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.futurepass.transferFuturepass(newOwner2.address).signAndSend(owner2KeyRing, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    balanceAfter = await owner2.getBalance();
    // assert newOwner2 is owner
    // expect(await fp2.owner()).to.equal(newOwner2.address);

    const extrinsicCost = balanceBefore.sub(balanceAfter);
    expect(extrinsicCost).to.be.lessThan(precompileCost);
    // Update all costs with allTxCosts
    allTxCosts["transferOwnership"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("proxy transfer from futurepass tx costs", async () => {
    const owner1 = Wallet.createRandom().connect(provider);
    const owner2 = Wallet.createRandom().connect(provider);
    const recipient = Wallet.createRandom().connect(provider);
    // create FPs for owner1 and owner2
    const fp1 = await createFuturepass(owner1, owner1.address);
    const fp2 = await createFuturepass(owner2, owner2.address);

    const transferAmount = 5; // 1 XRP
    // fund futurepasses
    await fundAccount(api, alithKeyring, fp1.address);
    await fundAccount(api, alithKeyring, fp2.address);

    // precompile approach
    let balanceBefore = await owner1.getBalance();
    const tx = await fp1.connect(owner1).proxyCall(CALL_TYPE.Call, recipient.address, parseEther(transferAmount), "0x");
    await tx.wait();
    let balanceAfter = await owner1.getBalance();
    const precompileCost = balanceBefore.sub(balanceAfter);
    let recipientBalance = await provider.getBalance(recipient.address);
    expect(recipientBalance).to.equal(parseEther(transferAmount));

    // extrinsic approach
    const owner2KeyRing = keyring.addFromSeed(hexToU8a(owner2.privateKey));
    balanceBefore = await provider.getBalance(fp2.address); // futurepass pays the tx fee for extrinsic approach
    await new Promise<void>((resolve) => {
      api.tx.futurepass
        .proxyExtrinsic(fp2.address, api.tx.assets.transfer(2, recipient.address, transferAmount * 1000000))
        .signAndSend(owner2KeyRing, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    balanceAfter = await provider.getBalance(fp2.address);
    recipientBalance = await provider.getBalance(recipient.address);
    expect(recipientBalance).to.equal(parseEther(2 * transferAmount));

    const extrinsicCost = balanceBefore.sub(balanceAfter).sub(parseEther(transferAmount));
    expect(extrinsicCost).to.be.lessThan(precompileCost);
    // Update all costs with allTxCosts
    allTxCosts["proxyCall"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });
});

async function fundAccount(
  api: ApiPromise,
  keyring: KeyringPair,
  address: string,
  amount: string | number = 10_000_000,
): Promise<void> {
  return new Promise<void>((resolve) => {
    api.tx.utility
      .batch([
        api.tx.assets.transfer(GAS_TOKEN_ID, address, amount), // 10 XRP
        api.tx.balances.transfer(address, amount), // 10 ROOT
      ])
      .signAndSend(keyring, ({ status }) => {
        if (status.isInBlock) resolve();
      });
  });
}

function parseEther(amount: number): BigNumber {
  return ethers.utils.parseEther(amount.toString());
}
