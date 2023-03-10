import { SignerWithAddress } from "@nomiclabs/hardhat-ethers/signers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { AddressOrPair } from "@polkadot/api/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Contract, Wallet, constants } from "ethers";
import { ethers } from "hardhat";
import web3 from "web3";

import { BOB_PRIVATE_KEY, NodeProcess, startNode, typedefs } from "../common";

const erc721Abi = [
  "event Transfer(address indexed from, address indexed to, uint256 tokenId)",
  "event Approval(address indexed owner, address indexed approved, uint256 tokenId)",
  "event ApprovalForAll(address indexed owner, address indexed operator, bool approved)",
  "function balanceOf(address who) public view returns (uint256)",
  "function ownerOf(uint256 tokenId) public view returns (address)",
  "function safeTransferFrom(address from, address to, uint256 tokenId)",
  "function transferFrom(address from, address to, uint256 tokenId)",
  "function approve(address to, uint256 tokenId)",
  "function getApproved(uint256 tokenId) public view returns (address)",
  "function setApprovalForAll(address operator, bool _approved)",
  "function isApprovedForAll(address owner, address operator) public view returns (bool)",
  "function name() public view returns (string memory)",
  "function symbol() public view returns (string memory)",
  "function tokenURI(uint256 tokenId) public view returns (string memory)",
  // Root specific precompiles
  "function mint(address owner, uint32 quantity)",
  "function ownedTokens(address who, uint16 limit, uint32 cursor) public view returns(uint32, uint32[] memory)",
  // Ownable
  "event OwnershipTransferred(address indexed oldOwner, address newOwner)",
  "function owner() public view returns (address)",
  "function renounceOwnership()",
  "function transferOwnership(address owner)",
];

// NFT Collection information
const name = "test-collection";
const metadataPath = { Https: "example.com/nft/metadata" };
const initial_balance = 10;

