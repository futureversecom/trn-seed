import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Contract, Wallet } from "ethers";
import { ethers } from "hardhat";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC20_ABI,
  GAS_TOKEN_ID,
  GasCosts,
  NATIVE_TOKEN_ID,
  NodeProcess,
  assetIdToERC20ContractAddress,
  saveGasCosts,
  startNode,
  typedefs,
} from "../../common";
import { MockERC20 } from "../../typechain-types";

describe("ERC20 Gas Estimates", function () {
  let node: NodeProcess;

  let provider: JsonRpcProvider;
  let api: ApiPromise;
  let bobSigner: Wallet;
  let alithSigner: Wallet;
  let erc20Precompile: MockERC20;
  let erc20Contract: MockERC20;
  let alith: KeyringPair;
  let bob: KeyringPair;

  const allCosts: { [key: string]: GasCosts } = {};

  // Setup api instance
  before(async () => {
    node = await startNode();
    await node.wait(); // wait for the node to be ready

    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);

    // Setup Root api instance and keyring
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);
    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));

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

  // ERC20 view functions
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

  // ERC20 functions
  it("setApproval gas estimates", async () => {
    const amount = 1000;
    // Estimate contract call
    const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.approve(bobSigner.address, amount);
    // Estimate precompile call
    const precompileGasEstimate = await erc20Precompile
      .connect(alithSigner)
      .estimateGas.approve(bobSigner.address, amount);

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.assets.approveTransfer(NATIVE_TOKEN_ID, bobSigner.address, amount).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await provider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);
    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["approval"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  it("transfer gas estimates", async () => {
    const amount = 100;

    // Estimate contract call
    const contractGasEstimate = await erc20Contract
      .connect(alithSigner)
      .estimateGas.transfer(bobSigner.address, amount);
    // Estimate precompile call
    const precompileGasEstimate = await erc20Precompile
      .connect(alithSigner)
      .estimateGas.transfer(bobSigner.address, amount);

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.assets.transfer(NATIVE_TOKEN_ID, bobSigner.address, amount).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await provider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);
    // Update all costs with gas info
    allCosts["transfer"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
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

    const balanceBefore = await bobSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.assets
        .transferApproved(NATIVE_TOKEN_ID, alithSigner.address, bobSigner.address, amount)
        .signAndSend(bob, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    const balanceAfter = await bobSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await provider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["transferFrom"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  // ERC20 metadata view functions
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

  it("symbol gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc20Contract.connect(alithSigner).estimateGas.symbol();
    // Estimate precompile call
    const precompileGasEstimate = await erc20Precompile.connect(alithSigner).estimateGas.symbol();

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["symbol"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
    };
  });
});
