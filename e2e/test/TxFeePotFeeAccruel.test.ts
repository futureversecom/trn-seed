import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Contract, Wallet, utils } from "ethers";
import { ethers } from "hardhat";
import web3 from "web3";

import TestCallData from "../artifacts/contracts/TestCall.sol/TestCall.json";
import { ALITH_PRIVATE_KEY, DEAD_ADDRESS, ERC20_ABI, NodeProcess, startNode, typedefs } from "../common";
import type { TestCall } from "../typechain-types";

const FIRST_ASSET_ID = 1124;

describe("TxFeePot fees accruel", () => {
  let node: NodeProcess;

  let api: ApiPromise;
  let alith: KeyringPair;
  let provider: JsonRpcProvider;
  let alithSigner: Wallet;
  let test: TestCall;
  let xrpInitialIssuance: number;
  let accruedFees: number;

  before(async () => {
    node = await startNode();

    // Substrate variables
    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    // create asset
    await new Promise<void>((resolve) => {
      api.tx.assetsExt.createAsset("test", "TEST", 18, 1, alith.address).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });

    // EVM variables
    provider = new JsonRpcProvider(`http://127.0.0.1:${node.rpcPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider);

    xrpInitialIssuance = +(await api.query.balances.totalIssuance()).toString();

    // common Test Contract deployment to be use by all the tests
    const fees = await provider.getFeeData();
    const factory = new ethers.ContractFactory(TestCallData.abi, TestCallData.bytecode, alithSigner);
    const estimatedGas = await provider.estimateGas(factory.getDeployTransaction());

    test = (await factory.connect(alithSigner).deploy({
      gasLimit: estimatedGas,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    })) as TestCall;
    await test.deployTransaction.wait();
    console.log("TestCall deployed to:", test.address);

    // record the fees now.
    accruedFees = +(await api.query.txFeePot.eraTxFees()).toString();
  });

  after(async () => await node.stop());

  it("Contract creation transaction accrues base fee in TxFeePot", async () => {
    const fees = await provider.getFeeData();
    accruedFees = +(await api.query.txFeePot.eraTxFees()).toString();

    const factory = new ethers.ContractFactory(TestCallData.abi, TestCallData.bytecode, alithSigner);
    const estimatedGas = await provider.estimateGas(factory.getDeployTransaction());

    test = (await factory.connect(alithSigner).deploy({
      gasLimit: estimatedGas,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    })) as TestCall;
    const receipt = await test.deployTransaction.wait();
    console.log("TestCall deployed to:", test.address);

    const feesFromContractDeployment = fees
      .lastBaseFeePerGas!.mul(receipt.gasUsed)
      .div(10 ** 12)
      .toNumber();
    const currentAccruedFees = +(await api.query.txFeePot.eraTxFees()).toString();
    expect(currentAccruedFees - accruedFees)
      .to.be.greaterThanOrEqual(feesFromContractDeployment)
      .and.lessThanOrEqual(feesFromContractDeployment + 1); // account for rounding errors
  });

  it("Contract call transaction accrues base fee in TxFeePot", async () => {
    const fees = await provider.getFeeData();
    accruedFees = +(await api.query.txFeePot.eraTxFees()).toString();

    const gasEstimate = await test.estimateGas.set(1, {
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    });
    const tx = await test.set(1, {
      gasLimit: gasEstimate,
      maxFeePerGas: fees.lastBaseFeePerGas!,
      maxPriorityFeePerGas: 0,
    });
    const receipt = await tx.wait();

    const feesFromContractCall = fees
      .lastBaseFeePerGas!.mul(receipt.gasUsed)
      .div(10 ** 12)
      .toNumber();
    const currentAccruedFees = +(await api.query.txFeePot.eraTxFees()).toString();
    expect(currentAccruedFees - accruedFees)
      .to.be.greaterThanOrEqual(feesFromContractCall)
      .and.lessThanOrEqual(feesFromContractCall + 1); // account for rounding errors
  });

  // This should not exist here but the tests are failing without it :(
  it("Extrinsic transactions accrue fee in TxFeePot", async () => {
    accruedFees = +(await api.query.txFeePot.eraTxFees()).toString();

    const tx = api.tx.assets.mint(
      // mint 1M tokens (18 decimals) to alith
      FIRST_ASSET_ID,
      alith.address,
      utils.parseEther("1").toString(),
    );
    await new Promise<void>((resolve) => {
      tx.signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });

    const feesFromExtrinsicLower = 310_000,
      feesFromExtrinsicUpper = 330_000;
    const currentAccruedFees = +(await api.query.txFeePot.eraTxFees()).toString();
    expect(currentAccruedFees - accruedFees)
      .to.be.greaterThan(feesFromExtrinsicLower)
      .and.lessThan(feesFromExtrinsicUpper);
  });

  it("Pre-compile contract transaction accrues base fee in TxFeePot", async () => {
    const fees = await provider.getFeeData();
    accruedFees = +(await api.query.txFeePot.eraTxFees()).toString();

    const erc20Token = FIRST_ASSET_ID.toString(16).padStart(8, "0");
    const erc20TokenAddress = web3.utils.toChecksumAddress(`0xCCCCCCCC${erc20Token}000000000000000000000000`);
    const erc20 = new Contract(erc20TokenAddress, ERC20_ABI, alithSigner);
    const gasEstimate = await erc20.estimateGas.transfer(DEAD_ADDRESS, 1, {
      maxFeePerGas: fees.lastBaseFeePerGas!,
    });
    const tx = await erc20.transfer(DEAD_ADDRESS, 1, {
      gasLimit: gasEstimate,
      maxFeePerGas: fees.lastBaseFeePerGas!,
    });
    const receipt = await tx.wait();

    const feesFromPrecompile = fees
      .lastBaseFeePerGas!.mul(receipt.gasUsed)
      .div(10 ** 12)
      .toNumber();
    const currentAccruedFees = +(await api.query.txFeePot.eraTxFees()).toString();
    expect(currentAccruedFees - accruedFees)
      .to.be.greaterThanOrEqual(feesFromPrecompile)
      .and.lessThanOrEqual(feesFromPrecompile + 1); // account for rounding errors
  });

  it("XRP total issuance remains unchanged", async () => {
    const totalIssuance = +(await api.query.balances.totalIssuance()).toString();
    expect(totalIssuance).to.equal(xrpInitialIssuance);
  });
});
