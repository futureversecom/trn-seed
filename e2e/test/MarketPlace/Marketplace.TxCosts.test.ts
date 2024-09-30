import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
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
  TxCosts,
  getScaledGasForExtrinsicFee,
  saveTxFees,
  saveTxGas,
  startNode,
  typedefs,
} from "../../common";

// NFT Collection information
const name = "test-collection";
const metadataPath = "https://example.com/nft/metadata/";
const initialIssuance = 0;
const maxIssuance = 1000;

describe("Marketplace Precompile Gas Estimates", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let bobSigner: Wallet;
  let alithSigner: Wallet;
  let erc721Precompile: Contract;
  let marketPlacePrecompile: Contract;
  let erc721PrecompileAddress: string;
  let collectionId: string;
  let alithKeyring: KeyringPair;
  let bobKeyring: KeyringPair;
  let provider: JsonRpcProvider;

  const allTxGasCosts: { [key: string]: TxCosts } = {};
  const allTxFeeCosts: { [key: string]: TxCosts } = {};

  // Setup api instance
  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);

    // Setup Root api instance and keyring
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    const keyring = new Keyring({ type: "ethereum" });
    bobKeyring = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));
    alithKeyring = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    provider = new JsonRpcProvider(`http://127.0.0.1:${node.rpcPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);

    // Create NFT collection using runtime, bob is collection owner
    await new Promise<void>((resolve, reject) => {
      api.tx.nft
        .createCollection(name, initialIssuance, maxIssuance, null, metadataPath, null, { xrpl: false })
        .signAndSend(bobKeyring, async ({ status, events }) => {
          if (status.isInBlock) {
            events.forEach(({ event: { data, method } }) => {
              if (method == "CollectionCreate") {
                const collection_uuid = (data.toJSON() as any)[0];
                collectionId = collection_uuid;

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
    // Register 0th marketplace id which would be used for other tests
    const entitlements = 1000;
    const marketplaceRegisterTx = await marketPlacePrecompile
      .connect(bobSigner)
      .registerMarketplace(bobSigner.address, entitlements);
    await marketplaceRegisterTx.wait();
  });

  after(async () => {
    await node.stop();
    saveTxGas(allTxGasCosts, "MarketPlace/TxCosts.md", "Marketplace Precompiles");
    saveTxFees(allTxFeeCosts, "MarketPlace/TxCosts.md", "Marketplace Precompiles");
  });

  it("register marketplace tx costs", async () => {
    // precompile
    const entitlements = 1000;
    const precompileGasCost = await marketPlacePrecompile.estimateGas.registerMarketplace(
      bobSigner.address,
      entitlements,
    );
    let balanceBefore = await bobSigner.getBalance();
    const tx = await marketPlacePrecompile.connect(bobSigner).registerMarketplace(bobSigner.address, entitlements);
    await tx.wait();
    let balanceAfter = await bobSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);

    // extrinsic
    const owner2 = Wallet.createRandom().connect(provider);
    balanceBefore = await bobSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.marketplace.registerMarketplace(owner2.address, entitlements).signAndSend(bobKeyring, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    balanceAfter = await bobSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
    expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

    // Update all costs
    allTxGasCosts["registerMarketplace"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileGasCost, // convert to XRP Drops(6)
      Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
    };
    allTxFeeCosts["registerMarketplace"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("sell nft", async () => {
    let sellNFTSeries = [14, 15, 16];
    let paymentAsset: string | number = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const fixedPrice = 1000000;
    const duration = 1000; //blocks
    const marketplaceId = 1;

    // precompile
    const precompileGasCost = await marketPlacePrecompile.estimateGas.sellNft(
      erc721Precompile.address,
      sellNFTSeries,
      alithSigner.address,
      paymentAsset,
      fixedPrice,
      duration,
      marketplaceId,
    );
    let balanceBefore = await bobSigner.getBalance();
    const tx = await marketPlacePrecompile
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
    await tx.wait();
    let balanceAfter = await bobSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);

    // extrinsic
    sellNFTSeries = [17, 18, 19];
    paymentAsset = 2;
    balanceBefore = await bobSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.marketplace
        .sellNft(collectionId, sellNFTSeries, alithSigner.address, paymentAsset, fixedPrice, duration, marketplaceId)
        .signAndSend(bobKeyring, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    balanceAfter = await bobSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
    expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

    // Update all costs
    allTxGasCosts["sellNft"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileGasCost, // convert to XRP Drops(6)
      Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
    };
    allTxFeeCosts["sellNft"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("auction nft", async () => {
    let auctionNFTSeries = [4, 5, 6];
    let paymentAsset: string | number = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const reservePrice = 1000000;
    const duration = 10000; //blocks
    const marketplaceId = 1;

    // precompile
    const precompileGasCost = await marketPlacePrecompile.estimateGas.auctionNft(
      erc721Precompile.address,
      auctionNFTSeries,
      paymentAsset,
      reservePrice,
      duration,
      marketplaceId,
    );
    let balanceBefore = await bobSigner.getBalance();
    const tx = await marketPlacePrecompile
      .connect(bobSigner)
      .auctionNft(erc721Precompile.address, auctionNFTSeries, paymentAsset, reservePrice, duration, marketplaceId);
    await tx.wait();
    let balanceAfter = await bobSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);

    // extrinsic
    auctionNFTSeries = [7, 8, 9];
    paymentAsset = 2;
    balanceBefore = await bobSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.marketplace
        .auctionNft(collectionId, auctionNFTSeries, paymentAsset, reservePrice, duration, marketplaceId)
        .signAndSend(bobKeyring, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    balanceAfter = await bobSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
    expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

    // Update all costs
    allTxGasCosts["auctionNft"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileGasCost, // convert to XRP Drops(6)
      Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
    };
    allTxFeeCosts["auctionNft"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("make simple offer", async () => {
    let offerSeries = 105;
    let paymentAsset: string | number = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const amount = 100000;
    const marketplaceId = 1;

    // precompile
    const precompileGasCost = await marketPlacePrecompile
      .connect(alithSigner)
      .estimateGas.makeSimpleOffer(erc721Precompile.address, offerSeries, amount, paymentAsset, marketplaceId);
    let balanceBefore = await alithSigner.getBalance();
    const tx = await marketPlacePrecompile
      .connect(alithSigner)
      .makeSimpleOffer(erc721Precompile.address, offerSeries, amount, paymentAsset, marketplaceId);
    await tx.wait();
    let balanceAfter = await alithSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);

    // extrinsic
    offerSeries = 200;
    paymentAsset = 2;
    balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.marketplace
        .makeSimpleOffer([collectionId, offerSeries], amount, paymentAsset, marketplaceId)
        .signAndSend(alithKeyring, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    balanceAfter = await alithSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
    expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

    // Update all costs
    allTxGasCosts["makeSimpleOffer"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileGasCost, // convert to XRP Drops(6)
      Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
    };
    allTxFeeCosts["makeSimpleOffer"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("buy", async () => {
    const sellNFTSeries = [104, 105, 106];
    const paymentAsset: string | number = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const fixedPrice = 1000000;
    const duration = 10000; //blocks
    const marketplaceId = 1;

    let sellNftTx = await marketPlacePrecompile
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
    let buyReceipt = await sellNftTx.wait();
    let [, listingId, , ,] = (buyReceipt?.events as any)[0].args;

    // precompile
    const precompileGasCost = await marketPlacePrecompile.connect(alithSigner).estimateGas.buy(listingId.toNumber());
    let balanceBefore = await alithSigner.getBalance();

    const tx = await marketPlacePrecompile.connect(alithSigner).buy(listingId.toNumber());
    await tx.wait();
    let balanceAfter = await alithSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);

    const sellNFTSeries2 = [107, 108, 109];

    sellNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .sellNft(
        erc721Precompile.address,
        sellNFTSeries2,
        alithSigner.address,
        paymentAsset,
        fixedPrice,
        duration,
        marketplaceId,
      );
    buyReceipt = await sellNftTx.wait();
    [, listingId, , ,] = (buyReceipt?.events as any)[0].args;

    // extrinsic
    balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.marketplace.buy(listingId.toNumber()).signAndSend(alithKeyring, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    balanceAfter = await alithSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
    expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

    // Update all costs
    allTxGasCosts["buy"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileGasCost, // convert to XRP Drops(6)
      Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
    };
    allTxFeeCosts["buy"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("make bid", async () => {
    const amount = 10000000;

    let auctionNFTSeries = [24, 25, 26];
    const paymentAsset: string | number = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const reservePrice = 1000000;
    const duration = 10000; //blocks
    const marketplaceId = 1;

    // precompile
    let auctionTx = await marketPlacePrecompile
      .connect(bobSigner)
      .auctionNft(erc721Precompile.address, auctionNFTSeries, paymentAsset, reservePrice, duration, marketplaceId);
    let auctionReceipt = await auctionTx.wait();
    let [, listingIdForAuction, , ,] = (auctionReceipt?.events as any)[0].args;

    let listingId = listingIdForAuction.toNumber();
    // precompile
    const precompileGasCost = await marketPlacePrecompile.connect(alithSigner).estimateGas.bid(listingId, amount);
    let balanceBefore = await alithSigner.getBalance();

    const tx = await marketPlacePrecompile.connect(alithSigner).bid(listingId, amount);
    await tx.wait();
    let balanceAfter = await alithSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);

    auctionNFTSeries = [34, 35, 36];

    // precompile
    auctionTx = await marketPlacePrecompile
      .connect(bobSigner)
      .auctionNft(erc721Precompile.address, auctionNFTSeries, paymentAsset, reservePrice, duration, marketplaceId);
    auctionReceipt = await auctionTx.wait();
    [, listingIdForAuction, , ,] = (auctionReceipt?.events as any)[0].args;
    listingId = listingIdForAuction.toNumber();

    // extrinsic
    balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.marketplace.bid(listingId, amount).signAndSend(alithKeyring, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    balanceAfter = await alithSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
    expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

    // Update all costs
    allTxGasCosts["bid"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileGasCost, // convert to XRP Drops(6)
      Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
    };
    allTxFeeCosts["bid"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("cancel sale", async () => {
    let sellNFTSeries = [172, 173];
    const buyer = "0xB67e643F69400ad0cBb5514886fBe3439d94ba85";
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const fixedPrice = 1000000;
    const duration = 1000; //blocks
    const marketplaceId = 1;

    let sellNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .sellNft(erc721Precompile.address, sellNFTSeries, buyer, paymentAsset, fixedPrice, duration, marketplaceId);
    let sellReceipt = await sellNftTx.wait();
    let [, listingIdForSale, , ,] = (sellReceipt?.events as any)[0].args;

    let listingId = listingIdForSale.toNumber();
    // precompile
    const precompileGasCost = await marketPlacePrecompile.estimateGas.cancelSale(listingId);
    let balanceBefore = await bobSigner.getBalance();

    const tx = await marketPlacePrecompile.connect(bobSigner).cancelSale(listingId);
    await tx.wait();
    let balanceAfter = await bobSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);

    sellNFTSeries = [182, 183];
    sellNftTx = await marketPlacePrecompile
      .connect(bobSigner)
      .sellNft(erc721Precompile.address, sellNFTSeries, buyer, paymentAsset, fixedPrice, duration, marketplaceId);
    sellReceipt = await sellNftTx.wait();
    [, listingIdForSale, , ,] = (sellReceipt?.events as any)[0].args;

    listingId = listingIdForSale.toNumber();
    // extrinsic
    balanceBefore = await bobSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.marketplace.cancelSale(listingId).signAndSend(bobKeyring, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    balanceAfter = await bobSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
    expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

    // Update all costs
    allTxGasCosts["cancelSale"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileGasCost, // convert to XRP Drops(6)
      Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
    };
    allTxFeeCosts["cancelSale"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("update fixed price", async () => {
    const sellNFTSeries = [300, 301];
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

    let updatedPrice = 98000;

    // precompile
    const precompileGasCost = await marketPlacePrecompile.estimateGas.updateFixedPrice(listingId, updatedPrice);
    let balanceBefore = await bobSigner.getBalance();

    const tx = await marketPlacePrecompile.connect(bobSigner).updateFixedPrice(listingId, updatedPrice);
    await tx.wait();
    let balanceAfter = await bobSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);

    updatedPrice = 99000;
    // extrinsic
    balanceBefore = await bobSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.marketplace.updateFixedPrice(listingId.toNumber(), updatedPrice).signAndSend(bobKeyring, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    balanceAfter = await bobSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
    expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

    // Update all costs
    allTxGasCosts["updateFixedPrice"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileGasCost, // convert to XRP Drops(6)
      Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
    };
    allTxFeeCosts["updateFixedPrice"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("make simple offer and owner accepts it ", async () => {
    const offerSeries = 160;
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const amount = 100;
    const marketplaceId = 1;

    let offerNftTx = await marketPlacePrecompile
      .connect(alithSigner)
      .makeSimpleOffer(erc721Precompile.address, offerSeries, amount, paymentAsset, marketplaceId);
    let receipt = await offerNftTx.wait();
    let [offerId] = (receipt?.events as any)[0].args;

    // precompile
    const precompileGasCost = await marketPlacePrecompile.connect(bobSigner).estimateGas.acceptOffer(offerId);
    let balanceBefore = await bobSigner.getBalance();

    const tx = await marketPlacePrecompile.connect(bobSigner).acceptOffer(offerId);
    await tx.wait();
    let balanceAfter = await bobSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);

    offerNftTx = await marketPlacePrecompile
      .connect(alithSigner)
      .makeSimpleOffer(erc721Precompile.address, 161, amount, paymentAsset, marketplaceId);
    receipt = await offerNftTx.wait();
    [offerId] = (receipt?.events as any)[0].args;

    // extrinsic
    balanceBefore = await bobSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.marketplace.acceptOffer(offerId.toNumber()).signAndSend(bobKeyring, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    balanceAfter = await bobSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
    expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

    // Update all costs
    allTxGasCosts["acceptOffer"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileGasCost, // convert to XRP Drops(6)
      Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
    };
    allTxFeeCosts["acceptOffer"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("make simple offer and cancel it ", async () => {
    const offerSeries = 398;
    const paymentAsset = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const amount = 100;
    const marketplaceId = 1;

    let offerNftTx = await marketPlacePrecompile
      .connect(alithSigner)
      .makeSimpleOffer(erc721Precompile.address, offerSeries, amount, paymentAsset, marketplaceId);
    let receipt = await offerNftTx.wait();
    let [offerId] = (receipt?.events as any)[0].args;

    // precompile
    const precompileGasCost = await marketPlacePrecompile.connect(alithSigner).estimateGas.cancelOffer(offerId);
    let balanceBefore = await alithSigner.getBalance();

    const tx = await marketPlacePrecompile.connect(alithSigner).cancelOffer(offerId);
    await tx.wait();
    let balanceAfter = await alithSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);

    offerNftTx = await marketPlacePrecompile
      .connect(alithSigner)
      .makeSimpleOffer(erc721Precompile.address, 399, amount, paymentAsset, marketplaceId);
    receipt = await offerNftTx.wait();
    [offerId] = (receipt?.events as any)[0].args;

    // extrinsic
    balanceBefore = await alithSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.marketplace.cancelOffer(offerId.toNumber()).signAndSend(alithKeyring, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });
    balanceAfter = await alithSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
    expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

    // Update all costs
    allTxGasCosts["cancelOffer"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileGasCost, // convert to XRP Drops(6)
      Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
    };
    allTxFeeCosts["cancelOffer"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });
});
