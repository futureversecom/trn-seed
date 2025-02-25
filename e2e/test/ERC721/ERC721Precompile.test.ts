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
  BurnAuth,
  ERC721_PRECOMPILE_ABI,
  NFT_PRECOMPILE_ABI,
  NFT_PRECOMPILE_ADDRESS,
  NodeProcess,
  ROOT_PRECOMPILE_ADDRESS,
  startNode,
  typedefs,
} from "../../common";

// NFT Collection information
const name = "test-collection";
const metadataPath = "https://example.com/nft/metadata/";
const initialIssuance = 12;
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

    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);

    // Setup Root api instance and keyring
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    const keyring = new Keyring({ type: "ethereum" });
    const bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));

    const provider = new JsonRpcProvider(`http://127.0.0.1:${node.rpcPort}`);
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
    let cursor, limit, new_cursor, tokens, total_owned;

    // First 5 tokens
    cursor = 0;
    limit = 5;
    [new_cursor, total_owned, tokens] = await erc721Precompile.ownedTokens(bobSigner.address, limit, cursor);
    expect(new_cursor).to.equal(5);
    expect(total_owned).to.equal(initialIssuance);
    expect(tokens).to.eql([0, 1, 2, 3, 4]);

    // Last 5 tokens, cursor should be 0 to indicate end of owned tokens
    cursor = 5;
    limit = 5;
    [new_cursor, total_owned, tokens] = await erc721Precompile.ownedTokens(bobSigner.address, limit, cursor);
    expect(new_cursor).to.equal(10);
    expect(total_owned).to.equal(initialIssuance);
    expect(tokens).to.eql([5, 6, 7, 8, 9]);

    // Tokens over owned tokens should return empty
    cursor = 10;
    limit = 5;
    [new_cursor, total_owned, tokens] = await erc721Precompile.ownedTokens(bobSigner.address, limit, cursor);
    expect(new_cursor).to.equal(0);
    expect(total_owned).to.equal(initialIssuance);
    expect(tokens).to.eql([10, 11]);

    // high limit should return ALL tokens owned by bob
    cursor = 0;
    limit = 500;
    [new_cursor, total_owned, tokens] = await erc721Precompile.ownedTokens(bobSigner.address, limit, cursor);
    expect(new_cursor).to.equal(0);
    expect(total_owned).to.equal(initialIssuance);
    expect(tokens).to.eql([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
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
      expect((receipt?.events as any)[i].args.from).to.equal(constants.AddressZero);
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

    tx = await newErc721Precompile.connect(bobSigner).mint(receiverAddress, quantity);
    receipt = await tx.wait();

    expect(await newErc721Precompile.balanceOf(receiverAddress)).to.equal(quantity);

    expect(await newErc721Precompile.totalSupply()).to.equal(quantity);
  });

  it("mint fails over max quantity per mint 1_000", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();

    // new collection with unlimited mintable supply
    const tx = await nftPrecompile.connect(bobSigner).initializeCollection(
      bobSigner.address,
      ethers.utils.hexlify(ethers.utils.toUtf8Bytes(name)),
      BigNumber.from(0), // no max issuance
      ethers.utils.hexlify(ethers.utils.toUtf8Bytes("https://example.com/nft/metadata/")),
      [alithSigner.address],
      [1000],
    );
    const receipt = await tx.wait();
    const erc721PrecompileAddress = (receipt?.events as any)[0].args.precompileAddress;
    const newErc721Precompile = new Contract(erc721PrecompileAddress, ERC721_PRECOMPILE_ABI, bobSigner);

    await newErc721Precompile
      .connect(bobSigner)
      .mint(receiverAddress, 1_001)
      .catch((err: any) => expect(err.message).contains("MintLimitExceeded"));
  });

  it("setMaxSupply", async () => {
    await erc721Precompile
      .connect(bobSigner)
      .setMaxSupply(100)
      .catch((err: any) => expect(err.message).contains("MaxIssuanceAlreadySet"));

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

    // setMaxSupply fails for new collection if not owner call
    await newErc721Precompile
      .connect(alithSigner)
      .setMaxSupply(100)
      .catch((err: any) => expect(err.message).contains("NotCollectionOwner"));

    // setMaxSupply succeeds for new collection if owner call
    tx = await newErc721Precompile.connect(bobSigner).setMaxSupply(100);
    receipt = await tx.wait();

    // validate event
    expect((receipt?.events as any)[0].event).to.equal("MaxSupplyUpdated");
    expect((receipt?.events as any)[0].args.maxSupply).to.equal(BigNumber.from(100));

    // setMaxSupply fails for new collection since supply was already set
    await newErc721Precompile
      .connect(bobSigner)
      .setMaxSupply(100)
      .catch((err: any) => expect(err.message).contains("MaxIssuanceAlreadySet"));
  });

  it("setBaseURI", async () => {
    // validate setBaseURI can only be set by owner
    await erc721Precompile
      .connect(alithSigner)
      .setBaseURI(ethers.utils.hexlify(ethers.utils.toUtf8Bytes("")))
      .catch((err: any) => expect(err.message).contains("NotCollectionOwner"));

    // validate cannot set invalid baseURI
    await erc721Precompile
      .connect(bobSigner)
      .setBaseURI(ethers.utils.hexlify(ethers.utils.toUtf8Bytes("abc")))
      .catch((err: any) => expect(err.message).contains("InvalidMetadataPath"));

    // validate setBaseURI can only be set by owner
    const tx = await erc721Precompile
      .connect(bobSigner)
      .setBaseURI(ethers.utils.hexlify(ethers.utils.toUtf8Bytes("https://example.com/nft/metadata/")));
    const receipt = await tx.wait();

    // validate event
    expect((receipt?.events as any)[0].event).to.equal("BaseURIUpdated");
    expect((receipt?.events as any)[0].args.baseURI).to.equal("https://example.com/nft/metadata/");

    // validate tokenURI returns correct URI
    expect(await erc721Precompile.tokenURI(0)).to.equal("https://example.com/nft/metadata/0");
  });

  it("togglePublicMint", async () => {
    // Enable public mint
    const tx = await erc721Precompile.connect(bobSigner).togglePublicMint(true);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("PublicMintToggled");
    expect((receipt?.events as any)[0].args.enabled).to.equal(true);

    // Disable again
    const tx2 = await erc721Precompile.connect(bobSigner).togglePublicMint(false);
    const receipt2 = await tx2.wait();
    expect((receipt2?.events as any)[0].event).to.equal("PublicMintToggled");
    expect((receipt2?.events as any)[0].args.enabled).to.equal(false);
  });

  it("setMintFee", async () => {
    const paymentAsset = ROOT_PRECOMPILE_ADDRESS;
    const mintFee = 100000;
    // Set Mint Fee
    const tx = await erc721Precompile.connect(bobSigner).setMintFee(paymentAsset, mintFee);
    const receipt = await tx.wait();
    expect((receipt?.events as any)[0].event).to.equal("MintFeeUpdated");
    expect((receipt?.events as any)[0].args.paymentAsset).to.equal(paymentAsset);
    expect((receipt?.events as any)[0].args.mintFee).to.equal(mintFee);

    // Set mint fee again
    const mintFee2 = 0;
    const tx2 = await erc721Precompile.connect(bobSigner).setMintFee(paymentAsset, mintFee2);
    const receipt2 = await tx2.wait();
    expect((receipt2?.events as any)[0].event).to.equal("MintFeeUpdated");
    expect((receipt2?.events as any)[0].args.paymentAsset).to.equal(paymentAsset);
    expect((receipt2?.events as any)[0].args.mintFee).to.equal(mintFee2);
  });

  // Tests whether enabling public mint allows any user to call mint and pay the mint fee
  it("enabling public mint works and charges fee", async () => {
    const tokenOwner = await Wallet.createRandom().getAddress();
    const paymentAsset = ROOT_PRECOMPILE_ADDRESS; // Use ROOT so we don't have to calculate fees
    const mintFee = 100000;
    const quantity = 10;

    // Trying to mint before enabling public mint should fail
    await erc721Precompile
      .connect(alithSigner)
      .mint(tokenOwner, quantity)
      .catch((err: any) => expect(err.message).contains("PublicMintDisabled"));

    // Set Mint Fee
    const mintFeeTx = await erc721Precompile.connect(bobSigner).setMintFee(paymentAsset, mintFee);
    await mintFeeTx.wait();

    // Enable public mint
    const togglePublicMintTx = await erc721Precompile.connect(bobSigner).togglePublicMint(true);
    await togglePublicMintTx.wait();

    const balanceBefore: any = ((await api.query.system.account(alithSigner.address)).toJSON() as any).data.free;
    const mintTx = await erc721Precompile.connect(alithSigner).mint(tokenOwner, quantity);
    await mintTx.wait();

    // Check tokenOwner received the tokens
    expect(await erc721Precompile.balanceOf(tokenOwner)).to.equal(quantity);

    // Calculate Alith's ROOT balance after
    const balanceAfter: any = ((await api.query.system.account(alithSigner.address)).toJSON() as any).data.free;
    const balanceDiff = balanceBefore - balanceAfter;
    const expectedDiff = mintFee * quantity;
    expect(balanceDiff).to.equal(expectedDiff);
  });

  it("transferFrom owner", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const tokenId = 0;

    // Transfer tokenId 0 to receiverAddress
    const tx = await erc721Precompile.connect(bobSigner).transferFrom(bobSigner.address, receiverAddress, tokenId);
    const receipt = await tx.wait();

    expect(receipt).to.emit(erc721Precompile, "Transfer").withArgs(bobSigner.address, receiverAddress, tokenId);

    // Receiver_address now owner of tokenId 1
    expect(await erc721Precompile.ownerOf(tokenId)).to.equal(receiverAddress);
  });

  it("approve and transferFrom via transaction", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const tokenId = 1;

    // getApproved should be zero address
    expect(await erc721Precompile.getApproved(tokenId)).to.equal(constants.AddressZero);

    // Bob approves alith for tokenId
    const approval = await erc721Precompile.approve(alithSigner.address, tokenId);
    expect(await approval.wait())
      .to.emit(erc721Precompile, "Approval")
      .withArgs(bobSigner.address, alithSigner.address, tokenId);

    // getApproved should be alith
    expect(await erc721Precompile.getApproved(tokenId)).to.equal(alithSigner.address);

    // alith transfers tokenId (Owned by Bob)
    const transfer = await erc721Precompile
      .connect(alithSigner)
      .transferFrom(bobSigner.address, receiverAddress, tokenId);

    expect(await transfer.wait())
      .to.emit(erc721Precompile, "Transfer")
      .withArgs(bobSigner.address, receiverAddress, tokenId);

    // Receiver_address now owner of tokenId
    expect(await erc721Precompile.ownerOf(tokenId)).to.equal(receiverAddress);
  });

  it("mint - over mintLimit fails", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const quantity = 1001; // MintLimit set to 1000 so this should fail

    const mint = await erc721Precompile.connect(alithSigner).mint(receiverAddress, quantity, { gasLimit: 50000 });
    await mint.wait().catch((err: any) => {
      expect(err.code).eq("CALL_EXCEPTION");
    });

    // Verify balance of receiver is 0
    expect(await erc721Precompile.balanceOf(receiverAddress)).to.equal(0);
  });

  it("setApprovalForAll, isApprovedForAll and safeTransferFrom", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const tokenId = 2;

    // Bob approves alith for tokenId
    const approval = await erc721Precompile.setApprovalForAll(alithSigner.address, true);
    let receipt = await approval.wait();
    expect((receipt?.events as any)[0].event).to.equal("ApprovalForAll");
    expect((receipt?.events as any)[0].args.owner).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.operator).to.equal(alithSigner.address);
    expect((receipt?.events as any)[0].args.approved).to.equal(true);

    // isApprovedForAll should be true
    expect(await erc721Precompile.isApprovedForAll(bobSigner.address, alithSigner.address)).to.be.true;

    // alith transfers tokenId (Owned by Bob)
    let transfer = await erc721Precompile
      .connect(alithSigner)
      .safeTransferFrom(bobSigner.address, receiverAddress, tokenId);
    receipt = await transfer.wait();
    expect((receipt?.events as any)[0].event).to.equal("Transfer");
    expect((receipt?.events as any)[0].args.from).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.to).to.equal(receiverAddress);
    expect((receipt?.events as any)[0].args.tokenId).to.equal(tokenId);

    // Receiver_address now owner of tokenId
    expect(await erc721Precompile.ownerOf(tokenId)).to.equal(receiverAddress);

    // Test sending to contracts implementing onErc721Received
    // Should Fail
    const factory = await ethers.getContractFactory("OnERC721ReceivedFails");
    const contractFails = await factory.connect(bobSigner).deploy();
    const tokenId_2 = 3;
    await erc721Precompile
      .connect(alithSigner)
      .estimateGas.safeTransferFrom(bobSigner.address, contractFails.address, tokenId_2, { gasLimit: 50000 })
      .catch((err) => expect(err.message).contains("ERC721: transfer to non ERC721Receiver implementer"));

    // Should succeed
    const factory2 = await ethers.getContractFactory("OnERC721ReceivedSucceeds");
    const contractSucceeds = await factory2.connect(bobSigner).deploy();
    transfer = await erc721Precompile
      .connect(alithSigner)
      .safeTransferFrom(bobSigner.address, contractSucceeds.address, tokenId_2, { gasLimit: 50000 });
    receipt = await transfer.wait();
    expect((receipt?.events as any)[0].event).to.equal("Transfer");
    expect((receipt?.events as any)[0].args.from).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.to).to.equal(contractSucceeds.address);
    expect((receipt?.events as any)[0].args.tokenId).to.equal(tokenId_2);
    expect(await erc721Precompile.ownerOf(tokenId_2)).to.equal(contractSucceeds.address);
  });

  it("name, symbol, ownerOf, tokenURI via EVM", async () => {
    const tokenId = 4;
    // Check state proxy calls
    expect(await precompileCaller.ownerOfProxy(tokenId)).to.equal(bobSigner.address);
    expect(await precompileCaller.nameProxy()).to.equal(name);
    expect(await precompileCaller.symbolProxy()).to.equal(name);
    expect(await precompileCaller.tokenURIProxy(tokenId)).to.equal(`https://example.com/nft/metadata/${tokenId}`);
  });

  it("approve and transferFrom via EVM", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const tokenId = 5;

    // Bob approves contract for tokenId
    const approval = await erc721Precompile.connect(bobSigner).approve(precompileCaller.address, tokenId);
    expect(await approval.wait())
      .to.emit(erc721Precompile, "Approval")
      .withArgs(bobSigner.address, precompileCaller.address, tokenId);
    // Approved should be correct
    expect(await erc721Precompile.getApproved(tokenId)).to.equal(precompileCaller.address);

    // Transfer tokenId to receiverAddress
    const transfer = await precompileCaller
      .connect(bobSigner)
      .transferFromProxy(bobSigner.address, receiverAddress, tokenId, {
        gasLimit: 50000,
      });
    expect(await transfer.wait())
      .to.emit(erc721Precompile, "Transfer")
      .withArgs(bobSigner.address, receiverAddress, tokenId);

    // contract_address now owner of tokenId
    expect(await precompileCaller.balanceOfProxy(receiverAddress)).to.equal(1);
    expect(await precompileCaller.ownerOfProxy(tokenId)).to.equal(receiverAddress);
  });

  it("approve and setApprovalForAll via EVM", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    let tokenId = 6;

    // Transfer NFT to contract so it can pass the approval
    let transfer = await erc721Precompile
      .connect(bobSigner)
      .transferFrom(bobSigner.address, precompileCaller.address, tokenId);
    await transfer.wait();
    // Check transfer worked, events asserted in previous tests
    expect(await precompileCaller.ownerOfProxy(tokenId)).to.equal(precompileCaller.address);

    // Approve receiverAddress
    const approve = await precompileCaller
      .connect(bobSigner)
      .approveProxy(alithSigner.address, tokenId, { gasLimit: 50000 });
    await approve.wait();

    // Check approval through proxy
    expect(await precompileCaller.getApprovedProxy(tokenId)).to.equal(alithSigner.address);

    // alith should now be able to transfer as she was approved with the above call
    transfer = await erc721Precompile
      .connect(alithSigner)
      .transferFrom(precompileCaller.address, receiverAddress, tokenId);
    await transfer.wait();
    // Check transfer worked
    expect(await precompileCaller.ownerOfProxy(tokenId)).to.equal(receiverAddress);

    tokenId = 7;
    // Transfer NFT to contract so it can pass the approval and transfer
    transfer = await erc721Precompile
      .connect(bobSigner)
      .transferFrom(bobSigner.address, precompileCaller.address, tokenId);
    await transfer.wait();
    // Check transfer worked
    expect(await precompileCaller.ownerOfProxy(tokenId)).to.equal(precompileCaller.address);

    // Approval before should be false
    expect(await precompileCaller.isApprovedForAllProxy(precompileCaller.address, alithSigner.address)).to.equal(false);

    const approvalForAll = await precompileCaller
      .connect(bobSigner)
      .setApprovalForAllProxy(alithSigner.address, true, { gasLimit: 50000 });
    await approvalForAll.wait();

    // Check approval through proxy
    expect(await precompileCaller.isApprovedForAllProxy(precompileCaller.address, alithSigner.address)).to.equal(true);

    // alith should now be able to transfer as she was approved with the above call
    transfer = await erc721Precompile
      .connect(alithSigner)
      .transferFrom(precompileCaller.address, receiverAddress, tokenId);
    await transfer.wait();
    // Check transfer worked
    expect(await precompileCaller.ownerOfProxy(tokenId)).to.equal(receiverAddress);
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

  it("burn", async () => {
    const tokenId = 9;

    // Sanity check
    const initial_balance = await erc721Precompile.balanceOf(bobSigner.address);

    // Burn tokenId
    const burn = await erc721Precompile.connect(bobSigner).burn(tokenId, { gasLimit: 50000 });

    expect(await burn.wait())
      .to.emit(erc721Precompile, "Transfer")
      .withArgs(bobSigner.address, constants.AddressZero, tokenId);

    // balance is now one less
    expect(await erc721Precompile.balanceOf(bobSigner.address)).to.equal(initial_balance - 1);
  });

  it("burn not approved fails", async () => {
    const tokenId = 10;

    // Set approval for all to false
    const approval = await erc721Precompile.setApprovalForAll(alithSigner.address, false);
    await approval.wait();

    // Sanity check
    const initial_balance = await erc721Precompile.balanceOf(bobSigner.address);

    // Burn tokenId from alith without approval should fail
    await erc721Precompile
      .connect(alithSigner)
      .burn(tokenId)
      .catch((err: any) => expect(err.message).contains("Caller not approved"));

    // balance is unchanged
    expect(await erc721Precompile.balanceOf(bobSigner.address)).to.equal(initial_balance);
  });

  it("burn as approved", async () => {
    const tokenId = 11;

    // Sanity check
    const initial_balance = await erc721Precompile.balanceOf(bobSigner.address);

    // Approve alith
    const approval = await erc721Precompile.connect(bobSigner).approve(alithSigner.address, tokenId);
    await approval.wait();

    // Burn tokenId from alith
    const burn = await erc721Precompile.connect(alithSigner).burn(tokenId, { gasLimit: 50000 });

    expect(await burn.wait())
      .to.emit(erc721Precompile, "Transfer")
      .withArgs(bobSigner.address, constants.AddressZero, tokenId);

    // balance is now one less
    const balance_after = await erc721Precompile.balanceOf(bobSigner.address);
    expect(balance_after).to.equal(initial_balance - 1);
  });

  it("owner, renounceOwnership, transferOwnership", async () => {
    // Check ownership is bob
    expect(await erc721Precompile.owner()).to.equal(bobSigner.address);

    // Transfer ownership
    const transferOwnership = await erc721Precompile.connect(bobSigner).transferOwnership(alithSigner.address);
    let receipt = await transferOwnership.wait();
    expect((receipt?.events as any)[0].event).to.equal("OwnershipTransferred");
    expect((receipt?.events as any)[0].args.previousOwner).to.equal(bobSigner.address);
    expect((receipt?.events as any)[0].args.newOwner).to.equal(alithSigner.address);

    // Check ownership is now alith
    expect(await erc721Precompile.owner()).to.equal(alithSigner.address);

    // Renounce ownership
    const renounceOwnership = await erc721Precompile.connect(alithSigner).renounceOwnership();
    receipt = await renounceOwnership.wait();
    expect((receipt?.events as any)[0].event).to.equal("OwnershipTransferred");
    expect((receipt?.events as any)[0].args.previousOwner).to.equal(alithSigner.address);
    expect((receipt?.events as any)[0].args.newOwner).to.equal(constants.AddressZero);

    // Check ownership is now zero address
    expect(await erc721Precompile.owner()).to.equal(constants.AddressZero);
  });

  it("supportsInterface", async () => {
    // ERC165
    expect(await erc721Precompile.supportsInterface(0x01ffc9a7)).to.be.true;
    // ERC721
    expect(await erc721Precompile.supportsInterface(0x80ac58cd)).to.be.true;
    // ERC721Metadata
    expect(await erc721Precompile.supportsInterface(0x5b5e139f)).to.be.true;
    // ERC721Burnable
    expect(await erc721Precompile.supportsInterface(0x42966c68)).to.be.true;
    // Ownable
    expect(await erc721Precompile.supportsInterface(0x0e083076)).to.be.true;
    // TRN721
    expect(await erc721Precompile.supportsInterface(0x2a4288ec)).to.be.true;

    // Test that 0xffffffff returns false (ERC165 requirement)
    expect(await erc721Precompile.supportsInterface(0xffffffff)).to.be.false;

    // Invalid random interface ID
    expect(await erc721Precompile.supportsInterface(0x12345678)).to.be.false;
  });

  it("supportsInterface via contract", async () => {
    // Deploy ERC721PrecompileERC165Validator contract
    const factory = await ethers.getContractFactory("ERC721PrecompileERC165Validator");
    const validator = await factory.connect(bobSigner).deploy();
    await validator.deployed();

    // Get all interface IDs from the validator contract
    const {
      erc165: erc165Id,
      erc721: erc721Id,
      erc721Metadata: metadataId,
      erc721Burnable: burnableId,
      trn721: trn721Id,
      ownable: ownableId,
    } = await validator.getAllInterfaceIds();

    // Validate individual interfaces
    expect(await erc721Precompile.supportsInterface(erc165Id)).to.be.true;
    expect(await erc721Precompile.supportsInterface(erc721Id)).to.be.true;
    expect(await erc721Precompile.supportsInterface(metadataId)).to.be.true;
    expect(await erc721Precompile.supportsInterface(burnableId)).to.be.true;
    expect(await erc721Precompile.supportsInterface(trn721Id)).to.be.true;
    expect(await erc721Precompile.supportsInterface(ownableId)).to.be.true;

    // Validate using the contract's validation function
    const [
      supportsERC165,
      supportsERC721,
      supportsERC721Metadata,
      supportsERC721Burnable,
      supportsTRN721,
      supportsOwnable,
    ] = await validator.validateContract(erc721Precompile.address);

    // Assert all interfaces are supported
    expect(supportsERC165).to.be.true;
    expect(supportsERC721).to.be.true;
    expect(supportsERC721Metadata).to.be.true;
    expect(supportsERC721Burnable).to.be.true;
    expect(supportsTRN721).to.be.true;
    expect(supportsOwnable).to.be.true;

    // // Log the interface IDs for reference
    // console.log("Interface IDs:");
    // console.log("ERC165:", erc165Id);
    // console.log("ERC721:", erc721Id);
    // console.log("ERC721Metadata:", metadataId);
    // console.log("ERC721Burnable:", burnableId);
    // console.log("TRN721:", trn721Id);
    // console.log("Ownable:", ownableId);
  });

  it("can issue and accept soulbound tokens", async () => {
    const receiverAddress = alithSigner.address;
    const quantity = 3;
    const receipt = await erc721Precompile.issue(receiverAddress, quantity, BurnAuth.Both).then((tx: any) => tx.wait());

    expect(receipt)
      .to.emit(erc721Precompile, "PendingIssuancesCreated")
      .withArgs(receiverAddress, [0, 1, 2], BurnAuth.Both);

    const pendingIssuances = await erc721Precompile.pendingIssuances(receiverAddress);
    expect(pendingIssuances[0]).to.deep.equal([0, 1, 2]);
    expect(pendingIssuances[1]).to.deep.equal([BurnAuth.Both, BurnAuth.Both, BurnAuth.Both]);

    for (const issuanceId of pendingIssuances[0]) {
      const receipt = await erc721Precompile
        .connect(alithSigner)
        .acceptIssuance(issuanceId)
        .then((tx: any) => tx.wait());

      const tokenId = receipt.events[0].args.tokenId;

      expect(receipt)
        .to.emit(erc721Precompile, "Issued")
        .withArgs(bobSigner.address, receiverAddress, tokenId, BurnAuth.Both);

      expect(await erc721Precompile.ownerOf(tokenId)).to.eq(receiverAddress);

      expect(await erc721Precompile.burnAuth(tokenId)).to.equal(BurnAuth.Both);
    }
  });
});
