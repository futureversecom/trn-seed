import {expect} from "chai";
import { ethers } from "hardhat";
import { Contract, ContractFactory, Wallet, utils, BigNumber, Signer} from 'ethers';

import {ApiPromise, HttpProvider, WsProvider, Keyring} from '@polkadot/api';
import {u8aToHex, stringToHex, hexToU8a} from '@polkadot/util';
import {AddressOrPair} from "@polkadot/api/types";
import {JsonRpcProvider, Provider} from "@ethersproject/providers";
import ERC721PrecompileCaller from '../artifacts/contracts/ERC721PrecompileCaller.sol/ERC721PrecompileCaller.json';

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
  ExtrinsicSignature: 'EthereumSignature',
  SessionKeys: '([u8; 32], [u8; 32])'
};

// NFT Collection information
const name = "test-collection";
const metadataPath = {"Https": "example.com/nft/metadata" }
const initial_balance = 10;

describe("ERC721 Precompile", function () {
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
  before(async () => {
    const wsProvider = new WsProvider(`ws://localhost:9944`);

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
    precompileCaller = await factory.connect(bobSigner).deploy(nftPrecompileAddress);
  });


  it('name, symbol, ownerOf, tokenURI, balanceOf', async () => {
    expect(
        await nftContract.name()
    ).to.equal(name);

    expect(
        await nftContract.symbol()
    ).to.equal(name);

    expect(
        await nftContract.ownerOf(1)
    ).to.equal(bobSigner.address);

    expect(
        await nftContract.balanceOf(bobSigner.address)
    ).to.equal(initial_balance);

    expect(
        await nftContract.tokenURI(1)
    ).to.equal("https://example.com/nft/metadata/1.json");
  }).timeout(150000);


  it('transferFrom owner', async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const serial_number = 0;

    // Transfer serial_number 0 to receiverAddress
    expect(
        await nftContract.transferFrom(bobSigner.address, receiverAddress, serial_number)
    ).to.emit(nftContract, 'Transfer').withArgs(bobSigner.address, receiverAddress, serial_number);
    await new Promise(r => setTimeout(r, 6000));

    // Receiver_address now owner of serial_number 1
    expect(
        await nftContract.ownerOf(serial_number)
    ).to.equal(receiverAddress);
  }).timeout(18000);


  it('approve and transferFrom via transaction', async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const serial_number = 1;

    // Bob approves alice for serial_number
    expect(
        await nftContract.approve(aliceSigner.address, serial_number)
    ).to.emit(nftContract, 'Approval').withArgs(bobSigner.address, bobSigner.address, serial_number);
    await new Promise(r => setTimeout(r, 6000));

    // getApproved should be alice
    expect(
        await nftContract.getApproved(serial_number)
    ).to.equal(aliceSigner.address);

    // Alice transfers serial_number (Owned by Bob)
    expect(
        await nftContract.connect(aliceSigner).transferFrom(bobSigner.address, receiverAddress, serial_number)
    ).to.emit(nftContract, 'Transfer').withArgs(bobSigner.address, receiverAddress, serial_number);
    await new Promise(r => setTimeout(r, 6000));

    // Receiver_address now owner of serial_number
    expect(
        await nftContract.ownerOf(serial_number)
    ).to.equal(receiverAddress);
  }).timeout(21000);


  it('name, symbol, ownerOf, tokenURI, balanceOf via EVM', async () => {
    const serial_number = 2;

    // Check state proxy calls
    expect(
        await precompileCaller.ownerOfProxy(serial_number)
    ).to.equal(bobSigner.address);

    expect(
        await precompileCaller.nameProxy()
    ).to.equal(name);

    expect(
        await precompileCaller.symbolProxy()
    ).to.equal(name);

    expect(
        await precompileCaller.tokenURIProxy(serial_number)
    ).to.equal(`https://example.com/nft/metadata/${serial_number}.json`);
  }).timeout(15000);
  
  
  it('approve and transferFrom via EVM', async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const serial_number = 3;

    // Bob approves contract for serial_number
    const approval = await nftContract.connect(bobSigner).approve(precompileCaller.address, serial_number)
    await approval.wait();
    // Approved should be correct
    expect(
        await nftContract.getApproved(serial_number)
    ).to.equal(precompileCaller.address);

    // Transfer serial_number to receiverAddress
    const transfer = await precompileCaller
        .connect(bobSigner)
        .transferFromProxy(bobSigner.address, receiverAddress, serial_number, {gasLimit: 50000})
    await transfer.wait();

    // contract_address now owner of serial_number
    expect(
        await precompileCaller.balanceOfProxy(receiverAddress)
    ).to.equal(1);
    expect(
        await precompileCaller.ownerOfProxy(serial_number)
    ).to.equal(receiverAddress);
  }).timeout(21000);
});
