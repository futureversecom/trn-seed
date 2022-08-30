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
import PrecompileCaller from '../artifacts/contracts/PrecompileCaller.sol/PrecompileCaller.json';

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

describe("ERC20 Precompile", function () {
  let seedSigner: Wallet;
  let xrpToken: Contract;
  let precompileCaller: Contract;
  let jsonProvider: Provider;
  const xrpTokenAddress = web3.utils.toChecksumAddress('0xCCCCCCCC00000002000000000000000000000000');
  const erc20Abi = [
    'event Transfer(address indexed from, address indexed to, uint256 value)',
    'event Approval(address indexed owner, address indexed spender, uint256 value)',
    'function approve(address spender, uint256 amount) public returns (bool)',
    'function allowance(address owner, address spender) public view returns (uint256)',
    'function balanceOf(address who) public view returns (uint256)',
    'function name() public view returns (string memory)',
    'function symbol() public view returns (string memory)',
    'function decimals() public view returns (uint8)',
    'function transfer(address who, uint256 amount)',
  ];
  // Setup api instance
  before(async () => {
    // Setup providers for jsonRPCs and WS
    jsonProvider = new JsonRpcProvider(`http://localhost:9933`);
    const wsProvider = new WsProvider(`ws://localhost:9944`);

    seedSigner = new Wallet('0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf').connect(jsonProvider); // 'development' seed
    xrpToken = new Contract(xrpTokenAddress, erc20Abi, seedSigner);
    console.log(`signer address: ${seedSigner.address}`);

    let factory = new ContractFactory(PrecompileCaller.abi, PrecompileCaller.bytecode, seedSigner);
    precompileCaller = await factory.deploy();
    console.log(`contract address=${precompileCaller.address}`);
  });

  it('name, symbol, decimals', async () => {
    expect(
        await xrpToken.decimals()
    ).to.equal(6);

    expect(
        await xrpToken.name()
    ).to.equal("XRP");

    expect(
        await xrpToken.symbol()
    ).to.equal("XRP");
  });

  it('XRP transfer, balanceOf', async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const transferAmount = 12345;
    await expect(
        xrpToken.transfer(receiverAddress, transferAmount)
    ).to.emit(xrpToken, 'Transfer').withArgs(seedSigner.address, receiverAddress, transferAmount);

    expect(await xrpToken.balanceOf(receiverAddress)).to.be.equal(transferAmount);
  }).timeout(15000);

  it('XRP transfer amounts via EVM', async () => {
    // fund the contract with some XRP
    let endowment = utils.parseEther('8');
    let endowTx = await seedSigner.sendTransaction(
        {
          to: precompileCaller.address,
          value: endowment,
          gasLimit: 500000,
        }
    );
    await endowTx.wait();
    expect(await jsonProvider.getBalance(precompileCaller.address)).to.be.equal(endowment);
    console.log('endowed 8 XRP');

    const receiverAddress = await Wallet.createRandom().getAddress();
    let tx = await precompileCaller.sendXRPAmounts(receiverAddress);
    await tx.wait();
  }).timeout(12000);

  it('approve and transferFrom', async () => {
    let approvedAmount = 12345;
    await expect(
        xrpToken.approve(precompileCaller.address, approvedAmount)
    ).to.emit(xrpToken, 'Approval').withArgs(seedSigner.address, precompileCaller.address, approvedAmount);

    expect(
        await xrpToken.allowance(seedSigner.address, precompileCaller.address)
    ).to.equal(approvedAmount);

    await expect(
        precompileCaller.takeXRP(approvedAmount)
    ).to.emit(xrpToken, 'Transfer').withArgs(seedSigner.address, precompileCaller.address, approvedAmount);

  }).timeout(15000);

  it('XRP transfer amounts via transaction', async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    // pairs of (input amount, actual transferred amount)
    // shows the behaviour of the xrp scaling rules
    let payments = [
      [utils.parseEther('1.000000'), utils.parseEther('1.000000')],
      // transfer smallest unit of xrp
      [utils.parseEther('0.000001'), utils.parseEther('0.000001')],
      // transfer 1.234567 xrp
      [utils.parseEther('1.234567'), utils.parseEther('1.234567')],
      // transfer < the smallest unit of xrp 0.000001, rounds up
      [utils.parseEther('0.00000099'), utils.parseEther('0.000001')],
      // transfer amounts with some part < the smallest unit of xrp
      [utils.parseEther('1.0000005'), utils.parseEther('1.000001')],
      [utils.parseEther('1.00000050000001'), utils.parseEther('1.000001')],
      [utils.parseEther('1.0000009999'), utils.parseEther('1.000001')],
    ];
    let total: BigNumber = BigNumber.from(0);

    for (const [payment, expected] of payments) {
      console.log(`Sending tx with balance: ${payment}`);
      let tx = await seedSigner.sendTransaction(
          {
            to: receiverAddress,
            value: payment,
          }
      );
      await tx.wait();
      let balance = await jsonProvider.getBalance(receiverAddress);
      total = total.add(expected);
      console.log(`input:       ${payment.toString()}\nreal:        ${expected.toString()}\nnew expected:${total.toString()}\nnew balance: ${balance}\n`);
      expect(balance).to.be.equal(total.toString());

      // sleep, prevents nonce issues
      await new Promise(r => setTimeout(r, 500));
    }
  }).timeout(60 * 1000);
});
