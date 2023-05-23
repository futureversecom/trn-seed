import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { BigNumber, Contract, Wallet } from "ethers";
import { ethers } from "hardhat";
import web3 from "web3";

import MockCreateContract from "../../artifacts/contracts/FuturepassIntegrator.sol/CreateTester.json";
import MockCreatePayableContract from "../../artifacts/contracts/FuturepassIntegrator.sol/CreateTesterPayable.json";
import {
  ALITH_PRIVATE_KEY,
  ERC20_ABI,
  FP_DELEGATE_RESERVE,
  FUTUREPASS_PRECOMPILE_ABI,
  FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
  FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
  GAS_TOKEN_ID,
  NodeProcess,
  startNode,
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
  let xrpERC20Precompile: Contract;

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
    provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider);

    futurepassRegistrar = new Contract(
      FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
      FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
      alithSigner,
    );
    xrpERC20Precompile = new Contract(XRP_PRECOMPILE_ADDRESS, ERC20_ABI, alithSigner);
  });

  afterEach(async () => await node.stop());

  async function createFuturepass(caller: Wallet, address: string) {
    // fund caller to pay for futurepass creation
    await fundAccount(api, alithKeyring, address);

    const tx = await futurepassRegistrar.connect(caller).create(address);
    const receipt = await tx.wait();

    const futurepass: string = (receipt?.events as any)[0].args.futurepass;
    return new Contract(futurepass, FUTUREPASS_PRECOMPILE_ABI, caller);
  }

  it("transfer value to futurepass address works", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // transfer value 5 XRP to futurepass
    let value = 5;
    const tx = await alithSigner.sendTransaction({ to: futurepassPrecompile.address, value: parseEther(value) });
    await tx.wait();
    // check if the value is reflected
    expect(await xrpERC20Precompile.balanceOf(futurepassPrecompile.address)).to.equal(value * 1_000_000);

    // transfer value 5 XRP to futurepass
    value = 5;
    await fundAccount(api, alithKeyring, futurepassPrecompile.address, value * 1_000_000);
    // check if the value is reflected
    expect(await xrpERC20Precompile.balanceOf(futurepassPrecompile.address)).to.equal(value * 2 * 1_000_000);
  });

  it("delegateType works", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // checkDelegate should return 0 value(ProxyType.NoPermission)
    expect(await futurepassPrecompile.delegateType(delegate.address)).to.equal(PROXY_TYPE.NoPermission);

    const tx = await futurepassPrecompile.connect(owner).registerDelegate(delegate.address, PROXY_TYPE.Any);
    await tx.wait();

    // checkDelegate should return PROXY_TYPE.Any
    expect(await futurepassPrecompile.delegateType(delegate.address)).to.equal(PROXY_TYPE.Any);
  });

  it("register delegate works", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // ensure delegate doesnt exist for FP
    expect(await futurepassPrecompile.delegateType(delegate.address)).to.equal(PROXY_TYPE.NoPermission);

    // registering with proxytype other than PROXY_TYPE.Any fails
    await futurepassPrecompile
      .connect(owner)
      .registerDelegate(delegate.address, PROXY_TYPE.NonTransfer)
      .catch((err: any) => expect(err.message).contains("PermissionDenied"));

    const tx = await futurepassPrecompile.connect(owner).registerDelegate(delegate.address, PROXY_TYPE.Any);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("FuturepassDelegateRegistered");
    expect((receipt?.events as any)[0].args.futurepass).to.equal(futurepassPrecompile.address);
    expect((receipt?.events as any)[0].args.delegate).to.equal(delegate.address);
    expect((receipt?.events as any)[0].args.proxyType).to.equal(PROXY_TYPE.Any);

    expect(await futurepassPrecompile.delegateType(delegate.address)).to.equal(PROXY_TYPE.Any);

    // registering the same delegate with the same PROXY_TYPE fails
    await futurepassPrecompile
      .connect(owner)
      .registerDelegate(delegate.address, PROXY_TYPE.Any)
      .catch((err: any) => expect(err.message).contains("DelegateAlreadyExists"));
  });

  // @note TBD - v2 functionality
  it.skip("delegate can register more delegates", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // register delegate
    let tx = await futurepassPrecompile.connect(owner).registerDelegate(delegate.address, PROXY_TYPE.Any);
    await tx.wait();

    // fund delegate so it can register another delegate
    await fundEOA(alithSigner, delegate.address);

    // delegate can register another delegate
    const delegate2 = Wallet.createRandom();
    tx = await futurepassPrecompile.connect(delegate).registerDelegate(delegate2.address, PROXY_TYPE.Any);
    await tx.wait();

    expect(await futurepassPrecompile.delegateType(delegate2.address)).to.equal(PROXY_TYPE.Any);
  });

  it("unregister delegate from owner", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // register delegate
    let tx = await futurepassPrecompile.connect(owner).registerDelegate(delegate.address, PROXY_TYPE.Any);
    await tx.wait();

    // unregister delegate as owner
    tx = await futurepassPrecompile.connect(owner).unregisterDelegate(delegate.address);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("FuturepassDelegateUnregistered");
    expect((receipt?.events as any)[0].args.futurepass).to.equal(futurepassPrecompile.address);
    expect((receipt?.events as any)[0].args.delegate).to.equal(delegate.address);
    expect(await futurepassPrecompile.delegateType(delegate.address)).to.equal(PROXY_TYPE.NoPermission);
  });

  it("unregister delegate from delegate (themself)", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // register delegate
    let tx = await futurepassPrecompile.connect(owner).registerDelegate(delegate.address, PROXY_TYPE.Any);
    await tx.wait();

    // fund delegate so it can unregister
    await fundEOA(alithSigner, delegate.address);

    // unregister delegate as delegate
    tx = await futurepassPrecompile.connect(delegate).unregisterDelegate(delegate.address);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("FuturepassDelegateUnregistered");
    expect((receipt?.events as any)[0].args.futurepass).to.equal(futurepassPrecompile.address);
    expect((receipt?.events as any)[0].args.delegate).to.equal(delegate.address);
    expect(await futurepassPrecompile.delegateType(delegate.address)).to.equal(PROXY_TYPE.NoPermission);
  });

  it("proxy call - transfer value from caller to recipient EOA via futurepass", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const recipient = Wallet.createRandom(); // create new recipient to transfer value to

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // Transfer funds to owner
    await fundEOA(alithSigner, owner.address);
    const futurepassBalanceBefore = await xrpERC20Precompile.balanceOf(futurepassPrecompile.address);
    let ownerBalanceBefore = await xrpERC20Precompile.balanceOf(owner.address);

    // ensure recipient and futurepass has zero balance
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(0);
    expect(futurepassBalanceBefore).to.equal(0);

    // proxy transfer of value from owner -> futurepass -> recipient
    // NOTE: This call will be failed due to an arithmetic error that happens in the insider code.
    // i.e 5_000_000 being interpretted as 4_999_999
    // This means the FP need to have at least 1 drop more than the actual amount being transferred.
    const amount = 5;
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, recipient.address, parseEther(amount), "0x", {
        value: parseEther(amount),
      })
      .catch((err: any) => expect(err.message).contains("cannot estimate gas"));

    // owner should have paid the gas for the previous failed tx. get the new balance
    ownerBalanceBefore = await xrpERC20Precompile.balanceOf(owner.address);

    // fund futurepass with 1 drop; assert balance
    await fundAccount(api, alithKeyring, futurepassPrecompile.address, 1);
    expect(await xrpERC20Precompile.balanceOf(futurepassPrecompile.address)).to.equal(1);

    // proxy transfer of value from owner -> futurepass -> recipient
    const transferAmount = 5;
    const tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, recipient.address, parseEther(transferAmount), "0x", {
        value: parseEther(transferAmount),
      });
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("Executed");
    expect((receipt?.events as any)[0].args.callType).to.equal(CALL_TYPE.Call);
    expect((receipt?.events as any)[0].args.target).to.equal(recipient.address);
    expect((receipt?.events as any)[0].args.value).to.equal(parseEther(transferAmount));
    expect((receipt?.events as any)[0].args.data).to.equal("0x00000000");

    // check recipient balance
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(transferAmount * 1_000_000);
    const recipientBalanceRes: any = (await api.query.assets.account(GAS_TOKEN_ID, recipient.address)).toJSON();
    expect(recipientBalanceRes.balance).to.equal(transferAmount * 1_000_000);

    // check futurepass balance, should remain to 1 drop
    expect(await xrpERC20Precompile.balanceOf(futurepassPrecompile.address)).to.equal(1);

    // ensure owner is charged the transfer amount + gas (not double the transfer amount)
    const ownerBalanceAfter = await xrpERC20Precompile.balanceOf(owner.address);
    const totalSpent = ownerBalanceBefore - ownerBalanceAfter;
    expect(totalSpent - transferAmount * 1_000_000).to.lessThan(transferAmount * 1_000_000);
  });

  it("proxy call - transfer value from futurepass to recipient EOA", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // Transfer funds(10 XRP) to the futurepass
    await fundAccount(api, alithKeyring, futurepassPrecompile.address);
    const futurepassBalanceBefore = await xrpERC20Precompile.balanceOf(futurepassPrecompile.address);
    const ownerBalanceBefore = await xrpERC20Precompile.balanceOf(owner.address);
    expect(futurepassBalanceBefore).to.equal(10 * 1_000_000);

    // create new recipient to transfer value to
    const recipient = Wallet.createRandom();
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(0);

    // proxy transfer of value from futurepass -> recipient.
    const transferAmount = 5;
    const tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, recipient.address, parseEther(transferAmount), "0x");
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("Executed");
    expect((receipt?.events as any)[0].args.callType).to.equal(CALL_TYPE.Call);
    expect((receipt?.events as any)[0].args.target).to.equal(recipient.address);
    expect((receipt?.events as any)[0].args.value).to.equal(parseEther(transferAmount));
    expect((receipt?.events as any)[0].args.data).to.equal("0x00000000");

    // check recipient balance
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(transferAmount * 1_000_000);
    const recipientBalanceRes: any = (await api.query.assets.account(GAS_TOKEN_ID, recipient.address)).toJSON();
    expect(recipientBalanceRes.balance).to.equal(transferAmount * 1_000_000);
    // check futurepass balance, should equal to futurepassBalanceBefore - transferAmount * 1_000_000
    expect(await xrpERC20Precompile.balanceOf(futurepassPrecompile.address)).to.equal(
      futurepassBalanceBefore - transferAmount * 1_000_000,
    );
    // check that owner only bared the gas cost. not the transferAmount
    const ownerBalanceAfter = await xrpERC20Precompile.balanceOf(owner.address);
    expect(ownerBalanceBefore - ownerBalanceAfter).to.lessThan(transferAmount * 1_000_000);
  });

  it("proxy call - transfer value to recipient EOA from futurepass and caller combined", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // Transfer funds(10 XRP) to the futurepass
    await fundAccount(api, alithKeyring, futurepassPrecompile.address);
    const futurepassBalanceBefore = await xrpERC20Precompile.balanceOf(futurepassPrecompile.address);
    const ownerBalanceBefore = await xrpERC20Precompile.balanceOf(owner.address);
    expect(futurepassBalanceBefore).to.equal(10 * 1_000_000);

    // create new recipient to transfer value to
    const recipient = Wallet.createRandom();
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(0);

    // Let's fund the FP 1 drop to the futurepass to avoid arithmetic error
    await fundAccount(api, alithKeyring, futurepassPrecompile.address, 1);

    // proxy transfer of value 12 XRP to recipient. futurepass only has 10 XRP. remaining 2 XRP has to be sent from the
    // caller with payable functionality
    const transferAmount = 12;
    const tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, recipient.address, parseEther(transferAmount), "0x", {
        value: parseEther(transferAmount - futurepassBalanceBefore / 1_000_000),
      });
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("Executed");
    expect((receipt?.events as any)[0].args.callType).to.equal(CALL_TYPE.Call);
    expect((receipt?.events as any)[0].args.target).to.equal(recipient.address);
    expect((receipt?.events as any)[0].args.value).to.equal(parseEther(transferAmount));
    expect((receipt?.events as any)[0].args.data).to.equal("0x00000000");

    // check recipient balance
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(transferAmount * 1_000_000);
    const recipientBalanceRes: any = (await api.query.assets.account(GAS_TOKEN_ID, recipient.address)).toJSON();
    expect(recipientBalanceRes.balance).to.equal(transferAmount * 1_000_000);
    // check futurepass balance, should equal to 1 Drop since 10 XRP from the balance was used
    expect(await xrpERC20Precompile.balanceOf(futurepassPrecompile.address)).to.equal(1);
    // check that owner bared (transferAmount - futurepassBalanceBefore) + the gas cost.
    const ownerBalanceAfter = await xrpERC20Precompile.balanceOf(owner.address);
    expect(ownerBalanceBefore - ownerBalanceAfter)
      .to.lessThan(transferAmount * 1_000_000)
      .to.greaterThan((transferAmount - futurepassBalanceBefore / 1_000_000) * 1_000_000);
  });

  it("proxy call - transfer value to recipient EOA fails if futurepass does not receive enough balance", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // Transfer funds(10 XRP) to the futurepass
    await fundAccount(api, alithKeyring, futurepassPrecompile.address);
    const futurepassBalanceBefore = await xrpERC20Precompile.balanceOf(futurepassPrecompile.address);
    const ownerBalanceBefore = await xrpERC20Precompile.balanceOf(owner.address);
    expect(futurepassBalanceBefore).to.equal(10 * 1_000_000);

    // create new recipient to transfer value to
    const recipient = Wallet.createRandom();
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(0);

    // Let's fund the FP 1 drop to the futurepass to avoid arithmetic error
    await fundAccount(api, alithKeyring, futurepassPrecompile.address, 1);

    // proxy transfer of value 25 XRP to recipient. futurepass only has 10 XRP. and the ownerBalanceBefore < 10 XRP.
    // we will send only 5 XRP from the owners account with payable functionality. The call should be failed due to insufficient funds.
    // And all the balance transfers should be reverted.
    const transferAmount = 25;
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, recipient.address, parseEther(transferAmount), "0x", {
        value: parseEther(5),
      })
      .catch((err: any) => expect(err.message).contains("cannot estimate gas"));

    // check recipient balance
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(0);
    // check futurepass balance, should equal to original futurepassBalanceBefore + Drop
    expect(await xrpERC20Precompile.balanceOf(futurepassPrecompile.address)).to.equal(futurepassBalanceBefore.add(1));
    // check that owner bared only the gas cost.
    const ownerBalanceAfter = await xrpERC20Precompile.balanceOf(owner.address);
    expect(ownerBalanceBefore - ownerBalanceAfter).to.lessThan(5 * 1_000_000);
  });

  it("proxy call - transfer value to contract from caller to contract via futurepass", async () => {
    // contract to test outgoing futurepass proxied calls
    const FuturepassFactory = await ethers.getContractFactory("CurrencyTester");
    const futurepassTester = await FuturepassFactory.connect(alithSigner).deploy();
    await futurepassTester.deployed();
    console.log("CurrencyTester deployed to:", futurepassTester.address);

    const owner = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);
    // Transfer funds to owner
    await fundEOA(alithSigner, owner.address);

    expect(await xrpERC20Precompile.balanceOf(futurepassTester.address)).to.equal(0);
    // fund the FP 1 drop, to avoid 5_000_000 being interpretted as 4_999_999
    await fundAccount(api, alithKeyring, futurepassPrecompile.address, 1);
    expect(await xrpERC20Precompile.balanceOf(futurepassPrecompile.address)).to.equal(1);

    // proxy transfer of value from futurepass to contract fails since this is staticcall
    // note: this is possible since contract has `receive() external payable` function
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.StaticCall, futurepassTester.address, parseEther(15), "0x", {
        value: parseEther(15),
      })
      .catch((err: any) => expect(err.message).contains("gas required exceeds allowance"));
    expect(await xrpERC20Precompile.balanceOf(futurepassTester.address)).to.equal(0);

    // proxy transfer of value from futurepass to contract succeeds if call
    // note: this is possible since contract has `receive() external payable` function
    const amount = 15;
    let tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, futurepassTester.address, parseEther(amount), "0x", {
        value: parseEther(amount),
      });
    let receipt = await tx.wait();
    expect((receipt?.events as any)[1].event).to.equal("Executed");
    expect((receipt?.events as any)[1].args.callType).to.equal(CALL_TYPE.Call);
    expect((receipt?.events as any)[1].args.target).to.equal(futurepassTester.address);
    expect((receipt?.events as any)[1].args.value).to.equal(parseEther(amount));
    expect((receipt?.events as any)[1].args.data).to.equal("0x00000000");

    // validate proxy based value transfer to contract payable receive function
    expect(await xrpERC20Precompile.balanceOf(futurepassTester.address)).to.equal(15_000_000);
    let contractBalanceRes: any = (await api.query.assets.account(GAS_TOKEN_ID, futurepassTester.address)).toJSON();
    expect(contractBalanceRes.balance).to.equal(15_000_000);

    // proxy transfer of value from futurepass to contract
    // note: here we call a payable function instead of default receive fallback function
    const fnCallData = futurepassTester.interface.encodeFunctionData("deposit");
    tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, futurepassTester.address, parseEther(amount), fnCallData, {
        value: parseEther(amount),
      });
    receipt = await tx.wait();
    expect((receipt?.events as any)[1].event).to.equal("Executed");
    expect((receipt?.events as any)[1].args.callType).to.equal(CALL_TYPE.Call);
    expect((receipt?.events as any)[1].args.target).to.equal(futurepassTester.address);
    expect((receipt?.events as any)[1].args.value).to.equal(parseEther(amount));
    expect((receipt?.events as any)[1].args.data).to.equal(fnCallData.substring(0, 10)); // "0x<8 hex chars for 4 bytes>"
    expect(fnCallData).to.equal("0xd0e30db0");

    // validate proxy based value transfer to payable function
    expect(await xrpERC20Precompile.balanceOf(futurepassTester.address)).to.equal(30_000_000);
    contractBalanceRes = (await api.query.assets.account(GAS_TOKEN_ID, futurepassTester.address)).toJSON();
    expect(contractBalanceRes.balance).to.equal(30_000_000);

    const futurepassContractBalance = await futurepassTester.deposits(futurepassPrecompile.address);
    expect(ethers.utils.formatEther(futurepassContractBalance)).to.equal("30.0");
  });

  it("proxy call - transfer value to contract from futurepass", async () => {
    // contract to test outgoing futurepass proxied calls
    const FuturepassFactory = await ethers.getContractFactory("CurrencyTester");
    const futurepassTester = await FuturepassFactory.connect(alithSigner).deploy();
    await futurepassTester.deployed();
    console.log("CurrencyTester deployed to:", futurepassTester.address);

    const owner = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // Transfer funds to futurepass (20 XRP)
    await fundAccount(api, alithKeyring, futurepassPrecompile.address, 20_000_000);

    expect(await xrpERC20Precompile.balanceOf(futurepassTester.address)).to.equal(0);

    // proxy transfer of value from futurepass to contract succeeds if call
    // note: this is possible since contract has `receive() external payable` function
    const amount = 5;
    let tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, futurepassTester.address, parseEther(amount), "0x");
    let receipt = await tx.wait();
    // note: 1st event in list is from target contract function execution
    expect((receipt?.events as any)[1].event).to.equal("Executed");
    expect((receipt?.events as any)[1].args.callType).to.equal(CALL_TYPE.Call);
    expect((receipt?.events as any)[1].args.target).to.equal(futurepassTester.address);
    expect((receipt?.events as any)[1].args.value).to.equal(parseEther(amount));
    expect((receipt?.events as any)[1].args.data).to.equal("0x00000000");

    // validate proxy based value transfer to contract payable receive function
    expect(await xrpERC20Precompile.balanceOf(futurepassTester.address)).to.equal(amount * 1_000_000);
    let contractBalanceRes: any = (await api.query.assets.account(GAS_TOKEN_ID, futurepassTester.address)).toJSON();
    expect(contractBalanceRes.balance).to.equal(amount * 1_000_000);

    // proxy transfer of value from futurepass to contract
    // note: here we call a payable function instead of default receive fallback function
    const fnCallData = futurepassTester.interface.encodeFunctionData("deposit");
    tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, futurepassTester.address, parseEther(amount), fnCallData);
    receipt = await tx.wait();
    // note: 1st event in list is from target contract function execution
    expect((receipt?.events as any)[1].event).to.equal("Executed");
    expect((receipt?.events as any)[1].args.callType).to.equal(CALL_TYPE.Call);
    expect((receipt?.events as any)[1].args.target).to.equal(futurepassTester.address);
    expect((receipt?.events as any)[1].args.value).to.equal(parseEther(amount));
    expect((receipt?.events as any)[1].args.data).to.equal(fnCallData.substring(0, 10)); // "0x<8 hex chars for 4 bytes>"
    expect(fnCallData).to.equal("0xd0e30db0");

    // validate proxy based value transfer to payable function
    expect(await xrpERC20Precompile.balanceOf(futurepassTester.address)).to.equal(amount * 2 * 1_000_000); // we transferred 2 times
    contractBalanceRes = (await api.query.assets.account(GAS_TOKEN_ID, futurepassTester.address)).toJSON();
    expect(contractBalanceRes.balance).to.equal(amount * 2 * 1_000_000);

    const futurepassContractBalance = await futurepassTester.deposits(futurepassPrecompile.address);
    expect(futurepassContractBalance).to.equal(ethers.utils.parseEther((amount * 2).toString()));
  });

  it("proxyCall - futurepass can deploy a contract using CREATE", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // transfer funds to owner
    await fundEOA(alithSigner, owner.address);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    const bytecode = MockCreateContract.bytecode;

    // calculate the expected contract address - this is based on deployer address (futurepass), bytecode and nonce
    // for contracts, the nonce is based on how many contract deployments the account has made
    const futurepassNonce = await provider.getTransactionCount(futurepassPrecompile.address);
    const expectedContractAddress = ethers.utils.getContractAddress({
      from: futurepassPrecompile.address,
      nonce: futurepassNonce,
    });

    // call the proxyCall function with the futurepass address and the encoded CREATE call data
    const tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Create, ethers.constants.AddressZero, ethers.constants.Zero, bytecode);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("ContractCreated");
    expect((receipt?.events as any)[0].args.callType).to.equal(CALL_TYPE.Create);
    expect((receipt?.events as any)[0].args.contract).to.equal(expectedContractAddress);
    expect((receipt?.events as any)[0].args.value).to.equal(ethers.constants.Zero);
    expect((receipt?.events as any)[0].args.salt).to.equal(ethers.constants.Zero);

    // validate nonce increases
    expect(await provider.getTransactionCount(futurepassPrecompile.address)).to.equal(futurepassNonce + 1);

    // validate contract functions can be called at the expected address
    const testCreateContract = new ethers.Contract(expectedContractAddress, MockCreateContract.abi, provider);
    expect(await testCreateContract.getValue()).to.equal(420);
  });

  it("proxyCall - futurepass can deploy a contract with constructor using CREATE", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // transfer funds to owner
    await fundEOA(alithSigner, owner.address);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    const amount = 100;
    const bytecodeWithContstructor = ethers.utils.solidityPack(
      ["bytes", "bytes"],
      [MockCreatePayableContract.bytecode, ethers.utils.defaultAbiCoder.encode(["uint256"], [amount])],
    );

    // fails to deploy contract - no amount or value provided (constructor requires amount to be paid to contract)
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Create, ethers.constants.AddressZero, ethers.constants.Zero, bytecodeWithContstructor)
      .catch((err: any) => expect(err.message).contains("cannot estimate gas"));

    // fails to deploy contract - no amount provided (constructor requires amount to be paid to contract)
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Create, ethers.constants.AddressZero, ethers.constants.Zero, bytecodeWithContstructor, {
        value: parseEther(1),
      })
      .catch((err: any) => expect(err.message).contains("cannot estimate gas"));

    const futurepassNonce = await provider.getTransactionCount(futurepassPrecompile.address);
    const expectedContractAddress = ethers.utils.getContractAddress({
      from: futurepassPrecompile.address,
      nonce: futurepassNonce,
    });

    // call the proxyCall function with the futurepass address and the encoded CREATE call data
    const tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Create, ethers.constants.AddressZero, amount, bytecodeWithContstructor, {
        value: parseEther(1),
      });
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("ContractCreated");
    expect((receipt?.events as any)[0].args.callType).to.equal(CALL_TYPE.Create);
    expect((receipt?.events as any)[0].args.contract).to.equal(expectedContractAddress);
    expect((receipt?.events as any)[0].args.value).to.equal(amount);
    expect((receipt?.events as any)[0].args.salt).to.equal(ethers.constants.Zero);

    // validate nonce increases
    expect(await provider.getTransactionCount(futurepassPrecompile.address)).to.equal(futurepassNonce + 1);

    // validate contract functions can be called at the expected address
    const testCreateContract = new ethers.Contract(expectedContractAddress, MockCreatePayableContract.abi, provider);
    expect(await testCreateContract.getValue()).to.equal(420);
    expect(await testCreateContract.getDeposit()).to.equal(100);
  });

  it("proxyCall - futurepass can deploy a contract using CREATE2", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // transfer funds to owner
    await fundEOA(alithSigner, owner.address);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    const bytecode = MockCreateContract.bytecode;

    // Define a salt value for CREATE2
    // calculate the expected contract address - this is based on deployer address (futurepass), bytecode and salt
    // the salt in the precompile is generated calculated using the last 32 bytes of the bytecode
    // Note:
    //    In solidity, the bytes type is an array of bytes, and it is typically represented as a hexadecimal string,
    //    where each byte is represented by 2 hexadecimal digits. Therefore, to get the last 32 bytes, you should get the
    //    last 64 characters from the bytecode string.
    const expectedSalt = "0x" + bytecode.slice(-64);
    const expectedContractAddress = ethers.utils.getCreate2Address(
      futurepassPrecompile.address,
      expectedSalt,
      ethers.utils.keccak256(bytecode),
    );

    // call the proxyCall function with the futurepass address and the encoded CREATE2 call data
    const tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Create2, ethers.constants.AddressZero, ethers.constants.Zero, bytecode);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("ContractCreated");
    expect((receipt?.events as any)[0].args.callType).to.equal(CALL_TYPE.Create2);
    expect((receipt?.events as any)[0].args.contract).to.equal(expectedContractAddress);
    expect((receipt?.events as any)[0].args.value).to.equal(ethers.constants.Zero);
    expect((receipt?.events as any)[0].args.salt).to.equal(expectedSalt);

    // validate contract functions can be called at the expected address
    const testCreateContract = new ethers.Contract(expectedContractAddress, MockCreateContract.abi, provider);
    expect(await testCreateContract.getValue()).to.equal(420);
  });

  it("proxyCall - futurepass can deploy a contract with constructor using CREATE2", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // transfer funds to owner
    await fundEOA(alithSigner, owner.address);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    const amount = 100;
    const bytecodeWithContstructor = ethers.utils.solidityPack(
      ["bytes", "bytes"],
      [MockCreatePayableContract.bytecode, ethers.utils.defaultAbiCoder.encode(["uint256"], [amount])],
    );

    // fails to deploy contract - no amount or value provided (constructor requires amount to be paid to contract)
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Create2, ethers.constants.AddressZero, ethers.constants.Zero, bytecodeWithContstructor)
      .catch((err: any) => expect(err.message).contains("cannot estimate gas"));

    // fails to deploy contract - no amount provided (constructor requires amount to be paid to contract)
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Create2, ethers.constants.AddressZero, ethers.constants.Zero, bytecodeWithContstructor, {
        value: parseEther(1),
      })
      .catch((err: any) => expect(err.message).contains("cannot estimate gas"));

    const expectedSalt = "0x" + bytecodeWithContstructor.slice(-64);
    const expectedContractAddress = ethers.utils.getCreate2Address(
      futurepassPrecompile.address,
      expectedSalt,
      ethers.utils.keccak256(bytecodeWithContstructor),
    );

    // call the proxyCall function with the futurepass address and the encoded CREATE2 call data
    const tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Create2, ethers.constants.AddressZero, amount, bytecodeWithContstructor, {
        value: parseEther(1),
      });
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("ContractCreated");
    expect((receipt?.events as any)[0].args.callType).to.equal(CALL_TYPE.Create2);
    expect((receipt?.events as any)[0].args.contract).to.equal(expectedContractAddress);
    expect((receipt?.events as any)[0].args.value).to.equal(amount);
    expect((receipt?.events as any)[0].args.salt).to.equal(expectedSalt);

    // validate contract functions can be called at the expected address
    const testCreateContract = new ethers.Contract(expectedContractAddress, MockCreatePayableContract.abi, provider);
    expect(await testCreateContract.getValue()).to.equal(420);
    expect(await testCreateContract.getDeposit()).to.equal(100);
  });

  it("futurepass can hold and transfer ERC20", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // transfer some funds to futurepass
    expect(await xrpERC20Precompile.balanceOf(futurepassPrecompile.address)).to.equal(0);
    await new Promise<void>((resolve) => {
      api.tx.assets
        .transfer(GAS_TOKEN_ID, futurepassPrecompile.address, 500_000)
        .signAndSend(alithKeyring, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    expect(await xrpERC20Precompile.balanceOf(futurepassPrecompile.address)).to.equal(500_000);

    // create new recipient to transfer funds to
    const recipient = Wallet.createRandom();
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(0);

    // transfer funds from futurepass to recipient - using proxied, encoded erc20 transfer call
    const transferCallData = xrpERC20Precompile.interface.encodeFunctionData("transfer", [recipient.address, 500_000]);
    const tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, xrpERC20Precompile.address, ethers.constants.Zero, transferCallData);
    await tx.wait();

    // validate proxy based funds transfer
    expect(await xrpERC20Precompile.balanceOf(futurepassPrecompile.address)).to.equal(0);
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(500_000);
  });

  it("futurepass can hold and transfer ERC721", async () => {
    // deploy ERC721 contract and mint a token
    const ERC721Factory = await ethers.getContractFactory("MockERC721");
    const erc721 = await ERC721Factory.connect(alithSigner).deploy();
    await erc721.deployed();
    console.log("MockERC721 deployed to:", erc721.address);

    const owner = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // mint owner a token
    let tokenId = 1;
    let gas = await erc721.connect(owner).estimateGas.safeMint(owner.address, tokenId);
    let tx = await erc721.connect(owner).safeMint(owner.address, tokenId, { gasLimit: gas });
    await tx.wait();

    // transfer ERC721 token to futurepass
    gas = await erc721.connect(owner).estimateGas.transferFrom(owner.address, futurepassPrecompile.address, tokenId);
    tx = await erc721
      .connect(owner)
      .transferFrom(owner.address, futurepassPrecompile.address, tokenId, { gasLimit: gas });
    await tx.wait();
    expect(await erc721.ownerOf(1)).to.equal(futurepassPrecompile.address);

    // mint futurepass a token
    tokenId = 2;
    gas = await erc721.connect(owner).estimateGas.safeMint(futurepassPrecompile.address, tokenId);
    tx = await erc721.connect(owner).safeMint(futurepassPrecompile.address, tokenId, { gasLimit: gas });
    await tx.wait();
    expect(await erc721.ownerOf(2)).to.equal(futurepassPrecompile.address);

    // transfer ERC721 token from futurepass to owner - using proxied, encoded ERC721 transferFrom call
    const transferFromCallData = erc721.interface.encodeFunctionData("transferFrom", [
      futurepassPrecompile.address,
      owner.address,
      2,
    ]);

    // proxy transfer of token from futurepass to contract fails since this is staticcall
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.StaticCall, erc721.address, ethers.constants.Zero, transferFromCallData)
      .catch((err: any) => expect(err.message).contains("gas required exceeds allowance"));

    // proxy transfer of value from futurepass to contract succeeds since this is call
    tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, erc721.address, ethers.constants.Zero, transferFromCallData);
    await tx.wait();

    // validate proxy based ERC721 token transfers
    expect(await erc721.ownerOf(1)).to.equal(futurepassPrecompile.address);
    expect(await erc721.ownerOf(2)).to.equal(owner.address);
  });

  it("futurepass can hold and transfer ERC1155", async () => {
    // deploy ERC1155 contract and mint a token
    const ERC1155Factory = await ethers.getContractFactory("MockERC1155");
    const erc1155 = await ERC1155Factory.connect(alithSigner).deploy();
    await erc1155.deployed();
    console.log("MockERC1155 deployed to:", erc1155.address);

    const owner = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // mint owner a token
    let tokenId = 1;
    let amount = 10;
    let gas = await erc1155.connect(owner).estimateGas.mint(owner.address, tokenId, amount, []);
    let tx = await erc1155.connect(owner).mint(owner.address, tokenId, amount, [], { gasLimit: gas });
    await tx.wait();

    // transfer ERC1155 token to futurepass
    gas = await erc1155
      .connect(owner)
      .estimateGas.safeTransferFrom(owner.address, futurepassPrecompile.address, tokenId, amount, []);
    tx = await erc1155
      .connect(owner)
      .safeTransferFrom(owner.address, futurepassPrecompile.address, tokenId, amount, [], { gasLimit: gas });
    await tx.wait();
    expect(await erc1155.balanceOf(futurepassPrecompile.address, tokenId)).to.equal(amount);

    // mint futurepass a token
    tokenId = 2;
    amount = 5;
    gas = await erc1155.connect(owner).estimateGas.mint(futurepassPrecompile.address, tokenId, amount, []);
    tx = await erc1155.connect(owner).mint(futurepassPrecompile.address, tokenId, amount, [], { gasLimit: gas });
    await tx.wait();
    expect(await erc1155.balanceOf(futurepassPrecompile.address, tokenId)).to.equal(amount);

    // transfer ERC1155 token from futurepass to owner - using proxied, encoded ERC1155 safeTransferFrom call
    const transferAmount = 3;
    const safeTransferFromCallData = erc1155.interface.encodeFunctionData("safeTransferFrom", [
      futurepassPrecompile.address,
      owner.address,
      tokenId,
      transferAmount,
      [],
    ]);
    tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, erc1155.address, ethers.constants.Zero, safeTransferFromCallData);
    await tx.wait();

    // validate proxy based ERC1155 token transfers
    expect(await erc1155.balanceOf(futurepassPrecompile.address, 1)).to.equal(10);
    expect(await erc1155.balanceOf(futurepassPrecompile.address, 2)).to.equal(2);
    expect(await erc1155.balanceOf(owner.address, 2)).to.equal(3);
  });

  it("whitelist - register delegate via proxyCall is allowed", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // ensure delegate doesnt exist for FP
    expect(await futurepassPrecompile.delegateType(delegate.address)).to.equal(PROXY_TYPE.NoPermission);
    // fund the FP, FP_DELEGATE_RESERVE amount of Root for the delegate reserve
    await fundAccount(api, alithKeyring, futurepassPrecompile.address, FP_DELEGATE_RESERVE);
    const fpBalance: any = (await api.query.system.account(futurepassPrecompile.address)).toJSON();
    expect(fpBalance.data.free).to.equal(FP_DELEGATE_RESERVE);

    // get registerDelegate call data
    const registerDelegateCallData = futurepassPrecompile.interface.encodeFunctionData("registerDelegate", [
      delegate.address,
      PROXY_TYPE.Any,
    ]);
    // do proxy call
    const tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, futurepassPrecompile.address, ethers.constants.Zero, registerDelegateCallData);
    await tx.wait();
    // check delegate is a delegate of the futurepass
    expect(await futurepassPrecompile.delegateType(delegate.address)).to.equal(PROXY_TYPE.Any);
  });

  it("whitelist - unregister delegate via proxyCall is allowed", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // fund the FP, FP_DELEGATE_RESERVE amount of Root for the delegate reserve
    await fundAccount(api, alithKeyring, futurepassPrecompile.address, FP_DELEGATE_RESERVE);
    const fpBalance: any = (await api.query.system.account(futurepassPrecompile.address)).toJSON();
    expect(fpBalance.data.free).to.equal(FP_DELEGATE_RESERVE);

    // register delegate
    let tx = await futurepassPrecompile.connect(owner).registerDelegate(delegate.address, PROXY_TYPE.Any);
    await tx.wait();
    // ensure delegate doesnt exist for FP
    expect(await futurepassPrecompile.delegateType(delegate.address)).to.equal(PROXY_TYPE.Any);

    // get unregisterDelegate call data
    const unregisterDelegateCallData = futurepassPrecompile.interface.encodeFunctionData("unregisterDelegate", [
      delegate.address,
    ]);
    // do proxy call
    tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, futurepassPrecompile.address, ethers.constants.Zero, unregisterDelegateCallData);
    await tx.wait();
    // check delegate is not a delegate of the futurepass
    expect(await futurepassPrecompile.delegateType(delegate.address)).to.equal(PROXY_TYPE.NoPermission);
  });

  it("whitelist - non whitelisted calls via proxyCall is not allowed", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const other = Wallet.createRandom().connect(provider);

    // create FP for owner
    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // fund the FP, FP_DELEGATE_RESERVE amount of Root for the delegate reserve
    await fundAccount(api, alithKeyring, futurepassPrecompile.address, FP_DELEGATE_RESERVE);
    const fpBalance: any = (await api.query.system.account(futurepassPrecompile.address)).toJSON();
    expect(fpBalance.data.free).to.equal(FP_DELEGATE_RESERVE);

    // create() not allowed
    let callData = futurepassRegistrar.interface.encodeFunctionData("create", [other.address]);
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, futurepassPrecompile.address, ethers.constants.Zero, callData)
      .catch((err: any) => expect(err.message).contains("cannot estimate gas"));

    // delegateType() not allowed
    callData = futurepassPrecompile.interface.encodeFunctionData("delegateType", [other.address]);
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, futurepassPrecompile.address, ethers.constants.Zero, callData)
      .catch((err: any) => expect(err.message).contains("cannot estimate gas"));

    // proxyCall() not allowed
    callData = futurepassPrecompile.interface.encodeFunctionData("proxyCall", [
      CALL_TYPE.Call,
      other.address,
      ethers.constants.Zero,
      [],
    ]);
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(CALL_TYPE.Call, futurepassPrecompile.address, ethers.constants.Zero, callData)
      .catch((err: any) => expect(err.message).contains("cannot estimate gas"));
  });

  // TODO: introduce functionality
  it.skip("Ownable - owner() function", async () => {
    const owner = Wallet.createRandom().connect(provider);

    const futurepassPrecompile = await createFuturepass(owner, owner.address);
    expect(await futurepassPrecompile.owner()).to.equal(owner.address);
  });

  it("Ownable - renounceOwnership() function", async () => {
    const owner = Wallet.createRandom().connect(provider);

    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    const tx = await futurepassPrecompile.connect(owner).renounceOwnership();
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("OwnershipTransferred");
    expect((receipt?.events as any)[0].args.previousOwner).to.equal(owner.address);
    expect((receipt?.events as any)[0].args.newOwner).to.equal(ethers.constants.AddressZero);

    // ensure ownership is now zero address
    // expect(await futurepassPrecompile.owner()).to.equal(ethers.constants.AddressZero); // TODO
  });

  it("Ownable - transferOwnership() function", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const newOwner = Wallet.createRandom();

    const futurepassPrecompile = await createFuturepass(owner, owner.address);

    // add newOwner as delegate // TODO: introduce this after fixing delegate -> owner bug
    // let tx = await futurepassPrecompile.connect(owner).registerDelegate(newOwner.address, PROXY_TYPE.Any);
    // await tx.wait();
    // expect(await futurepassPrecompile.owner()).to.equal(owner.address); // TODO

    const tx = await futurepassPrecompile.connect(owner).transferOwnership(newOwner.address);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("OwnershipTransferred");
    expect((receipt?.events as any)[0].args.previousOwner).to.equal(owner.address);
    expect((receipt?.events as any)[0].args.newOwner).to.equal(newOwner.address);

    // ensure ownership is now new owner
    // expect(await futurepassPrecompile.owner()).to.equal(newOwner.address); // TODO
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

async function fundEOA(signer: Wallet, address: string, value: string = "10000") {
  const tx = await signer.sendTransaction({ to: address, value: ethers.utils.parseEther(value) });
  await tx.wait();
}

function parseEther(amount: number): BigNumber {
  return ethers.utils.parseEther(amount.toString());
}
