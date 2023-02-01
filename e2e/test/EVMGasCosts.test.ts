import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Wallet, utils } from "ethers";
import { ethers } from "hardhat";
import web3 from "web3";

import MockERC20Data from "../artifacts/contracts/MockERC20.sol/MockERC20.json";
import { ALITH_PRIVATE_KEY, BOB_PRIVATE_KEY, DEAD_ADDRESS, NodeProcess, startNode, typedefs } from "../common";
import type { MockERC20 } from "../typechain-types";

const FIRST_ASSET_ID = 1124;
const BASE_GAS_COST = 21_000;
const BASE_FEE_PER_GAS = 15_000_000_000_000;
const PRIORITY_FEE_PER_GAS = 1_500_000_000;
const MAX_FEE_PER_GAS = BASE_FEE_PER_GAS * 2 + PRIORITY_FEE_PER_GAS;

// Note: Tests must be run in order, synchronously
describe("EVM gas costs", () => {
  let node: NodeProcess;

  let api: ApiPromise;
  let alith: KeyringPair;

  let provider: JsonRpcProvider;
  let alithSigner: Wallet;
  let bobSigner: Wallet;
  let erc20Contract: MockERC20;

  before(async () => {
    node = await startNode();

    // Substrate variables
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    api = await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
    });
    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    // create and mint asset - ID 1124
    const txs = [
      api.tx.assetsExt.createAsset("test", "TEST", 18, 1, alith.address),
      api.tx.assets.mint(FIRST_ASSET_ID, alith.address, utils.parseEther("1000").toString()),
    ];
    await new Promise<void>((resolve) => {
      api.tx.utility.batch(txs).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    console.log("Created and minted asset:", FIRST_ASSET_ID);

    // EVM variables
    provider = new JsonRpcProvider(`http://localhost:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider);
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);
  });

  after(async () => await node.stop());

  it("default gas fees", async () => {
    const fees = await provider.getFeeData();
    expect(fees.lastBaseFeePerGas?.toNumber()).to.eql(BASE_FEE_PER_GAS); // base fee = 15000 gwei
    expect(fees.maxFeePerGas?.toNumber()).to.eql(MAX_FEE_PER_GAS);
    expect(fees.maxPriorityFeePerGas?.toNumber()).to.eql(PRIORITY_FEE_PER_GAS);
    expect(fees.gasPrice?.toNumber()).to.eql(BASE_FEE_PER_GAS);
  });

  it("gas cost for evm call", async () => {
    const fees = await provider.getFeeData();
    const nonce = await alithSigner.getTransactionCount();
    const unsignedTx = {
      // eip1559 tx
      type: 2,
      from: alithSigner.address,
      to: bobSigner.address,
      nonce,
      data: "",
      gasLimit: BASE_GAS_COST,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
      chainId: 7672,
    };
    const signedTx = await alithSigner.signTransaction(unsignedTx);
    const tx = await provider.sendTransaction(signedTx);
    const receipt = await tx.wait();

    // assert gas used
    expect(receipt.gasUsed?.toNumber()).to.eql(BASE_GAS_COST);
    expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(BASE_GAS_COST);
    expect(receipt.effectiveGasPrice?.toNumber()).to.eql(BASE_FEE_PER_GAS);

    // assert XRP used
    const xrpGasCost = receipt.gasUsed.mul(receipt.effectiveGasPrice);
    const xrpCost6DP = xrpGasCost.div(10 ** 12).toNumber();
    const xrpCostScaled = +utils.formatEther(xrpGasCost);
    expect(xrpCost6DP).to.eql(315000);
    expect(xrpCostScaled).to.eql(0.315);
  });

  it("gas cost for evm call with manually set base fee", async () => {
    const manuallySetBaseFee = 42_000_000_000;
    const setEvmBaseFeeTx = api.tx.feeControl.setEvmBaseFee(manuallySetBaseFee);
    const sudoWrapper = api.tx.sudo.sudo(setEvmBaseFeeTx);

    await new Promise<void>((resolve) => {
      sudoWrapper.signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });

    const newBaseFeeSetting = await api.query.feeControl.evmBaseFeePerGas();
    expect(newBaseFeeSetting).to.be.eq(manuallySetBaseFee);

    const fees = await provider.getFeeData();
    expect(fees.lastBaseFeePerGas).to.be.eq(manuallySetBaseFee);

    const nonce = await alithSigner.getTransactionCount();
    const unsignedTx = {
      // eip1559 tx
      type: 2,
      from: alithSigner.address,
      to: bobSigner.address,
      nonce,
      data: "",
      gasLimit: BASE_GAS_COST,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
      chainId: 7672,
    };
    const signedTx = await alithSigner.signTransaction(unsignedTx);
    const tx = await provider.sendTransaction(signedTx);
    const receipt = await tx.wait();

    // assert gas used
    expect(receipt.gasUsed?.toNumber()).to.eql(BASE_GAS_COST);
    expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(BASE_GAS_COST);
    // Should be the same as the manually set fee
    expect(receipt.effectiveGasPrice?.toNumber()).to.eql(manuallySetBaseFee);

    // assert XRP used
    const xrpGasCost = receipt.gasUsed.mul(receipt.effectiveGasPrice);
    const xrpCost6DP = xrpGasCost.div(10 ** 12).toNumber();
    const xrpCostScaled = +utils.formatEther(xrpGasCost);
    expect(xrpCost6DP).to.eql(882);
    expect(xrpCostScaled).to.eql(0.000882);

    // Reset base fee back to original state so as not to affect other tests
    const resetEvmBaseFeeTx = api.tx.feeControl.setEvmBaseFee(BASE_FEE_PER_GAS);
    await new Promise<void>((resolve) => {
      api.tx.sudo.sudo(resetEvmBaseFeeTx).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
  });

  it("gas cost for XRP transfer", async () => {
    const fees = await provider.getFeeData();
    const alithBalanceBefore = await alithSigner.getBalance();

    const tx = await alithSigner.sendTransaction({
      to: DEAD_ADDRESS,
      value: utils.parseEther("1"),
      gasLimit: BASE_GAS_COST,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0, // no miner tip
    });
    const receipt = await tx.wait();
    expect(receipt.gasUsed?.toNumber()).to.eql(BASE_GAS_COST);
    expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(BASE_GAS_COST);

    // assert gas used
    const totalPaid = receipt.effectiveGasPrice?.mul(BASE_GAS_COST).add(utils.parseEther("1"));
    const alithBalanceAfter = await alithSigner.getBalance();
    expect(alithBalanceBefore.sub(alithBalanceAfter)).to.eql(totalPaid);

    // assert XRP used
    const oneXRP6DP = 1_000_000,
      oneXRPScaled = 1;
    const xrpCost6DP = totalPaid.div(10 ** 12).toNumber() - oneXRP6DP; // subtract XRP sent
    const xrpCostScaled = +utils.formatEther(totalPaid) - oneXRPScaled; // subtract XRP sent
    expect(xrpCost6DP).to.eql(315000);
    expect(+xrpCostScaled.toFixed(3)).to.eql(0.315);
  });

  it("gas cost for XRP transfer with a set base fee of zero", async () => {
    const setEvmBaseFeeTx = api.tx.feeControl.setEvmBaseFee(0);
    const sudoWrapper = api.tx.sudo.sudo(setEvmBaseFeeTx);
    await new Promise<void>((resolve) => {
      sudoWrapper.signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });

    // Check newly set base fee
    const newBaseFeeSetting = await api.query.feeControl.evmBaseFeePerGas();
    expect(newBaseFeeSetting).to.be.eq(0);

    const fees = await provider.getFeeData();
    expect(fees.lastBaseFeePerGas).to.be.eq(0);

    const alithBalanceBefore = await alithSigner.getBalance();

    const tx = await alithSigner.sendTransaction({
      to: DEAD_ADDRESS,
      value: utils.parseEther("1"),
      gasLimit: BASE_GAS_COST,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0, // no miner tip
    });
    const receipt = await tx.wait();
    expect(receipt.gasUsed?.toNumber()).to.eql(BASE_GAS_COST);
    expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(BASE_GAS_COST);

    // assert gas used
    const totalPaid = utils.parseEther("1");
    const alithBalanceAfter = await alithSigner.getBalance();
    expect(alithBalanceBefore.sub(alithBalanceAfter)).to.eql(totalPaid);

    // assert XRP used
    const oneXRP6DP = 1_000_000,
      oneXRPScaled = 1;
    const xrpCost6DP = totalPaid.div(10 ** 12).toNumber() - oneXRP6DP; // subtract XRP sent
    const xrpCostScaled = +utils.formatEther(totalPaid) - oneXRPScaled; // subtract XRP sent
    expect(xrpCost6DP).to.eql(0);
    expect(+xrpCostScaled.toFixed(3)).to.eql(0);

    // Reset base fee back to original state so as not to affect other tests
    const resetEvmBaseFeeTx = api.tx.feeControl.setEvmBaseFee(BASE_FEE_PER_GAS);
    await new Promise<void>((resolve) => {
      api.tx.sudo.sudo(resetEvmBaseFeeTx).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
  });

  it("gas cost for deploying erc20 contract", async () => {
    const fees = await provider.getFeeData();
    const alithBalanceBefore = await alithSigner.getBalance();

    const factory = new ethers.ContractFactory(MockERC20Data.abi, MockERC20Data.bytecode, alithSigner);
    const actualGasEstimate = await provider.estimateGas(factory.getDeployTransaction());
    erc20Contract = (await factory.connect(alithSigner).deploy({
      gasLimit: actualGasEstimate,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    })) as MockERC20;
    const receipt = await erc20Contract.deployTransaction.wait();
    console.log("erc20Contract deployed to:", erc20Contract.address);

    // assert gas used
    const totalPaid = receipt.effectiveGasPrice?.mul(actualGasEstimate);
    const alithBalanceAfter = await alithSigner.getBalance();
    expect(alithBalanceBefore.sub(alithBalanceAfter)).to.eql(totalPaid);

    // assert XRP used
    const xrpCost6DP = totalPaid.div(10 ** 12).toNumber();
    const xrpCostScaled = +utils.formatEther(totalPaid);
    expect(xrpCost6DP).to.be.greaterThanOrEqual(52_500_000).and.lessThanOrEqual(52_600_000);
    expect(xrpCostScaled).to.be.greaterThanOrEqual(52.5).and.lessThanOrEqual(52.6);
  });

  it("gas cost for token mint", async () => {
    const fees = await provider.getFeeData();
    const alithBalanceBefore = await alithSigner.getBalance();

    const wantGasEstimateLower = 75_000,
      wantGasEstimateUpper = 75_500;
    const actualGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.mint(alithSigner.address, 1000, {
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    });
    expect(actualGasEstimate.toNumber()).to.be.greaterThan(wantGasEstimateLower).and.lessThan(wantGasEstimateUpper);

    const tx = await erc20Contract.connect(alithSigner).mint(alithSigner.address, 1000, {
      gasLimit: actualGasEstimate,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    });
    const receipt = await tx.wait();

    // assert gas used
    const wantGasUsedLower = 71_000,
      wantGasUsedUpper = 71_500;
    expect(receipt.gasUsed?.toNumber()).to.be.greaterThan(wantGasUsedLower).and.lessThan(wantGasUsedUpper);
    expect(receipt.cumulativeGasUsed?.toNumber()).to.be.greaterThan(wantGasUsedLower).and.lessThan(wantGasUsedUpper);

    const totalPaid = receipt.effectiveGasPrice?.mul(actualGasEstimate);
    const alithBalanceAfter = await alithSigner.getBalance();
    expect(alithBalanceBefore.sub(alithBalanceAfter)).to.eql(totalPaid);

    // assert XRP used
    const xrpCost6DP = totalPaid.div(10 ** 12).toNumber();
    const xrpCostScaled = +utils.formatEther(totalPaid);
    expect(xrpCost6DP).to.be.greaterThanOrEqual(1_125_000).and.lessThanOrEqual(1_135_000);
    expect(xrpCostScaled).to.be.greaterThanOrEqual(1.125).and.lessThanOrEqual(1.135);
  });

  it("gas cost for token transfer", async () => {
    const fees = await provider.getFeeData();
    const alithBalanceBefore = await alithSigner.getBalance();

    const wantGasEstimateLower = 50_500,
      wantGasEstimateUpper = 51_000;
    const actualGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.transfer(bobSigner.address, 500, {
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    });
    expect(actualGasEstimate.toNumber()).to.be.greaterThan(wantGasEstimateLower).and.lessThan(wantGasEstimateUpper);

    const tx = await erc20Contract.connect(alithSigner).transfer(bobSigner.address, 500, {
      gasLimit: actualGasEstimate,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    });
    const receipt = await tx.wait();

    // assert gas used
    const wantGasUsedLower = 49_000,
      wantGasUsedUpper = 49_500;
    expect(receipt.gasUsed?.toNumber()).to.be.greaterThan(wantGasUsedLower).and.lessThan(wantGasUsedUpper);
    expect(receipt.cumulativeGasUsed?.toNumber()).to.be.greaterThan(wantGasUsedLower).and.lessThan(wantGasUsedUpper);

    const totalPaid = receipt.effectiveGasPrice?.mul(actualGasEstimate);
    const alithBalanceAfter = await alithSigner.getBalance();
    expect(alithBalanceBefore.sub(alithBalanceAfter)).to.eql(totalPaid);

    // assert XRP used
    const xrpCost6DP = totalPaid.div(10 ** 12).toNumber();
    const xrpCostScaled = +utils.formatEther(totalPaid);
    expect(xrpCost6DP).to.be.greaterThanOrEqual(761_000).and.lessThanOrEqual(765_000);
    expect(xrpCostScaled).to.be.greaterThanOrEqual(0.761).and.lessThanOrEqual(0.765);
  });

  it("gas cost for pre-compile token transfer", async () => {
    // connect to erc20 precompile
    const erc20Token = FIRST_ASSET_ID.toString(16).padStart(8, "0");
    const erc20TokenAddress = web3.utils.toChecksumAddress(`0xCCCCCCCC${erc20Token}000000000000000000000000`);
    const ERC20Factory = await ethers.getContractFactory("MockERC20");
    const erc20PrecompileContract = ERC20Factory.connect(alithSigner).attach(erc20TokenAddress);

    // transfer token via precompile tests
    const fees = await provider.getFeeData();
    const alithBalanceBefore = await alithSigner.getBalance();

    const wantGasEstimate = 23_243;
    const actualGasEstimate = await erc20PrecompileContract
      .connect(alithSigner)
      .estimateGas.transfer(bobSigner.address, 500, {
        maxFeePerGas: fees.lastBaseFeePerGas!,
        maxPriorityFeePerGas: 0,
      });
    expect(actualGasEstimate.toNumber()).to.eql(wantGasEstimate);

    const tx = await erc20PrecompileContract.connect(alithSigner).transfer(bobSigner.address, 500, {
      gasLimit: actualGasEstimate,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    });
    const receipt = await tx.wait();

    // assert gas used
    const wantGasUsed = 22_953;
    expect(receipt.gasUsed?.toNumber()).to.eql(wantGasUsed);
    expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(wantGasUsed);

    const totalPaid = receipt.effectiveGasPrice?.mul(actualGasEstimate);
    const alithBalanceAfter = await alithSigner.getBalance();
    expect(alithBalanceBefore.sub(alithBalanceAfter)).to.eql(totalPaid);

    // assert XRP used
    const xrpCost6DP = totalPaid.div(10 ** 12).toNumber();
    const xrpCostScaled = +utils.formatEther(totalPaid);
    expect(xrpCost6DP).to.eql(348_645);
    expect(xrpCostScaled).to.eql(0.348645);
  });
});
