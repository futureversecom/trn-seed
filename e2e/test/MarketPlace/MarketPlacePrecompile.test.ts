import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { hexToString, hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { BigNumber, Contract, Wallet } from "ethers";
import Web3 from "web3";
import web3 from "web3";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC721_PRECOMPILE_ABI,
  MARKETPLACE_PRECOMPILE_ADDRESS,
  MARKETPLACE_ABI,
  NodeProcess,
  startNode,
  typedefs,
} from "../../common";

// NFT Collection information
const name = "test-collection";
const metadataPath = "https://example.com/nft/metadata/";
const initialIssuance = 0;
const maxIssuance = 1000;

describe("Marketplace Precompile", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let bobSigner: Wallet;
  let alithSigner: Wallet;
  let erc721Precompile: Contract;
  let marketPlacePrecompile: Contract;
  let erc721PrecompileAddress: string;

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

    const quantity = 499;
    const tx = await erc721Precompile.connect(bobSigner).mint(bobSigner.address, quantity);
    await tx.wait();
    expect(await erc721Precompile.totalSupply()).to.equal(quantity);
    // Deploy marketplace contract
    marketPlacePrecompile = new Contract(MARKETPLACE_PRECOMPILE_ADDRESS, MARKETPLACE_ABI, bobSigner);
  });

  after(async () => await node.stop());

  /// Happy Flow

  it("register marketplace", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const entitlements = 1000;
    const tx = await marketPlacePrecompile.connect(bobSigner).registerMarketplace(receiverAddress, entitlements);
    const receipt = await tx.wait();
    const [creator, marketplaceId, marketplaceAccount] = (receipt?.events as any)[0].args;
    expect((receipt?.events as any)[0].event).to.equal("MarketplaceRegister");
    expect(creator).to.equal(bobSigner.address);
    expect(marketplaceId.toNumber()).to.gte(0);
    expect(marketplaceAccount).to.equal(receiverAddress);

    // verify marketplace account when id is given
    const marketplaceAccountFromPrecompile = await marketPlacePrecompile.getMarketplaceAccount(
      marketplaceId.toNumber(),
    );
    expect(marketplaceAccountFromPrecompile).to.equal(receiverAddress);
  });

  it("sell nft with marketplace", async () => {
    const sellNFTSeries = [14, 15, 16];
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const fixedPrice = 1000000;
    const duration = 1000; //blocks
    const marketplaceId = 1;

    const sellNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .sellNft(
        erc721Precompile.address,
        sellNFTSeries,
        alithSigner.address,
        paymentAsset,
        fixedPrice,
        duration,
        marketplaceId,
      );
    const receipt = await sellNftTx.wait();
    const [seller, listingId, fixedPriceFromCall, serialNumbers, collectionAddress, marketplaceIdArgs] = (
      receipt?.events as any
    )[0].args;
    expect((receipt?.events as any)[0].event).to.equal("FixedPriceSaleListNFT");
    expect(collectionAddress).to.equal(erc721Precompile.address);
    expect(listingId.toNumber()).to.gte(0);
    expect(fixedPriceFromCall.toNumber()).to.equal(fixedPrice);
    expect(seller).to.equal(bobSigner.address);
    const s = serialNumbers.map((s: BigNumber) => s.toNumber());
    expect(JSON.stringify(s)).to.equal(JSON.stringify(sellNFTSeries));
    expect(marketplaceIdArgs.toNumber()).to.gte(0);
  });

  it("auction nft with marketplace", async () => {
    const auctionNFTSeries = [4, 5, 6];
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const reservePrice = 1000000;
    const duration = 10000; //blocks
    const marketplaceId = 1;

    const auctionNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .auctionNft(erc721Precompile.address, auctionNFTSeries, paymentAsset, reservePrice, duration, marketplaceId);
    const receipt = await auctionNftTx.wait();
    const [collectionId, listingId, reservePriceFromChain, seller, serialNumbers] = (receipt?.events as any)[0].args;
    expect((receipt?.events as any)[0].event).to.equal("AuctionOpenNFT");
    expect(collectionId.toNumber()).to.gte(0);
    expect(listingId.toNumber()).to.gte(0);
    expect(reservePriceFromChain.toNumber()).to.equal(reservePrice);
    expect(seller).to.equal(bobSigner.address);
    const s = serialNumbers.map((s: BigNumber) => s.toNumber());
    expect(JSON.stringify(s)).to.equal(JSON.stringify(auctionNFTSeries));
  });

  it("make simple offer", async () => {
    const offerSeries = 10;
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const amount = 1000000;
    const marketplaceId = 1;

    const offerNftTx = await marketPlacePrecompile
      .connect(alithSigner)
      .makeSimpleOffer(erc721Precompile.address, offerSeries, amount, paymentAsset, marketplaceId);
    const receipt = await offerNftTx.wait();
    const [offerId, buyer, collectionId, serialId, marketplaceIdArgs] = (receipt?.events as any)[0].args;
    expect((receipt?.events as any)[0].event).to.equal("Offer");
    expect(offerId.toNumber()).to.gte(0);
    expect(collectionId.toNumber()).to.gte(0);
    expect(serialId.toNumber()).to.equal(offerSeries);
    expect(buyer).to.equal(alithSigner.address);
    expect(marketplaceIdArgs.toNumber()).to.gte(0);
  });

  it("buy", async () => {
    const sellNFTSeries = [104, 105, 106];
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const fixedPrice = 1000000;
    const duration = 10000; //blocks
    const marketplaceId = 1;

    const sellNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .sellNft(
        erc721Precompile.address,
        sellNFTSeries,
        alithSigner.address,
        paymentAsset,
        fixedPrice,
        duration,
        marketplaceId,
      );
    const buyReceipt = await sellNftTx.wait();
    const [, listingId, , ,] = (buyReceipt?.events as any)[0].args;

    const buyNftTx = await marketPlacePrecompile.connect(alithSigner).buy(listingId);
    const receipt = await buyNftTx.wait();

    const [collectionId, listingIdFromChain, fixedPriceFromChain, seller, serialNumbers] = (receipt?.events as any)[0]
      .args;
    expect((receipt?.events as any)[0].event).to.equal("FixedPriceSaleComplete");
    expect(collectionId.toNumber()).to.gte(0);
    expect(listingIdFromChain.toNumber()).to.equal(listingId);
    expect(fixedPriceFromChain.toNumber()).to.equal(fixedPrice);
    expect(seller).to.equal(bobSigner.address);
    const s = serialNumbers.map((s: BigNumber) => s.toNumber());
    expect(JSON.stringify(s)).to.equal(JSON.stringify(sellNFTSeries));
  });

  it("make bid", async () => {
    const amount = 10000000;

    const auctionNFTSeries = [27, 28];
    const paymentAsset: string | number = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const reservePrice = 1000000;
    const duration = 10000; //blocks
    const marketplaceId = 1;

    // precompile
    const auctionTx = await marketPlacePrecompile
      .connect(bobSigner)
      .auctionNft(erc721Precompile.address, auctionNFTSeries, paymentAsset, reservePrice, duration, marketplaceId);
    const auctionReceipt = await auctionTx.wait();
    const [, listingIdForAuction, , ,] = (auctionReceipt?.events as any)[0].args;

    const listingId = listingIdForAuction.toNumber();

    const bidNftTx = await marketPlacePrecompile.connect(alithSigner).bid(listingId, amount);
    const receipt = await bidNftTx.wait();
    const [bidder, listingIdFromChain, amountFromChain] = (receipt?.events as any)[0].args;
    expect((receipt?.events as any)[0].event).to.equal("Bid");
    expect(listingIdFromChain.toNumber()).to.equal(listingId);
    expect(amountFromChain.toNumber()).to.equal(amount);
    expect(bidder).to.equal(alithSigner.address);
  });

  it("cancel sale", async () => {
    const sellNFTSeries = [20, 21];
    const buyer = "0xB67e643F69400ad0cBb5514886fBe3439d94ba85";
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const fixedPrice = 1000000;
    const duration = 1000; //blocks
    const marketplaceId = 1;

    const sellNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .sellNft(erc721Precompile.address, sellNFTSeries, buyer, paymentAsset, fixedPrice, duration, marketplaceId);
    const sellReceipt = await sellNftTx.wait();
    const [, listingId, , ,] = (sellReceipt?.events as any)[0].args;

    const cancelSaleTx = await marketPlacePrecompile.connect(bobSigner).cancelSale(listingId);
    const receipt = await cancelSaleTx.wait();

    const [collectionId, listingIdCanceled, caller, seriesIds, marketplaceIdArgs] = (receipt?.events as any)[0].args;
    expect((receipt?.events as any)[0].event).to.equal("FixedPriceSaleClose");

    expect(collectionId.toNumber()).to.gte(0);
    expect(listingIdCanceled.toNumber()).to.gte(0);
    expect(caller).to.equal(bobSigner.address);
    const s = seriesIds.map((s: BigNumber) => s.toNumber());
    expect(JSON.stringify(s)).to.equal(JSON.stringify(sellNFTSeries));
    expect(marketplaceIdArgs.toNumber()).to.gte(0);

    const auctionNFTSeries = [24, 25, 26];
    const reservePrice = 70000;

    const auctionNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .auctionNft(erc721Precompile.address, auctionNFTSeries, paymentAsset, reservePrice, duration, marketplaceId);
    const auctionReceipt = await auctionNftTx.wait();
    const [, listingIdForAuction, , ,] = (auctionReceipt?.events as any)[0].args;

    const cancelAuctionSaleTx = await marketPlacePrecompile.connect(bobSigner).cancelSale(listingIdForAuction);
    const auctionCancelReceipt = await cancelAuctionSaleTx.wait();

    const [collectionId1, listingIdCanceled1, caller1, seriesIds1, marketplaceIdArgs1] = (
      auctionCancelReceipt?.events as any
    )[0].args;
    expect((receipt?.events as any)[0].event).to.equal("FixedPriceSaleClose");

    expect(collectionId1.toNumber()).to.gte(0);
    expect(listingIdCanceled1.toNumber()).to.gte(0);
    expect(caller1).to.equal(bobSigner.address);
    const s1 = seriesIds1.map((s: BigNumber) => s.toNumber());
    expect(JSON.stringify(s1)).to.equal(JSON.stringify(auctionNFTSeries));
    expect(marketplaceIdArgs1.toNumber()).to.gte(0);
  });

  it("update fixed price", async () => {
    const sellNFTSeries = [200, 201];
    const buyer = "0xB67e643F69400ad0cBb5514886fBe3439d94ba85";
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const fixedPrice = 8000;
    const duration = 1000; //blocks
    const marketplaceId = 1;

    const sellNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .sellNft(erc721Precompile.address, sellNFTSeries, buyer, paymentAsset, fixedPrice, duration, marketplaceId);
    const sellReceipt = await sellNftTx.wait();
    const [, listingId, , ,] = (sellReceipt?.events as any)[0].args;

    const updatedPrice = 98000;
    const updatedPriceTx = await marketPlacePrecompile.connect(bobSigner).updateFixedPrice(listingId, updatedPrice);
    const updatedPriceTxReceipt = await updatedPriceTx.wait();
    expect((updatedPriceTxReceipt?.events as any)[0].event).to.equal("FixedPriceSaleUpdate");
    const [collectionId, listingId1, newPrice, caller, seriesIds, marketplaceIdArgs] = (
      updatedPriceTxReceipt?.events as any
    )[0].args;
    expect(collectionId.toNumber()).to.gte(0);
    expect(listingId1.toNumber()).to.gte(0);
    expect(newPrice.toNumber()).to.equal(updatedPrice);
    expect(caller).to.equal(bobSigner.address);
    const s = seriesIds.map((s: BigNumber) => s.toNumber());
    expect(JSON.stringify(s)).to.equal(JSON.stringify(sellNFTSeries));
    expect(marketplaceIdArgs.toNumber()).to.gte(0);
  });

  it("make simple offer and owner accepts it ", async () => {
    const offerSeries = 100;
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const amount = 100;
    const marketplaceId = 1;

    const offerNftTx = await marketPlacePrecompile
      .connect(alithSigner)
      .makeSimpleOffer(erc721Precompile.address, offerSeries, amount, paymentAsset, marketplaceId);
    const receipt = await offerNftTx.wait();
    const [offerId, , , , marketplaceIdArgs] = (receipt?.events as any)[0].args;
    expect((receipt?.events as any)[0].event).to.equal("Offer");
    expect(offerId.toNumber()).to.gte(0);
    expect(marketplaceIdArgs.toNumber()).to.gte(0);

    const acceptOfferTx = await marketPlacePrecompile.connect(bobSigner).acceptOffer(offerId);
    const acceptOfferReceipt = await acceptOfferTx.wait();
    const [offerId1, amount1, sender, collectionId, seriesId, marketplaceIdArgs1] = (
      acceptOfferReceipt?.events as any
    )[0].args;
    expect((acceptOfferReceipt?.events as any)[0].event).to.equal("OfferAccept");
    expect(offerId1.toNumber()).to.equal(offerId.toNumber());
    expect(amount1.toNumber()).to.equal(amount);
    expect(sender).to.equal(bobSigner.address);
    expect(seriesId.toNumber()).to.equal(offerSeries);
    expect(collectionId.toNumber()).to.gte(0);
    expect(marketplaceIdArgs1.toNumber()).to.gte(0);
  });

  it("make simple offer and cancel it ", async () => {
    const offerSeries = 101;
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const amount = 100;
    const marketplaceId = 1;

    const offerNftTx = await marketPlacePrecompile
      .connect(alithSigner)
      .makeSimpleOffer(erc721Precompile.address, offerSeries, amount, paymentAsset, marketplaceId);
    const receipt = await offerNftTx.wait();
    const [offerId, , , , marketplaceIdArgs] = (receipt?.events as any)[0].args;
    expect((receipt?.events as any)[0].event).to.equal("Offer");
    expect(offerId.toNumber()).to.gte(0);
    expect(marketplaceIdArgs.toNumber()).to.gte(0);

    const cancelOfferTx = await marketPlacePrecompile.connect(alithSigner).cancelOffer(offerId);
    const cancelOfferReceipt = await cancelOfferTx.wait();

    const [offerId1, caller, collectionId, seriesId, marketplaceIdArgs1] = (cancelOfferReceipt?.events as any)[0].args;
    expect((cancelOfferReceipt?.events as any)[0].event).to.equal("OfferCancel");
    expect(offerId1.toNumber()).to.equal(offerId.toNumber());
    expect(collectionId.toNumber()).to.gte(0);
    expect(seriesId.toNumber()).to.equal(offerSeries);
    expect(caller).to.equal(alithSigner.address);
    expect(marketplaceIdArgs1.toNumber()).to.gte(0);
  });

  it("get offer from offer id", async () => {
    const offerSeries = 101;
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const amount = 100;
    const marketplaceId = 1;

    const offerNftTx = await marketPlacePrecompile
      .connect(alithSigner)
      .makeSimpleOffer(erc721Precompile.address, offerSeries, amount, paymentAsset, marketplaceId);
    const receipt = await offerNftTx.wait();
    const [offerId, , , , marketplaceIdArgs] = (receipt?.events as any)[0].args;
    expect((receipt?.events as any)[0].event).to.equal("Offer");
    expect(offerId.toNumber()).to.gte(0);
    expect(marketplaceIdArgs.toNumber()).to.gte(0);

    const offerDetails = await marketPlacePrecompile.getOfferFromId(offerId.toNumber());
    const [collectionId, seriesId, amountFromCall, caller] = offerDetails;
    expect(amountFromCall.toNumber()).to.equal(amount);
    expect(collectionId).to.gte(0);
    expect(seriesId).to.equal(offerSeries);
    expect(caller).to.equal(alithSigner.address);
  });

  it("get listing from listing id", async () => {
    const sellNFTSeries = [208, 210];
    const buyer = "0xB67e643F69400ad0cBb5514886fBe3439d94ba85";
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const fixedPrice = 800;
    const duration = 1000; //blocks
    const marketplaceId = 1;

    const sellNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .sellNft(erc721Precompile.address, sellNFTSeries, buyer, paymentAsset, fixedPrice, duration, marketplaceId);
    const sellReceipt = await sellNftTx.wait();
    const [, listingId, , ,] = (sellReceipt?.events as any)[0].args;

    const listingDetails = await marketPlacePrecompile.getListingFromId(listingId.toNumber());
    const [typeOfListing, collectionId, seriesId, amountFromCall] = listingDetails;
    expect(amountFromCall.toNumber()).to.equal(fixedPrice);
    expect(collectionId).to.gte(0);
    expect(JSON.stringify(seriesId)).to.equal(JSON.stringify(sellNFTSeries));
    expect(hexToString(typeOfListing)).to.equal("fixed_price_listing_for_nft");
  });

  /// SAD flow

  it("sell nft fails", async () => {
    const sellNFTSeries: [] = [];
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const fixedPrice = 1000000;
    const duration = 1000; //blocks
    const marketplaceId = 1;

    await marketPlacePrecompile
      .connect(bobSigner)
      .sellNft(
        erc721Precompile.address,
        sellNFTSeries,
        alithSigner.address,
        paymentAsset,
        fixedPrice,
        duration,
        marketplaceId,
      )
      .catch((err: any) => expect(err.message).contains("EmptyTokens"));
  });

  it("update fixed price listing fails of non existing listing id", async () => {
    const listingId = 500;
    const updatedPrice = 9772;
    await marketPlacePrecompile
      .connect(bobSigner)
      .updateFixedPrice(listingId, updatedPrice)
      .catch((err: any) => {
        expect(err.message).contains("revert Not fixed price");
      });
  });

  it("update fixed price listing fails by an other account", async () => {
    const sellNFTSeries = [307, 309];
    const buyer = "0xB67e643F69400ad0cBb5514886fBe3439d94ba85";
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const fixedPrice = 800;
    const duration = 1000; //blocks
    const marketplaceId = 1;

    const sellNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .sellNft(erc721Precompile.address, sellNFTSeries, buyer, paymentAsset, fixedPrice, duration, marketplaceId);
    const sellReceipt = await sellNftTx.wait();
    const [, listingId, , ,] = (sellReceipt?.events as any)[0].args;

    const updatedPrice = 9772;
    await marketPlacePrecompile
      .connect(alithSigner)
      .updateFixedPrice(listingId, updatedPrice)
      .catch((err: any) => {
        expect(err.message).contains("NotSeller");
      });
  });

  it("bid listing fails for fixed price listing", async () => {
    const sellNFTSeries = [317, 319];
    const buyer = "0xB67e643F69400ad0cBb5514886fBe3439d94ba85";
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const fixedPrice = 800;
    const duration = 1000; //blocks
    const marketplaceId = 1;

    const sellNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .sellNft(erc721Precompile.address, sellNFTSeries, buyer, paymentAsset, fixedPrice, duration, marketplaceId);
    const sellReceipt = await sellNftTx.wait();
    const [, listingId, , ,] = (sellReceipt?.events as any)[0].args;

    const amount = 9772;
    await marketPlacePrecompile
      .connect(alithSigner)
      .bid(listingId, amount)
      .catch((err: any) => {
        expect(err.message).contains("NotForAuction");
      });
  });

  it("bid listing with lower price", async () => {
    const auctionNFTSeries = [141, 142];
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const reservePrice = 1000000;
    const duration = 10000; //blocks
    const marketplaceId = 1;

    const auctionNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .auctionNft(erc721Precompile.address, auctionNFTSeries, paymentAsset, reservePrice, duration, marketplaceId);
    const receipt = await auctionNftTx.wait();
    const [, listingId, , ,] = (receipt?.events as any)[0].args;

    const amount = 9772;
    await marketPlacePrecompile
      .connect(alithSigner)
      .bid(listingId, amount)
      .catch((err: any) => {
        expect(err.message).contains("BidTooLow");
      });
  });

  it("make simple offer fails", async () => {
    const offerSeries = 87;
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const amount = 0;
    const marketplaceId = 1;

    await marketPlacePrecompile
      .connect(alithSigner)
      .makeSimpleOffer(erc721Precompile.address, offerSeries, amount, paymentAsset, marketplaceId)
      .catch((err: any) => {
        expect(err.message).contains("ZeroOffer");
      });
  });

  it("make simple offer fails if initiated by owner", async () => {
    const offerSeries = 77;
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const amount = 100;
    const marketplaceId = 1;

    await marketPlacePrecompile
      .connect(bobSigner)
      .makeSimpleOffer(erc721Precompile.address, offerSeries, amount, paymentAsset, marketplaceId)
      .catch((err: any) => {
        expect(err.message).contains("IsTokenOwner");
      });
  });

  it("cancel offer fails for invalid offer", async () => {
    const offerId = 77;

    await marketPlacePrecompile
      .connect(bobSigner)
      .acceptOffer(offerId)
      .catch((err: any) => {
        expect(err.message).contains("Offer details not found");
      });
  });
});
