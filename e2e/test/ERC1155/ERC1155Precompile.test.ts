import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { BigNumber, Contract, Wallet, constants } from "ethers";
import { ethers } from "hardhat";
import Web3 from "web3";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC1155_PRECOMPILE_ABI,
  FUTUREPASS_PRECOMPILE_ABI,
  NodeProcess,
  SFT_PRECOMPILE_ABI,
  SFT_PRECOMPILE_ADDRESS,
  getCollectionPrecompileAddress,
  startNode,
  typedefs,
  getSftCollectionPrecompileAddress,
} from "../../common";

// SFT Collection information
const name = "test-collection";
const metadataPath = "https://example.com/sft/metadata/";

describe("ERC1155 Precompile", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let bobSigner: Wallet;
  let alithSigner: Wallet;
  let sftPrecompile: Contract;
  let erc1155Precompile: Contract;
  let precompileCaller: Contract;

  // Setup api instance
  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);

    // Setup Root api instance and keyring
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    const keyring = new Keyring({ type: "ethereum" });
    const bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));

    const provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);

    let erc1155PrecompileAddress: string;

    // Create SFT collection using runtime, bob is collection owner
    await new Promise<void>((resolve, reject) => {
      api.tx.sft
        .createCollection(name, null, metadataPath, null)
        .signAndSend(bob, async ({ status, events }) => {
          if (status.isInBlock) {
            events.forEach(({ event: { data, method } }) => {
              if (method == "CollectionCreate") {
                const collection_uuid = (data.toJSON() as any)[0];
                console.log(`Collection UUID: ${collection_uuid}`);

                const collection_id_hex = (+collection_uuid).toString(16).padStart(8, "0");
                erc1155PrecompileAddress = Web3.utils.toChecksumAddress(
                  `0xBBBBBBBB${collection_id_hex}000000000000000000000000`,
                );
                console.log(`SFT Collection Address: ${erc1155PrecompileAddress}`);
                erc1155Precompile = new Contract(erc1155PrecompileAddress, ERC1155_PRECOMPILE_ABI, bobSigner);
                resolve();
              }
            });
          }
        })
        .catch((err) => reject(err));
    });

    // Deploy sft contract
    sftPrecompile = new Contract(SFT_PRECOMPILE_ADDRESS, SFT_PRECOMPILE_ABI, bobSigner);

    // Deploy PrecompileCaller contract
    const factory = await ethers.getContractFactory("ERC1155PrecompileCaller");
    precompileCaller = await factory.connect(bobSigner).deploy(erc1155PrecompileAddress!);
    await precompileCaller.deployed();
  });

  after(async () => await node.stop());

  async function createToken(initialIssuance: number, tokenOwner: Wallet = bobSigner) {
    const tokenName = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("MyToken"));
    const maxIssuance = 0;
    const tx = await erc1155Precompile
      .connect(bobSigner)
      .createToken(tokenName, initialIssuance, maxIssuance, tokenOwner.address);
    const receipt = await tx.wait();
    const serialNumber = (receipt?.events as any)[0].args.serialNumber;

    return serialNumber;
  }

  it("initializeCollection succeeds", async () => {
    const owner = alithSigner.address;
    const name = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("My Collection"));
    const metadataPath = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("https://example.com/sft/metadata"));
    const royaltyAddresses = [alithSigner.address];
    const royaltyEntitlements = [1000];

    // Generate expected precompile address
    const collectionId = await api.query.nft.nextCollectionId();
    const expectedPrecompileAddress = getSftCollectionPrecompileAddress(+collectionId);

    const initializeTx = await sftPrecompile
      .connect(bobSigner)
      .initializeCollection(owner, name, metadataPath, royaltyAddresses, royaltyEntitlements);
    const receipt = await initializeTx.wait();

    expect((receipt?.events as any)[0].event).to.equal("InitializeSftCollection");
    expect((receipt?.events as any)[0].args.collectionOwner).to.equal(alithSigner.address);
    expect((receipt?.events as any)[0].args.precompileAddress).to.equal(expectedPrecompileAddress);
  });

  it("createToken", async () => {
    const tokenName = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("MyToken"));
    const maxIssuance = 0;
    const initialIssuance = 0;
    const tx = await erc1155Precompile
      .connect(bobSigner)
      .createToken(tokenName, initialIssuance, maxIssuance, constants.AddressZero);
    const receipt = await tx.wait();

    const event = (receipt?.events as any)[0].event;
    const serialNumber = (receipt?.events as any)[0].args.serialNumber;
    expect(event).to.equal("TokenCreated");
    expect(serialNumber).to.equal(0);
    expect(await erc1155Precompile.exists(serialNumber)).to.equal(true);
  });

  it("mint", async () => {
    const serialNumber = await createToken(0);
    const initialIssuance = 100;
    const receiverAddress = await Wallet.createRandom().getAddress();

    const mintTx = await erc1155Precompile.connect(bobSigner).mint(receiverAddress, serialNumber, initialIssuance);
    const receipt = await mintTx.wait();

    // Verify balance is correct
    expect(await erc1155Precompile.balanceOf(receiverAddress, serialNumber)).to.equal(initialIssuance);

    // Verify event
    expect((receipt?.events as any)[0].event).to.equal("TransferSingle");
    expect((receipt?.events as any)[0].args.operator).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.from).to.equal(constants.AddressZero);
    expect((receipt?.events as any)[0].args.to).to.equal(receiverAddress);
    expect((receipt?.events as any)[0].args.id).to.equal(serialNumber);
    expect((receipt?.events as any)[0].args.value).to.equal(initialIssuance);
  });

  it("mintBatch", async () => {
    const serialNumber1 = await createToken(0);
    const serialNumber2 = await createToken(0);
    const initialIssuance1 = 101;
    const initialIssuance2 = 202;
    const receiverAddress = await Wallet.createRandom().getAddress();

    const mintTx = await erc1155Precompile
      .connect(bobSigner)
      .mintBatch(receiverAddress, [serialNumber1, serialNumber2], [initialIssuance1, initialIssuance2]);
    const receipt = await mintTx.wait();

    // Verify balance is correct for both tokens
    expect(await erc1155Precompile.balanceOf(receiverAddress, serialNumber1)).to.equal(initialIssuance1);
    expect(await erc1155Precompile.balanceOf(receiverAddress, serialNumber2)).to.equal(initialIssuance2);

    // Verify event
    expect((receipt?.events as any)[0].event).to.equal("TransferBatch");
    expect((receipt?.events as any)[0].args.operator).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.from).to.equal(constants.AddressZero);
    expect((receipt?.events as any)[0].args.to).to.equal(receiverAddress);
    expect((receipt?.events as any)[0].args.ids).to.eql([BigNumber.from(serialNumber1), BigNumber.from(serialNumber2)]);
    expect((receipt?.events as any)[0].args.balances).to.eql([
      BigNumber.from(initialIssuance1),
      BigNumber.from(initialIssuance2),
    ]);
  });

  it("mintBatch invalid input lengths", async () => {
    const serialNumber1 = await createToken(0);
    const initialIssuance1 = 101;
    const initialIssuance2 = 202;
    const receiverAddress = await Wallet.createRandom().getAddress();

    // Should fail as the input lengths are different
    let errorFound = false;
    await erc1155Precompile
      .connect(bobSigner)
      .mintBatch(receiverAddress, [serialNumber1], [initialIssuance1, initialIssuance2])
      .catch((err: any) => {
        errorFound = true;
        expect(err.message).contains("ids and amounts length mismatch");
      });

    // Double check error is thrown
    expect(errorFound).to.equal(true);

    // Verify balance is correct for both tokens
    expect(await erc1155Precompile.balanceOf(receiverAddress, serialNumber1)).to.equal(0);
  });

  it("balanceOf, balanceOfBatch", async () => {
    const initialIssuance1 = 101;
    const initialIssuance2 = 202;
    const serialNumber1 = await createToken(initialIssuance1);
    const serialNumber2 = await createToken(initialIssuance2);

    // Verify balanceOf works
    expect(await erc1155Precompile.balanceOf(bobSigner.address, serialNumber1)).to.equal(initialIssuance1);
    expect(await erc1155Precompile.balanceOf(bobSigner.address, serialNumber2)).to.equal(initialIssuance2);

    // Verify balanceOfBatch works
    expect(
      await erc1155Precompile.balanceOfBatch([bobSigner.address, bobSigner.address], [serialNumber1, serialNumber2]),
    ).to.eql([BigNumber.from(initialIssuance1), BigNumber.from(initialIssuance2)]);
  });

  it("totalSupply, exists, uri", async () => {
    const receiverAddress = await Wallet.createRandom();
    const initialIssuance = 123;
    const serialNumber = await createToken(initialIssuance, receiverAddress);

    // Verify total supply
    expect(await erc1155Precompile.totalSupply(serialNumber)).to.equal(initialIssuance);
    // Verify exists
    expect(await erc1155Precompile.exists(serialNumber)).to.equal(true);
    expect(await erc1155Precompile.exists(serialNumber + 1)).to.equal(false);
    // Verify uri
    expect(await erc1155Precompile.uri(serialNumber)).to.equal(`https://example.com/sft/metadata/${serialNumber}`);
  });

  it("setBaseURI", async () => {
    const receiverAddress = await Wallet.createRandom();
    const initialIssuance = 123;
    const serialNumber = await createToken(initialIssuance, receiverAddress);
    const newMetadataPath = "https://example.com/sft/updated/";

    const mintTx = await erc1155Precompile
      .connect(bobSigner)
      .setBaseURI(ethers.utils.hexlify(ethers.utils.toUtf8Bytes(newMetadataPath)));
    const receipt = await mintTx.wait();

    // validate event
    expect((receipt?.events as any)[0].event).to.equal("BaseURIUpdated");
    expect((receipt?.events as any)[0].args.baseURI).to.equal(newMetadataPath);

    // Verify URI set correctly
    expect(await erc1155Precompile.uri(serialNumber)).to.equal(`https://example.com/sft/updated/${serialNumber}`);
  });

  it("setMaxSupply", async () => {
    const receiverAddress = await Wallet.createRandom();
    const initialIssuance = 123;
    const serialNumber = await createToken(initialIssuance, receiverAddress);

    // Shouldn't work as maxIssuance is lower than total supply
    let maxIssuance = 122;
    let errorFound = false;
    await erc1155Precompile
      .connect(bobSigner)
      .setMaxSupply(serialNumber, maxIssuance)
      .catch((err: any) => {
        errorFound = true;
        expect(err.message).contains("InvalidMaxIssuance");
      });

    // Double check error is thrown
    expect(errorFound).to.equal(true);

    // Should work now
    maxIssuance = 123;
    const mintTx = await erc1155Precompile.connect(bobSigner).setMaxSupply(serialNumber, maxIssuance);
    const receipt = await mintTx.wait();

    // validate event
    expect((receipt?.events as any)[0].event).to.equal("MaxSupplyUpdated");
    expect((receipt?.events as any)[0].args.maxSupply).to.equal(maxIssuance);
  });

  it("burn", async () => {
    const initialIssuance = 100;
    const serialNumber = await createToken(initialIssuance);

    const burnAmount = 69;
    const tx = await erc1155Precompile.connect(bobSigner).burn(bobSigner.address, serialNumber, burnAmount);
    const receipt = await tx.wait();

    // Verify balance is correct
    expect(await erc1155Precompile.balanceOf(bobSigner.address, serialNumber)).to.equal(initialIssuance - burnAmount);

    // Verify event
    expect((receipt?.events as any)[0].event).to.equal("TransferSingle");
    expect((receipt?.events as any)[0].args.operator).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.from).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.to).to.equal(constants.AddressZero);
    expect((receipt?.events as any)[0].args.id).to.equal(serialNumber);
    expect((receipt?.events as any)[0].args.value).to.equal(burnAmount);
  });

  it("burnBatch", async () => {
    const initialIssuance1 = 101;
    const initialIssuance2 = 202;
    const serialNumber1 = await createToken(initialIssuance1);
    const serialNumber2 = await createToken(initialIssuance2);

    const burnAmount = 69;
    const tx = await erc1155Precompile
      .connect(bobSigner)
      .burnBatch(bobSigner.address, [serialNumber1, serialNumber2], [burnAmount, burnAmount]);
    const receipt = await tx.wait();

    // Verify balance is correct for both tokens
    expect(await erc1155Precompile.balanceOf(bobSigner.address, serialNumber1)).to.equal(initialIssuance1 - burnAmount);
    expect(await erc1155Precompile.balanceOf(bobSigner.address, serialNumber2)).to.equal(initialIssuance2 - burnAmount);

    // Verify event
    expect((receipt?.events as any)[0].event).to.equal("TransferBatch");
    expect((receipt?.events as any)[0].args.operator).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.from).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.to).to.equal(constants.AddressZero);
    expect((receipt?.events as any)[0].args.ids).to.eql([BigNumber.from(serialNumber1), BigNumber.from(serialNumber2)]);
    expect((receipt?.events as any)[0].args.balances).to.eql([BigNumber.from(burnAmount), BigNumber.from(burnAmount)]);
  });

  it("SetApprovalForAll, isApprovedForAll", async () => {
    await createToken(100);

    const tx = await erc1155Precompile.setApprovalForAll(alithSigner.address, true);
    await tx.wait();

    // Verify isApprovedForAll is correct
    expect(await erc1155Precompile.isApprovedForAll(bobSigner.address, alithSigner.address)).to.equal(true);

    // set approval to false
    const tx2 = await erc1155Precompile.setApprovalForAll(alithSigner.address, false);
    await tx2.wait();

    // Verify isApprovedForAll is removed
    expect(await erc1155Precompile.isApprovedForAll(bobSigner.address, alithSigner.address)).to.equal(false);
  });

  it("SafeTransferFrom approved address", async () => {
    const initialIssuance = 100;
    const serialNumber = await createToken(initialIssuance);

    const tx = await erc1155Precompile.setApprovalForAll(alithSigner.address, true);
    await tx.wait();
    // Verify isApprovedForAll is correct
    expect(await erc1155Precompile.isApprovedForAll(bobSigner.address, alithSigner.address)).to.equal(true);

    const transferAmount = 69;
    const callData = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("data"));
    const tx2 = await erc1155Precompile
      .connect(alithSigner)
      .safeTransferFrom(bobSigner.address, alithSigner.address, serialNumber, transferAmount, callData);
    const receipt = await tx2.wait();

    // Verify event
    expect((receipt?.events as any)[0].event).to.equal("TransferSingle");
    expect((receipt?.events as any)[0].args.operator).to.equal(alithSigner.address);
    expect((receipt?.events as any)[0].args.from).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.to).to.equal(alithSigner.address);
    expect((receipt?.events as any)[0].args.id).to.equal(serialNumber);
    expect((receipt?.events as any)[0].args.value).to.equal(transferAmount);

    // Verify ownership
    expect(await erc1155Precompile.balanceOf(bobSigner.address, serialNumber)).to.equal(
      initialIssuance - transferAmount,
    );
    expect(await erc1155Precompile.balanceOf(alithSigner.address, serialNumber)).to.equal(transferAmount);

    // Remove approval
    const tx3 = await erc1155Precompile.setApprovalForAll(alithSigner.address, false);
    await tx3.wait();
    // Verify isApprovedForAll is correct
    expect(await erc1155Precompile.isApprovedForAll(bobSigner.address, alithSigner.address)).to.equal(false);

    // transfer should now fail as approval was removed
    let errorFound = false;
    await erc1155Precompile
      .connect(alithSigner)
      .safeTransferFrom(bobSigner.address, alithSigner.address, serialNumber, 1, callData)
      .catch((err: any) => {
        errorFound = true;
        expect(err.message).contains("Caller is not token owner or approved");
      });

    // Double check error is thrown
    expect(errorFound).to.equal(true);
  });

  it("SafeTransferFrom owner", async () => {
    const initialIssuance = 100;
    const serialNumber = await createToken(initialIssuance);

    const transferAmount = 69;
    const callData = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("data"));
    const tx = await erc1155Precompile.safeTransferFrom(
      bobSigner.address,
      alithSigner.address,
      serialNumber,
      transferAmount,
      callData,
    );
    const receipt = await tx.wait();

    // Verify event
    expect((receipt?.events as any)[0].event).to.equal("TransferSingle");
    expect((receipt?.events as any)[0].args.operator).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.from).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.to).to.equal(alithSigner.address);
    expect((receipt?.events as any)[0].args.id).to.equal(serialNumber);
    expect((receipt?.events as any)[0].args.value).to.equal(transferAmount);

    // Verify ownership
    expect(await erc1155Precompile.balanceOf(bobSigner.address, serialNumber)).to.equal(
      initialIssuance - transferAmount,
    );
    expect(await erc1155Precompile.balanceOf(alithSigner.address, serialNumber)).to.equal(transferAmount);
  });

  it("SafeBatchTransferFrom owner", async () => {
    const initialIssuance1 = 101;
    const initialIssuance2 = 202;
    const serialNumber1 = await createToken(initialIssuance1);
    const serialNumber2 = await createToken(initialIssuance2);

    const transferAmount1 = 69;
    const transferAmount2 = 71;
    const callData = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("data"));
    const tx = await erc1155Precompile.safeBatchTransferFrom(
      bobSigner.address,
      alithSigner.address,
      [serialNumber1, serialNumber2],
      [transferAmount1, transferAmount2],
      callData,
    );
    const receipt = await tx.wait();

    // Verify event
    expect((receipt?.events as any)[0].event).to.equal("TransferBatch");
    expect((receipt?.events as any)[0].args.operator).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.from).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.to).to.equal(alithSigner.address);
    expect((receipt?.events as any)[0].args.ids).to.eql([BigNumber.from(serialNumber1), BigNumber.from(serialNumber2)]);
    expect((receipt?.events as any)[0].args.balances).to.eql([
      BigNumber.from(transferAmount1),
      BigNumber.from(transferAmount2),
    ]);

    // Verify ownership
    expect(await erc1155Precompile.balanceOf(bobSigner.address, serialNumber1)).to.equal(
      initialIssuance1 - transferAmount1,
    );
    expect(await erc1155Precompile.balanceOf(alithSigner.address, serialNumber1)).to.equal(transferAmount1);
    expect(await erc1155Precompile.balanceOf(bobSigner.address, serialNumber2)).to.equal(
      initialIssuance2 - transferAmount2,
    );
    expect(await erc1155Precompile.balanceOf(alithSigner.address, serialNumber2)).to.equal(transferAmount2);
  });

  it("BalanceOf, BalanceOfBatch via Proxy", async () => {
    const initialIssuance1 = 101;
    const initialIssuance2 = 202;
    const serialNumber1 = await createToken(initialIssuance1);
    const serialNumber2 = await createToken(initialIssuance2);

    // Verify balanceOf works
    expect(await precompileCaller.balanceOfProxy(bobSigner.address, serialNumber1)).to.equal(initialIssuance1);
    expect(await precompileCaller.balanceOfProxy(bobSigner.address, serialNumber2)).to.equal(initialIssuance2);

    // Verify balanceOfBatch works
    expect(
      await precompileCaller.balanceOfBatchProxy(
        [bobSigner.address, bobSigner.address],
        [serialNumber1, serialNumber2],
      ),
    ).to.eql([BigNumber.from(initialIssuance1), BigNumber.from(initialIssuance2)]);
  });

  it("SetApprovalForAll, isApprovedForAll via Proxy", async () => {
    await createToken(100);

    const setApprovalGasEstimate = await precompileCaller.estimateGas.setApprovalForAllProxy(alithSigner.address, true);
    const tx = await precompileCaller.setApprovalForAllProxy(alithSigner.address, true, {
      gasLimit: setApprovalGasEstimate,
    });
    await tx.wait();

    // Verify isApprovedForAll is correct
    expect(await precompileCaller.isApprovedForAllProxy(precompileCaller.address, alithSigner.address)).to.equal(true);

    // set approval to false
    const setApprovalGasEstimate2 = await precompileCaller.estimateGas.setApprovalForAllProxy(
      alithSigner.address,
      false,
    );
    const tx2 = await precompileCaller.setApprovalForAllProxy(alithSigner.address, false, {
      gasLimit: setApprovalGasEstimate2,
    });
    await tx2.wait();

    // Verify isApprovedForAll is removed
    expect(await precompileCaller.isApprovedForAllProxy(precompileCaller.address, alithSigner.address)).to.equal(false);
  });

  it("SafeTransferFrom approved address via Proxy", async () => {
    const initialIssuance = 100;
    const serialNumber = await createToken(initialIssuance);

    // Approve contract
    const setApprovalGasEstimate = await precompileCaller.estimateGas.setApprovalForAllProxy(alithSigner.address, true);
    const tx = await erc1155Precompile.setApprovalForAll(precompileCaller.address, true, {
      gasLimit: setApprovalGasEstimate,
    });
    await tx.wait();
    // Verify isApprovedForAll is correct
    expect(await erc1155Precompile.isApprovedForAll(bobSigner.address, precompileCaller.address)).to.equal(true);

    const transferAmount = 69;
    const callData = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("data"));
    const transferFromGasEstimate = await precompileCaller.estimateGas.safeTransferFromProxy(
      bobSigner.address,
      alithSigner.address,
      serialNumber,
      transferAmount,
      callData,
    );
    const tx2 = await precompileCaller.safeTransferFromProxy(
      bobSigner.address,
      alithSigner.address,
      serialNumber,
      transferAmount,
      callData,
      { gasLimit: transferFromGasEstimate },
    );
    await tx2.wait();

    // Verify ownership
    expect(await erc1155Precompile.balanceOf(bobSigner.address, serialNumber)).to.equal(
      initialIssuance - transferAmount,
    );
    expect(await erc1155Precompile.balanceOf(alithSigner.address, serialNumber)).to.equal(transferAmount);
  });

  it("SafeBatchTransferFrom approved address via Proxy", async () => {
    const initialIssuance1 = 101;
    const initialIssuance2 = 202;
    const serialNumber1 = await createToken(initialIssuance1);
    const serialNumber2 = await createToken(initialIssuance2);

    // Approve contract
    const tx = await erc1155Precompile.setApprovalForAll(precompileCaller.address, true, { gasLimit: 50000 });
    await tx.wait();
    // Verify isApprovedForAll is correct
    expect(await erc1155Precompile.isApprovedForAll(bobSigner.address, precompileCaller.address)).to.equal(true);

    const transferAmount1 = 69;
    const transferAmount2 = 71;
    const callData = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("data"));
    const tx2 = await precompileCaller.safeBatchTransferFromProxy(
      bobSigner.address,
      alithSigner.address,
      [serialNumber1, serialNumber2],
      [transferAmount1, transferAmount2],
      callData,
      { gasLimit: 50000 },
    );
    await tx2.wait();

    // Verify ownership
    expect(await erc1155Precompile.balanceOf(bobSigner.address, serialNumber1)).to.equal(
      initialIssuance1 - transferAmount1,
    );
    expect(await erc1155Precompile.balanceOf(alithSigner.address, serialNumber1)).to.equal(transferAmount1);
    expect(await erc1155Precompile.balanceOf(bobSigner.address, serialNumber2)).to.equal(
      initialIssuance2 - transferAmount2,
    );
    expect(await erc1155Precompile.balanceOf(alithSigner.address, serialNumber2)).to.equal(transferAmount2);
  });

  it("Ownable - owner, renounceOwnership, transferOwnership", async () => {
    // Check ownership is bob
    expect(await erc1155Precompile.owner()).to.equal(bobSigner.address);

    // Transfer ownership
    const transferOwnership = await erc1155Precompile.connect(bobSigner).transferOwnership(alithSigner.address);
    let receipt = await transferOwnership.wait();
    expect((receipt?.events as any)[0].event).to.equal("OwnershipTransferred");
    expect((receipt?.events as any)[0].args.previousOwner).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.newOwner).to.equal(alithSigner.address);

    // Check ownership is now alith
    expect(await erc1155Precompile.owner()).to.equal(alithSigner.address);

    // Renounce ownership
    const renounceOwnership = await erc1155Precompile.connect(alithSigner).renounceOwnership();
    receipt = await renounceOwnership.wait();
    expect((receipt?.events as any)[0].event).to.equal("OwnershipTransferred");
    expect((receipt?.events as any)[0].args.previousOwner).to.equal(alithSigner.address);
    expect((receipt?.events as any)[0].args.newOwner).to.equal(constants.AddressZero);

    // Check ownership is now zero address
    expect(await erc1155Precompile.owner()).to.equal(constants.AddressZero);
  });
});
