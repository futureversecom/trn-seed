import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Contract, Wallet } from "ethers";
import { ethers } from "hardhat";
import web3 from "web3";

import MockERC20Data from "../artifacts/contracts/MockERC20.sol/MockERC20.json";
import {
  ALITH_PRIVATE_KEY,
  ERC20_ABI,
  FP_DELIGATE_RESERVE,
  FUTUREPASS_PRECOMPILE_ABI,
  FUTUREPASS_REGISTRAR_PRECOMPILE_ABI,
  FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS,
  GAS_TOKEN_ID,
  NATIVE_TOKEN_ID,
  NodeProcess,
  startNode,
  typedefs,
} from "../common";

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
  let futurepassPrecompile: Contract;
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

  async function createFuturepass(caller: Wallet, owner: string): boolean {
    // fund caller to pay for futurepass creation
    await fundAccount(api, alithKeyring, caller.address);

    const tx = await futurepassRegistrar.connect(caller).create(owner);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("FuturepassCreated");
    expect((receipt?.events as any)[0].args.owner).to.equal(owner);

    const futurepass = (receipt?.events as any)[0].args.futurepass;
    expect(await futurepassRegistrar.futurepassOf(owner)).to.equal(futurepass);
    futurepassPrecompile = new Contract(futurepass, FUTUREPASS_PRECOMPILE_ABI, caller);
    return true;
  }

  it("test create futurepass", async () => {
    const owner = Wallet.createRandom().address;

    expect(await createFuturepass(alithSigner, owner)).true;
  });

  it("isDelegate, delegateType works", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // create FP for owner
    expect(await createFuturepass(owner, owner.address)).true;

    // isDelegate should return false.
    expect(await futurepassPrecompile.isDelegate(delegate.address)).to.equal(false);
    // checkDelegate should return 0 value(ProxyType.NoPermission)
    expect(await futurepassPrecompile.delegateType(delegate.address)).to.equal(PROXY_TYPE.NoPermission);

    const tx = await futurepassPrecompile.connect(owner).registerDelegate(delegate.address, PROXY_TYPE.Any);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("FuturepassDelegateRegistered");
    expect((receipt?.events as any)[0].args.futurepass).to.equal(futurepassPrecompile.address);
    expect((receipt?.events as any)[0].args.delegate).to.equal(delegate.address);
    expect((receipt?.events as any)[0].args.proxyType).to.equal(PROXY_TYPE.Any);

    // isDelegate should return true.
    expect(await futurepassPrecompile.isDelegate(delegate.address)).to.equal(true);
    // checkDelegate should return PROXY_TYPE.Any
    expect(await futurepassPrecompile.delegateType(delegate.address)).to.equal(PROXY_TYPE.Any);
  });

  it("register delegate works", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // create FP for owner
    expect(await createFuturepass(owner, owner.address)).true;

    // ensure delegate doesnt exist for FP
    expect(await futurepassPrecompile.isDelegate(delegate.address)).to.equal(false);

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
    expect(await createFuturepass(owner, owner.address)).true;

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
    expect(await createFuturepass(owner, owner.address)).true;

    // register delegate
    let tx = await futurepassPrecompile.connect(owner).registerDelegate(delegate.address, PROXY_TYPE.Any);
    await tx.wait();

    // unregister delegate as owner
    tx = await futurepassPrecompile.connect(owner).unregisterDelegate(delegate.address);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("FuturepassDelegateUnregistered");
    expect((receipt?.events as any)[0].args.futurepass).to.equal(futurepassPrecompile.address);
    expect((receipt?.events as any)[0].args.delegate).to.equal(delegate.address);
    expect(await futurepassPrecompile.isDelegate(delegate.address)).to.equal(false);
  });

  it("unregister delegate from delegate (themself)", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // create FP for owner
    expect(await createFuturepass(owner, owner.address)).true;

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
    expect(await futurepassPrecompile.isDelegate(delegate.address)).to.equal(false);
  });

  it("proxy call can transfer value to EOA", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // create FP for owner
    expect(await createFuturepass(owner, owner.address)).true;
    // Transfer funds to owner
    await fundEOA(alithSigner, owner.address);
    const futurepassBalanceBefore = await xrpERC20Precompile.balanceOf(futurepassPrecompile.address);
    let ownerBalanceBefore = await xrpERC20Precompile.balanceOf(owner.address);

    // create new recipient to transfer value to
    const recipient = Wallet.createRandom();
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(0);
    // check futurepass has zero balance
    expect(futurepassBalanceBefore).to.equal(0);

    // proxy transfer of value from owner -> futurepass -> recipient
    // NOTE: This call will be failed due to an arithmetic error that happens in the insider code.
    // i.e 5_000_000 being interpretted as 4_999_999
    // This means the FP need to have at least 1 drop more than the actual amount being transferred.
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(recipient.address, CALL_TYPE.Call, "0x", {
        value: ethers.utils.parseEther("5"),
      })
      .catch((err: any) => expect(err.message).contains("cannot estimate gas"));

    // owner should have paid the gas for the previous failed tx. get the new balance
    ownerBalanceBefore = await xrpERC20Precompile.balanceOf(owner.address);

    // Let's fund the FP 1 drop and try again
    await fundAccount(api, alithKeyring, futurepassPrecompile.address, 1);
    expect(await xrpERC20Precompile.balanceOf(futurepassPrecompile.address)).to.equal(1);
    // proxy transfer of value from owner -> futurepass -> recipient
    const transferAmount = 5;
    const tx = await futurepassPrecompile.connect(owner).proxyCall(recipient.address, CALL_TYPE.Call, "0x", {
      value: ethers.utils.parseEther(transferAmount.toString()),
    });
    await tx.wait();

    // check recipient balance
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(transferAmount * 1_000_000);
    const recipientBalanceRes: any = (await api.query.assets.account(GAS_TOKEN_ID, recipient.address)).toJSON();
    expect(recipientBalanceRes.balance).to.equal(transferAmount * 1_000_000);
    // check futurepass balance, should equla to 1 drop
    expect(await xrpERC20Precompile.balanceOf(futurepassPrecompile.address)).to.equal(1);
    // check owner balance
    const ownerBalanceAfter = await xrpERC20Precompile.balanceOf(owner.address);
    const totalSpent = ownerBalanceBefore - ownerBalanceAfter;

    // check the owner is charged the transfer amount + gas, not double the transfer amount
    expect(totalSpent - transferAmount * 1_000_000).to.lessThan(transferAmount * 1_000_000);
    // TODO - do the gas calculation and tally here
  });

  it("proxy call can transfer value to contract", async () => {
    // contract to test outgoing futurepass proxied calls
    const FuturepassFactory = await ethers.getContractFactory("CurrencyTester");
    const futurepassTester = await FuturepassFactory.connect(alithSigner).deploy();
    await futurepassTester.deployed();
    console.log("CurrencyTester deployed to:", futurepassTester.address);

    const owner = Wallet.createRandom().connect(provider);

    // create FP for owner
    expect(await createFuturepass(owner, owner.address)).true;
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
      .proxyCall(futurepassTester.address, CALL_TYPE.StaticCall, "0x", {
        value: ethers.utils.parseEther("15"),
      })
      .catch((err: any) => expect(err.message).contains("gas required exceeds allowance"));

    // proxy transfer of value from futurepass to contract succeeds if call
    // note: this is possible since contract has `receive() external payable` function
    let tx = await futurepassPrecompile.connect(owner).proxyCall(futurepassTester.address, CALL_TYPE.Call, "0x", {
      value: ethers.utils.parseEther("15"),
    });
    await tx.wait();

    // validate proxy based value transfer to contract payable receive function
    expect(await xrpERC20Precompile.balanceOf(futurepassTester.address)).to.equal(15_000_000);
    let contractBalanceRes: any = (await api.query.assets.account(GAS_TOKEN_ID, futurepassTester.address)).toJSON();
    expect(contractBalanceRes.balance).to.equal(15_000_000);

    // proxy transfer of value from futurepass to contract
    // note: here we call a payable function instead of default receive fallback function
    const fnCallData = futurepassTester.interface.encodeFunctionData("deposit");
    tx = await futurepassPrecompile.connect(owner).proxyCall(futurepassTester.address, CALL_TYPE.Call, fnCallData, {
      value: ethers.utils.parseEther("5"),
    });
    await tx.wait();

    // validate proxy based value transfer to payable function
    expect(await xrpERC20Precompile.balanceOf(futurepassTester.address)).to.equal(20_000_000);
    contractBalanceRes = (await api.query.assets.account(GAS_TOKEN_ID, futurepassTester.address)).toJSON();
    expect(contractBalanceRes.balance).to.equal(20_000_000);

    const futurepassContractBalance = await futurepassTester.deposits(futurepassPrecompile.address);
    expect(ethers.utils.formatEther(futurepassContractBalance)).to.equal("20.0");
  });

  it("futurepass can hold and transfer ERC20", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // create FP for owner
    expect(await createFuturepass(owner, owner.address)).true;

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
      .proxyCall(xrpERC20Precompile.address, CALL_TYPE.Call, transferCallData);
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
    expect(await createFuturepass(owner, owner.address)).true;

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
      .proxyCall(erc721.address, CALL_TYPE.StaticCall, transferFromCallData)
      .catch((err: any) => expect(err.message).contains("gas required exceeds allowance"));

    // proxy transfer of value from futurepass to contract succeeds since this is call
    tx = await futurepassPrecompile.connect(owner).proxyCall(erc721.address, CALL_TYPE.Call, transferFromCallData);
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
    expect(await createFuturepass(owner, owner.address)).true;

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
    tx = await futurepassPrecompile.connect(owner).proxyCall(erc1155.address, CALL_TYPE.Call, safeTransferFromCallData);
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
    expect(await createFuturepass(owner, owner.address)).true;

    // ensure delegate doesnt exist for FP
    expect(await futurepassPrecompile.isDelegate(delegate.address)).to.equal(false);
    // fund the FP, FP_DELIGATE_RESERVE amount of Root for the delegate reserve
    await fundAccount(api, alithKeyring, futurepassPrecompile.address, FP_DELIGATE_RESERVE);
    const fpBalance: any = (await api.query.system.account(futurepassPrecompile.address)).toJSON();
    expect(fpBalance.data.free).to.equal(FP_DELIGATE_RESERVE);

    // get registerDelegate call data
    const registerDelegateCallData = futurepassPrecompile.interface.encodeFunctionData("registerDelegate", [
      delegate.address,
      PROXY_TYPE.Any,
    ]);
    // do proxy call
    const tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(futurepassPrecompile.address, CALL_TYPE.Call, registerDelegateCallData);
    await tx.wait();
    // check delegate is a delegate of the futurepass
    expect(await futurepassPrecompile.delegateType(delegate.address)).to.equal(PROXY_TYPE.Any);

    //TODO : check who pays the fee
  });

  it("whitelist - unregister delegate via proxyCall is allowed", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // create FP for owner
    expect(await createFuturepass(owner, owner.address)).true;

    // fund the FP, FP_DELIGATE_RESERVE amount of Root for the delegate reserve
    await fundAccount(api, alithKeyring, futurepassPrecompile.address, FP_DELIGATE_RESERVE);
    const fpBalance: any = (await api.query.system.account(futurepassPrecompile.address)).toJSON();
    expect(fpBalance.data.free).to.equal(FP_DELIGATE_RESERVE);

    // register delegate
    let tx = await futurepassPrecompile.connect(owner).registerDelegate(delegate.address, PROXY_TYPE.Any);
    await tx.wait();
    // ensure delegate doesnt exist for FP
    expect(await futurepassPrecompile.isDelegate(delegate.address)).to.equal(true);

    // get unregisterDelegate call data
    const unregisterDelegateCallData = futurepassPrecompile.interface.encodeFunctionData("unregisterDelegate", [
      delegate.address,
    ]);
    // do proxy call
    tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(futurepassPrecompile.address, CALL_TYPE.Call, unregisterDelegateCallData);
    await tx.wait();
    // check delegate is not a delegate of the futurepass
    expect(await futurepassPrecompile.isDelegate(delegate.address)).to.equal(false);

    //TODO : check who pays the fee
  });

  it("whitelist - non whitelisted calls via proxyCall is not allowed", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const other = Wallet.createRandom().connect(provider);

    // create FP for owner
    expect(await createFuturepass(owner, owner.address)).true;

    // fund the FP, FP_DELIGATE_RESERVE amount of Root for the delegate reserve
    await fundAccount(api, alithKeyring, futurepassPrecompile.address, FP_DELIGATE_RESERVE);
    const fpBalance: any = (await api.query.system.account(futurepassPrecompile.address)).toJSON();
    expect(fpBalance.data.free).to.equal(FP_DELIGATE_RESERVE);

    // create() not allowed
    let CallData = futurepassRegistrar.interface.encodeFunctionData("create", [other.address]);
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(futurepassPrecompile.address, CALL_TYPE.Call, CallData)
      .catch((err: any) => expect(err.message).contains("cannot estimate gas"));

    // isDelegate() not allowed
    CallData = futurepassPrecompile.interface.encodeFunctionData("isDelegate", [other.address]);
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(futurepassPrecompile.address, CALL_TYPE.Call, CallData)
      .catch((err: any) => expect(err.message).contains("cannot estimate gas"));

    // delegateType() not allowed
    CallData = futurepassPrecompile.interface.encodeFunctionData("delegateType", [other.address]);
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(futurepassPrecompile.address, CALL_TYPE.Call, CallData)
      .catch((err: any) => expect(err.message).contains("cannot estimate gas"));

    // proxyCall() not allowed
    CallData = futurepassPrecompile.interface.encodeFunctionData("proxyCall", [other.address, CALL_TYPE.Call, []]);
    await futurepassPrecompile
      .connect(owner)
      .proxyCall(futurepassPrecompile.address, CALL_TYPE.Call, CallData)
      .catch((err: any) => expect(err.message).contains("cannot estimate gas"));
  });

  // TODO: introduce functionality in v2
  it.skip("futurepass can deploy a contract using CREATE", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // create FP for owner
    expect(await createFuturepass(owner, owner.address)).true;

    const erc20Bytecode = MockERC20Data.bytecode;

    // calculate the expected contract address
    const futurepassNonce = await provider.getTransactionCount(futurepassPrecompile.address);
    const expectedContractAddress = ethers.utils.getContractAddress({
      from: futurepassPrecompile.address,
      nonce: futurepassNonce,
    });

    // call the proxyCall function with the futurepass address and the encoded CREATE call data
    const tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(ethers.constants.AddressZero, CALL_TYPE.Create, erc20Bytecode);
    const receipt = await tx.wait();

    console.warn(receipt);

    // verify that the created contract has the same bytecode as the template contract
    const deployedContractBytecode = await provider.getCode(expectedContractAddress);
    expect(deployedContractBytecode).to.equal(erc20Bytecode);

    // // verify that the created contract's owner is the futurepass
    // const createdContract = MockERC20Factory.attach(expectedContractAddress);
    // expect(await createdContract.owner()).to.equal(futurepass);
  });

  // TODO: introduce functionality
  it.skip("futurepass can deploy a contract using CREATE2", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // create FP for owner
    expect(await createFuturepass(owner, owner.address)).true;

    const erc20Bytecode = MockERC20Data.bytecode;

    // Define a salt value for CREATE2
    // const salt = ethers.utils.hexZeroPad(ethers.utils.randomBytes(32), 32);
    // const salt = ethers.utils.hexZeroPad(ethers.utils.hexlify(ethers.BigNumber.from(ethers.utils.randomBytes(32))), 32);
    const salt = ethers.utils.id("1234");
    ethers.utils.getCreate2Address(futurepassPrecompile.address, salt, ethers.utils.keccak256(erc20Bytecode));

    // Encode the CREATE2 call to deploy the template contract
    const deployCallData = ethers.utils.hexConcat([ethers.utils.hexZeroPad("0xff", 32), erc20Bytecode, salt]);

    // Call the proxyCall function with the futurepass address and the encoded CREATE2 call data
    const tx = await futurepassPrecompile
      .connect(owner)
      .proxyCall(ethers.constants.AddressZero, CALL_TYPE.Create2, deployCallData);
    await tx.wait();

    // // Verify that the created contract has the same bytecode as the template contract
    // const deployedContractBytecode = await provider.getCode(expectedContractAddress);
    // expect(deployedContractBytecode).to.equal(erc20Bytecode);

    // // Verify that the created contract's owner is the futurepass
    // const createdContract = MockERC20Factory.attach(expectedContractAddress);
    // expect(await createdContract.owner()).to.equal(futurepass);
  });
});
