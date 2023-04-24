/* import { JsonRpcProvider } from "@ethersproject/providers";
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

    provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed

    const TestFactory = await ethers.getContractFactory("TestCall");
    test = await TestFactory.connect(alithSigner).deploy();
    await test.deployed();
    // test = TestFactory.connect(alithSigner).attach('0x14726BAf58C847469E57e5e04adE7837bcef8D77');
    console.log("TestCall deployed to:", test.address);

    const TestProxyFactory = await ethers.getContractFactory("TestCallProxy");
    testProxy = await TestProxyFactory.connect(alithSigner).deploy(test.address);
    await testProxy.deployed();
    // testProxy = TestProxyFactory.connect(alithSigner).attach('0x14726BAf58C847469E57e5e04adE7837bcef8D77');
    console.log("TestCallProxy deployed to:", testProxy.address);
  });

  after(async () => await node.stop());

  it("TestCall:set() estimates and uses ~43_000-46_000 gas", async () => {
    const gas = await test.estimateGas.set(1);
    expect(gas).to.be.greaterThan(43_000).and.lessThan(46_000);
    const tx = await test.set(1, { gasLimit: gas });
    const receipt = await tx.wait();
    expect(receipt.gasUsed.toNumber()).to.be.greaterThan(43_000).and.lessThan(46_000);
  });

  it("TestCallProxy:set() estimates and uses ~28_000-33_000 gas", async () => {
    const gas = await testProxy.estimateGas.set(2);
    expect(gas).to.be.greaterThan(28_000).and.lessThan(33_000);
    const tx = await testProxy.set(2, { gasLimit: gas });
    const receipt = await tx.wait();
    expect(receipt.gasUsed.toNumber()).to.be.greaterThan(28_000).and.lessThan(33_000);
  });

  it("TestCallProxy:setWithAddress() estimates and uses ~27_000-30_000 gas", async () => {
    const gas = await testProxy.estimateGas.setWithAddress(test.address, 3);
    expect(gas).to.be.greaterThan(27_000).and.lessThan(30_000);
    const tx = await testProxy.setWithAddress(test.address, 3, { gasLimit: gas });
    const receipt = await tx.wait();
    expect(receipt.gasUsed.toNumber()).to.be.greaterThan(27_000).and.lessThan(30_000);
  });
});
 */