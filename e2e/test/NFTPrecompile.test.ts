import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { expect } from "chai";
import { Contract, Wallet } from "ethers";
import { ethers } from "hardhat";

import {
  ALITH_PRIVATE_KEY,
  BOB_PRIVATE_KEY,
  NFT_PRECOMPILE_ABI,
  NFT_PRECOMPILE_ADDRESS,
  NodeProcess,
  getCollectionPrecompileAddress,
  startNode,
  typedefs,
} from "../common";

describe("NFT Precompile", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let alithSigner: Wallet;
  let bobSigner: Wallet;
  let nftProxy: Contract;

  // Setup api instance
  before(async () => {
    node = await startNode();

    // Substrate variables
    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    api = await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
    });

    // Ethereum variables
    const provider = new JsonRpcProvider(`http://127.0.0.1:${node.rpcPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);

    nftProxy = new Contract(NFT_PRECOMPILE_ADDRESS, NFT_PRECOMPILE_ABI, bobSigner);
  });

  after(async () => await node.stop());

  it("initialize collection succeeds", async () => {
    const owner = alithSigner.address;
    const name = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("My Collection"));
    const maxIssuance = 100;
    const metadataPath = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("https://example.com/nft/metadata"));
    const royaltyAddresses = [alithSigner.address];
    const royaltyEntitlements = [1000];

    // Generate expected precompile address
    const collectionId = await api.query.nft.nextCollectionId();
    const expectedPrecompileAddress = getCollectionPrecompileAddress(+collectionId);

    const initializeTx = await nftProxy
      .connect(bobSigner)
      .initializeCollection(owner, name, maxIssuance, metadataPath, royaltyAddresses, royaltyEntitlements);
    const receipt = await initializeTx.wait();

    expect((receipt?.events as any)[0].event).to.equal("InitializeCollection");
    expect((receipt?.events as any)[0].args.collectionOwner).to.equal(alithSigner.address);
    expect((receipt?.events as any)[0].args.precompileAddress).to.equal(expectedPrecompileAddress);
  });

  it("initialize collection fails - invalid metadata URI", async () => {
    const owner = alithSigner.address;

    const name = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("My Collection"));
    const maxIssuance = 100;
    const metadataPath = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("tcp://example.com/nft/metadata"));
    const royaltyAddresses = [alithSigner.address];
    const royaltyEntitlements = [1000];

    await nftProxy
      .connect(bobSigner)
      .initializeCollection(owner, name, maxIssuance, metadataPath, royaltyAddresses, royaltyEntitlements)
      .catch((err: any) => expect(err.message).contains("NFT: Invalid metadata_path: scheme not supported"));
  });
});
