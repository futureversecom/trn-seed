import { time, loadFixture } from "@nomicfoundation/hardhat-network-helpers";
import { anyValue } from "@nomicfoundation/hardhat-chai-matchers/withArgs";
import {expect} from "chai";
import { ethers } from "hardhat";
import { Contract, ContractFactory, Wallet, utils, BigNumber } from 'ethers';
import web3 from 'web3';

import { ApiPromise, HttpProvider, WsProvider, Keyring } from '@polkadot/api';
import { u8aToHex, stringToHex, hexToU8a } from '@polkadot/util';
import { AddressOrPair } from "@polkadot/api/types";
import { JsonRpcProvider, Provider } from "@ethersproject/providers";

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

describe("ERC721 Precompile", function () {
  let api: ApiPromise;
  let keyring: Keyring;
  let alice: AddressOrPair;
  let bob: AddressOrPair;
  let seedSigner: Wallet;
  let nftContract: Contract;
  let precompileCaller: Contract;
  let jsonProvider: Provider;
  // NFT Collection information
  const name = "test-collection";
  const metadataPath = {"Https": "example.com/nft/metadata" }
  const initial_balance = 10;
  // Address for first NFT collection
  const nftPrecompileAddress = web3.utils.toChecksumAddress('0xAAAAAAAA00000464000000000000000000000000');
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
  // Setup api instance
  before(async () => {
    // Setup providers for jsonRPCs and WS
    jsonProvider = new JsonRpcProvider(`http://localhost:9933`);
    const wsProvider = new WsProvider(`ws://localhost:9944`);

    // Bob as seed signer
    seedSigner = new Wallet('0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf').connect(jsonProvider); // 'development' seed
    nftContract = new Contract(nftPrecompileAddress, erc721Abi, seedSigner);
    console.log(`signer address: ${seedSigner.address}`);

    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });

    keyring = new Keyring({ type: 'ethereum' });
    alice = keyring.addFromSeed(hexToU8a('0xcb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854'));
    bob = keyring.addFromSeed(hexToU8a('0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf'));

    console.log(`Connected to Root network`);
    console.log(`Genesis hash: ${api.genesisHash.toHex()}`);

    // Create NFT collection using runtime
    let txHash = await api.tx.nft.createCollection(name, initial_balance, null, null, metadataPath, null).signAndSend(bob);
    console.log(`Minted NFT collection ${txHash}`);
  });

  it('name, symbol, ownerOf, balanceOf', async () => {
    // Pause to allow block to finalize NFT collection creation
    await new Promise(r => setTimeout(r, 6000));

    console.log("Calling name()")
    expect(
        await nftContract.name()
    ).to.equal(name);
    // await new Promise(r => setTimeout(r, 500));

    console.log("Calling symbol()")
    expect(
        await nftContract.symbol()
    ).to.equal(name);
    // await new Promise(r => setTimeout(r, 500));

    console.log("Calling ownerOf(token_id)")
    expect(
        await nftContract.ownerOf(1)
    ).to.equal(seedSigner.address);
    // await new Promise(r => setTimeout(r, 500));

    console.log("Calling balanceOf(seedSigner)")
    expect(
        await nftContract.balanceOf(seedSigner.address)
    ).to.equal(initial_balance);
  }).timeout(15000);

  it('transferFrom', async () => {
    await new Promise(r => setTimeout(r, 6000));

    const receiverAddress = await Wallet.createRandom().getAddress();
    const token_id = 0;
    console.log(`Balance: ${await jsonProvider.getBalance(seedSigner.address)}`);
    // expect(
    //     await nftContract.approve(seedSigner.address, token_id)
    // ).to.emit(nftContract, 'Approval').withArgs(seedSigner.address, seedSigner.address, token_id);
    // await new Promise(r => setTimeout(r, 1000));

    expect(
        await nftContract.transferFrom(seedSigner.address, receiverAddress, token_id)
    ).to.emit(nftContract, 'Transfer').withArgs(seedSigner.address, receiverAddress, token_id);

    await new Promise(r => setTimeout(r, 6000));
    expect(
        await nftContract.balanceOf(receiverAddress)
    ).to.equal(1);
  }).timeout(18000);

});
