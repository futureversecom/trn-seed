import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { AddressOrPair } from "@polkadot/api/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { constants, Contract, Wallet } from "ethers";
import { ethers } from "hardhat";
import web3 from "web3";

import { typedefs } from "../common";

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

const nftAbi = [
	"event InitializeCollection(address indexed collectionOwner, address precompileAddress)",
	"function initializeCollection(address owner, bytes name, uint32 maxIssuance, uint8 metadataType, bytes metadataPath, address[] royaltyAddresses, uint32[] royaltyEntitlements) returns (address, uint32)",
];

// NFT Collection information
const name = "test-collection";
const metadataPath = { Https: "example.com/nft/metadata" };
const initial_balance = 10;

describe("ERC721 Precompile", function () {
	let api: ApiPromise;
	let keyring: Keyring;
	let bob: AddressOrPair;
	let bobSigner: any;
	let aliceSigner: any;
	let nftContract: Contract;
	// Address for NFT collection
	let nftPrecompileAddress: string;
	let precompileCaller: Contract;
	// Setup api instance
	before(async () => {
		const wsProvider = new WsProvider(`ws://localhost:9944`);

		// Setup Root api instance and keyring
		api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
		keyring = new Keyring({ type: "ethereum" });
		bob = keyring.addFromSeed(
			hexToU8a(
				"0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf"
			)
		);
		[aliceSigner, bobSigner] = await ethers.getSigners();

		// Create NFT collection using runtime, bob is collection owner
		await new Promise<void>((resolve) => {
			api.tx.nft
				.createCollection(name, initial_balance, null, null, metadataPath, null)
				.signAndSend(bob, async ({ status, events }) => {
					if (status.isInBlock) {
						events.forEach(({ event: { data, method } }) => {
							if (method == "CollectionCreate") {
								let collection_uuid = (data.toJSON() as any)[0];
								const collection_id_hex = (+collection_uuid)
									.toString(16)
									.padStart(8, "0");
								nftPrecompileAddress = web3.utils.toChecksumAddress(
									`0xAAAAAAAA${collection_id_hex}000000000000000000000000`
								);
								nftContract = new Contract(
									nftPrecompileAddress,
									erc721Abi,
									bobSigner
								);
								resolve();
							}
						});
					}
				});
		});

		// Deploy PrecompileCaller contract
		const factory = await ethers.getContractFactory("ERC721PrecompileCaller");
		precompileCaller = await factory
			.connect(bobSigner)
			.deploy(nftPrecompileAddress);
	});

	it("name, symbol, ownerOf, tokenURI, balanceOf", async () => {
		expect(await nftContract.name()).to.equal(name);

		expect(await nftContract.symbol()).to.equal(name);

		expect(await nftContract.ownerOf(1)).to.equal(bobSigner.address);

		expect(await nftContract.balanceOf(bobSigner.address)).to.equal(
			initial_balance
		);

		expect(await nftContract.tokenURI(1)).to.equal(
			"https://example.com/nft/metadata/1.json"
		);
	});

	it("ownedTokens", async () => {
		let cursor, limit, new_cursor, tokens;

		// First 5 tokens
		cursor = 0;
		limit = 5;
		[new_cursor, tokens] = await nftContract.ownedTokens(
			bobSigner.address,
			limit,
			cursor
		);
		expect(new_cursor).to.equal(5);
		expect(tokens).to.eql([0, 1, 2, 3, 4]);

		// Last 5 tokens, cursor should be 0 to indicate end of owned tokens
		cursor = 5;
		limit = 5;
		[new_cursor, tokens] = await nftContract.ownedTokens(
			bobSigner.address,
			limit,
			cursor
		);
		expect(new_cursor).to.equal(0);
		expect(tokens).to.eql([5, 6, 7, 8, 9]);

		// Tokens over owned tokens should return empty
		cursor = 10;
		limit = 5;
		[new_cursor, tokens] = await nftContract.ownedTokens(
			bobSigner.address,
			limit,
			cursor
		);
		expect(new_cursor).to.equal(0);
		expect(tokens).to.eql([]);

		// high limit should return ALL tokens owned by bob
		cursor = 0;
		limit = 500;
		[new_cursor, tokens] = await nftContract.ownedTokens(
			bobSigner.address,
			limit,
			cursor
		);
		expect(new_cursor).to.equal(0);
		expect(tokens).to.eql([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
	});

	it("mint", async () => {
		const receiverAddress = await Wallet.createRandom().getAddress();
		const quantity = 6;

		const mint = await nftContract
			.connect(bobSigner)
			.mint(receiverAddress, quantity);
		let receipt = await mint.wait();

		for (let i = 0; i < quantity; i++) {
			// Check token ownership
			expect(await nftContract.ownerOf(initial_balance + i)).to.equal(
				receiverAddress
			);

			// Check event thrown
			expect((receipt?.events as any)[i].event).to.equal("Transfer");
			expect((receipt?.events as any)[i].args.from).to.equal(bobSigner.address);
			expect((receipt?.events as any)[i].args.to).to.equal(receiverAddress);
			expect((receipt?.events as any)[i].args.tokenId).to.equal(
				initial_balance + i
			);
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

		// Bob approves alice for serial_number
		const approval = await nftContract.approve(
			aliceSigner.address,
			serial_number
		);
		expect(await approval.wait())
			.to.emit(nftContract, "Approval")
			.withArgs(bobSigner.address, aliceSigner.address, serial_number);

		// getApproved should be alice
		expect(await nftContract.getApproved(serial_number)).to.equal(
			aliceSigner.address
		);

		// Alice transfers serial_number (Owned by Bob)
		const transfer = await nftContract
			.connect(aliceSigner)
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

		// Bob approves alice for serial_number
		const approval = await nftContract.setApprovalForAll(
			aliceSigner.address,
			true
		);
		let receipt = await approval.wait();
		expect((receipt?.events as any)[0].event).to.equal("ApprovalForAll");
		expect((receipt?.events as any)[0].args.owner).to.equal(bobSigner.address);
		expect((receipt?.events as any)[0].args.operator).to.equal(
			aliceSigner.address
		);
		expect((receipt?.events as any)[0].args.approved).to.equal(true);

		// isApprovedForAll should be true
		expect(
			await nftContract.isApprovedForAll(bobSigner.address, aliceSigner.address)
		).to.be.true;

		// Alice transfers serial_number (Owned by Bob)
		let transfer = await nftContract
			.connect(aliceSigner)
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
			.connect(aliceSigner)
			.estimateGas.safeTransferFrom(
				bobSigner.address,
				contractFails.address,
				serial_number_2,
				{ gasLimit: 50000 }
			)
			.catch((err) =>
				expect(err.message).contains(
					"ERC721: transfer to non ERC721Receiver implementer"
				)
			);

		// Should succeed
		const factory2 = await ethers.getContractFactory(
			"OnERC721ReceivedSucceeds"
		);
		const contractSucceeds = await factory2.connect(bobSigner).deploy();
		transfer = await nftContract
			.connect(aliceSigner)
			.safeTransferFrom(
				bobSigner.address,
				contractSucceeds.address,
				serial_number_2,
				{ gasLimit: 50000 }
			);
		receipt = await transfer.wait();
		expect((receipt?.events as any)[0].event).to.equal("Transfer");
		expect((receipt?.events as any)[0].args.from).to.equal(bobSigner.address);
		expect((receipt?.events as any)[0].args.to).to.equal(
			contractSucceeds.address
		);
		expect((receipt?.events as any)[0].args.tokenId).to.equal(serial_number_2);
		expect(await nftContract.ownerOf(serial_number_2)).to.equal(
			contractSucceeds.address
		);
	});

	it("name, symbol, ownerOf, tokenURI via EVM", async () => {
		const serial_number = 5;

		// Check state proxy calls
		expect(await precompileCaller.ownerOfProxy(serial_number)).to.equal(
			bobSigner.address
		);

		expect(await precompileCaller.nameProxy()).to.equal(name);

		expect(await precompileCaller.symbolProxy()).to.equal(name);

		expect(await precompileCaller.tokenURIProxy(serial_number)).to.equal(
			`https://example.com/nft/metadata/${serial_number}.json`
		);
	});

	it("approve and transferFrom via EVM", async () => {
		const receiverAddress = await Wallet.createRandom().getAddress();
		const serial_number = 6;

		// Bob approves contract for serial_number
		const approval = await nftContract
			.connect(bobSigner)
			.approve(precompileCaller.address, serial_number);
		expect(await approval.wait())
			.to.emit(nftContract, "Approval")
			.withArgs(bobSigner.address, precompileCaller.address, serial_number);
		// Approved should be correct
		expect(await nftContract.getApproved(serial_number)).to.equal(
			precompileCaller.address
		);

  it('name, symbol, ownerOf, tokenURI via EVM', async () => {
    const serial_number = 4;

		// contract_address now owner of serial_number
		expect(await precompileCaller.balanceOfProxy(receiverAddress)).to.equal(1);
		expect(await precompileCaller.ownerOfProxy(serial_number)).to.equal(
			receiverAddress
		);
	});

	it("owner, renounceOwnership, transferOwnership", async () => {
		// Check ownership is bob
		expect(await nftContract.owner()).to.equal(bobSigner.address);

		// Transfer ownership
		const transferOwnership = await nftContract
			.connect(bobSigner)
			.transferOwnership(aliceSigner.address);
		let receipt = await transferOwnership.wait();
		expect((receipt?.events as any)[0].event).to.equal("OwnershipTransferred");
		expect((receipt?.events as any)[0].args.oldOwner).to.equal(
			bobSigner.address
		);
		expect((receipt?.events as any)[0].args.newOwner).to.equal(
			aliceSigner.address
		);

		// Check ownership is now alice
		expect(await nftContract.owner()).to.equal(aliceSigner.address);

		// Renounce ownership
		const renounceOwnership = await nftContract
			.connect(aliceSigner)
			.renounceOwnership();
		receipt = await renounceOwnership.wait();
		expect((receipt?.events as any)[0].event).to.equal("OwnershipTransferred");
		expect((receipt?.events as any)[0].args.oldOwner).to.equal(
			aliceSigner.address
		);
		expect((receipt?.events as any)[0].args.newOwner).to.equal(
			constants.AddressZero
		);

  it('approve and transferFrom via EVM', async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const serial_number = 5;

	it("initialize collection", async () => {
		// Precompile address for nft precompile is 1721
		const nftPrecompileAddress = "0x00000000000000000000000000000000000006b9";
		const nftProxy = new Contract(nftPrecompileAddress, nftAbi, bobSigner);

		const owner = aliceSigner.address;
		const name = ethers.utils.formatBytes32String("My Collection");
		const maxIssuance = 100;
		const metadataType = 1;
		const metadataPath = ethers.utils.formatBytes32String("example.com");
		const royaltyAddresses = [aliceSigner.address];
		const royaltyEntitlements = [1000];

		// Generate expected precompile address
		const collectionId = await api.query.nft.nextCollectionId();
		const collectionIdBin = (+collectionId).toString(2).padStart(22, "0");
		const parachainIdBin = (100).toString(2).padStart(10, "0");
		const collectionUuid = parseInt(collectionIdBin + parachainIdBin, 2);
		const collectionIdHex = (+collectionUuid).toString(16).padStart(8, "0");
		const precompile_address = web3.utils.toChecksumAddress(
			`0xAAAAAAAA${collectionIdHex}000000000000000000000000`
		);


  it('approve and setApprovalForAll via EVM', async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    let serial_number = 6;

    // Transfer NFT to contract so it can pass the approval
    let transfer = await nftContract.connect(bobSigner).transferFrom(bobSigner.address, precompileCaller.address, serial_number);
    await transfer.wait();
    // Check transfer worked, events asserted in previous tests
    expect(
        await precompileCaller.ownerOfProxy(serial_number)
    ).to.equal(precompileCaller.address);

    // Approve receiverAddress
    const approve = await precompileCaller.connect(bobSigner).approveProxy(aliceSigner.address, serial_number, {gasLimit: 50000});
    await approve.wait();

    // Check approval through proxy
    expect(
        await precompileCaller.getApprovedProxy(serial_number)
    ).to.equal(aliceSigner.address);

    // Alice should now be able to transfer as she was approved with the above call
    transfer = await nftContract.connect(aliceSigner).transferFrom(precompileCaller.address, receiverAddress, serial_number);
    await transfer.wait();
    // Check transfer worked
    expect(
        await precompileCaller.ownerOfProxy(serial_number)
    ).to.equal(receiverAddress);

    serial_number = 7;
    // Transfer NFT to contract so it can pass the approval and transfer
    transfer = await nftContract.connect(bobSigner).transferFrom(bobSigner.address, precompileCaller.address, serial_number);
    await transfer.wait();
    // Check transfer worked
    expect(
        await precompileCaller.ownerOfProxy(serial_number)
    ).to.equal(precompileCaller.address);

    // Approval before should be false
    expect(
        await precompileCaller.isApprovedForAllProxy(precompileCaller.address, aliceSigner.address)
    ).to.equal(false);

    const approvalForAll = await precompileCaller.connect(bobSigner).setApprovalForAllProxy(aliceSigner.address, true, {gasLimit: 50000});
    await approvalForAll.wait();

    // Check approval through proxy
    expect(
        await precompileCaller.isApprovedForAllProxy(precompileCaller.address, aliceSigner.address)
    ).to.equal(true);

    // Alice should now be able to transfer as she was approved with the above call
    transfer = await nftContract.connect(aliceSigner).transferFrom(precompileCaller.address, receiverAddress, serial_number);
    await transfer.wait();
    // Check transfer worked
    expect(
        await precompileCaller.ownerOfProxy(serial_number)
    ).to.equal(receiverAddress);
  });


  it('approve and safeTransferFrom via EVM', async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const serial_number = 8;

    // Bob approves contract for serial_number
    const approval = await nftContract.connect(bobSigner).approve(precompileCaller.address, serial_number);
    await approval.wait();
    // Approved should be correct
    expect(
        await nftContract.getApproved(serial_number)
    ).to.equal(precompileCaller.address);

    // Transfer serial_number to receiverAddress
    const transfer = await precompileCaller
        .connect(bobSigner)
        .safeTransferFromProxy(bobSigner.address, receiverAddress, serial_number, {gasLimit: 50000})
    expect(
        await transfer.wait()
    ).to.emit(nftContract, 'Transfer').withArgs(bobSigner.address, receiverAddress, serial_number);

    // receiver address now owner of serial_number
    expect(
        await precompileCaller.balanceOfProxy(receiverAddress)
    ).to.equal(1);
    expect(
        await precompileCaller.ownerOfProxy(serial_number)
    ).to.equal(receiverAddress);
  });


  it('owner, renounceOwnership, transferOwnership', async () => {
    // Check ownership is bob
    expect(
        await nftContract.owner()
    ).to.equal(bobSigner.address);

    // Transfer ownership
    const transferOwnership = await nftContract.connect(bobSigner).transferOwnership(aliceSigner.address);
    let receipt = await transferOwnership.wait();
    expect((receipt?.events as any)[0].event).to.equal('OwnershipTransferred');
    expect((receipt?.events as any)[0].args.oldOwner).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.newOwner).to.equal(aliceSigner.address);

    // Check ownership is now alice
    expect(
        await nftContract.owner()
    ).to.equal(aliceSigner.address);

    // Renounce ownership
    const renounceOwnership = await nftContract.connect(aliceSigner).renounceOwnership();
    receipt = await renounceOwnership.wait();
    expect((receipt?.events as any)[0].event).to.equal('OwnershipTransferred');
    expect((receipt?.events as any)[0].args.oldOwner).to.equal(aliceSigner.address);
    expect((receipt?.events as any)[0].args.newOwner).to.equal(constants.AddressZero);

    // Check ownership is now zero address
    expect(
        await nftContract.owner()
    ).to.equal(constants.AddressZero);
  });


  it('initialize collection', async () => {
    // Precompile address for nft precompile is 1721
    const nftPrecompileAddress = '0x00000000000000000000000000000000000006b9';
    const nftProxy = new Contract(nftPrecompileAddress, nftAbi, bobSigner);

    const owner = aliceSigner.address;
    const name = ethers.utils.formatBytes32String("My Collection");
    const maxIssuance = 100;
    const metadataType = 1;
    const metadataPath = ethers.utils.formatBytes32String("example.com");
    const royaltyAddresses = [aliceSigner.address];
    const royaltyEntitlements = [1000];

    // Generate expected precompile address
    const collectionId = await api.query.nft.nextCollectionId();
    const collectionIdBin = (+collectionId).toString(2).padStart(22, '0');
    const parachainIdBin = (100).toString(2).padStart(10, '0');
    const collectionUuid = parseInt(collectionIdBin + parachainIdBin, 2);
    const collectionIdHex = (+collectionUuid).toString(16).padStart(8, '0');
    const precompile_address = web3.utils.toChecksumAddress(`0xAAAAAAAA${collectionIdHex}000000000000000000000000`);

    const initializeTx = await nftProxy.connect(bobSigner).initializeCollection(
        owner,
        name,
        maxIssuance,
        metadataType,
        metadataPath,
        royaltyAddresses,
        royaltyEntitlements
    );
    let receipt = await initializeTx.wait();
    expect((receipt?.events as any)[0].event).to.equal('InitializeCollection');
    expect((receipt?.events as any)[0].args.collectionOwner).to.equal(aliceSigner.address);
    expect((receipt?.events as any)[0].args.precompileAddress).to.equal(precompile_address);
  });
});
