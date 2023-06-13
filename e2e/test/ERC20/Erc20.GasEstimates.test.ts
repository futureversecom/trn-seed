import { JsonRpcProvider } from "@ethersproject/providers";
import { expect } from "chai";
import { Contract, Wallet, utils } from "ethers";
import { ethers } from "hardhat";
import web3 from "web3";

import ERC20Data from "../../artifacts/contracts/MockERC20.sol/MockERC20.json";
import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC20_ABI,
  GasCosts,
  NodeProcess,
  assetIdToERC20ContractAddress,
  saveGasCosts,
  startNode,
} from "../../common";

describe.only("ERC20 Gas Estimates", function () {
  let node: NodeProcess;

  let provider: JsonRpcProvider;
  let bobSigner: Wallet;
  let alithSigner: Wallet;
  let erc20Precompile: Contract;
  let erc20Contract: Contract;

  const allCosts: { [key: string]: GasCosts } = {};

  // Setup api instance
  before(async () => {
    node = await startNode();
    await node.wait(); // wait for the node to be ready
    console.log(`url:http://127.0.0.1:${node.httpPort}`);
    provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);

    // Create ERC20 token
    const erc20PrecompileAddress = web3.utils.toChecksumAddress(assetIdToERC20ContractAddress(2));

    // Create precompiles contract
    erc20Precompile = new Contract(erc20PrecompileAddress, ERC20_ABI, alithSigner);

    // Deploy OpenZeppelin ERC20 contract
    const factory = new ethers.ContractFactory(ERC20Data.abi, ERC20Data.bytecode, alithSigner);
    erc20Contract = await factory.connect(alithSigner).deploy();
    // const tokenAmount = 10000;
    // Estimate contract call
    await erc20Contract.connect(alithSigner).mint(alithSigner.address, utils.parseEther("101.000000"));
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
    allCosts["Approval"] = {
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
    allCosts["safeTransferFrom"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });

  it("transferFrom gas estimates", async () => {
    const amount = 100;

    // Estimate contract call
    const contractGasEstimate = await erc20Contract
      .connect(bobSigner)
      .estimateGas.transferFrom(bobSigner.address, alithSigner.address, amount);
    // Estimate precompile call
    const precompileGasEstimate = await erc20Precompile
      .connect(bobSigner)
      .estimateGas.transferFrom(bobSigner.address, alithSigner.address, amount);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["transferFrom"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });
});
