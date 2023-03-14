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
  ERC721_PRECOMPILE_ABI,
  NFT_PRECOMPILE_ABI,
  NFT_PRECOMPILE_ADDRESS,
  NodeProcess,
  startNode,
  typedefs,
} from "../common";

// NFT Collection information
const name = "test-collection";
const metadataPath = { Https: "example.com/nft/metadata/" };
const initialIssuance = 10;
const maxIssuance = 100;

describe("ERC721 Precompile", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let bobSigner: Wallet;
  let alithSigner: Wallet;
  let nftPrecompile: Contract;
  let erc721Precompile: Contract;
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

    let erc721PrecompileAddress: string;

    // Create NFT collection using runtime, bob is collection owner
    await new Promise<void>((resolve, reject) => {
      api.tx.nft
        .createCollection(name, initialIssuance, maxIssuance, null, metadataPath, null, { xrpl: false })
        .signAndSend(bob, async ({ status, events }) => {
          if (status.isInBlock) {
            events.forEach(({ event: { data, method } }) => {
              if (method == "CollectionCreate") {
                const collection_uuid = (data.toJSON() as any)[0];
                console.log(`Collection UUID: ${collection_uuid}`);

                const collection_id_hex = (+collection_uuid).toString(16).padStart(8, "0");
                erc721PrecompileAddress = Web3.utils.toChecksumAddress(
                  `0xAAAAAAAA${collection_id_hex}000000000000000000000000`,
                );
                console.log(`NFT Collection Address: ${erc721PrecompileAddress}`);
                erc721Precompile = new Contract(erc721PrecompileAddress, ERC721_PRECOMPILE_ABI, bobSigner);
                resolve();
              }
            });
          }
        })
        .catch((err) => reject(err));
    });

    // Ethereum variables
    nftPrecompile = new Contract(NFT_PRECOMPILE_ADDRESS, NFT_PRECOMPILE_ABI, bobSigner);

    // Deploy PrecompileCaller contract
    const factory = await ethers.getContractFactory("ERC721PrecompileCaller");
    precompileCaller = await factory.connect(bobSigner).deploy(erc721PrecompileAddress!);
    await precompileCaller.deployed();
  });

  after(async () => await node.stop());

  it("name, symbol, ownerOf, tokenURI, balanceOf, totalSupply", async () => {
    expect(await erc721Precompile.name()).to.equal(name);
    expect(await erc721Precompile.symbol()).to.equal(name);
    await erc721Precompile
      .ownerOf(initialIssuance)
      .catch((err: any) => expect(err.message).contains("ERC721: Token does not exist"));
    expect(await erc721Precompile.ownerOf(initialIssuance - 1)).to.equal(bobSigner.address);
    expect(await erc721Precompile.balanceOf(bobSigner.address)).to.equal(initialIssuance);
    expect(await erc721Precompile.tokenURI(1)).to.equal("https://example.com/nft/metadata/1");
    expect(await erc721Precompile.totalSupply()).to.equal(initialIssuance);
  });

  it("ownedTokens", async () => {
    let cursor, limit, new_cursor, tokens;

    // First 5 tokens
    cursor = 0;
    limit = 5;
    [new_cursor, tokens] = await erc721Precompile.ownedTokens(bobSigner.address, limit, cursor);
    expect(new_cursor).to.equal(5);
    expect(tokens).to.eql([0, 1, 2, 3, 4]);

    // Last 5 tokens, cursor should be 0 to indicate end of owned tokens
    cursor = 5;
    limit = 5;
    [new_cursor, tokens] = await erc721Precompile.ownedTokens(bobSigner.address, limit, cursor);
    expect(new_cursor).to.equal(0);
    expect(tokens).to.eql([5, 6, 7, 8, 9]);

    // Tokens over owned tokens should return empty
    cursor = 10;
    limit = 5;
    [new_cursor, tokens] = await erc721Precompile.ownedTokens(bobSigner.address, limit, cursor);
    expect(new_cursor).to.equal(0);
    expect(tokens).to.eql([]);

    // high limit should return ALL tokens owned by bob
    cursor = 0;
    limit = 500;
    [new_cursor, tokens] = await erc721Precompile.ownedTokens(bobSigner.address, limit, cursor);
    expect(new_cursor).to.equal(0);
    expect(tokens).to.eql([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
  });

  it("mint - succeeds as owner", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const quantity = 6;

    const tx = await erc721Precompile.connect(bobSigner).mint(receiverAddress, quantity);
    const receipt = await tx.wait();

    for (let i = 0; i < quantity; i++) {
      // Check token ownership
      expect(await erc721Precompile.ownerOf(initialIssuance + i)).to.equal(receiverAddress);

      // Check event thrown
      expect((receipt?.events as any)[i].event).to.equal("Transfer");
      expect((receipt?.events as any)[i].args.from).to.equal(bobSigner.address);
      expect((receipt?.events as any)[i].args.to).to.equal(receiverAddress);
      expect((receipt?.events as any)[i].args.tokenId).to.equal(initialIssuance + i);
    }

    // verify balance
    expect(await erc721Precompile.balanceOf(receiverAddress)).to.equal(quantity);

    // Verify total supply updated
    expect(await erc721Precompile.totalSupply()).to.equal(initialIssuance + quantity);
  });

  it("mint - fails if max supply is exceeded", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();

    await erc721Precompile
      .connect(bobSigner)
      .mint(receiverAddress, 100)
      .catch((err: any) => expect(err.message).contains("MaxIssuanceReached"));
  });

  it("mint - no limit on mint if maxSupply not set", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const quantity = 499;

    // new collection with unlimited mintable supply
    let tx = await nftPrecompile.connect(bobSigner).initializeCollection(
      bobSigner.address,
      ethers.utils.hexlify(ethers.utils.toUtf8Bytes(name)),
      BigNumber.from(0), // no max issuance
      ethers.utils.hexlify(ethers.utils.toUtf8Bytes("https://example.com/nft/metadata/")),
      [alithSigner.address],
      [1000],
    );
    let receipt = await tx.wait();
    const erc721PrecompileAddress = (receipt?.events as any)[0].args.precompileAddress;
    const newErc721Precompile = new Contract(erc721PrecompileAddress, ERC721_PRECOMPILE_ABI, bobSigner);

  it("mint - over mintLimit fails", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const quantity = 1001; // MintLimit set to 1000 so this should fail

    const mint = await erc721Precompile.connect(alithSigner).mint(receiverAddress, quantity, { gasLimit: 50000 });
    await mint.wait().catch((err: any) => {
      expect(err.code).eq('CALL_EXCEPTION');
    });

    // Verify balance of alith is 0
    expect(await erc721Precompile.balanceOf(alithSigner.address)).to.equal(0);
  });

  it("approve and safeTransferFrom via EVM", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const tokenId = 8;

    // Bob approves contract for tokenId
    const approval = await erc721Precompile.connect(bobSigner).approve(precompileCaller.address, tokenId);
    await approval.wait();
    // Approved should be correct
    expect(await erc721Precompile.getApproved(tokenId)).to.equal(precompileCaller.address);

    // Transfer tokenId to receiverAddress
    const transfer = await precompileCaller
      .connect(bobSigner)
      .safeTransferFromProxy(bobSigner.address, receiverAddress, tokenId, { gasLimit: 50000 });
    expect(await transfer.wait())
      .to.emit(erc721Precompile, "Transfer")
      .withArgs(bobSigner.address, receiverAddress, tokenId);

    // receiver address now owner of tokenId
    expect(await precompileCaller.balanceOfProxy(receiverAddress)).to.equal(1);
    expect(await precompileCaller.ownerOfProxy(tokenId)).to.equal(receiverAddress);
  });

  it("owner, renounceOwnership, transferOwnership", async () => {
    // Check ownership is bob
    expect(await erc721Precompile.owner()).to.equal(bobSigner.address);

    // Transfer ownership
    const transferOwnership = await erc721Precompile.connect(bobSigner).transferOwnership(alithSigner.address);
    let receipt = await transferOwnership.wait();
    expect((receipt?.events as any)[0].event).to.equal("OwnershipTransferred");
    expect((receipt?.events as any)[0].args.oldOwner).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.newOwner).to.equal(alithSigner.address);

    // Check ownership is now alith
    expect(await erc721Precompile.owner()).to.equal(alithSigner.address);

    // Renounce ownership
    const renounceOwnership = await erc721Precompile.connect(alithSigner).renounceOwnership();
    receipt = await renounceOwnership.wait();
    expect((receipt?.events as any)[0].event).to.equal("OwnershipTransferred");
    expect((receipt?.events as any)[0].args.oldOwner).to.equal(alithSigner.address);
    expect((receipt?.events as any)[0].args.newOwner).to.equal(constants.AddressZero);

    // Check ownership is now zero address
    expect(await erc721Precompile.owner()).to.equal(constants.AddressZero);
  });
});
