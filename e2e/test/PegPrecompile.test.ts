import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { AddressOrPair } from "@polkadot/api/types";
import { hexToU8a } from "@polkadot/util";
import { expect } from "chai";
import { Contract, Wallet } from "ethers";
import { ethers } from "hardhat";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  ERC721_PRECOMPILE_ABI,
  NFT_PRECOMPILE_ABI,
  NFT_PRECOMPILE_ADDRESS,
  NodeProcess,
  PEG_PRECOMPILE_ABI,
  PEG_PRECOMPILE_ADDRESS,
  assetIdToERC20ContractAddress,
  getCollectionPrecompileAddress,
  getNextAssetId,
  startNode,
  typedefs,
} from "../common";

describe("Peg Precompile", function () {
  let node: NodeProcess;
  let api: ApiPromise;
  let alithSigner: Wallet;
  let bobSigner: Wallet;
  let nftProxy: Contract;
  let pegProxy: Contract;
  let alith: AddressOrPair;

  // Setup api instance
  before(async () => {
    node = await startNode();

    // Substrate variables
    const wsProvider = new WsProvider(`ws://localhost:${node.wsPort}`);
    api = await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
    });
    const keyring = new Keyring({ type: "ethereum" });

    // Ethereum variables
    const provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    pegProxy = new Contract(PEG_PRECOMPILE_ADDRESS, PEG_PRECOMPILE_ABI, bobSigner);
    nftProxy = new Contract(NFT_PRECOMPILE_ADDRESS, NFT_PRECOMPILE_ABI, bobSigner);
  });

  after(async () => await node.stop());

  it("erc721withdraw works", async () => {
    // Create an NFT collection
    const owner = alithSigner.address;
    const name = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("Test Collection"));
    const maxIssuance = 1000;
    const metadataPath = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("https://example.com/nft/metadata"));
    const royaltyAddresses = [alithSigner.address];
    const royaltyEntitlements = [1000];

    // Generate expected precompile address
    const collectionId = await api.query.nft.nextCollectionId();
    const precompileAddress = getCollectionPrecompileAddress(+collectionId);
    const initializeTx = await nftProxy
      .connect(alithSigner)
      .initializeCollection(owner, name, maxIssuance, metadataPath, royaltyAddresses, royaltyEntitlements);
    await initializeTx.wait();

    // Mint some tokens to aliths address
    const quantity = 100;
    const erc721Proxy = new Contract(precompileAddress, ERC721_PRECOMPILE_ABI, bobSigner);
    const mintTx = await erc721Proxy.connect(alithSigner).mint(owner, quantity, { gasLimit: 50000 });
    await mintTx.wait();

    // Get storage and edit the originChain field to "Ethereum"
    // This is because only tokens that are bridged from Ethereum can be bridged back
    // So we need to manually change the Origin chain to trick the node
    const collectionIdBin = (+collectionId).toString(2).padStart(22, "0");
    const parachainIdBin = (100).toString(2).padStart(10, "0");
    const collectionUuid = parseInt(collectionIdBin + parachainIdBin, 2);
    const collectionInfo = await api.query.nft.collectionInfo(collectionUuid);
    const collectionInfoJson = collectionInfo.toJSON() as any;
    collectionInfoJson["originChain"] = "Ethereum";

    const collectionInfoHex = api.createType("PalletNftCollectionInformation", collectionInfoJson).toHex();
    const collectionInfoStorageKey = api.query.nft.collectionInfo.key(collectionUuid);

    // Add storage for RootNftToErc721 mapping
    const erc721EthAddress = await Wallet.createRandom().getAddress();
    const rootToErc721StorageKey = api.query.nftPeg.rootNftToErc721.key(collectionUuid);
    const rootToErc721Hex = api.createType("EthAddress", erc721EthAddress);

    // Batch and send set storage transactions
    const txs = [
      api.tx.sudo.sudo(api.tx.system.setStorage([[collectionInfoStorageKey, collectionInfoHex]])),
      api.tx.sudo.sudo(api.tx.system.setStorage([[rootToErc721StorageKey, rootToErc721Hex]])),
    ];
    await new Promise<void>((resolve) => {
      api.tx.utility.batch(txs).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });

    // Perform withdraw
    const receiverAddress = await Wallet.createRandom().getAddress();
    const tokenAddresses = [precompileAddress];
    const serialNumbersInner = [0, 1, 2, 3];
    const serialNumbers = [serialNumbersInner];
    const eventProofId = await api.query.ethBridge.nextEventProofId();
    const withdrawTx = await pegProxy
      .connect(alithSigner)
      .erc721Withdraw(receiverAddress, tokenAddresses, serialNumbers, { gasLimit: 500000 });
    const receipt = await withdrawTx.wait();

    // Verify event
    const event = (receipt?.events as any)[0].event;
    const args = (receipt?.events as any)[0].args;
    expect(event).to.equal("Erc721Withdrawal");
    expect(args.eventProofId).to.equal(eventProofId);
    expect(args.beneficiary).to.equal(receiverAddress);
    expect(args.tokenAddress).to.equal(precompileAddress);
    expect(args.serialNumbers).to.eql(serialNumbersInner);
  });

  it("erc20withdraw works", async () => {
    // Get assetId
    const assetId = await getNextAssetId(api);
    const assetAddress = assetIdToERC20ContractAddress(assetId);

    // Get storage key for assetToErc20
    const erc20EthAddress = await Wallet.createRandom().getAddress();
    const assetToErc20StorageKey = api.query.erc20Peg.assetIdToErc20.key(assetId);
    const assetToErc20Hex = api.createType("EthAddress", erc20EthAddress);

    // Activate withdrawals
    // Create asset
    // Mint asset
    // Set storage key for assetId -> Erc20 address
    const txs = [
      api.tx.sudo.sudo(api.tx.erc20Peg.activateWithdrawals(true)),
      api.tx.assetsExt.createAsset("test", "TEST", 6, 1, alithSigner.address),
      api.tx.assets.mint(assetId, alithSigner.address, 2_000_000),
      api.tx.sudo.sudo(api.tx.system.setStorage([[assetToErc20StorageKey, assetToErc20Hex]])),
    ];
    await new Promise<void>((resolve) => {
      api.tx.utility.batch(txs).signAndSend(alith, ({ status }) => {
        if (status.isInBlock) resolve();
      });
    });

    // Perform withdraw
    const eventProofId = await api.query.ethBridge.nextEventProofId();
    const receiverAddress = await Wallet.createRandom().getAddress();
    const balance = 1337;
    const withdrawTx = await pegProxy
      .connect(alithSigner)
      .erc20Withdraw(receiverAddress, assetAddress, balance, { gasLimit: 500000 });
    const receipt = await withdrawTx.wait();

    // Verify event
    const event = (receipt?.events as any)[0].event;
    const args = (receipt?.events as any)[0].args;
    expect(event).to.equal("Erc20Withdrawal");
    expect(args.eventProofId).to.equal(eventProofId);
    expect(args.beneficiary).to.equal(receiverAddress);
    expect(args.tokenAddress).to.equal(assetAddress);
    expect(args.balance).to.equal(balance);
  });
});
