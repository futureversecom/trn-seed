import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { expect } from "chai";
import { Contract, Wallet } from "ethers";
import { ethers } from "hardhat";

import {
    ALITH_PRIVATE_KEY,
    BOB_PRIVATE_KEY,
    PEG_PRECOMPILE_ABI,
    PEG_PRECOMPILE_ADDRESS,
    NFT_PRECOMPILE_ABI,
    NFT_PRECOMPILE_ADDRESS,
    ERC721_PRECOMPILE_ABI,
    //NodeProcess,
    getCollectionPrecompileAddress,
    //startNode,
    typedefs,
} from "../common";
import Web3 from "web3";
import {hexToU8a} from "@polkadot/util";
import { AddressOrPair } from "@polkadot/api/types";

describe("NFT Precompile", function () {
    //let node: NodeProcess;

    let api: ApiPromise;
    let alithSigner: Wallet;
    let bobSigner: Wallet;
    let nftProxy: Contract;
    let pegProxy: Contract;
    let alith: AddressOrPair;

    // Setup api instance
    before(async () => {
        //node = await startNode();

        // Substrate variables
        const wsProvider = new WsProvider(`ws://localhost:9944`);
        api = await ApiPromise.create({
            provider: wsProvider,
            types: typedefs,
        });
        const keyring = new Keyring({ type: "ethereum" });

        // Ethereum variables
        const provider = new JsonRpcProvider(`http://127.0.0.1:9933`);
        alithSigner = new Wallet(ALITH_PRIVATE_KEY).connect(provider); // 'development' seed
        bobSigner = new Wallet(BOB_PRIVATE_KEY).connect(provider);
        alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
        pegProxy = new Contract(PEG_PRECOMPILE_ADDRESS, PEG_PRECOMPILE_ABI, bobSigner);
        nftProxy = new Contract(NFT_PRECOMPILE_ADDRESS, NFT_PRECOMPILE_ABI, bobSigner);

        // Enable withdrawals
        await api.tx.sudo
            .sudo(api.tx.erc20Peg.activateWithdrawals(true))
            .signAndSend(alith)
            .catch((err) => console.log(err));
    });

    //after(async () => await node.stop());

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
        console.log(`Precompile address: ${precompileAddress}`);

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
        const collectionIdBin = (+collectionId).toString(2).padStart(22, "0");
        const parachainIdBin = (100).toString(2).padStart(10, "0");
        const collectionUuid = parseInt(collectionIdBin + parachainIdBin, 2);
        console.log(`CollectionUuid: ${collectionUuid}\n`);

        const collectionInfo = await api.query.nft.collectionInfo(collectionUuid);
        const collectionInfoJson = collectionInfo.toJSON();
        console.log(collectionInfoJson);
        collectionInfoJson["originChain"] = "Ethereum";

        const collectionInfoHex = api.createType('CollectionInformation', collectionInfoJson).toHex();
        console.log(`Storage Value: ${collectionInfoHex}\n`);
        const collectionInfoStorageKey = api.query.nft.collectionInfo.key(collectionUuid);
        console.log(`Storage Key: ${collectionInfoStorageKey}\n`);
        // Set storage
        // await api.tx.sudo
        //     .sudo(api.tx.system.setStorage([[collectionInfoStorageKey, collectionInfoHex]]))
        //     .signAndSend(alith)
        //     .catch((err) => console.log(err));


        console.log("Minted tokens");
        // Perform withdraw
        const receiverAddress = await Wallet.createRandom().getAddress();
        const tokenAddresses = [precompileAddress];
        const serialNumbersInner = [0,1,2,3,4,5,6,7,8,9];
        const serialNumbers = [serialNumbersInner];
        const eventProofId = await api.query.ethBridge.nextEventProofId();
        console.log(`event proof id: ${eventProofId}`);
        const withdrawTx = await pegProxy.connect(owner).erc721Withdraw(receiverAddress, tokenAddresses, serialNumbers)
        const receipt = await withdrawTx.wait();

        console.log(receipt);
        expect((receipt?.events as any)[0].event).to.equal("Erc721Withdrawal");
        expect((receipt?.events as any)[0].args.beneficiary).to.equal(receiverAddress);
        expect((receipt?.events as any)[0].args.eventProofId).to.equal(eventProofId);
        expect((receipt?.events as any)[0].args.tokenAddress).to.equal(precompileAddress);
        expect((receipt?.events as any)[0].args.serialNumbers).to.equal(serialNumbersInner);
        //expect((receipt?. as any)[0].args.serialNumbers).to.equal(serialNumbers);

    });
});
