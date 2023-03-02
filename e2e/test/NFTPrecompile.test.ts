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
  "function initializeCollection(address owner, bytes name, uint32 maxIssuance, bytes metadataPath, address[] royaltyAddresses, uint32[] royaltyEntitlements) returns (address, uint32)",
];

describe("NFT Precompile", function () {
  let node: NodeProcess;

  let api: ApiPromise;
  let provider: JsonRpcProvider;
  let alithSigner: Wallet;
  let bobSigner: Wallet;
  let nftProxy: Contract;

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

    nftProxy = new Contract(nftPrecompileAddress, nftAbi, bobSigner);
  });

  after(async () => await node.stop());

  it("initialize collection succeeds", async () => {
    const owner = alithSigner.address;
    const name = ethers.utils.formatBytes32String("My Collection");
    const maxIssuance = 100;
    const metadataPath = ethers.utils.hexlify(ethers.utils.toUtf8Bytes('https://example.com/nft/metadata'));
    const royaltyAddresses = [alithSigner.address];
    const royaltyEntitlements = [1000];

    const initializeTx = await nftProxy
      .connect(bobSigner)
      .initializeCollection(
        owner,
        name,
        maxIssuance,
        metadataPath,
        royaltyAddresses,
        royaltyEntitlements,
      );
    const receipt = await initializeTx.wait();

    // Generate expected precompile address
    const collectionId = await api.query.nft.nextCollectionId();
    const expectedPrecompileAddress = getCollectionPrecompileAddress(+collectionId);

    expect((receipt?.events as any)[0].event).to.equal("InitializeCollection");
    expect((receipt?.events as any)[0].args.collectionOwner).to.equal(alithSigner.address);
    expect((receipt?.events as any)[0].args.precompileAddress).to.equal(expectedPrecompileAddress);
  });

  it("initialize collection fails - invalid metadata URI", async () => {
    const owner = alithSigner.address;
    const name = ethers.utils.formatBytes32String("My Collection");
    const maxIssuance = 100;
    const metadataPath = ethers.utils.hexlify(ethers.utils.toUtf8Bytes('tcp://example.com/nft/metadata'));
    const royaltyAddresses = [alithSigner.address];
    const royaltyEntitlements = [1000];

    await nftProxy
      .connect(bobSigner)
      .initializeCollection(
        owner,
        name,
        maxIssuance,
        metadataPath,
        royaltyAddresses,
        royaltyEntitlements,
      )
      .catch((err: any) => expect(err.message).contains("NFT: Invalid metadata_path: scheme not supported"));
  });
});
