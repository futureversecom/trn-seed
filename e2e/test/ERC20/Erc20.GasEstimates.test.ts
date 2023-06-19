import { JsonRpcProvider } from "@ethersproject/providers";
import { expect } from "chai";
import { Contract, Wallet } from "ethers";
import { ethers } from "hardhat";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC20_ABI,
  GAS_TOKEN_ID,
  GasCosts,
  NodeProcess,
  assetIdToERC20ContractAddress,
  saveGasCosts,
  startNode,
} from "../../common";
import { MockERC20 } from "../../typechain-types";

describe("ERC20 Gas Estimates", function () {
  let node: NodeProcess;

  let provider: JsonRpcProvider;
  let bobSigner: Wallet;
  let alithSigner: Wallet;
  let erc20Precompile: MockERC20;
  let erc20Contract: MockERC20;

  const allCosts: { [key: string]: GasCosts } = {};

  // Setup api instance
  before(async () => {
    node = await startNode();
    await node.wait(); // wait for the node to be ready

    provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);

    // Create ERC20 token
    const erc20PrecompileAddress = assetIdToERC20ContractAddress(GAS_TOKEN_ID);

    // Create precompiles contract
    erc20Precompile = new Contract(erc20PrecompileAddress, ERC20_ABI, alithSigner) as MockERC20;

    // Deploy ERC20 contract
    const ERC20Factory = await ethers.getContractFactory("MockERC20");
    erc20Contract = await ERC20Factory.connect(alithSigner).deploy();
    await erc20Contract.deployed();
    console.log("MockERC20 deployed to:", erc20Contract.address);

    // Mint 100 tokens to alith
    const gas = await erc20Contract.connect(alithSigner).estimateGas.mint(alithSigner.address, 1000);
    const tx = await erc20Contract.connect(alithSigner).mint(alithSigner.address, 1000, { gasLimit: gas });
    await tx.wait();
  });

  after(async () => {
    saveGasCosts(allCosts, "ERC20/GasCosts.md", "ERC20 Precompiles");

    await node.stop();
  });

  it("name gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.name();
    // Estimate precompile call
    const precompileGasEstimate = await erc20Precompile.connect(alithSigner).estimateGas.name();

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["name"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
    };
  });

  it("decimals gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.decimals();
    // Estimate precompile call
    const precompileGasEstimate = await erc20Precompile.connect(alithSigner).estimateGas.decimals();

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate.add(50));

    // Update all costs with gas info
    allCosts["decimals"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
    };
  });

  it("balanceOf gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.balanceOf(bobSigner.address);
    // Estimate precompile call
    const precompileGasEstimate = await erc20Precompile.connect(alithSigner).estimateGas.balanceOf(bobSigner.address);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["balanceOf"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
    };
  });

  it("totalSupply gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.totalSupply();
    // Estimate precompile call
    const precompileGasEstimate = await erc20Precompile.connect(alithSigner).estimateGas.totalSupply();

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["totalSupply"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
    };
  });

  it("setApproval gas estimates", async () => {
    const amount = 1000;
    // Estimate contract call
    const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.approve(bobSigner.address, amount);
    // Estimate precompile call
    const precompileGasEstimate = await erc20Precompile
      .connect(alithSigner)
      .estimateGas.approve(bobSigner.address, amount);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["approval"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
    };
  });

  it("allowance gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc20Contract
      .connect(alithSigner)
      .estimateGas.allowance(alithSigner.address, bobSigner.address);
    // Estimate precompile call
    const precompileGasEstimate = await erc20Precompile
      .connect(alithSigner)
      .estimateGas.allowance(alithSigner.address, bobSigner.address);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["allowance"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
    };
  });

  it("transfer gas estimates", async () => {
    const amount = 100;
    const startingAmount = await erc20Precompile.balanceOf(alithSigner.address);
    console.log("Precompile alith balance:", startingAmount?.toString());

    const startingAmountCan = await erc20Contract.balanceOf(alithSigner.address);
    console.log("Canonical alith balance:", startingAmountCan?.toString());
    // Estimate contract call
    const contractGasEstimate = await erc20Contract
      .connect(alithSigner)
      .estimateGas.transfer(bobSigner.address, amount);
    // Estimate precompile call
    const precompileGasEstimate = await erc20Precompile
      .connect(alithSigner)
      .estimateGas.transfer(bobSigner.address, amount);

    // Update all costs with gas info
    allCosts["transfer"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });

  it("transferFrom gas estimates", async () => {
    const amount = 100;

    // // set approval to transfer tokens back from bob to alice
    let gas = await erc20Precompile.connect(alithSigner).estimateGas.approve(bobSigner.address, amount);
    let tx = await erc20Precompile.connect(alithSigner).approve(bobSigner.address, amount, { gasLimit: gas });
    await tx.wait();
    gas = await erc20Contract.connect(alithSigner).estimateGas.approve(bobSigner.address, amount);
    tx = await erc20Contract.connect(alithSigner).approve(bobSigner.address, amount, { gasLimit: gas });
    await tx.wait();

    // Estimate contract call
    const contractGasEstimate = await erc20Contract
      .connect(bobSigner)
      .estimateGas.transferFrom(alithSigner.address, bobSigner.address, amount);
    // Estimate precompile call
    const precompileGasEstimate = await erc20Precompile
      .connect(bobSigner)
      .estimateGas.transferFrom(alithSigner.address, bobSigner.address, amount);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["transferFrom"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });
});
