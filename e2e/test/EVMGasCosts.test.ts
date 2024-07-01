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

  before(async () => {
    node = await startNode();

    // Substrate variables
    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
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
    provider = new JsonRpcProvider(`http://127.0.0.1:${node.rpcPort}`);
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
    const totalFeePaid = receipt.effectiveGasPrice?.mul(BASE_GAS_COST);
    const totalPaid = totalFeePaid.add(utils.parseEther("1"));
    const alithBalanceAfter = await alithSigner.getBalance();
    expect(alithBalanceBefore.sub(alithBalanceAfter)).to.eql(totalPaid);

    // assert XRP used
    const xrpCost6DP = totalFeePaid.div(10 ** 12).toNumber();
    const xrpCostScaled = +utils.formatEther(totalFeePaid);
    expect(xrpCost6DP).to.eql(315000);
    expect(+xrpCostScaled.toFixed(3)).to.eql(0.315);
  });

  it("gas cost for deploying erc20 contract", async () => {
    const fees = await provider.getFeeData();

    const alithBalanceBefore = await alithSigner.getBalance();

    const factory = new ethers.ContractFactory(MockERC20Data.abi, MockERC20Data.bytecode, alithSigner);
    const actualGasEstimate = await provider.estimateGas(factory.getDeployTransaction());
    const estimatedTxCost = actualGasEstimate.mul(fees.gasPrice!);
    const erc20Contract = (await factory.connect(alithSigner).deploy({
      gasLimit: actualGasEstimate,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    })) as MockERC20;
    const receipt = await erc20Contract.deployTransaction.wait();
    console.log("erc20Contract deployed to:", erc20Contract.address);

    // assert gas used
    const FeePaidUpfront = receipt.effectiveGasPrice?.mul(actualGasEstimate);
    const actualCost = receipt.effectiveGasPrice?.mul(receipt.gasUsed);
    const refund = receipt.effectiveGasPrice?.mul(actualGasEstimate.sub(receipt.gasUsed));
    expect(estimatedTxCost).to.eql(FeePaidUpfront);
    expect(actualCost).to.eql(FeePaidUpfront?.sub(refund));
    const alithBalanceAfter = await alithSigner.getBalance();
    expect(alithBalanceBefore.sub(alithBalanceAfter)).to.eql(actualCost);

    // assert XRP used
    const xrpCost6DP = actualCost.div(10 ** 12).toNumber();
    const xrpCostScaled = +utils.formatEther(actualCost);
    expect(xrpCost6DP).to.eql(21810720);
    expect(+xrpCostScaled.toFixed(6)).to.eql(21.81072);
  });

  it("gas cost for token mint", async () => {
    const fees = await provider.getFeeData();

    //create deploy erc20 first
    const factory = new ethers.ContractFactory(MockERC20Data.abi, MockERC20Data.bytecode, alithSigner);
    const deployGasEstimate = await provider.estimateGas(factory.getDeployTransaction());
    const erc20Contract = (await factory.connect(alithSigner).deploy({
      gasLimit: deployGasEstimate,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    })) as MockERC20;
    await erc20Contract.deployTransaction.wait();
    console.log("erc20Contract deployed to:", erc20Contract.address);

    // mint
    const alithBalanceBefore = await alithSigner.getBalance();
    const wantGasEstimateLower = 75_000,
      wantGasEstimateUpper = 75_500;
    const actualGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.mint(alithSigner.address, 1000, {
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    });
    const estimatedTxCost = actualGasEstimate.mul(fees.gasPrice!);
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

    const feePaidUpFront = receipt.effectiveGasPrice?.mul(actualGasEstimate);
    const actualFee = receipt.effectiveGasPrice?.mul(receipt.gasUsed);
    const refund = receipt.effectiveGasPrice?.mul(actualGasEstimate.sub(receipt.gasUsed));
    expect(estimatedTxCost).to.eql(feePaidUpFront);
    expect(actualFee).to.eql(feePaidUpFront?.sub(refund));
    const alithBalanceAfter = await alithSigner.getBalance();
    expect(alithBalanceBefore.sub(alithBalanceAfter)).to.eql(actualFee);

    // assert XRP used
    const xrpCost6DP = actualFee.div(10 ** 12).toNumber();
    const xrpCostScaled = +utils.formatEther(actualFee);
    expect(xrpCost6DP).to.eql(1068135);
    expect(+xrpCostScaled.toFixed(6)).to.eql(1.068135);
  });

  it("gas cost for token transfer", async () => {
    const fees = await provider.getFeeData();

    //create deploy erc20 first
    const factory = new ethers.ContractFactory(MockERC20Data.abi, MockERC20Data.bytecode, alithSigner);
    const deployGasEstimate = await provider.estimateGas(factory.getDeployTransaction());
    const erc20Contract = (await factory.connect(alithSigner).deploy({
      gasLimit: deployGasEstimate,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    })) as MockERC20;
    await erc20Contract.deployTransaction.wait();
    console.log("erc20Contract deployed to:", erc20Contract.address);

    // mint some tokens
    const mintGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.mint(alithSigner.address, 1000, {
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    });
    const mintTx = await erc20Contract.connect(alithSigner).mint(alithSigner.address, 1000, {
      gasLimit: mintGasEstimate,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    });
    await mintTx.wait();

    // transfer
    const alithBalanceBefore = await alithSigner.getBalance();

    const actualGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.transfer(bobSigner.address, 500, {
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    });
    const estimatedTxCost = actualGasEstimate.mul(fees.gasPrice!);
    expect(actualGasEstimate.toNumber()).to.equal(52707);

    const tx = await erc20Contract.connect(alithSigner).transfer(bobSigner.address, 500, {
      gasLimit: actualGasEstimate,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    });
    const receipt = await tx.wait();

    // assert gas used
    expect(receipt.gasUsed?.toNumber()).to.equal(52095);
    expect(receipt.cumulativeGasUsed?.toNumber()).to.be.greaterThanOrEqual(52095);

    const feePaidUpFront = receipt.effectiveGasPrice?.mul(actualGasEstimate);
    const actualFee = receipt.effectiveGasPrice?.mul(receipt.gasUsed);
    const refund = receipt.effectiveGasPrice?.mul(actualGasEstimate.sub(receipt.gasUsed));
    expect(estimatedTxCost).to.eql(feePaidUpFront);
    expect(actualFee).to.eql(feePaidUpFront?.sub(refund));
    const alithBalanceAfter = await alithSigner.getBalance();
    expect(alithBalanceBefore.sub(alithBalanceAfter)).to.eql(actualFee);

    // assert XRP used
    const xrpCost6DP = actualFee.div(10 ** 12).toNumber();
    const xrpCostScaled = +utils.formatEther(actualFee);
    expect(xrpCost6DP).to.eql(781425);
    expect(+xrpCostScaled.toFixed(6)).to.eql(0.781425);
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

    const wantGasEstimate = 44_142;
    const actualGasEstimate = await erc20PrecompileContract
      .connect(alithSigner)
      .estimateGas.transfer(bobSigner.address, 500, {
        maxFeePerGas: fees.lastBaseFeePerGas!,
        maxPriorityFeePerGas: 0,
      });
    const estimatedTxCost = actualGasEstimate.mul(fees.gasPrice!);
    expect(actualGasEstimate.toNumber()).to.eql(wantGasEstimate);

    const tx = await erc20PrecompileContract.connect(alithSigner).transfer(bobSigner.address, 500, {
      gasLimit: actualGasEstimate,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    });
    const receipt = await tx.wait();

    // assert gas used
    const wantGasUsed = 42265;
    expect(receipt.gasUsed?.toNumber()).to.eql(wantGasUsed);
    expect(receipt.cumulativeGasUsed?.toNumber()).to.eql(wantGasUsed);

    const feePaidUpFront = receipt.effectiveGasPrice?.mul(actualGasEstimate);
    const actualFee = receipt.effectiveGasPrice?.mul(receipt.gasUsed);
    const refund = receipt.effectiveGasPrice?.mul(actualGasEstimate.sub(receipt.gasUsed));
    expect(estimatedTxCost).to.eql(feePaidUpFront);
    expect(actualFee).to.eql(feePaidUpFront?.sub(refund));
    const alithBalanceAfter = await alithSigner.getBalance();
    expect(alithBalanceBefore.sub(alithBalanceAfter)).to.eql(actualFee);

    // assert XRP used
    const xrpCost6DP = actualFee.div(10 ** 12).toNumber();
    const xrpCostScaled = +utils.formatEther(actualFee);
    expect(xrpCost6DP).to.eql(633975);
    expect(+xrpCostScaled.toFixed(6)).to.eql(0.633975);
  });
});
