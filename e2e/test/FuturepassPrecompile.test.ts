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
  FUTUREPASS_PRECOMPILE_ABI,
  FUTUREPASS_PRECOMPILE_ADDRESS,
  GAS_TOKEN_ID,
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
  let futurepassProxy: Contract;
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

    futurepassProxy = new Contract(FUTUREPASS_PRECOMPILE_ADDRESS, FUTUREPASS_PRECOMPILE_ABI, alithSigner);
    xrpERC20Precompile = new Contract(XRP_PRECOMPILE_ADDRESS, ERC20_ABI, alithSigner);
  });

  afterEach(async () => await node.stop());

  // This testcase is included in futurepass substrate tests
  it.skip("create futurepass succeeds for account with balance", async () => {
    const owner = Wallet.createRandom().address;

    // fund owner to pay for futurepass creation
    await fundEOA(alithSigner, owner);

    const tx = await futurepassProxy.connect(alithSigner).create(owner);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("FuturepassCreated");
    expect((receipt?.events as any)[0].args.owner).to.equal(owner);

    expect(await futurepassProxy.futurepassOf(owner)).to.equal((receipt?.events as any)[0].args.futurepass);
  });

  // This testcase is included in futurepass substrate tests
  it.skip("create futurepass succeeds for account with no balance", async () => {
    const owner = Wallet.createRandom().address;
    const tx = await futurepassProxy.connect(alithSigner).create(owner);
    const receipt = await tx.wait();

    expect((receipt?.events as any)[0].event).to.equal("FuturepassCreated");
    expect((receipt?.events as any)[0].args.owner).to.equal(owner);

    expect(await futurepassProxy.futurepassOf(owner)).to.equal((receipt?.events as any)[0].args.futurepass);
  });

  it("create futurepass fails - already existing account", async () => {
    const owner = Wallet.createRandom().address;
    const tx = await futurepassProxy.connect(alithSigner).create(owner);
    await tx.wait();

    // should fail upon creation of FP for same owner again
    await futurepassProxy
      .connect(alithSigner)
      .create(owner)
      .catch((err: any) => {
        expect(err.message).contains("AccountAlreadyRegistered");
      });
  });

  it("isDelegate, delegateType works", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // fund owner so it can create FP
    // await fundEOA(alithSigner, owner.address);
    await fundAccount(api, alithKeyring, owner.address); // TODO <- why is this required?

    // create FP for owner
    let tx = await futurepassProxy.connect(owner).create(owner.address);
    await tx.wait();
    const futurepass = await futurepassProxy.futurepassOf(owner.address);

    // isDelegate should return false.
    expect(await futurepassProxy.isDelegate(futurepass, delegate.address)).to.equal(false);
    // checkDelegate should return 0 value(ProxyType.NoPermission)
    expect(await futurepassProxy.delegateType(futurepass, delegate.address)).to.equal(PROXY_TYPE.NoPermission);

    tx = await futurepassProxy.connect(owner).registerDelegate(futurepass, delegate.address, PROXY_TYPE.Any);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("FuturepassDelegateRegistered");
    expect((receipt?.events as any)[0].args.futurepass).to.equal(futurepass);
    expect((receipt?.events as any)[0].args.delegate).to.equal(delegate.address);
    expect((receipt?.events as any)[0].args.proxyType).to.equal(PROXY_TYPE.Any);

    // isDelegate should return false.
    expect(await futurepassProxy.isDelegate(futurepass, delegate.address)).to.equal(true);
    // checkDelegate should return PROXY_TYPE.Any
    expect(await futurepassProxy.delegateType(futurepass, delegate.address)).to.equal(PROXY_TYPE.Any);
  });

  it("register delegate works", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // fund owner so it can create FP
    // await fundEOA(alithSigner, owner.address);
    await fundAccount(api, alithKeyring, owner.address); // TODO <- why is this required?

    // create FP for owner
    let tx = await futurepassProxy.connect(owner).create(owner.address);
    await tx.wait();
    const futurepass = await futurepassProxy.futurepassOf(owner.address);

    // ensure delegate doesnt exist for FP
    expect(await futurepassProxy.isDelegate(futurepass, delegate.address)).to.equal(false);

    // registering with proxytype other than PROXY_TYPE.Any fails
    await futurepassProxy
      .connect(owner)
      .registerDelegate(futurepass, delegate.address, PROXY_TYPE.NonTransfer)
      .catch((err: any) => expect(err.message).contains("PermissionDenied"));

    tx = await futurepassProxy.connect(owner).registerDelegate(futurepass, delegate.address, PROXY_TYPE.Any);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("FuturepassDelegateRegistered");
    expect((receipt?.events as any)[0].args.futurepass).to.equal(futurepass);
    expect((receipt?.events as any)[0].args.delegate).to.equal(delegate.address);
    expect((receipt?.events as any)[0].args.proxyType).to.equal(PROXY_TYPE.Any);
    expect(await futurepassProxy.delegateType(futurepass, delegate.address)).to.equal(PROXY_TYPE.Any);

    // registering the same delegate with the same PROXY_TYPE fails
    await futurepassProxy
      .connect(owner)
      .registerDelegate(futurepass, delegate.address, PROXY_TYPE.Any)
      .catch((err: any) => expect(err.message).contains("DelegateAlreadyExists"));
  });

  // @note TBD - v2 functionality
  it.skip("delegate can register more delegates", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // fund owner so it can create FP
    await fundEOA(alithSigner, owner.address);

    // create FP for owner
    let tx = await futurepassProxy.connect(owner).create(owner.address);
    await tx.wait();
    const futurepass = await futurepassProxy.futurepassOf(owner.address);

    // register delegate
    tx = await futurepassProxy.connect(owner).registerDelegate(futurepass, delegate.address, PROXY_TYPE.Any);
    await tx.wait();

    // fund delegate so it can register another delegate
    await fundEOA(alithSigner, delegate.address);

    // delegate can register another delegate
    const delegate2 = Wallet.createRandom();
    tx = await futurepassProxy.connect(delegate).registerDelegate(futurepass, delegate2.address, PROXY_TYPE.Any);
    await tx.wait();
    expect(await futurepassProxy.delegateType(futurepass, delegate2.address)).to.equal(PROXY_TYPE.Any);
  });

  it("unregister delegate from owner", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // fund owner so it can create FP
    // await fundEOA(alithSigner, owner.address);
    await fundAccount(api, alithKeyring, owner.address); // TODO <- why is this required?

    // create FP for owner
    let tx = await futurepassProxy.connect(owner).create(owner.address);
    await tx.wait();
    const futurepass = await futurepassProxy.futurepassOf(owner.address);

    // register delegate
    tx = await futurepassProxy.connect(owner).registerDelegate(futurepass, delegate.address, PROXY_TYPE.Any);
    await tx.wait();

    // unregister delegate as owner
    tx = await futurepassProxy.connect(owner).unregisterDelegate(futurepass, delegate.address);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("FuturepassDelegateUnregistered");
    expect((receipt?.events as any)[0].args.futurepass).to.equal(futurepass);
    expect((receipt?.events as any)[0].args.delegate).to.equal(delegate.address);
    expect(await futurepassProxy.isDelegate(futurepass, delegate.address)).to.equal(false);
  });

  it("unregister delegate from delegate (themself)", async () => {
    const owner = Wallet.createRandom().connect(provider);
    const delegate = Wallet.createRandom().connect(provider);

    // fund owner so it can create FP
    // await fundEOA(alithSigner, owner.address);
    await fundAccount(api, alithKeyring, owner.address); // TODO <- why is this required?

    // create FP for owner
    let tx = await futurepassProxy.connect(owner).create(owner.address);
    await tx.wait();
    const futurepass = await futurepassProxy.futurepassOf(owner.address);

    // register delegate
    tx = await futurepassProxy.connect(owner).registerDelegate(futurepass, delegate.address, PROXY_TYPE.Any);
    await tx.wait();

    // fund delegate so it can unregister
    await fundEOA(alithSigner, delegate.address);

    // unregister delegate as delegate
    tx = await futurepassProxy.connect(delegate).unregisterDelegate(futurepass, delegate.address);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("FuturepassDelegateUnregistered");
    expect((receipt?.events as any)[0].args.futurepass).to.equal(futurepass);
    expect((receipt?.events as any)[0].args.delegate).to.equal(delegate.address);
    expect(await futurepassProxy.isDelegate(futurepass, delegate.address)).to.equal(false);
  });

  it("proxy call can transfer value to EOA", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // fund owner
    await fundEOA(alithSigner, owner.address);

    // create FP for owner
    let tx = await futurepassProxy.connect(alithSigner).create(owner.address);
    await tx.wait();
    const futurepass = await futurepassProxy.futurepassOf(owner.address);

    // create new recipient to transfer value to
    const recipient = Wallet.createRandom();
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(0);

    // proxy transfer of value from futurepass to recipient
    tx = await futurepassProxy.connect(owner).proxyCall(futurepass, recipient.address, CALL_TYPE.Call, "0x", {
      value: ethers.utils.parseEther("15"),
    });
    await tx.wait();

    // validate proxy based value transfer to recipient
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(15_000_000);
    const recipientBalanceRes: any = (await api.query.assets.account(GAS_TOKEN_ID, recipient.address)).toJSON();
    expect(recipientBalanceRes.balance).to.equal(15_000_000);
  });

  it("proxy call can transfer value to contract", async () => {
    // contract to test outgoing futurepass proxied calls
    const FuturepassFactory = await ethers.getContractFactory("CurrencyTester");
    const futurepassTester = await FuturepassFactory.connect(alithSigner).deploy();
    await futurepassTester.deployed();
    console.log("CurrencyTester deployed to:", futurepassTester.address);

    const owner = Wallet.createRandom().connect(provider);

    // fund owner
    await fundEOA(alithSigner, owner.address);

    // create FP for owner
    let tx = await futurepassProxy.connect(alithSigner).create(owner.address);
    await tx.wait();
    const futurepass = await futurepassProxy.futurepassOf(owner.address);

    expect(await xrpERC20Precompile.balanceOf(futurepassTester.address)).to.equal(0);

    // proxy transfer of value from futurepass to contract fails since this is staticcall
    // note: this is possible since contract has `receive() external payable` function
    await futurepassProxy
      .connect(owner)
      .proxyCall(futurepass, futurepassTester.address, CALL_TYPE.StaticCall, "0x", {
        value: ethers.utils.parseEther("15"),
      })
      .catch((err: any) => expect(err.message).contains("gas required exceeds allowance"));

    // proxy transfer of value from futurepass to contract succeeds if call
    // note: this is possible since contract has `receive() external payable` function
    tx = await futurepassProxy.connect(owner).proxyCall(futurepass, futurepassTester.address, CALL_TYPE.Call, "0x", {
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
    tx = await futurepassProxy
      .connect(owner)
      .proxyCall(futurepass, futurepassTester.address, CALL_TYPE.Call, fnCallData, {
        value: ethers.utils.parseEther("5"),
      });
    await tx.wait();

    // validate proxy based value transfer to payable function
    expect(await xrpERC20Precompile.balanceOf(futurepassTester.address)).to.equal(20_000_000);
    contractBalanceRes = (await api.query.assets.account(GAS_TOKEN_ID, futurepassTester.address)).toJSON();
    expect(contractBalanceRes.balance).to.equal(20_000_000);

    const futurepassContractBalance = await futurepassTester.deposits(futurepass);
    expect(ethers.utils.formatEther(futurepassContractBalance)).to.equal("20.0");
  });

  it("futurepass can hold and transfer ERC20", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // fund owner
    await fundEOA(alithSigner, owner.address);

    // create FP for owner
    let tx = await futurepassProxy.connect(alithSigner).create(owner.address);
    await tx.wait();
    const futurepass = await futurepassProxy.futurepassOf(owner.address);

    // transfer some funds to futurepass
    expect(await xrpERC20Precompile.balanceOf(futurepass)).to.equal(0);
    await new Promise<void>((resolve) => {
      api.tx.assets.transfer(GAS_TOKEN_ID, futurepass, 500_000).signAndSend(alithKeyring, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    expect(await xrpERC20Precompile.balanceOf(futurepass)).to.equal(500_000);

    // create new recipient to transfer funds to
    const recipient = Wallet.createRandom();
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(0);

    // transfer funds from futurepass to recipient - using proxied, encoded erc20 transfer call
    const transferCallData = xrpERC20Precompile.interface.encodeFunctionData("transfer", [recipient.address, 500_000]);
    tx = await futurepassProxy
      .connect(owner)
      .proxyCall(futurepass, xrpERC20Precompile.address, CALL_TYPE.Call, transferCallData);
    await tx.wait();

    // validate proxy based funds transfer
    expect(await xrpERC20Precompile.balanceOf(futurepass)).to.equal(0);
    expect(await xrpERC20Precompile.balanceOf(recipient.address)).to.equal(500_000);
  });

  it("futurepass can hold and transfer ERC721", async () => {
    // deploy ERC721 contract and mint a token
    const ERC721Factory = await ethers.getContractFactory("MockERC721");
    const erc721 = await ERC721Factory.connect(alithSigner).deploy();
    await erc721.deployed();
    console.log("MockERC721 deployed to:", erc721.address);

    const owner = Wallet.createRandom().connect(provider);

    // fund owner
    await fundEOA(alithSigner, owner.address);

    // create FP for owner
    let tx = await futurepassProxy.connect(alithSigner).create(owner.address);
    await tx.wait();
    const futurepass = await futurepassProxy.futurepassOf(owner.address);

    // mint owner a token
    let tokenId = 1;
    let gas = await erc721.connect(owner).estimateGas.safeMint(owner.address, tokenId);
    tx = await erc721.connect(owner).safeMint(owner.address, tokenId, { gasLimit: gas });
    await tx.wait();

    // transfer ERC721 token to futurepass
    gas = await erc721.connect(owner).estimateGas.transferFrom(owner.address, futurepass, tokenId);
    tx = await erc721.connect(owner).transferFrom(owner.address, futurepass, tokenId, { gasLimit: gas });
    await tx.wait();
    expect(await erc721.ownerOf(1)).to.equal(futurepass);

    // mint futurepass a token
    tokenId = 2;
    gas = await erc721.connect(owner).estimateGas.safeMint(futurepass, tokenId);
    tx = await erc721.connect(owner).safeMint(futurepass, tokenId, { gasLimit: gas });
    await tx.wait();
    expect(await erc721.ownerOf(2)).to.equal(futurepass);

    // transfer ERC721 token from futurepass to owner - using proxied, encoded ERC721 transferFrom call
    const transferFromCallData = erc721.interface.encodeFunctionData("transferFrom", [futurepass, owner.address, 2]);

    // proxy transfer of token from futurepass to contract fails since this is staticcall
    await futurepassProxy
      .connect(owner)
      .proxyCall(futurepass, erc721.address, CALL_TYPE.StaticCall, transferFromCallData)
      .catch((err: any) => expect(err.message).contains("gas required exceeds allowance"));

    // proxy transfer of value from futurepass to contract succeeds since this is call
    tx = await futurepassProxy
      .connect(owner)
      .proxyCall(futurepass, erc721.address, CALL_TYPE.Call, transferFromCallData);
    await tx.wait();

    // validate proxy based ERC721 token transfers
    expect(await erc721.ownerOf(1)).to.equal(futurepass);
    expect(await erc721.ownerOf(2)).to.equal(owner.address);
  });

  it("futurepass can hold and transfer ERC1155", async () => {
    // deploy ERC1155 contract and mint a token
    const ERC1155Factory = await ethers.getContractFactory("MockERC1155");
    const erc1155 = await ERC1155Factory.connect(alithSigner).deploy();
    await erc1155.deployed();
    console.log("MockERC1155 deployed to:", erc1155.address);

    const owner = Wallet.createRandom().connect(provider);

    // fund owner
    await fundEOA(alithSigner, owner.address);

    // create FP for owner
    let tx = await futurepassProxy.connect(alithSigner).create(owner.address);
    await tx.wait();
    const futurepass = await futurepassProxy.futurepassOf(owner.address);

    // mint owner a token
    let tokenId = 1;
    let amount = 10;
    let gas = await erc1155.connect(owner).estimateGas.mint(owner.address, tokenId, amount, []);
    tx = await erc1155.connect(owner).mint(owner.address, tokenId, amount, [], { gasLimit: gas });
    await tx.wait();

    // transfer ERC1155 token to futurepass
    gas = await erc1155.connect(owner).estimateGas.safeTransferFrom(owner.address, futurepass, tokenId, amount, []);
    tx = await erc1155
      .connect(owner)
      .safeTransferFrom(owner.address, futurepass, tokenId, amount, [], { gasLimit: gas });
    await tx.wait();
    expect(await erc1155.balanceOf(futurepass, tokenId)).to.equal(amount);

    // mint futurepass a token
    tokenId = 2;
    amount = 5;
    gas = await erc1155.connect(owner).estimateGas.mint(futurepass, tokenId, amount, []);
    tx = await erc1155.connect(owner).mint(futurepass, tokenId, amount, [], { gasLimit: gas });
    await tx.wait();
    expect(await erc1155.balanceOf(futurepass, tokenId)).to.equal(amount);

    // transfer ERC1155 token from futurepass to owner - using proxied, encoded ERC1155 safeTransferFrom call
    const transferAmount = 3;
    const safeTransferFromCallData = erc1155.interface.encodeFunctionData("safeTransferFrom", [
      futurepass,
      owner.address,
      tokenId,
      transferAmount,
      [],
    ]);
    tx = await futurepassProxy
      .connect(owner)
      .proxyCall(futurepass, erc1155.address, CALL_TYPE.Call, safeTransferFromCallData);
    await tx.wait();

    // validate proxy based ERC1155 token transfers
    expect(await erc1155.balanceOf(futurepass, 1)).to.equal(10);
    expect(await erc1155.balanceOf(futurepass, 2)).to.equal(2);
    expect(await erc1155.balanceOf(owner.address, 2)).to.equal(3);
  });

  // TODO: introduce functionality in v2
  it.skip("futurepass can deploy a contract using CREATE", async () => {
    const owner = Wallet.createRandom().connect(provider);

    // fund owner
    await fundEOA(alithSigner, owner.address);

    // create FP for owner
    let tx = await futurepassProxy.connect(alithSigner).create(owner.address);
    await tx.wait();
    const futurepass = await futurepassProxy.futurepassOf(owner.address);

    const erc20Bytecode = MockERC20Data.bytecode;

    // calculate the expected contract address
    const futurepassNonce = await provider.getTransactionCount(futurepass);
    const expectedContractAddress = ethers.utils.getContractAddress({ from: futurepass, nonce: futurepassNonce });

    // call the proxyCall function with the futurepass address and the encoded CREATE call data
    tx = await futurepassProxy
      .connect(owner)
      .proxyCall(futurepass, ethers.constants.AddressZero, CALL_TYPE.Create, erc20Bytecode);
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

    // fund owner
    await fundEOA(alithSigner, owner.address);

    // create FP for owner
    let tx = await futurepassProxy.connect(alithSigner).create(owner.address);
    await tx.wait();
    const futurepass = await futurepassProxy.futurepassOf(owner.address);

    const erc20Bytecode = MockERC20Data.bytecode;

    // Define a salt value for CREATE2
    // const salt = ethers.utils.hexZeroPad(ethers.utils.randomBytes(32), 32);
    // const salt = ethers.utils.hexZeroPad(ethers.utils.hexlify(ethers.BigNumber.from(ethers.utils.randomBytes(32))), 32);
    const salt = ethers.utils.id("1234");
    ethers.utils.getCreate2Address(
      futurepass,
      salt,
      ethers.utils.keccak256(erc20Bytecode),
    );

    // Encode the CREATE2 call to deploy the template contract
    const deployCallData = ethers.utils.hexConcat([ethers.utils.hexZeroPad("0xff", 32), erc20Bytecode, salt]);

    // Call the proxyCall function with the futurepass address and the encoded CREATE2 call data
    tx = await futurepassProxy
      .connect(owner)
      .proxyCall(futurepass, ethers.constants.AddressZero, CALL_TYPE.Create2, deployCallData);
    await tx.wait();

    // // Verify that the created contract has the same bytecode as the template contract
    // const deployedContractBytecode = await provider.getCode(expectedContractAddress);
    // expect(deployedContractBytecode).to.equal(erc20Bytecode);

    // // Verify that the created contract's owner is the futurepass
    // const createdContract = MockERC20Factory.attach(expectedContractAddress);
    // expect(await createdContract.owner()).to.equal(futurepass);
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