describe.skip("ERC721 Precompile", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let keyring: Keyring;
  let bob: AddressOrPair;
  let bobSigner: SignerWithAddress;
  let alithSigner: SignerWithAddress;
  let nftContract: Contract;
  // Address for NFT collection
  let nftPrecompileAddress: string;
  let precompileCaller: Contract;

  // Setup api instance
  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);

    // Setup Root api instance and keyring
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    keyring = new Keyring({ type: "ethereum" });
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));
    [alithSigner, bobSigner] = await ethers.getSigners();

    // Create NFT collection using runtime, bob is collection owner
    await new Promise<void>((resolve, reject) => {
      api.tx.nft
        .createCollection(name, initial_balance, null, null, metadataPath, null)
        .signAndSend(bob, async ({ status, events }) => {
          if (status.isInBlock) {
            events.forEach(({ event: { data, method } }) => {
              if (method == "CollectionCreate") {
                const collection_uuid = (data.toJSON() as any)[0];
                const collection_id_hex = (+collection_uuid).toString(16).padStart(8, "0");
                nftPrecompileAddress = web3.utils.toChecksumAddress(
                  `0xAAAAAAAA${collection_id_hex}000000000000000000000000`,
                );
                nftContract = new Contract(nftPrecompileAddress, erc721Abi, bobSigner);
                resolve();
              }
            });
          }
        })
        .catch((err) => reject(err));
    });

    // Deploy PrecompileCaller contract
    const factory = await ethers.getContractFactory("ERC721PrecompileCaller");
    precompileCaller = await factory.connect(bobSigner).deploy(nftPrecompileAddress);
    await precompileCaller.deployed();
  });

  after(async () => await node.stop());

  it("name, symbol, ownerOf, tokenURI, balanceOf", async () => {
    expect(await nftContract.name()).to.equal(name);

    expect(await nftContract.symbol()).to.equal(name);

    expect(await nftContract.ownerOf(1)).to.equal(bobSigner.address);

    expect(await nftContract.balanceOf(bobSigner.address)).to.equal(initial_balance);

    expect(await nftContract.tokenURI(1)).to.equal("https://example.com/nft/metadata/1.json");
  });

  it("ownedTokens", async () => {
    let cursor, limit, new_cursor, tokens;

    // First 5 tokens
    cursor = 0;
    limit = 5;
    [new_cursor, tokens] = await nftContract.ownedTokens(bobSigner.address, limit, cursor);
    expect(new_cursor).to.equal(5);
    expect(tokens).to.eql([0, 1, 2, 3, 4]);

    // Last 5 tokens, cursor should be 0 to indicate end of owned tokens
    cursor = 5;
    limit = 5;
    [new_cursor, tokens] = await nftContract.ownedTokens(bobSigner.address, limit, cursor);
    expect(new_cursor).to.equal(0);
    expect(tokens).to.eql([5, 6, 7, 8, 9]);

    // Tokens over owned tokens should return empty
    cursor = 10;
    limit = 5;
    [new_cursor, tokens] = await nftContract.ownedTokens(bobSigner.address, limit, cursor);
    expect(new_cursor).to.equal(0);
    expect(tokens).to.eql([]);

    // high limit should return ALL tokens owned by bob
    cursor = 0;
    limit = 500;
    [new_cursor, tokens] = await nftContract.ownedTokens(bobSigner.address, limit, cursor);
    expect(new_cursor).to.equal(0);
    expect(tokens).to.eql([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
  });

  it("mint", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const quantity = 6;

    const mint = await nftContract.connect(bobSigner).mint(receiverAddress, quantity);
    const receipt = await mint.wait();

    for (let i = 0; i < quantity; i++) {
      // Check token ownership
      expect(await nftContract.ownerOf(initial_balance + i)).to.equal(receiverAddress);

      // Check event thrown
      expect((receipt?.events as any)[i].event).to.equal("Transfer");
      expect((receipt?.events as any)[i].args.from).to.equal(bobSigner.address);
      expect((receipt?.events as any)[i].args.to).to.equal(receiverAddress);
      expect((receipt?.events as any)[i].args.tokenId).to.equal(initial_balance + i);
    }

    // Verify balance is correct
    expect(await nftContract.balanceOf(receiverAddress)).to.equal(quantity);
  });

  it("transferFrom owner", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const serial_number = 0;

    // Transfer serial_number 0 to receiverAddress
    const transfer = await nftContract
      .connect(bobSigner)
      .transferFrom(bobSigner.address, receiverAddress, serial_number);
    expect(await transfer.wait())
      .to.emit(nftContract, "Transfer")
      .withArgs(bobSigner.address, receiverAddress, serial_number);

    // Receiver_address now owner of serial_number 1
    expect(await nftContract.ownerOf(serial_number)).to.equal(receiverAddress);
  });

  it("approve and transferFrom via transaction", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const serial_number = 1;

    // getApproved should be zero address
    expect(await nftContract.getApproved(serial_number)).to.equal(constants.AddressZero);

    // Bob approves alith for serial_number
    const approval = await nftContract.approve(alithSigner.address, serial_number);
    expect(await approval.wait())
      .to.emit(nftContract, "Approval")
      .withArgs(bobSigner.address, alithSigner.address, serial_number);

    // getApproved should be alith
    expect(await nftContract.getApproved(serial_number)).to.equal(alithSigner.address);

    // alith transfers serial_number (Owned by Bob)
    const transfer = await nftContract
      .connect(alithSigner)
      .transferFrom(bobSigner.address, receiverAddress, serial_number);

    expect(await transfer.wait())
      .to.emit(nftContract, "Transfer")
      .withArgs(bobSigner.address, receiverAddress, serial_number);

    // Receiver_address now owner of serial_number
    expect(await nftContract.ownerOf(serial_number)).to.equal(receiverAddress);
  });

  it("setApprovalForAll, isApprovedForAll and safeTransferFrom", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const serial_number = 2;

    // Bob approves alith for serial_number
    const approval = await nftContract.setApprovalForAll(alithSigner.address, true);
    let receipt = await approval.wait();
    expect((receipt?.events as any)[0].event).to.equal("ApprovalForAll");
    expect((receipt?.events as any)[0].args.owner).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.operator).to.equal(alithSigner.address);
    expect((receipt?.events as any)[0].args.approved).to.equal(true);

    // isApprovedForAll should be true
    expect(await nftContract.isApprovedForAll(bobSigner.address, alithSigner.address)).to.be.true;

    // alith transfers serial_number (Owned by Bob)
    let transfer = await nftContract
      .connect(alithSigner)
      .safeTransferFrom(bobSigner.address, receiverAddress, serial_number);
    receipt = await transfer.wait();
    expect((receipt?.events as any)[0].event).to.equal("Transfer");
    expect((receipt?.events as any)[0].args.from).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.to).to.equal(receiverAddress);
    expect((receipt?.events as any)[0].args.tokenId).to.equal(serial_number);

    // Receiver_address now owner of serial_number
    expect(await nftContract.ownerOf(serial_number)).to.equal(receiverAddress);

    // Test sending to contracts implementing onErc721Received
    // Should Fail
    const factory = await ethers.getContractFactory("OnERC721ReceivedFails");
    const contractFails = await factory.connect(bobSigner).deploy();
    const serial_number_2 = 3;
    await nftContract
      .connect(alithSigner)
      .estimateGas.safeTransferFrom(bobSigner.address, contractFails.address, serial_number_2, { gasLimit: 50000 })
      .catch((err) => expect(err.message).contains("ERC721: transfer to non ERC721Receiver implementer"));

    // Should succeed
    const factory2 = await ethers.getContractFactory("OnERC721ReceivedSucceeds");
    const contractSucceeds = await factory2.connect(bobSigner).deploy();
    transfer = await nftContract
      .connect(alithSigner)
      .safeTransferFrom(bobSigner.address, contractSucceeds.address, serial_number_2, { gasLimit: 50000 });
    receipt = await transfer.wait();
    expect((receipt?.events as any)[0].event).to.equal("Transfer");
    expect((receipt?.events as any)[0].args.from).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.to).to.equal(contractSucceeds.address);
    expect((receipt?.events as any)[0].args.tokenId).to.equal(serial_number_2);
    expect(await nftContract.ownerOf(serial_number_2)).to.equal(contractSucceeds.address);
  });

  it("name, symbol, ownerOf, tokenURI via EVM", async () => {
    const serial_number = 4;

    // Check state proxy calls
    expect(await precompileCaller.ownerOfProxy(serial_number)).to.equal(bobSigner.address);

    expect(await precompileCaller.nameProxy()).to.equal(name);

    expect(await precompileCaller.symbolProxy()).to.equal(name);

    expect(await precompileCaller.tokenURIProxy(serial_number)).to.equal(
      `https://example.com/nft/metadata/${serial_number}.json`,
    );
  });

  it("approve and transferFrom via EVM", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const serial_number = 5;

    // Bob approves contract for serial_number
    const approval = await nftContract.connect(bobSigner).approve(precompileCaller.address, serial_number);
    expect(await approval.wait())
      .to.emit(nftContract, "Approval")
      .withArgs(bobSigner.address, precompileCaller.address, serial_number);
    // Approved should be correct
    expect(await nftContract.getApproved(serial_number)).to.equal(precompileCaller.address);

    // Transfer serial_number to receiverAddress
    const transfer = await precompileCaller
      .connect(bobSigner)
      .transferFromProxy(bobSigner.address, receiverAddress, serial_number, {
        gasLimit: 50000,
      });
    expect(await transfer.wait())
      .to.emit(nftContract, "Transfer")
      .withArgs(bobSigner.address, receiverAddress, serial_number);

    // contract_address now owner of serial_number
    expect(await precompileCaller.balanceOfProxy(receiverAddress)).to.equal(1);
    expect(await precompileCaller.ownerOfProxy(serial_number)).to.equal(receiverAddress);
  });

  it("approve and setApprovalForAll via EVM", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    let serial_number = 6;

    // Transfer NFT to contract so it can pass the approval
    let transfer = await nftContract
      .connect(bobSigner)
      .transferFrom(bobSigner.address, precompileCaller.address, serial_number);
    await transfer.wait();
    // Check transfer worked, events asserted in previous tests
    expect(await precompileCaller.ownerOfProxy(serial_number)).to.equal(precompileCaller.address);

    // Approve receiverAddress
    const approve = await precompileCaller
      .connect(bobSigner)
      .approveProxy(alithSigner.address, serial_number, { gasLimit: 50000 });
    await approve.wait();

    // Check approval through proxy
    expect(await precompileCaller.getApprovedProxy(serial_number)).to.equal(alithSigner.address);

    // alith should now be able to transfer as she was approved with the above call
    transfer = await nftContract
      .connect(alithSigner)
      .transferFrom(precompileCaller.address, receiverAddress, serial_number);
    await transfer.wait();
    // Check transfer worked
    expect(await precompileCaller.ownerOfProxy(serial_number)).to.equal(receiverAddress);

    serial_number = 7;
    // Transfer NFT to contract so it can pass the approval and transfer
    transfer = await nftContract
      .connect(bobSigner)
      .transferFrom(bobSigner.address, precompileCaller.address, serial_number);
    await transfer.wait();
    // Check transfer worked
    expect(await precompileCaller.ownerOfProxy(serial_number)).to.equal(precompileCaller.address);

    // Approval before should be false
    expect(await precompileCaller.isApprovedForAllProxy(precompileCaller.address, alithSigner.address)).to.equal(false);

    const approvalForAll = await precompileCaller
      .connect(bobSigner)
      .setApprovalForAllProxy(alithSigner.address, true, { gasLimit: 50000 });
    await approvalForAll.wait();

    // Check approval through proxy
    expect(await precompileCaller.isApprovedForAllProxy(precompileCaller.address, alithSigner.address)).to.equal(true);

    // alith should now be able to transfer as she was approved with the above call
    transfer = await nftContract
      .connect(alithSigner)
      .transferFrom(precompileCaller.address, receiverAddress, serial_number);
    await transfer.wait();
    // Check transfer worked
    expect(await precompileCaller.ownerOfProxy(serial_number)).to.equal(receiverAddress);
  });

  it("approve and safeTransferFrom via EVM", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const serial_number = 8;

    // Bob approves contract for serial_number
    const approval = await nftContract.connect(bobSigner).approve(precompileCaller.address, serial_number);
    await approval.wait();
    // Approved should be correct
    expect(await nftContract.getApproved(serial_number)).to.equal(precompileCaller.address);

    // Transfer serial_number to receiverAddress
    const transfer = await precompileCaller
      .connect(bobSigner)
      .safeTransferFromProxy(bobSigner.address, receiverAddress, serial_number, { gasLimit: 50000 });
    expect(await transfer.wait())
      .to.emit(nftContract, "Transfer")
      .withArgs(bobSigner.address, receiverAddress, serial_number);

    // receiver address now owner of serial_number
    expect(await precompileCaller.balanceOfProxy(receiverAddress)).to.equal(1);
    expect(await precompileCaller.ownerOfProxy(serial_number)).to.equal(receiverAddress);
  });

  it("owner, renounceOwnership, transferOwnership", async () => {
    // Check ownership is bob
    expect(await nftContract.owner()).to.equal(bobSigner.address);

    // Transfer ownership
    const transferOwnership = await nftContract.connect(bobSigner).transferOwnership(alithSigner.address);
    let receipt = await transferOwnership.wait();
    expect((receipt?.events as any)[0].event).to.equal("OwnershipTransferred");
    expect((receipt?.events as any)[0].args.oldOwner).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.newOwner).to.equal(alithSigner.address);

    // Check ownership is now alith
    expect(await nftContract.owner()).to.equal(alithSigner.address);

    // Renounce ownership
    const renounceOwnership = await nftContract.connect(alithSigner).renounceOwnership();
    receipt = await renounceOwnership.wait();
    expect((receipt?.events as any)[0].event).to.equal("OwnershipTransferred");
    expect((receipt?.events as any)[0].args.oldOwner).to.equal(alithSigner.address);
    expect((receipt?.events as any)[0].args.newOwner).to.equal(constants.AddressZero);

    // Check ownership is now zero address
    expect(await nftContract.owner()).to.equal(constants.AddressZero);
  });
});
