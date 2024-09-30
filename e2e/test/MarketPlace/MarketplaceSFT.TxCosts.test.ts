import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { BigNumber, Contract, Wallet } from "ethers";
import { ethers } from "hardhat";
import Web3 from "web3";
import web3 from "web3";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC1155_PRECOMPILE_ABI,
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

// SFT Collection information
const name = "test-sft-collection";
const metadataPath = "https://example.com/sft/metadata/";

describe("Marketplace SFT Precompile Gas Estimates", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let bobSigner: Wallet;
  let alithSigner: Wallet;
  let erc1155Precompile: Contract;
  let marketPlacePrecompile: Contract;
  let erc1155PrecompileAddress: string;
  let collectionId: string;
  let bobKeyring: KeyringPair;
  let provider: JsonRpcProvider;

  const allTxGasCosts: { [key: string]: TxCosts } = {};
  const allTxFeeCosts: { [key: string]: TxCosts } = {};

  // Setup api instance
  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    // Setup Root api instance and keyring
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    const keyring = new Keyring({ type: "ethereum" });
    bobKeyring = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));

    provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);

    // Create SFT collection using runtime, bob is collection owner
    await new Promise<void>((resolve, reject) => {
      api.tx.sft
        .createCollection(name, null, metadataPath, null)
        .signAndSend(bobKeyring, async ({ status, events }) => {
          if (status.isInBlock) {
            events.forEach(({ event: { data, method } }) => {
              if (method == "CollectionCreate") {
                const collection_uuid = (data.toJSON() as any)[0];
                collectionId = collection_uuid;

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

    let quantity = 499;
    // create token 1
    const tokenName1 = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("MyToken1"));
    const tx1 = await erc1155Precompile.connect(bobSigner).createToken(tokenName1, quantity, 0, bobSigner.address);
    await tx1.wait();

    // create token 2
    quantity = 899;
    const tokenName2 = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("MyToken2"));
    const tx2 = await erc1155Precompile.connect(bobSigner).createToken(tokenName2, quantity, 0, bobSigner.address);
    await tx2.wait();

    // create token 3
    quantity = 1005;
    const tokenName = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("MyToken3"));
    const tx3 = await erc1155Precompile.connect(bobSigner).createToken(tokenName, quantity, 0, bobSigner.address);
    await tx3.wait();

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
    saveTxGas(allTxGasCosts, "MarketPlace/SFTTxCosts.md", "Marketplace Precompiles SFT");
    saveTxFees(allTxFeeCosts, "MarketPlace/SFTTxCosts.md", "Marketplace Precompiles SFT");
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

  it("sell sft with marketplaceId", async () => {
    const sellSFTSeries = [0, 1, 2];
    const quantities = [10, 15, 12];
    let paymentAsset: string | number = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const fixedPrice = 1000000;
    const duration = 1000; //blocks
    const marketplaceId = 1;

    // precompile
    const precompileGasCost = await marketPlacePrecompile.estimateGas.sellSft(
      erc1155Precompile.address,
      sellSFTSeries,
      quantities,
      alithSigner.address,
      paymentAsset,
      fixedPrice,
      duration,
      marketplaceId,
    );
    let balanceBefore = await bobSigner.getBalance();
    const tx = await marketPlacePrecompile
      .connect(bobSigner)
      .sellSft(
        erc1155Precompile.address,
        sellSFTSeries,
        quantities,
        alithSigner.address,
        paymentAsset,
        fixedPrice,
        duration,
        marketplaceId,
      );
    const receipt = await tx.wait();
    let balanceAfter = await bobSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);
    const [seller, listingId, fixedPriceFromCall, serialNumbers, collectionAddress, marketplaceIdArgs, _quantities] = (
      receipt?.events as any
    )[0].args;
    expect((receipt?.events as any)[0].event).to.equal("FixedPriceSaleListSFT");
    expect(collectionAddress).to.equal(erc1155Precompile.address);
    expect(listingId.toNumber()).to.gte(0);
    expect(fixedPriceFromCall.toNumber()).to.equal(fixedPrice);
    expect(seller).to.equal(bobSigner.address);
    const s = serialNumbers.map((s: BigNumber) => s.toNumber());
    expect(JSON.stringify(s)).to.equal(JSON.stringify(sellSFTSeries));
    const q = _quantities.map((s: BigNumber) => s.toNumber());
    expect(JSON.stringify(q)).to.equal(JSON.stringify(quantities));
    expect(marketplaceIdArgs.toNumber()).to.gte(0);

    // extrinsic
    paymentAsset = 2;
    const tokens = api.createType(
      "PalletMarketplaceListingTokens",
      {
        collectionId: collectionId,
        serialNumbers: [
          [0, 10],
          [1, 21],
          [2, 5],
        ],
      },
      1,
    );
    balanceBefore = await bobSigner.getBalance();
    await new Promise<void>((resolve) => {
      api.tx.marketplace
        .sell(tokens, alithSigner.address, paymentAsset, fixedPrice, duration, marketplaceId)
        .signAndSend(bobKeyring, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    balanceAfter = await bobSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
    expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

    // Update all costs
    allTxGasCosts["sellSft"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileGasCost, // convert to XRP Drops(6)
      Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
    };
    allTxFeeCosts["sellSft"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });

  it("auction sft with marketplaceId", async () => {
    const auctionSFTSeries = [0, 1, 2];
    let paymentAsset: string | number = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000"); //xrp token address
    const reservePrice = 1000000;
    const duration = 10000; //blocks
    const marketplaceId = 1;
    const quantities = [5, 10, 15];

    // precompile
    const precompileGasCost = await marketPlacePrecompile.estimateGas.auctionSft(
      erc1155Precompile.address,
      auctionSFTSeries,
      quantities,
      paymentAsset,
      reservePrice,
      duration,
      marketplaceId,
    );
    let balanceBefore = await bobSigner.getBalance();
    const tx = await marketPlacePrecompile
      .connect(bobSigner)
      .auctionSft(
        erc1155Precompile.address,
        auctionSFTSeries,
        quantities,
        paymentAsset,
        reservePrice,
        duration,
        marketplaceId,
      );
    const receipt = await tx.wait();
    const [collectionIdArgs, listingId, reservePriceFromChain, seller, serialNumbers, marketplaceIdArgs] = (
      receipt?.events as any
    )[0].args;
    expect((receipt?.events as any)[0].event).to.equal("AuctionOpenSFT");
    expect(collectionIdArgs.toNumber()).to.gte(0);
    expect(listingId.toNumber()).to.gte(0);
    expect(reservePriceFromChain.toNumber()).to.equal(reservePrice);
    expect(seller).to.equal(bobSigner.address);
    const s = serialNumbers.map((s: BigNumber) => s.toNumber());
    expect(JSON.stringify(s)).to.equal(JSON.stringify(auctionSFTSeries));
    expect(marketplaceIdArgs.toNumber()).to.gte(0);

    let balanceAfter = await bobSigner.getBalance();
    const precompileFeeCost = balanceBefore.sub(balanceAfter);

    // extrinsic
    paymentAsset = 2;
    balanceBefore = await bobSigner.getBalance();
    const tokens = api.createType(
      "PalletMarketplaceListingTokens",
      {
        collectionId: collectionId,
        serialNumbers: [
          [0, 10],
          [1, 21],
          [2, 5],
        ],
      },
      1,
    );

    await new Promise<void>((resolve) => {
      api.tx.marketplace
        .auction(tokens, paymentAsset, reservePrice, duration, marketplaceId)
        .signAndSend(bobKeyring, ({ status }) => {
          if (status.isInBlock) resolve();
        });
    });
    balanceAfter = await bobSigner.getBalance();
    const extrinsicFeeCost = balanceBefore.sub(balanceAfter);
    const extrinsicGasCost = await getScaledGasForExtrinsicFee(provider, extrinsicFeeCost);
    expect(extrinsicFeeCost).to.be.lessThan(precompileFeeCost);

    // Update all costs
    allTxGasCosts["auctionSft"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileGasCost, // convert to XRP Drops(6)
      Extrinsic: extrinsicGasCost, // convert to XRP Drops(6)
    };
    allTxFeeCosts["auctionSft"] = {
      Contract: BigNumber.from(0), // no contract
      Precompile: precompileFeeCost.div(1000000000000n), // convert to XRP Drops(6)
      Extrinsic: extrinsicFeeCost.div(1000000000000n), // convert to XRP Drops(6)
    };
  });
});
