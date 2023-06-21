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
  ERC721_PRECOMPILE_ABI,
  GasCosts,
  NodeProcess,
  collectionIdToERC721Address,
  saveGasCosts,
  startNode,
  typedefs,
} from "../../common";
import { MockERC721 } from "../../typechain-types";

// NFT Collection information
const name = "test-collection";
const metadataPath = "https://example.com/nft/metadata/";
const initialIssuance = 10;
const maxIssuance = 2000;

describe("ERC721 Gas Estimates", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let provider: JsonRpcProvider;
  let bobSigner: Wallet;
  let alithSigner: Wallet;
  let erc721Precompile: MockERC721;
  let erc721Contract: MockERC721;
  let collectionId: number;
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

    // Create ERC721 token
    let erc721PrecompileAddress: string;

    // Create NFT collection using runtime, alith is collection owner
    await new Promise<void>((resolve, reject) => {
      api.tx.nft
        .createCollection(name, initialIssuance, maxIssuance, null, metadataPath, null, { xrpl: false })
        .signAndSend(alith, async ({ status, events }) => {
          if (status.isInBlock) {
            events.forEach(({ event: { data, method } }) => {
              if (method == "CollectionCreate") {
                const collection_uuid = (data.toJSON() as any)[0];
                console.log(`Collection UUID: ${collection_uuid}`);
                collectionId = collection_uuid as number;
                erc721PrecompileAddress = collectionIdToERC721Address(collection_uuid);
                console.log(`NFT Collection Address: ${erc721PrecompileAddress}`);
                // Create precompiles contract
                erc721Precompile = new Contract(
                  erc721PrecompileAddress,
                  ERC721_PRECOMPILE_ABI,
                  alithSigner,
                ) as MockERC721;
                resolve();
              }
            });
          }
        })
        .catch((err) => reject(err));
    });
    // Deploy ERC721 contract
    const ERC721Factory = await ethers.getContractFactory("MockERC721");
    erc721Contract = await ERC721Factory.connect(alithSigner).deploy();
    await erc721Contract.deployed();
    console.log("MockERC721 deployed to:", erc721Contract.address);

    // Estimate contract call to mint token with tokenId 0 to alith
    const contractGasEstimate = await erc721Contract.connect(alithSigner).estimateGas.safeMint(alithSigner.address, 0);

    // Estimate precompile call to mint 100 tokens
    const precompileGasEstimate = await erc721Precompile
      .connect(alithSigner)
      .estimateGas.mint(alithSigner.address, 100);

    const tokenId = 0;
    // do the actual mint
    let tx = await erc721Contract
      .connect(alithSigner)
      .safeMint(alithSigner.address, tokenId, { gasLimit: contractGasEstimate });
    await tx.wait();
    const quantity = 100;
    tx = await erc721Precompile
      .connect(alithSigner)
      .mint(alithSigner.address, quantity, { gasLimit: precompileGasEstimate });
    await tx.wait();
  });

  after(async () => {
    saveGasCosts(allCosts, "ERC721/GasCosts.md", "ERC721 Precompiles");

    await node.stop();
  });

  // ERC721 view functions
  it("balanceOf gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc721Contract.connect(alithSigner).estimateGas.balanceOf(bobSigner.address);
    // Estimate precompile call
    const precompileGasEstimate = await erc721Precompile.connect(alithSigner).estimateGas.balanceOf(bobSigner.address);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["balanceOf"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
    };
  });

  it("ownerOf gas estimates", async () => {
    const tokenId = 0;
    // Estimate contract call
    const contractGasEstimate = await erc721Contract.connect(alithSigner).estimateGas.ownerOf(tokenId);
    // Estimate precompile call
    const precompileGasEstimate = await erc721Precompile.connect(alithSigner).estimateGas.ownerOf(tokenId);

    // Update all costs with gas info
    allCosts["ownerOf"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });

  it("get approved gas estimates", async () => {
    const tokenId = 0;
    // Estimate contract call
    const contractGasEstimate = await erc721Contract.connect(alithSigner).estimateGas.getApproved(tokenId);
    // Estimate precompile call
    const precompileGasEstimate = await erc721Precompile.connect(alithSigner).estimateGas.getApproved(tokenId);

    // Update all costs with gas info
    allCosts["getApproved"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });

  it("is approval for all gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc721Contract
      .connect(alithSigner)
      .estimateGas.isApprovedForAll(alithSigner.address, bobSigner.address);
    // Estimate precompile call
    const precompileGasEstimate = await erc721Precompile
      .connect(alithSigner)
      .estimateGas.isApprovedForAll(alithSigner.address, bobSigner.address);

    // Update all costs with gas info
    allCosts["isApprovedForAll"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });

  // ERC721 functions (transactions)
  it("mint gas estimates", async () => {
    // Estimate contract call to mint token with tokenId 0 to alith
    const contractGasEstimate = await erc721Contract.connect(alithSigner).estimateGas.safeMint(alithSigner.address, 1);

    // Estimate precompile call to mint 100 tokens
    const precompileGasEstimate = await erc721Precompile.connect(alithSigner).estimateGas.mint(alithSigner.address, 10);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.nft.mint(collectionId, 10, null).signAndSend(alith, ({ status }) => {
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

  it("setApproval gas estimates", async () => {
    const serialNumber = 0;
    // Estimate contract call
    const contractGasEstimate = await erc721Contract
      .connect(alithSigner)
      .estimateGas.approve(bobSigner.address, serialNumber);
    // Estimate precompile call
    const precompileGasEstimate = await erc721Precompile
      .connect(alithSigner)
      .estimateGas.approve(bobSigner.address, serialNumber);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      const serialId = 1;
      // const tokenId = api.registry.createType('TokenId', [collectionId, serialId]);
      api.tx.tokenApprovals
        .erc721Approval(alithSigner.address, bobSigner.address, [collectionId, serialId])
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await provider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["approve"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  it("transferFrom gas estimates", async () => {
    const serialNumber = 0;

    // // set approval to transfer tokens back from bob to alice
    let gas = await erc721Precompile.connect(alithSigner).estimateGas.approve(bobSigner.address, serialNumber);
    let tx = await erc721Precompile.connect(alithSigner).approve(bobSigner.address, serialNumber, { gasLimit: gas });
    await tx.wait();
    gas = await erc721Contract.connect(alithSigner).estimateGas.approve(bobSigner.address, serialNumber);
    tx = await erc721Contract.connect(alithSigner).approve(bobSigner.address, serialNumber, { gasLimit: gas });
    await tx.wait();

    // Estimate contract call
    const contractGasEstimate = await erc721Contract
      .connect(bobSigner)
      .estimateGas.transferFrom(alithSigner.address, bobSigner.address, serialNumber);
    // Estimate precompile call
    const precompileGasEstimate = await erc721Precompile
      .connect(bobSigner)
      .estimateGas.transferFrom(alithSigner.address, bobSigner.address, serialNumber);

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      const serialId = 1;
      api.tx.nft.transfer(collectionId, [serialId], bobSigner.address).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await provider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);
    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["transferFrom"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  it("safetransferFrom gas estimates", async () => {
    const serialNumber = 0;

    // Estimate contract call
    const contractGasEstimate = await erc721Contract
      .connect(bobSigner)
      .estimateGas["safeTransferFrom(address,address,uint256)"](alithSigner.address, bobSigner.address, serialNumber);

    // Estimate precompile call
    const precompileGasEstimate = await erc721Precompile
      .connect(bobSigner)
      .estimateGas["safeTransferFrom(address,address,uint256)"](alithSigner.address, bobSigner.address, serialNumber);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["safetransferFrom"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });

  it("set approval for all gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc721Contract
      .connect(alithSigner)
      .estimateGas.setApprovalForAll(bobSigner.address, true);
    // Estimate precompile call
    const precompileGasEstimate = await erc721Precompile
      .connect(alithSigner)
      .estimateGas.setApprovalForAll(bobSigner.address, true);

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.tokenApprovals
        .erc721ApprovalForAll(alithSigner.address, bobSigner.address, collectionId, true)
        .signAndSend(alith, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await provider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    // Update all costs with gas info
    allCosts["setApprovalForAll"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  // ERC721 metadata functions
  it("name gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc721Contract.connect(alithSigner).estimateGas.name();
    // Estimate precompile call
    const precompileGasEstimate = await erc721Precompile.connect(alithSigner).estimateGas.name();

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["name"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
    };
  });

  it("symbol gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc721Contract.connect(alithSigner).estimateGas.symbol();
    // Estimate precompile call
    const precompileGasEstimate = await erc721Precompile.connect(alithSigner).estimateGas.symbol();

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["symbol"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
    };
  });

  it("tokenURI gas estimates", async () => {
    const tokenId = 0;
    // Estimate contract call
    const contractGasEstimate = await erc721Contract.connect(alithSigner).estimateGas.tokenURI(tokenId);
    // Estimate precompile call
    const precompileGasEstimate = await erc721Precompile.connect(alithSigner).estimateGas.tokenURI(tokenId);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate);

    // Update all costs with gas info
    allCosts["tokenURI"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0, // No extrinsic
    };
  });

  // Ownable view function
  it("owner gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc721Contract.connect(alithSigner).estimateGas.owner();
    // Estimate precompile call
    const precompileGasEstimate = await erc721Precompile.connect(alithSigner).estimateGas.owner();

    // Update all costs with gas info
    allCosts["owner"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: 0,
    };
  });

  // Ownable function (transactions)
  it("transfer ownership gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc721Contract
      .connect(alithSigner)
      .estimateGas.transferOwnership(bobSigner.address);
    // Estimate precompile call
    const precompileGasEstimate = await erc721Precompile
      .connect(alithSigner)
      .estimateGas.transferOwnership(bobSigner.address);

    const balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.nft.setOwner(collectionId, bobSigner.address).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    const balanceAfter = await alithSigner.getBalance();
    const extrinsicCost = balanceBefore.sub(balanceAfter);
    const fees = await provider.getFeeData();
    const extrinsicScaled = extrinsicCost.div(fees.gasPrice!);

    expect(precompileGasEstimate).to.be.lessThan(contractGasEstimate.add(400));
    expect(extrinsicScaled).to.be.lessThan(precompileGasEstimate);

    // Update all costs with gas info
    allCosts["transferOwnership"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });

  it("renounceOwnership gas estimates", async () => {
    // Estimate contract call
    const contractGasEstimate = await erc721Contract.connect(alithSigner).estimateGas.renounceOwnership();
    // Estimate precompile call
    const precompileGasEstimate = await erc721Precompile.connect(bobSigner).estimateGas.renounceOwnership();

    const balanceBefore = await bobSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.nft.setOwner(collectionId, "0x0000000000000000000000000000000000000000").signAndSend(bob, ({ status }) => {
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
    allCosts["renounceOwnership"] = {
      Contract: contractGasEstimate.toNumber(),
      Precompile: precompileGasEstimate.toNumber(),
      Extrinsic: extrinsicScaled.toNumber(),
    };
  });
});
