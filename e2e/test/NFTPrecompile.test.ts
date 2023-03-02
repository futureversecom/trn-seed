import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { expect } from "chai";
import { Contract, Wallet } from "ethers";
import { ethers } from "hardhat";

import { ALITH_PRIVATE_KEY, BOB_PRIVATE_KEY, getCollectionPrecompileAddress, NodeProcess, startNode, typedefs } from "../common";

// Precompile address for nft precompile is 1721
const nftPrecompileAddress = "0x00000000000000000000000000000000000006b9";

const nftAbi = [
  "event InitializeCollection(address indexed collectionOwner, address precompileAddress)",
  "function initializeCollection(address owner, bytes name, uint32 maxIssuance, uint8 metadataType, bytes metadataPath, address[] royaltyAddresses, uint32[] royaltyEntitlements) returns (address, uint32)",
];

describe("NFT Precompile", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let provider: JsonRpcProvider;
  let alithSigner: Wallet;
  let bobSigner: Wallet;

  // Setup api instance
  before(async () => {
    node = await startNode();
    await node.wait(); // wait for the node to be ready

    // Substrate variables
    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.wsPort}`);
    api = await ApiPromise.create({
      provider: wsProvider,
      types: typedefs,
    });

    // Ethereum variables
    provider = new JsonRpcProvider(`http://127.0.0.1:${node.httpPort}`);
    alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
    bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);
  });

  after(async () => await node.stop());

  it("initialize collection", async () => {
    const nftProxy = new Contract(nftPrecompileAddress, nftAbi, bobSigner);

    const owner = alithSigner.address;
    const name = ethers.utils.formatBytes32String("My Collection");
    const maxIssuance = 100;
    const metadataType = 1;
    const metadataPath = ethers.utils.hexlify(ethers.utils.toUtf8Bytes('example.com/nft/metadata'));
    const royaltyAddresses = [alithSigner.address];
    const royaltyEntitlements = [1000];

    // Generate expected precompile address
    const collectionId = await api.query.nft.nextCollectionId();
    const expectedPrecompileAddress = getCollectionPrecompileAddress(+collectionId);

    const initializeTx = await nftProxy
      .connect(bobSigner)
      .initializeCollection(
        owner,
        name,
        maxIssuance,
        metadataType,
        metadataPath,
        royaltyAddresses,
        royaltyEntitlements,
      );
    const receipt = await initializeTx.wait();
    expect((receipt?.events as any)[0].event).to.equal("InitializeCollection");
    expect((receipt?.events as any)[0].args.collectionOwner).to.equal(alithSigner.address);
    expect((receipt?.events as any)[0].args.precompileAddress).to.equal(expectedPrecompileAddress);
  });
});
