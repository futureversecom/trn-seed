import { expect } from "chai";
import { ethers } from 'hardhat';
import { Wallet } from 'ethers';
import { JsonRpcProvider } from "@ethersproject/providers";
import { ALICE_PRIVATE_KEY } from "../common";
import type { TestCall, TestCallProxy } from '../typechain-types';

describe("TestCall", () => {
	let provider: JsonRpcProvider;
	let aliceSigner: Wallet;
  let test: TestCall;
  let testProxy: TestCallProxy;

  before(async () => {
    provider = new JsonRpcProvider(`http://localhost:9933`);
    aliceSigner = new Wallet(ALICE_PRIVATE_KEY).connect(provider); // 'development' seed

    const TestFactory = await ethers.getContractFactory('TestCall');
    test = await TestFactory.connect(aliceSigner).deploy();
    await test.deployed();
    // test = TestFactory.connect(aliceSigner).attach('0x14726BAf58C847469E57e5e04adE7837bcef8D77');
    console.log('TestCall deployed to:', test.address);

    const TestProxyFactory = await ethers.getContractFactory('TestCallProxy');
    testProxy = await TestProxyFactory.connect(aliceSigner).deploy(test.address);
    await testProxy.deployed();
    // testProxy = TestProxyFactory.connect(aliceSigner).attach('0x14726BAf58C847469E57e5e04adE7837bcef8D77');
    console.log('TestCallProxy deployed to:', testProxy.address);
	});

  it("TestCall:set() estimates and uses ~40,000-50,000 gas", async () => {
		const gas = await test.estimateGas.set(1);
    expect(gas).to.be.greaterThan(43000).and.lessThan(46000);
    const tx = await test.set(1);
    const receipt = await tx.wait();
    expect(receipt.gasUsed.toNumber()).to.be.greaterThan(43000).and.lessThan(46000);
	});

  it("TestCallProxy:set() estimates and uses ~29,000-32,000 gas", async () => {
    const gas = await testProxy.estimateGas.set(2);
    expect(gas).to.be.greaterThan(29000).and.lessThan(32000);
    const tx = await testProxy.set(2);
    const receipt = await tx.wait();
    expect(receipt.gasUsed.toNumber()).to.be.greaterThan(29000).and.lessThan(32000);
	});

  it("TestCallProxy:setWithAddress() estimates and uses ~27,000-29,000 gas", async () => {
    const gas = await testProxy.estimateGas.setWithAddress(test.address, 3);
    expect(gas).to.be.greaterThan(27000).and.lessThan(29000);
    const tx = await testProxy.setWithAddress(test.address, 3);
    const receipt = await tx.wait();
    expect(receipt.gasUsed.toNumber()).to.be.greaterThan(27000).and.lessThan(29000);
	});
});
