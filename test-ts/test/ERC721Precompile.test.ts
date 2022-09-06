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
import ERC721PrecompileCaller from '../artifacts/contracts/ERC721PrecompileCaller.sol/ERC721PrecompileCaller.json';
import ERC721CallerSimple from '../artifacts/contracts/ERC721CallerSimple.sol/ERC721CallerSimple.json';

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
  let bob: AddressOrPair;
  let bobSigner: Wallet;
  let aliceSigner: Wallet;
  let bobContract: Contract;
  let aliceContract: Contract;
  let jsonProvider: Provider;
  // NFT Collection information
  const name = "test-collection";
  const metadataPath = {"Https": "example.com/nft/metadata" }
  const initial_balance = 10;
  // Address for first NFT collection
  let nftPrecompileAddress: string = web3.utils.toChecksumAddress('0xAAAAAAAA00000464000000000000000000000000');
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

    // Setup Root api instance and keyring
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    keyring = new Keyring({ type: 'ethereum' });
    bob = keyring.addFromSeed(hexToU8a('0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf'));
    console.log(`Connected to Root network`);

    // Bob as seed signer
    bobSigner = new Wallet('0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf').connect(jsonProvider); // 'development' seed
    aliceSigner = new Wallet('0xcb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854').connect(jsonProvider); // 'development' seed
    console.log(`Balance of Alice: ${await jsonProvider.getBalance(aliceSigner.address)}`);
    console.log(`Balance of Bob: ${await jsonProvider.getBalance(bobSigner.address)}`);

    // Create NFT collection using runtime, bob is collection owner
    api.tx.nft.createCollection(name, initial_balance, null, null, metadataPath, null).signAndSend(bob, async({ status, events }) => {
      if (status.isInBlock) {
        events.forEach(({ event: {data, method}}) => {
          if (method == 'CollectionCreate') {
            let collection_uuid = (data.toJSON() as any)[0];
            console.log(`Minted NFT collection, collection_uuid: ${collection_uuid}`);

            const collection_id_hex = (+collection_uuid).toString(16).padStart(8, '0');
            nftPrecompileAddress = web3.utils.toChecksumAddress(`0xAAAAAAAA${collection_id_hex}000000000000000000000000`);

            // Create two contracts, one for alice, one for Bob. This will allow approval tests
            bobContract = new Contract(nftPrecompileAddress, erc721Abi, bobSigner);
            aliceContract = new Contract(nftPrecompileAddress, erc721Abi, aliceSigner);

            console.log(`NFT precompile address: ${nftPrecompileAddress}`);
          }
        })
      }
    });

    // Pause to allow block to finalize NFT collection creation
    await new Promise(r => setTimeout(r, 6000));
  });
  //
  //
  // it('name, symbol, ownerOf, tokenURI, balanceOf', async () => {
  //   expect(
  //       await bobContract.name()
  //   ).to.equal(name);
  //   // await new Promise(r => setTimeout(r, 500));
  //
  //   expect(
  //       await bobContract.symbol()
  //   ).to.equal(name);
  //   // await new Promise(r => setTimeout(r, 500));
  //
  //   expect(
  //       await bobContract.ownerOf(1)
  //   ).to.equal(bobSigner.address);
  //   // await new Promise(r => setTimeout(r, 500));
  //
  //   expect(
  //       await bobContract.balanceOf(bobSigner.address)
  //   ).to.equal(initial_balance);
  //
  //   expect(
  //       await bobContract.tokenURI(1)
  //   ).to.equal("https://example.com/nft/metadata/1.json");
  // }).timeout(15000);
  //
  //
  // it('transferFrom owner', async () => {
  //   const receiverAddress = await Wallet.createRandom().getAddress();
  //   const token_id = 0;
  //
  //   // Transfer token_id 0 to receiverAddress
  //   expect(
  //       await bobContract.transferFrom(bobSigner.address, receiverAddress, token_id)
  //   ).to.emit(bobContract, 'Transfer').withArgs(bobSigner.address, receiverAddress, token_id);
  //   await new Promise(r => setTimeout(r, 6000));
  //
  //   // Receiver_address now owner of token_id 1
  //   expect(
  //       await bobContract.ownerOf(token_id)
  //   ).to.equal(receiverAddress);
  // }).timeout(18000);
  //
  //
  // it('approve and transferFrom via transaction', async () => {
  //   const receiverAddress = await Wallet.createRandom().getAddress();
  //   const token_id = 1;
  //
  //   // Bob approves alice for token_id
  //   expect(
  //       await bobContract.approve(aliceSigner.address, token_id)
  //   ).to.emit(bobContract, 'Approval').withArgs(bobSigner.address, bobSigner.address, token_id);
  //   await new Promise(r => setTimeout(r, 6000));
  //
  //   // getApproved should be alice
  //   expect(
  //       await bobContract.getApproved(token_id)
  //   ).to.equal(aliceSigner.address);
  //
  //   // Alice transfers token_id (Owned by Bob)
  //   expect(
  //       await aliceContract.transferFrom(bobSigner.address, receiverAddress, token_id)
  //   ).to.emit(aliceContract, 'Transfer').withArgs(aliceSigner.address, receiverAddress, token_id);
  //   await new Promise(r => setTimeout(r, 6000));
  //
  //   // Receiver_address now owner of token_id
  //   expect(
  //       await bobContract.ownerOf(token_id)
  //   ).to.equal(receiverAddress);
  // }).timeout(21000);

  // it('approve and transferFrom via EVM Simple contract', async () => {
  //   const receiverAddress = await Wallet.createRandom().getAddress();
  //   const token_id = 2;
  //   // Deploy PrecompileSimpleCaller contract
  //   let factory = await ethers.getContractFactory("ERC721CallerSimple");
  //   const precompileCaller = await factory.connect(aliceSigner).deploy();
  //   console.log(`CONTRACT DEPLOYED: ${precompileCaller.address}`);
  //
  //   await new Promise(r => setTimeout(r, 6000));
  //
  //   // Bob approves contract for token_id
  //   expect(
  //       await bobContract.approve(precompileCaller.address, token_id)
  //   ).to.emit(bobContract, 'Approval').withArgs(bobSigner.address, precompileCaller.address, token_id);
  //   await new Promise(r => setTimeout(r, 12000));
  //
  //   // Approved should be correct
  //   expect(
  //       await bobContract.getApproved(token_id)
  //   ).to.equal(precompileCaller.address);
  //
  //   console.log("TESTING TRANSFER FROM");
  //
  //   expect(
  //       await precompileCaller.connect(bobSigner).transferFromProxy(nftPrecompileAddress, bobSigner.address, receiverAddress, token_id, {gasLimit: 500000})
  //   ).to.emit(bobContract, 'Transfer').withArgs(bobSigner.address, receiverAddress, token_id);
  //   await new Promise(r => setTimeout(r, 6000));
  //   console.log("Transferred");
  //
  //   // contract_address now owner of token_id
  //   expect(
  //       await bobContract.ownerOf(token_id)
  //   ).to.equal(receiverAddress);
  // }).timeout(21000000000000);

  it('approve and transferFrom via EVM', async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const token_id = 3;
    // Deploy PrecompileSimpleCaller contract
    let factory = await ethers.getContractFactory("ERC721PrecompileCaller");
    const precompileCaller = await factory.connect(aliceSigner).deploy(nftPrecompileAddress);
    console.log(`CONTRACT DEPLOYED: ${precompileCaller.address}`);
    await new Promise(r => setTimeout(r, 6000));

    // Check state check proxy calls
    // expect(
    //     await precompileCaller.ownerOfProxy(token_id)
    // ).to.equal(bobSigner.address);
    //
    // expect(
    //     await precompileCaller.nameProxy()
    // ).to.equal(name);
    //
    // expect(
    //     await precompileCaller.symbolProxy()
    // ).to.equal(name);
    //
    // expect(
    //     await precompileCaller.tokenURIProxy(token_id)
    // ).to.equal(`https://example.com/nft/metadata/${token_id}.json`);

    // Bob approves contract for token_id
    // expect(
    //     await bobContract.approve(precompileCaller.address, token_id)
    // ).to.emit(bobContract, 'Approval').withArgs(bobSigner.address, precompileCaller.address, token_id);
    // await new Promise(r => setTimeout(r, 12000));

    // Approved should be correct
    // expect(
    //     await bobContract.getApproved(token_id)
    // ).to.equal(precompileCaller.address);

    console.log("TESTING TRANSFER FROM");
    await precompileCaller.connect(bobSigner).transferFromProxy(nftPrecompileAddress, receiverAddress, token_id, {gasLimit: 50000})
    //
    // expect(
    //     await precompileCaller.connect(bobSigner).transferFromProxy(nftPrecompileAddress, receiverAddress, token_id, {gasLimit: 500000})
    // ).to.emit(bobContract, 'Transfer').withArgs(precompileCaller.address, receiverAddress, 12);
    await new Promise(r => setTimeout(r, 6000));
    console.log("Transferred");

    // contract_address now owner of token_id
    expect(
        await bobContract.ownerOf(token_id)
    ).to.equal(receiverAddress);
  }).timeout(21000000000000);

  // it('approve and transferFrom via EVM', async () => {
  //   const receiverAddress = await Wallet.createRandom().getAddress();
  //   const token_id = 2;
  //   // Deploy PrecompileCaller contract
  //   let factory = await ethers.getContractFactory("ERC721PrecompileCaller");
  //   const precompileCaller = await factory.connect(aliceSigner).deploy(nftPrecompileAddress);
  //
  //   // Check state check proxy calls
  //   // expect(
  //   //     await precompileCaller.ownerOfProxy(token_id)
  //   // ).to.equal(bobSigner.address);
  //   //
  //   console.log("GETTING NAME");
  //
  //   // expect(
  //   //     await precompileCaller.nameProxy()
  //   // ).to.equal(name);
  //   //
  //   // expect(
  //   //     await precompileCaller.symbolProxy()
  //   // ).to.equal(name);
  //   //
  //   // expect(
  //   //     await precompileCaller.tokenURIProxy(token_id)
  //   // ).to.equal(`https://example.com/nft/metadata/${token_id}.json`);
  //
  //   // Endow contract
  //   // let endowment = utils.parseEther('8');
  //   // let endowTx = await bobSigner.sendTransaction(
  //   //     {
  //   //       to: precompileCaller.address,
  //   //       value: endowment,
  //   //       gasLimit: 500000,
  //   //     }
  //   // );
  //   // await endowTx.wait();
  //   // expect(await jsonProvider.getBalance(precompileCaller.address)).to.be.equal(endowment);
  //   // console.log(`contract balance = ${await jsonProvider.getBalance(precompileCaller.address)}`);
  //   await new Promise(r => setTimeout(r, 2000));
  //
  //   console.log("TESTING TRANSFER FROM");
  //   await new Promise(r => setTimeout(r, 6000));
  //
  //   // TOTAL FEE: 5377111711661793536
  //   // ACTUAL FE: 2690534859080896768
  //   // BALANCE:   999913344500000000000000
  //   // FEE: 79164089
  //   expect(
  //       await precompileCaller.connect(bobSigner).transferFromProxy(bobSigner.address, receiverAddress, token_id)
  //   ).to.emit(bobContract, 'Transfer').withArgs(bobSigner.address, receiverAddress, token_id);
  //
  //   // // Bob approves contract for token_id
  //   // expect(
  //   //     await bobContract.approve(precompileCaller.address, token_id)
  //   // ).to.emit(bobContract, 'Approval').withArgs(bobSigner.address, precompileCaller.address, token_id);
  //   // await new Promise(r => setTimeout(r, 6000));
  //   //
  //   // console.log("Approved")
  //   //
  //   // // getApproved should be contract
  //   // expect(
  //   //     await bobContract.getApproved(token_id)
  //   // ).to.equal(precompileCaller.address);
  //   // console.log(`Approved account: ${await bobContract.getApproved(token_id)}`)
  //   // console.log(`Expected account: ${precompileCaller.address}`)
  //   // console.log("Transferring");
  //   //
  //   // // Contract transfers token_id (Owned by Bob)
  //   // await new Promise(r => setTimeout(r, 6000));
  //   //
  //   // expect(
  //   //     await precompileCaller.transferFromProxy(bobSigner.address, receiverAddress, token_id)
  //   // ).to.emit(bobContract, 'Transfer').withArgs(bobSigner.address, receiverAddress, token_id);
  //
  //   await new Promise(r => setTimeout(r, 6000));
  //   console.log("Transferred");
  //   // Receiver_address now owner of token_id
  //   expect(
  //       await bobContract.ownerOf(token_id)
  //   ).to.equal(receiverAddress);
  // }).timeout(21000000000000);

});
