// import {expect} from "chai";
import { ethers } from "hardhat";
import { Contract, Wallet} from 'ethers';
import {ApiPromise, WsProvider, Keyring} from '@polkadot/api';
import {hexToU8a} from '@polkadot/util';
import {AddressOrPair} from "@polkadot/api/types";
// import ERC721PrecompileCaller from '../artifacts/contracts/ERC721PrecompileCaller.sol/ERC721PrecompileCaller.json';

const erc721Abi = [
  'event Transfer(address indexed from, address indexed to, uint256 tokenId)',
  'event Approval(address indexed owner, address indexed approved, uint256 tokenId)',
  'event ApprovalForAll(address indexed owner, address indexed operator, bool approved)',
  'function balanceOf(address who) public view returns (uint256)',
  'function ownerOf(uint256 tokenId) public view returns (address)',
  'function safeTransferFrom(address from, address to, uint256 tokenId)',
  'function transferFrom(address from, address to, uint256 tokenId)',
  'function approve(address to, uint256 tokenId)',
  'function getApproved(uint256 tokenId) public view returns (address)',
  'function setApprovalForAll(address operator, bool _approved)',
  'function isApprovedForAll(address owner, address operator) public view returns (bool)',
  'function safeTransferFrom(address from, address to, uint256 tokenId, bytes data)',
  'function name() public view returns (string memory)',
  'function symbol() public view returns (string memory)',
  'function tokenURI(uint256 tokenId) public view returns (string memory)',
];

const typedefs = {
  AccountId: 'EthereumAccountId',
  AccountId20: 'EthereumAccountId',
  AccountId32: 'EthereumAccountId',
  Address: 'AccountId',
  LookupSource: 'AccountId',
  Lookup0: 'AccountId',
  EthereumSignature: {
    r: 'H256',
    s: 'H256',
    v: 'U8'
  },
  ExtrinsicSignature: 'EthereumSignature'
};

// NFT Collection information
const name = "test-collection";
const metadataPath = {"Https": "example.com/nft/metadata" }
const initial_balance = 10;

const main = async () => {
  let api: ApiPromise;
  let keyring: Keyring;
  let bob: AddressOrPair;
  let bobSigner: any;
  let aliceSigner: any;
  let nftContract: Contract;
  // Address for NFT collection
  let nftPrecompileAddress: string;
  let precompileCaller: Contract;
  // Setup api instance
    const wsProvider = new WsProvider(`ws://localhost:9944`);
    // const wsProvider = new WsProvider(`wss://porcini.au.rootnet.app/ws`);


    // Setup Root api instance and keyring
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    keyring = new Keyring({ type: 'ethereum' });
    bob = keyring.addFromSeed(hexToU8a('0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf'));
    [aliceSigner, bobSigner] = await ethers.getSigners();

    // Create NFT collection using runtime, bob is collection owner
    await new Promise<void>((resolve) => {
      api.tx.nft
        .createCollection(name, initial_balance, null, null, metadataPath, null)
        .signAndSend(bob, async ({status, events}) => {
          if (status.isInBlock) {
            events.forEach(({event: {data, method}}) => {
              if (method == 'CollectionCreate') {
                let collection_uuid = (data.toJSON() as any)[0];
                const collection_id_hex = (+collection_uuid).toString(16).padStart(8, '0');
                nftPrecompileAddress = ethers.utils.getAddress(`0xAAAAAAAA${collection_id_hex}000000000000000000000000`);
                nftContract = new Contract(nftPrecompileAddress, erc721Abi, bobSigner);
                resolve();
              }
            })
          }
        });
    });

    // Deploy PrecompileCaller contract
    const factory = await ethers.getContractFactory("ERC721PrecompileCaller");
    // precompileCaller = await factory.connect(bobSigner).deploy(nftPrecompileAddress);
}


main()
  .then(console.log)
  .catch(console.log)