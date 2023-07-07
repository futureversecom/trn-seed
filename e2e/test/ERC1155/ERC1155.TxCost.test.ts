import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { BigNumber, Contract, Wallet } from "ethers";
import { ethers } from "hardhat";

import ERC1155Data from "../../artifacts/contracts/ERC1155.sol/ERC1155.json";
import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC1155_PRECOMPILE_ABI,
  NodeProcess,
  SFT_PRECOMPILE_ABI,
  SFT_PRECOMPILE_ADDRESS,
  TxCosts,
  getScaledGasForExtrinsicFee,
  getSftCollectionPrecompileAddress,
  saveTxFees,
  saveTxGas,
  startNode,
  typedefs,
} from "../../common";

describe("ERC1155 Gas Estimates", function () {
  let node: NodeProcess;

  let provider: JsonRpcProvider;
  let api: ApiPromise;
  let bobSigner: Wallet;
  let alithSigner: Wallet;
  let sftPrecompile: Contract;
  let erc1155Precompile: Contract;
  let erc1155Contract: Contract;
  let collectionId: any;
  let alith: KeyringPair;

  const allCosts: { [key: string]: TxCosts } = {};
  const allTxFeeCosts: { [key: string]: TxCosts } = {};

  // Setup api instance
  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);

    // Setup Root api instance and keyring
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);

    // Create SFT collection, bob is collection owner
    collectionId = await api.query.nft.nextCollectionId();
    const erc1155PrecompileAddress = getSftCollectionPrecompileAddress(+collectionId);

    // Deploy sft contract
    sftPrecompile = new Contract(SFT_PRECOMPILE_ADDRESS, SFT_PRECOMPILE_ABI, alithSigner);
    const name = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("My Collection"));
    const metadataPath = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("https://example.com/sft/metadata"));
    const initializeTx = await sftPrecompile
      .connect(alithSigner)
      .initializeCollection(alithSigner.address, name, metadataPath, [], []);
    await initializeTx.wait();
    // Create precompiles contract
    erc1155Precompile = new Contract(erc1155PrecompileAddress, ERC1155_PRECOMPILE_ABI, alithSigner);

    const tokenName = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("MyToken"));
    const tx2 = await erc1155Precompile.connect(alithSigner).createToken(tokenName, 1000, 0, alithSigner.address);
    await tx2.wait();

    // Deploy OpenZeppelin ERC1155 contract
    const factory = new ethers.ContractFactory(ERC1155Data.abi, ERC1155Data.bytecode, alithSigner);
    erc1155Contract = await factory.connect(alithSigner).deploy("https://example.com/sft/metadata");
    const callData = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("data"));
    const tx = await erc1155Contract.connect(alithSigner).mint(alithSigner.address, 0, 10000, callData);
    await tx.wait();
  });

  after(async () => {
    saveTxGas(allCosts, "ERC1155/TxCosts.md", "ERC1155 Precompiles");
    saveTxFees(allTxFeeCosts, "ERC1155/TxCosts.md", "ERC1155 Precompiles");
    await node.stop();
  });

  it("uri gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc1155Contract.connect(alithSigner).estimateGas.uri(0);
    // Estimate precompile call
    const precompileGasEstimate = await erc1155Precompile.connect(alithSigner).estimateGas.uri(0);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["uri"] = {
      Contract: contractGasEstimate,
      Precompile: precompileGasEstimate,
      Extrinsic: BigNumber.from(0), // No extrinsic
    };
  });

  it("balanceOf gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc1155Contract.connect(alithSigner).estimateGas.balanceOf(bobSigner.address, 0);
    // Estimate precompile call
    const precompileGasEstimate = await erc1155Precompile
      .connect(alithSigner)
      .estimateGas.balanceOf(bobSigner.address, 0);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["balanceOf"] = {
      Contract: contractGasEstimate,
      Precompile: precompileGasEstimate,
      Extrinsic: BigNumber.from(0), // No extrinsic
    };
  });

  it("balanceOfBatch gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc1155Contract
      .connect(alithSigner)
      .estimateGas.balanceOfBatch([bobSigner.address, alithSigner.address], [0, 0]);
    // Estimate precompile call
    const precompileGasEstimate = await erc1155Precompile
      .connect(alithSigner)
      .estimateGas.balanceOfBatch([bobSigner.address, alithSigner.address], [0, 0]);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["balanceOfBatch"] = {
      Contract: contractGasEstimate,
      Precompile: precompileGasEstimate,
      Extrinsic: BigNumber.from(0), // No extrinsic
    };
  });

  it("setApprovalForAll gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc1155Contract
      .connect(alithSigner)
      .estimateGas.setApprovalForAll(bobSigner.address, true);
    // Estimate precompile call
    const precompileGasEstimate = await erc1155Precompile
      .connect(alithSigner)
      .estimateGas.setApprovalForAll(bobSigner.address, true);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["setApprovalForAll"] = {
      Contract: contractGasEstimate,
      Precompile: precompileGasEstimate,
      Extrinsic: BigNumber.from(0), // No extrinsic
    };
  });

  it("isApprovedForAll gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc1155Contract
      .connect(alithSigner)
      .estimateGas.isApprovedForAll(alithSigner.address, bobSigner.address);
    // Estimate precompile call
    const precompileGasEstimate = await erc1155Precompile
      .connect(alithSigner)
      .estimateGas.isApprovedForAll(alithSigner.address, bobSigner.address);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["isApprovedForAll"] = {
      Contract: contractGasEstimate,
      Precompile: precompileGasEstimate,
      Extrinsic: BigNumber.from(0), // No extrinsic
    };
  });

  it("safeTransferFrom gas estimates", async () => {
    const callData = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("data"));

    // Estimate contract call
    const contractGasEstimate = await erc1155Contract
      .connect(alithSigner)
      .estimateGas.safeTransferFrom(alithSigner.address, bobSigner.address, 0, 10, callData);
    // Estimate precompile call
    const precompileGasEstimate = await erc1155Precompile
      .connect(alithSigner)
      .estimateGas.safeTransferFrom(alithSigner.address, bobSigner.address, 0, 10, callData);
    // precompile call cost
    let balanceBefore = await alithSigner.getBalance();
    const tx = await erc1155Precompile
      .connect(alithSigner)
      .safeTransferFrom(alithSigner.address, bobSigner.address, 0, 10, callData);
    await tx.wait();
    let balanceAfter = await alithSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);
    // Perform extrinsic call and calculate gas based on difference in balance
    balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.sft.transfer(collectionId, [[0, 10]], bobSigner.address).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    balanceAfter = await alithSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasScaled = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicGasScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs
    allCosts["safeTransferFrom"] = {
      Contract: contractGasEstimate,
      Precompile: precompileGasEstimate,
      Extrinsic: extrinsicGasScaled,
    };
    allTxFeeCosts["safeTransferFrom"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("safeBatchTransferFrom gas estimates", async () => {
    const callData = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("data"));

    // Estimate contract call
    const contractGasEstimate = await erc1155Contract
      .connect(alithSigner)
      .estimateGas.safeBatchTransferFrom(alithSigner.address, bobSigner.address, [0, 0], [10, 12], callData);
    // Estimate precompile call
    const precompileGasEstimate = await erc1155Precompile
      .connect(alithSigner)
      .estimateGas.safeBatchTransferFrom(alithSigner.address, bobSigner.address, [0, 0], [10, 12], callData);
    // precompile call cost
    let balanceBefore = await alithSigner.getBalance();
    const tx = await erc1155Precompile
      .connect(alithSigner)
      .safeBatchTransferFrom(alithSigner.address, bobSigner.address, [0, 0], [10, 12], callData);
    await tx.wait();
    let balanceAfter = await alithSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);
    // Perform extrinsic call and calculate gas based on difference in balance
    balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.sft
        .transfer(
          collectionId,
          [
            [0, 10],
            [0, 12],
          ],
          bobSigner.address,
        )
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    balanceAfter = await alithSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasScaled = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicGasScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["safeBatchTransferFrom"] = {
      Contract: contractGasEstimate,
      Precompile: precompileGasEstimate,
      Extrinsic: extrinsicGasScaled,
    };
    allTxFeeCosts["safeBatchTransferFrom"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("mint gas estimates", async () => {
    const callData = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("data"));

    // Estimate contract call
    const contractGasEstimate = await erc1155Contract
      .connect(alithSigner)
      .estimateGas.mint(alithSigner.address, 0, 10, callData);
    // Estimate precompile call
    const precompileGasEstimate = await erc1155Precompile
      .connect(alithSigner)
      .estimateGas.mint(alithSigner.address, 0, 10);
    // precompile call cost
    let balanceBefore = await alithSigner.getBalance();
    const tx = await erc1155Precompile.connect(alithSigner).mint(alithSigner.address, 0, 10);
    await tx.wait();
    let balanceAfter = await alithSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);
    // Perform extrinsic call and calculate gas based on difference in balance
    balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.sft.mint(collectionId, [[0, 10]], alithSigner.address).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    balanceAfter = await alithSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasScaled = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicGasScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs
    allCosts["mint"] = {
      Contract: contractGasEstimate,
      Precompile: precompileGasEstimate,
      Extrinsic: extrinsicGasScaled,
    };
    allTxFeeCosts["mint"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("mintBatch gas estimates", async () => {
    const callData = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("data"));

    // Estimate contract call
    const contractGasEstimate = await erc1155Contract
      .connect(alithSigner)
      .estimateGas.mintBatch(alithSigner.address, [0, 0], [10, 12], callData);
    // Estimate precompile call
    const precompileGasEstimate = await erc1155Precompile
      .connect(alithSigner)
      .estimateGas.mintBatch(alithSigner.address, [0, 0], [10, 12]);
    // precompile call cost
    let balanceBefore = await alithSigner.getBalance();
    const tx = await erc1155Precompile.connect(alithSigner).mintBatch(alithSigner.address, [0, 0], [10, 12]);
    await tx.wait();
    let balanceAfter = await alithSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);
    // Perform extrinsic call and calculate gas based on difference in balance
    balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.sft
        .mint(
          collectionId,
          [
            [0, 10],
            [0, 12],
          ],
          alithSigner.address,
        )
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    balanceAfter = await alithSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasScaled = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicGasScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs
    allCosts["mintBatch"] = {
      Contract: contractGasEstimate,
      Precompile: precompileGasEstimate,
      Extrinsic: extrinsicGasScaled,
    };
    allTxFeeCosts["mintBatch"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("burn gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc1155Contract.connect(alithSigner).estimateGas.burn(alithSigner.address, 0, 10);
    // Estimate precompile call
    const precompileGasEstimate = await erc1155Precompile
      .connect(alithSigner)
      .estimateGas.burn(alithSigner.address, 0, 10);
    // precompile call cost
    let balanceBefore = await alithSigner.getBalance();
    const tx = await erc1155Precompile.connect(alithSigner).burn(alithSigner.address, 0, 10);
    await tx.wait();
    let balanceAfter = await alithSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);
    // Perform extrinsic call and calculate gas based on difference in balance
    balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.sft.burn(collectionId, [[0, 10]]).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    balanceAfter = await alithSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasScaled = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicGasScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs
    allCosts["burn"] = {
      Contract: contractGasEstimate,
      Precompile: precompileGasEstimate,
      Extrinsic: extrinsicGasScaled,
    };
    allTxFeeCosts["burn"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("burnBatch gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc1155Contract
      .connect(alithSigner)
      .estimateGas.burnBatch(alithSigner.address, [0, 0], [10, 12]);
    // Estimate precompile call
    const precompileGasEstimate = await erc1155Precompile
      .connect(alithSigner)
      .estimateGas.burnBatch(alithSigner.address, [0, 0], [10, 12]);
    // precompile call cost
    let balanceBefore = await alithSigner.getBalance();
    const tx = await erc1155Precompile.connect(alithSigner).burnBatch(alithSigner.address, [0, 0], [10, 12]);
    await tx.wait();
    let balanceAfter = await alithSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);
    // Perform extrinsic call and calculate gas based on difference in balance
    balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.sft
        .burn(collectionId, [
          [0, 10],
          [0, 12],
        ])
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    balanceAfter = await alithSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasScaled = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicGasScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs
    allCosts["burnBatch"] = {
      Contract: contractGasEstimate,
      Precompile: precompileGasEstimate,
      Extrinsic: extrinsicGasScaled,
    };
    allTxFeeCosts["burnBatch"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });
});
