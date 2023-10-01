import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { BigNumber, Contract, Wallet, constants } from "ethers";
import { ethers } from "hardhat";
import Web3 from "web3";
import web3 from "web3";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC721_PRECOMPILE_ABI,
  MARKETPLACE_PRECOMPILE_ADDRESS,
  MARKET_PLACE_ABI,
  NFT_PRECOMPILE_ABI,
  NFT_PRECOMPILE_ADDRESS,
  NodeProcess,
  getCollectionPrecompileAddress,
  startNode,
  typedefs,
} from "../../common";

// NFT Collection information
const name = "test-collection";
const metadataPath = "https://example.com/nft/metadata/";
const initialIssuance = 0;
const maxIssuance = 1000;

describe.only("Marketplace Precompile", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let bobSigner: Wallet;
  let alithSigner: Wallet;
  let nftPrecompile: Contract;
  let erc721Precompile: Contract;
  let marketPlacePrecompile: Contract;
  let erc721PrecompileAddress: string;

  // Setup api instance
  before(async () => {
    // node = await startNode();

    const wsProvider = new WsProvider(`ws://localhost:9944`);

    // Setup Root api instance and keyring
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    const keyring = new Keyring({ type: "ethereum" });
    const bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));

    const provider = new JsonRpcProvider(`http://127.0.0.1:9933`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);

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
                // erc721PrecompileAddress = getCollectionPrecompileAddress(collection_uuid);
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

    // // Deploy PrecompileCaller contract
    // const factory = await ethers.getContractFactory("ERC721PrecompileCaller");
    // precompileCaller = await factory.connect(bobSigner).deploy(erc721PrecompileAddress!);
    // await precompileCaller.deployed();

    // Deploy marketplace contract
    marketPlacePrecompile = new Contract(MARKETPLACE_PRECOMPILE_ADDRESS, MARKET_PLACE_ABI, bobSigner);
  });

  // after(async () => await node.stop());

  it("register marketplace", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    console.log("receiverAddress::", receiverAddress);
    const entitlements = 1000;
    const tx = await marketPlacePrecompile.connect(bobSigner).registerMarketplace(receiverAddress, entitlements);
    const receipt = await tx.wait();
    const [creator, marketplaceId, marketplaceAccount] = (receipt?.events as any)[0].args;
    console.log("marketplaceId:::", marketplaceId.toString());
    expect((receipt?.events as any)[0].event).to.equal("MarketplaceRegister");
    expect(creator).to.equal(bobSigner.address);
    expect(marketplaceId.toNumber()).to.gte(0);
    expect(marketplaceAccount).to.equal(receiverAddress);

    // verify marketplace account when id is given
    const marketplaceAccountFromPrecompile = await marketPlacePrecompile.getMarketplaceAccount(
      marketplaceId.toNumber(),
    );
    console.log("marketplaceAccountFromPrecompile::", marketplaceAccountFromPrecompile);
    expect(marketplaceAccountFromPrecompile).to.equal(receiverAddress);
  });

  it("mint and auction some nft", async () => {
    const quantity = 499;
    const tx = await erc721Precompile.connect(bobSigner).mint(bobSigner.address, quantity);
    await tx.wait();
    expect(await erc721Precompile.totalSupply()).to.equal(quantity);
    const nftAddress = erc721PrecompileAddress;
    console.log("nftAddress::", nftAddress);
    const auctionNFTSeries = [4, 5, 6];
    const buyer = "0xB67e643F69400ad0cBb5514886fBe3439d94ba85";
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    console.log("paymentAsset::", paymentAsset);
    const fixedPrice = 1000000;
    const duration = 10000; //blocks
    const marketplaceId = 0;

    const auctionNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .auctionNft(erc721Precompile.address, auctionNFTSeries, paymentAsset, fixedPrice, duration, marketplaceId);
    const receipt = await auctionNftTx.wait();
    console.log("Auction receipt:::", JSON.stringify(receipt));
    const [collectionId, listingId, reservePrice, seller, serialNumbers] = (receipt?.events as any)[0].args;
    expect((receipt?.events as any)[0].event).to.equal("AuctionOpen");
    expect(collectionId.toNumber()).to.gte(0);
    expect(listingId.toNumber()).to.gte(0);
    expect(reservePrice.toNumber()).to.equal(fixedPrice);
    expect(seller).to.equal(bobSigner.address);
    const s = serialNumbers.map((s: BigNumber) => s.toNumber());
    console.log("s::", s);
    expect(JSON.stringify(s)).to.equal(JSON.stringify(auctionNFTSeries));
  });

  it("make simple offer", async () => {
    // const offerNFTSeries = [41,35,26];
    const offerSeries = 10;
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    console.log("paymentAsset::", paymentAsset);
    const amount = 1000000;
    const marketplaceId = 0;

    const offerNftTx = await marketPlacePrecompile
      .connect(alithSigner)
      .makeSimpleOffer(erc721Precompile.address, offerSeries, amount, paymentAsset, marketplaceId);
    const receipt = await offerNftTx.wait();
    console.log("Simple offer receipt:::", JSON.stringify(receipt));

    console.log("receipt event:::", JSON.stringify(receipt.events));
    const [offerId, buyer, collectionId, serialId] = (receipt?.events as any)[0].args;
    expect((receipt?.events as any)[0].event).to.equal("Offer");
    expect(offerId.toNumber()).to.gte(0);
    expect(collectionId.toNumber()).to.gte(0);
    expect(serialId.toNumber()).to.equal(offerSeries);
    expect(buyer).to.equal(alithSigner.address);
  });

  it("make bid", async () => {
    const listingId = 1;
    const amount = 1000000;

    const bidNftTx = await marketPlacePrecompile.connect(alithSigner).bid(listingId, amount);
    const receipt = await bidNftTx.wait();
    console.log("Bid receipt:::", JSON.stringify(receipt));

    console.log("receipt event:::", JSON.stringify(receipt.events));
    // const [collectionId, listingId, reservePrice, seller, serialNumbers] = (receipt?.events as any)[0].args;
    // expect((receipt?.events as any)[0].event).to.equal("AuctionOpen");
    // expect(collectionId.toNumber()).to.gte(0);
    // expect(listingId.toNumber()).to.gte(0);
    // expect(reservePrice.toNumber()).to.equal(fixedPrice);
    // expect(seller).to.equal(bobSigner.address);
  });

  it("cancel sale", async () => {
    const listingId = 0;
    const amount = 1000000;

    const cancelSaleTx = await marketPlacePrecompile.connect(bobSigner).cancelSale(listingId);
    const receipt = await cancelSaleTx.wait();
    console.log("cancelSale receipt:::", JSON.stringify(receipt));

    console.log("receipt event:::", JSON.stringify(receipt.events));
    // const [collectionId, listingId, reservePrice, seller, serialNumbers] = (receipt?.events as any)[0].args;
    // expect((receipt?.events as any)[0].event).to.equal("AuctionOpen");
    // expect(collectionId.toNumber()).to.gte(0);
    // expect(listingId.toNumber()).to.gte(0);
    // expect(reservePrice.toNumber()).to.equal(fixedPrice);
    // expect(seller).to.equal(bobSigner.address);
  });

  it("mint and sell some nft", async () => {
    const sellNFTSeries = [14, 15, 16];
    const buyer = "0xB67e643F69400ad0cBb5514886fBe3439d94ba85";
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    console.log("paymentAsset::", paymentAsset);
    const fixedPrice = 1000000;
    const duration = 1000; //blocks
    const marketplaceId = 0;

    const sellNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .sellNft(erc721Precompile.address, sellNFTSeries, buyer, paymentAsset, fixedPrice, duration, marketplaceId);
    const receipt = await sellNftTx.wait();
    const [seller, listingId, fixedPriceFromCall, serialNumbers, collectionAddress] = (receipt?.events as any)[0].args;
    expect((receipt?.events as any)[0].event).to.equal("FixedPriceSaleList");
    expect(collectionAddress).to.equal(erc721Precompile.address);
    expect(listingId.toNumber()).to.gte(0);
    expect(fixedPriceFromCall.toNumber()).to.equal(fixedPrice);
    expect(seller).to.equal(bobSigner.address);
    const s = serialNumbers.map((s: BigNumber) => s.toNumber());
    console.log("s::", s);
    expect(JSON.stringify(s)).to.equal(JSON.stringify(sellNFTSeries));
  });

  // it("mint - no limit on mint if maxSupply not set", async () => {
  //   const receiverAddress = await Wallet.createRandom().getAddress();
  //   const quantity = 499;
  //
  //   // new collection with unlimited mintable supply
  //   let tx = await nftPrecompile.connect(bobSigner).initializeCollection(
  //     bobSigner.address,
  //     ethers.utils.hexlify(ethers.utils.toUtf8Bytes(name)),
  //     BigNumber.from(0), // no max issuance
  //     ethers.utils.hexlify(ethers.utils.toUtf8Bytes("https://example.com/nft/metadata/")),
  //     [alithSigner.address],
  //     [1000],
  //   );
  //   let receipt = await tx.wait();
  //   const erc721PrecompileAddress = (receipt?.events as any)[0].args.precompileAddress;
  //   const newErc721Precompile = new Contract(erc721PrecompileAddress, ERC721_PRECOMPILE_ABI, bobSigner);
  //
  //   tx = await newErc721Precompile.connect(bobSigner).mint(receiverAddress, quantity);
  //   receipt = await tx.wait();
  //
  //   expect(await newErc721Precompile.balanceOf(receiverAddress)).to.equal(quantity);
  //
  //   expect(await newErc721Precompile.totalSupply()).to.equal(quantity);
  // });
  //
  // it("mint fails over max quantity per mint 1_000", async () => {
  //   const receiverAddress = await Wallet.createRandom().getAddress();
  //
  //   // new collection with unlimited mintable supply
  //   const tx = await nftPrecompile.connect(bobSigner).initializeCollection(
  //     bobSigner.address,
  //     ethers.utils.hexlify(ethers.utils.toUtf8Bytes(name)),
  //     BigNumber.from(0), // no max issuance
  //     ethers.utils.hexlify(ethers.utils.toUtf8Bytes("https://example.com/nft/metadata/")),
  //     [alithSigner.address],
  //     [1000],
  //   );
  //   const receipt = await tx.wait();
  //   const erc721PrecompileAddress = (receipt?.events as any)[0].args.precompileAddress;
  //   const newErc721Precompile = new Contract(erc721PrecompileAddress, ERC721_PRECOMPILE_ABI, bobSigner);
  //
  //   await newErc721Precompile
  //     .connect(bobSigner)
  //     .mint(receiverAddress, 1_001)
  //     .catch((err: any) => expect(err.message).contains("MintLimitExceeded"));
  // });

  // it("setMaxSupply", async () => {
  //   await erc721Precompile
  //     .connect(bobSigner)
  //     .setMaxSupply(100)
  //     .catch((err: any) => expect(err.message).contains("MaxIssuanceAlreadySet"));
  //
  //   // new collection with unlimited mintable supply
  //   let tx = await nftPrecompile.connect(bobSigner).initializeCollection(
  //     bobSigner.address,
  //     ethers.utils.hexlify(ethers.utils.toUtf8Bytes(name)),
  //     BigNumber.from(0), // no max issuance
  //     ethers.utils.hexlify(ethers.utils.toUtf8Bytes("https://example.com/nft/metadata/")),
  //     [alithSigner.address],
  //     [1000],
  //   );
  //   let receipt = await tx.wait();
  //   const erc721PrecompileAddress = (receipt?.events as any)[0].args.precompileAddress;
  //   const newErc721Precompile = new Contract(erc721PrecompileAddress, ERC721_PRECOMPILE_ABI, bobSigner);
  //
  //   // setMaxSupply fails for new collection if not owner call
  //   await newErc721Precompile
  //     .connect(alithSigner)
  //     .setMaxSupply(100)
  //     .catch((err: any) => expect(err.message).contains("NotCollectionOwner"));
  //
  //   // setMaxSupply succeeds for new collection if owner call
  //   tx = await newErc721Precompile.connect(bobSigner).setMaxSupply(100);
  //   receipt = await tx.wait();
  //
  //   // validate event
  //   expect((receipt?.events as any)[0].event).to.equal("MaxSupplyUpdated");
  //   expect((receipt?.events as any)[0].args.maxSupply).to.equal(BigNumber.from(100));
  //
  //   // setMaxSupply fails for new collection since supply was already set
  //   await newErc721Precompile
  //     .connect(bobSigner)
  //     .setMaxSupply(100)
  //     .catch((err: any) => expect(err.message).contains("MaxIssuanceAlreadySet"));
  // });
  //
  // it("setBaseURI", async () => {
  //   // validate setBaseURI can only be set by owner
  //   await erc721Precompile
  //     .connect(alithSigner)
  //     .setBaseURI(ethers.utils.hexlify(ethers.utils.toUtf8Bytes("")))
  //     .catch((err: any) => expect(err.message).contains("NotCollectionOwner"));
  //
  //   // validate cannot set invalid baseURI
  //   await erc721Precompile
  //     .connect(bobSigner)
  //     .setBaseURI(ethers.utils.hexlify(ethers.utils.toUtf8Bytes("abc")))
  //     .catch((err: any) => expect(err.message).contains("InvalidMetadataPath"));
  //
  //   // validate setBaseURI can only be set by owner
  //   const tx = await erc721Precompile
  //     .connect(bobSigner)
  //     .setBaseURI(ethers.utils.hexlify(ethers.utils.toUtf8Bytes("https://example.com/nft/metadata/")));
  //   const receipt = await tx.wait();
  //
  //   // validate event
  //   expect((receipt?.events as any)[0].event).to.equal("BaseURIUpdated");
  //   expect((receipt?.events as any)[0].args.baseURI).to.equal("https://example.com/nft/metadata/");
  //
  //   // validate tokenURI returns correct URI
  //   expect(await erc721Precompile.tokenURI(0)).to.equal("https://example.com/nft/metadata/0");
  // });
  //
  // it("transferFrom owner", async () => {
  //   const receiverAddress = await Wallet.createRandom().getAddress();
  //   const tokenId = 0;
  //
  //   // Transfer tokenId 0 to receiverAddress
  //   const tx = await erc721Precompile.connect(bobSigner).transferFrom(bobSigner.address, receiverAddress, tokenId);
  //   const receipt = await tx.wait();
  //
  //   expect(receipt).to.emit(erc721Precompile, "Transfer").withArgs(bobSigner.address, receiverAddress, tokenId);
  //
  //   // Receiver_address now owner of tokenId 1
  //   expect(await erc721Precompile.ownerOf(tokenId)).to.equal(receiverAddress);
  // });
  //
  // it("approve and transferFrom via transaction", async () => {
  //   const receiverAddress = await Wallet.createRandom().getAddress();
  //   const tokenId = 1;
  //
  //   // getApproved should be zero address
  //   expect(await erc721Precompile.getApproved(tokenId)).to.equal(constants.AddressZero);
  //
  //   // Bob approves alith for tokenId
  //   const approval = await erc721Precompile.approve(alithSigner.address, tokenId);
  //   expect(await approval.wait())
  //     .to.emit(erc721Precompile, "Approval")
  //     .withArgs(bobSigner.address, alithSigner.address, tokenId);
  //
  //   // getApproved should be alith
  //   expect(await erc721Precompile.getApproved(tokenId)).to.equal(alithSigner.address);
  //
  //   // alith transfers tokenId (Owned by Bob)
  //   const transfer = await erc721Precompile
  //     .connect(alithSigner)
  //     .transferFrom(bobSigner.address, receiverAddress, tokenId);
  //
  //   expect(await transfer.wait())
  //     .to.emit(erc721Precompile, "Transfer")
  //     .withArgs(bobSigner.address, receiverAddress, tokenId);
  //
  //   // Receiver_address now owner of tokenId
  //   expect(await erc721Precompile.ownerOf(tokenId)).to.equal(receiverAddress);
  // });

  // it("mint - over mintLimit fails", async () => {
  //   const receiverAddress = await Wallet.createRandom().getAddress();
  //   const quantity = 1001; // MintLimit set to 1000 so this should fail
  //
  //   const mint = await erc721Precompile.connect(alithSigner).mint(receiverAddress, quantity, { gasLimit: 50000 });
  //   await mint.wait().catch((err: any) => {
  //     expect(err.code).eq("CALL_EXCEPTION");
  //   });
  //
  //   // Verify balance of receiver is 0
  //   expect(await erc721Precompile.balanceOf(receiverAddress)).to.equal(0);
  // });
  //
  // it("setApprovalForAll, isApprovedForAll and safeTransferFrom", async () => {
  //   const receiverAddress = await Wallet.createRandom().getAddress();
  //   const tokenId = 2;
  //
  //   // Bob approves alith for tokenId
  //   const approval = await erc721Precompile.setApprovalForAll(alithSigner.address, true);
  //   let receipt = await approval.wait();
  //   expect((receipt?.events as any)[0].event).to.equal("ApprovalForAll");
  //   expect((receipt?.events as any)[0].args.owner).to.equal(bobSigner.address);
  //   expect((receipt?.events as any)[0].args.operator).to.equal(alithSigner.address);
  //   expect((receipt?.events as any)[0].args.approved).to.equal(true);
  //
  //   // isApprovedForAll should be true
  //   expect(await erc721Precompile.isApprovedForAll(bobSigner.address, alithSigner.address)).to.be.true;
  //
  //   // alith transfers tokenId (Owned by Bob)
  //   let transfer = await erc721Precompile
  //     .connect(alithSigner)
  //     .safeTransferFrom(bobSigner.address, receiverAddress, tokenId);
  //   receipt = await transfer.wait();
  //   expect((receipt?.events as any)[0].event).to.equal("Transfer");
  //   expect((receipt?.events as any)[0].args.from).to.equal(bobSigner.address);
  //   expect((receipt?.events as any)[0].args.to).to.equal(receiverAddress);
  //   expect((receipt?.events as any)[0].args.tokenId).to.equal(tokenId);
  //
  //   // Receiver_address now owner of tokenId
  //   expect(await erc721Precompile.ownerOf(tokenId)).to.equal(receiverAddress);
  //
  //   // Test sending to contracts implementing onErc721Received
  //   // Should Fail
  //   const factory = await ethers.getContractFactory("OnERC721ReceivedFails");
  //   const contractFails = await factory.connect(bobSigner).deploy();
  //   const tokenId_2 = 3;
  //   await erc721Precompile
  //     .connect(alithSigner)
  //     .estimateGas.safeTransferFrom(bobSigner.address, contractFails.address, tokenId_2, { gasLimit: 50000 })
  //     .catch((err) => expect(err.message).contains("ERC721: transfer to non ERC721Receiver implementer"));
  //
  //   // Should succeed
  //   const factory2 = await ethers.getContractFactory("OnERC721ReceivedSucceeds");
  //   const contractSucceeds = await factory2.connect(bobSigner).deploy();
  //   transfer = await erc721Precompile
  //     .connect(alithSigner)
  //     .safeTransferFrom(bobSigner.address, contractSucceeds.address, tokenId_2, { gasLimit: 50000 });
  //   receipt = await transfer.wait();
  //   expect((receipt?.events as any)[0].event).to.equal("Transfer");
  //   expect((receipt?.events as any)[0].args.from).to.equal(bobSigner.address);
  //   expect((receipt?.events as any)[0].args.to).to.equal(contractSucceeds.address);
  //   expect((receipt?.events as any)[0].args.tokenId).to.equal(tokenId_2);
  //   expect(await erc721Precompile.ownerOf(tokenId_2)).to.equal(contractSucceeds.address);
  // });
  //
  // it("name, symbol, ownerOf, tokenURI via EVM", async () => {
  //   const tokenId = 4;
  //   // Check state proxy calls
  //   expect(await precompileCaller.ownerOfProxy(tokenId)).to.equal(bobSigner.address);
  //   expect(await precompileCaller.nameProxy()).to.equal(name);
  //   expect(await precompileCaller.symbolProxy()).to.equal(name);
  //   expect(await precompileCaller.tokenURIProxy(tokenId)).to.equal(`https://example.com/nft/metadata/${tokenId}`);
  // });
  //
  // it("approve and transferFrom via EVM", async () => {
  //   const receiverAddress = await Wallet.createRandom().getAddress();
  //   const tokenId = 5;
  //
  //   // Bob approves contract for tokenId
  //   const approval = await erc721Precompile.connect(bobSigner).approve(precompileCaller.address, tokenId);
  //   expect(await approval.wait())
  //     .to.emit(erc721Precompile, "Approval")
  //     .withArgs(bobSigner.address, precompileCaller.address, tokenId);
  //   // Approved should be correct
  //   expect(await erc721Precompile.getApproved(tokenId)).to.equal(precompileCaller.address);
  //
  //   // Transfer tokenId to receiverAddress
  //   const transfer = await precompileCaller
  //     .connect(bobSigner)
  //     .transferFromProxy(bobSigner.address, receiverAddress, tokenId, {
  //       gasLimit: 50000,
  //     });
  //   expect(await transfer.wait())
  //     .to.emit(erc721Precompile, "Transfer")
  //     .withArgs(bobSigner.address, receiverAddress, tokenId);
  //
  //   // contract_address now owner of tokenId
  //   expect(await precompileCaller.balanceOfProxy(receiverAddress)).to.equal(1);
  //   expect(await precompileCaller.ownerOfProxy(tokenId)).to.equal(receiverAddress);
  // });

  // it("approve and setApprovalForAll via EVM", async () => {
  //   const receiverAddress = await Wallet.createRandom().getAddress();
  //   let tokenId = 6;
  //
  //   // Transfer NFT to contract so it can pass the approval
  //   let transfer = await erc721Precompile
  //     .connect(bobSigner)
  //     .transferFrom(bobSigner.address, precompileCaller.address, tokenId);
  //   await transfer.wait();
  //   // Check transfer worked, events asserted in previous tests
  //   expect(await precompileCaller.ownerOfProxy(tokenId)).to.equal(precompileCaller.address);
  //
  //   // Approve receiverAddress
  //   const approve = await precompileCaller
  //     .connect(bobSigner)
  //     .approveProxy(alithSigner.address, tokenId, { gasLimit: 50000 });
  //   await approve.wait();
  //
  //   // Check approval through proxy
  //   expect(await precompileCaller.getApprovedProxy(tokenId)).to.equal(alithSigner.address);
  //
  //   // alith should now be able to transfer as she was approved with the above call
  //   transfer = await erc721Precompile
  //     .connect(alithSigner)
  //     .transferFrom(precompileCaller.address, receiverAddress, tokenId);
  //   await transfer.wait();
  //   // Check transfer worked
  //   expect(await precompileCaller.ownerOfProxy(tokenId)).to.equal(receiverAddress);
  //
  //   tokenId = 7;
  //   // Transfer NFT to contract so it can pass the approval and transfer
  //   transfer = await erc721Precompile
  //     .connect(bobSigner)
  //     .transferFrom(bobSigner.address, precompileCaller.address, tokenId);
  //   await transfer.wait();
  //   // Check transfer worked
  //   expect(await precompileCaller.ownerOfProxy(tokenId)).to.equal(precompileCaller.address);
  //
  //   // Approval before should be false
  //   expect(await precompileCaller.isApprovedForAllProxy(precompileCaller.address, alithSigner.address)).to.equal(false);
  //
  //   const approvalForAll = await precompileCaller
  //     .connect(bobSigner)
  //     .setApprovalForAllProxy(alithSigner.address, true, { gasLimit: 50000 });
  //   await approvalForAll.wait();
  //
  //   // Check approval through proxy
  //   expect(await precompileCaller.isApprovedForAllProxy(precompileCaller.address, alithSigner.address)).to.equal(true);
  //
  //   // alith should now be able to transfer as she was approved with the above call
  //   transfer = await erc721Precompile
  //     .connect(alithSigner)
  //     .transferFrom(precompileCaller.address, receiverAddress, tokenId);
  //   await transfer.wait();
  //   // Check transfer worked
  //   expect(await precompileCaller.ownerOfProxy(tokenId)).to.equal(receiverAddress);
  // });
  //
  // it("approve and safeTransferFrom via EVM", async () => {
  //   const receiverAddress = await Wallet.createRandom().getAddress();
  //   const tokenId = 8;
  //
  //   // Bob approves contract for tokenId
  //   const approval = await erc721Precompile.connect(bobSigner).approve(precompileCaller.address, tokenId);
  //   await approval.wait();
  //   // Approved should be correct
  //   expect(await erc721Precompile.getApproved(tokenId)).to.equal(precompileCaller.address);
  //
  //   // Transfer tokenId to receiverAddress
  //   const transfer = await precompileCaller
  //     .connect(bobSigner)
  //     .safeTransferFromProxy(bobSigner.address, receiverAddress, tokenId, { gasLimit: 50000 });
  //   expect(await transfer.wait())
  //     .to.emit(erc721Precompile, "Transfer")
  //     .withArgs(bobSigner.address, receiverAddress, tokenId);
  //
  //   // receiver address now owner of tokenId
  //   expect(await precompileCaller.balanceOfProxy(receiverAddress)).to.equal(1);
  //   expect(await precompileCaller.ownerOfProxy(tokenId)).to.equal(receiverAddress);
  // });
  //
  // it("owner, renounceOwnership, transferOwnership", async () => {
  //   // Check ownership is bob
  //   expect(await erc721Precompile.owner()).to.equal(bobSigner.address);
  //
  //   // Transfer ownership
  //   const transferOwnership = await erc721Precompile.connect(bobSigner).transferOwnership(alithSigner.address);
  //   let receipt = await transferOwnership.wait();
  //   expect((receipt?.events as any)[0].event).to.equal("OwnershipTransferred");
  //   expect((receipt?.events as any)[0].args.previousOwner).to.equal(bobSigner.address);
  //   expect((receipt?.events as any)[0].args.newOwner).to.equal(alithSigner.address);
  //
  //   // Check ownership is now alith
  //   expect(await erc721Precompile.owner()).to.equal(alithSigner.address);
  //
  //   // Renounce ownership
  //   const renounceOwnership = await erc721Precompile.connect(alithSigner).renounceOwnership();
  //   receipt = await renounceOwnership.wait();
  //   expect((receipt?.events as any)[0].event).to.equal("OwnershipTransferred");
  //   expect((receipt?.events as any)[0].args.previousOwner).to.equal(alithSigner.address);
  //   expect((receipt?.events as any)[0].args.newOwner).to.equal(constants.AddressZero);
  //
  //   // Check ownership is now zero address
  //   expect(await erc721Precompile.owner()).to.equal(constants.AddressZero);
  // });
});
