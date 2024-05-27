import { JsonRpcProvider } from "@ethersproject/providers";
import { expect } from "chai";
import { Wallet } from "ethers";
import { ethers } from "hardhat";

import { ALITH_PRIVATE_KEY, NodeProcess, startNode } from "../common";
import type { TestCall, TestCallProxy } from "../typechain-types";

describe("TestCall", () => {
  let node: NodeProcess;

  let provider: JsonRpcProvider;
  let alithSigner: Wallet;
  let test: TestCall;
  let testProxy: TestCallProxy;

  before(async () => {
    node = await startNode();

    await node.wait(); // wait for the node to be ready

    provider = new JsonRpcProvider(`http://127.0.0.1:${node.rpcPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider);

    const TestFactory = await ethers.getContractFactory("TestCall");
    test = await TestFactory.connect(alithSigner).deploy();
    await test.deployed();
    // test = TestFactory.connect(alithSigner).attach("0xc01Ee7f10EA4aF4673cFff62710E1D7792aBa8f3");
    // console.log("TestCall deployed to:", test.address);

    const TestProxyFactory = await ethers.getContractFactory("TestCallProxy");
    testProxy = await TestProxyFactory.connect(alithSigner).deploy(test.address);
    await testProxy.deployed();
    // testProxy = TestProxyFactory.connect(alithSigner).attach("0x970951a12F975E6762482ACA81E57D5A2A4e73F4");
    // console.log("TestCallProxy deployed to:", testProxy.address);
  });

  after(async () => await node.stop());

  it("ensure TestCall contract bytecode is specific size for tests", async () => {
    // Note: Changing the TestCall contract will change the bytecode size - which will result in different gas estimates
    const bytecode = await provider.getCode(test.address);
    expect(bytecode.length).to.eq(1136);
  });

  it("ensure TestCallProxy contract bytecode is specific size for tests", async () => {
    // Note: Changing the TestCallProxy contract will change the bytecode size - which will result in different gas estimates
    const bytecode = await provider.getCode(testProxy.address);
    expect(bytecode.length).to.eq(3260);
  });

  it("TestCall:set() estimates 45_085 gas and uses 43_702 gas", async () => {
    const gas = await test.estimateGas.set(1);
    expect(gas).to.eq(45_085);
    const tx = await test.set(1, { gasLimit: gas });
    const receipt = await tx.wait();
    expect(receipt.gasUsed).to.eq(43_702);
  });

  // dependent on TestCall:set()
  it("TestCallProxy:set() estimates 31_479 gas and uses 29_357 gas", async () => {
    const gas = await testProxy.estimateGas.set(2);
    expect(gas).to.eq(32_791);
    const tx = await testProxy.set(2, { gasLimit: gas });
    const receipt = await tx.wait();
    expect(receipt.gasUsed).to.eq(32_157);
  });

  // dependent on TestCall:set()
  // dependent on TestCallProxy:set()
  it("TestCallProxy:setWithAddress() estimates 28_355 gas and uses 27_923 gas", async () => {
    const gas = await testProxy.estimateGas.setWithAddress(test.address, 3);
    expect(gas).to.eq(32_120);
    const tx = await testProxy.setWithAddress(test.address, 3, { gasLimit: gas });
    const receipt = await tx.wait();
    expect(receipt.gasUsed).to.eq(30_723);
  });

  // dependent on TestCall:set()
  it("TestCall:get() estimates 23_706 gas", async () => {
    const gas = await test.estimateGas.get();
    expect(gas).to.eq(23_706);
  });

  // dependent on TestCall:set()
  it("TestCallProxy:get() estimates 29_125 gas", async () => {
    const gas = await testProxy.estimateGas.get();
    expect(gas).to.eq(29_125);
  });

  it("TestCall:deposit() estimates 44_980 gas and uses 43_542 gas", async () => {
    const gas = await test.estimateGas.deposit({ value: ethers.utils.parseEther("1") });
    expect(gas).to.eq(44_980);
    const tx = await test.deposit({ gasLimit: gas, value: ethers.utils.parseEther("1") });
    const receipt = await tx.wait();
    expect(receipt.gasUsed).to.eq(43_542);
  });

  // dependent on TestCall:deposit()
  it("TestCallProxy:deposit() estimates 39_705 gas and uses 23_642 gas", async () => {
    const gas = await testProxy.estimateGas.deposit({ value: ethers.utils.parseEther("1") });
    expect(gas).to.eq(38_424);
    const tx = await test.deposit({ gasLimit: gas, value: ethers.utils.parseEther("1") });
    const receipt = await tx.wait();
    // gas estimates for payable (eth-forwarding) functions have a larger discrepancy in actual tx gas usage
    expect(receipt.gasUsed).to.eq(26_442);

    // ensure TestCallProxy contract has no ether (all ether was forwarded to TestCall contract)
    expect(await provider.getBalance(testProxy.address)).to.eq(0);

    // ensure TestCall contract has 2 ether (1 ether from prev test, 1 from this test)
    expect(await provider.getBalance(test.address)).to.eq(ethers.utils.parseEther("2"));
  });
});
