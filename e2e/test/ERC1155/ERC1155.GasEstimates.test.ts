import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Contract, Wallet } from "ethers";
import { ethers } from "hardhat";

import ERC1155Data from "../../artifacts/contracts/ERC1155.sol/ERC1155.json";
import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC1155_PRECOMPILE_ABI,
  GasCosts,
  NodeProcess,
  SFT_PRECOMPILE_ABI,
  SFT_PRECOMPILE_ADDRESS,
  getSftCollectionPrecompileAddress,
  saveGasCosts,
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

  const allCosts: { [key: string]: GasCosts } = {};

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
    saveGasCosts(allCosts, "ERC1155/GasCosts.md", "ERC1155 Precompiles");

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
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
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
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
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
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
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
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
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
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
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
    // Perform extrinsic call and calculate gas based on difference in balance
    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.sft.transfer(collectionId, [[0, 10]], bobSigner.address).signAndSend(alith, ({ status }) => {
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
    allCosts["safeTransferFrom"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
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
    // Perform extrinsic call and calculate gas based on difference in balance
    const balanceBefore = await alithSigner.getBalance();
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
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await provider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["safeBatchTransferFrom"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
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
    // Perform extrinsic call and calculate gas based on difference in balance
    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.sft.mint(collectionId, [[0, 10]], alithSigner.address).signAndSend(alith, ({ status }) => {
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
    allCosts["mint"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
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
    // Perform extrinsic call and calculate gas based on difference in balance
    const balanceBefore = await alithSigner.getBalance();
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
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await provider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["mintBatch"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  it("burn gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc1155Contract.connect(alithSigner).estimateGas.burn(alithSigner.address, 0, 10);
    // Estimate precompile call
    const precompileGasEstimate = await erc1155Precompile
      .connect(alithSigner)
      .estimateGas.burn(alithSigner.address, 0, 10);
    // Perform extrinsic call and calculate gas based on difference in balance
    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.sft.burn(collectionId, [[0, 10]]).signAndSend(alith, ({ status }) => {
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
    allCosts["burn"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
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
    // Perform extrinsic call and calculate gas based on difference in balance
    const balanceBefore = await alithSigner.getBalance();
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
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await provider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["burnBatch"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });
});
