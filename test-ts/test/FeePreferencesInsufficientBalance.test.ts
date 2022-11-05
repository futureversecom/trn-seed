// Call an EVM transaction with fee preferences for an account that has zero native token balance, ensuring that the preferred asset with liquidity is spent instead

import { loadFixture } from "@nomicfoundation/hardhat-network-helpers";
import {expect} from "chai";
import { Contract, Wallet, utils } from 'ethers';
import web3 from 'web3';

import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { hexToU8a } from '@polkadot/util';
import { KeyringPair } from "@polkadot/keyring/types";
import { JsonRpcProvider } from "@ethersproject/providers";

import { executeForPreviousEvent } from '../util/index'

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

const FEE_PROXY_ADDRESS = '0x00000000000000000000000000000000000004bb';

const FEE_PROXY_ABI = [
  'function callWithFeePreferences(address asset, uint128 maxPayment, address target, bytes input)',
];

const ERC20_ABI = [
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

const NATIVE_TOKEN_ID = 1;

const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

const assetIdToERC20ContractAddress = (assetId: string | Number): string => {
  const asset_id_hex = (+assetId).toString(16).padStart(8, '0');
  return web3.utils.toChecksumAddress(`0xCCCCCCCC${asset_id_hex}000000000000000000000000`);
}

describe("Fee Preferences", function () {
  const ALICE_PRIVATE_KEY = '0xcb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854';
  const BOB_PRIVATE_KEY = '0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf';
  const EMPTY_ACCT_PRIVATE_KEY = '0xf8d74108dbe199c4a6e4ef457046db37c325ba3f709b14cabfa1885663e4c589';

  let bob: KeyringPair;
  let insufficientAccount: KeyringPair;
  let insufficientAccountSigner: Wallet;
  let xrpToken: Contract;
  let feeToken: Contract;
  let api: ApiPromise;

  async function setup() {
    const wsProvider = new WsProvider(`ws://localhost:9944`);  
    const keyring = new Keyring({ type: 'ethereum' });
    const alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));
  
    // Empty with regards to native balance only
    const insufficientAccount = keyring.addFromSeed(hexToU8a(EMPTY_ACCT_PRIVATE_KEY));
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    await api.tx.assetsExt.createAsset().signAndSend(alice);
  
    await sleep(4000);
  
    // mint some tokens
    const feeTokenAssetId = 1124;
    await api.tx.assets.mint(feeTokenAssetId, alice.address, 2_000_000_000_000_000).signAndSend(alice);
    await sleep(7000);
    // Give the account a low amount of tokens
    await api.tx.assets.mint(feeTokenAssetId, insufficientAccount.address, 2_000).signAndSend(alice);
  
    await sleep(4000);
  
    // add liquidity for XRP<->token
    const xrpTokenId = 2;
    await api.tx.dex.addLiquidity(
      feeTokenAssetId,
      xrpTokenId,
        100_000_000_000,
        100_000_000_000,
        100_000_000_000,
        100_000_000_000,
      0,
    ).signAndSend(alice);
    
    await sleep(4000);
  }

  // Setup api instance and keyring wallet addresses for alice and bob
  beforeEach(async () => {
    // Setup providers for jsonRPCs and WS
    const jsonProvider = new JsonRpcProvider(`http://localhost:9933`);
    const keyring = new Keyring({ type: 'ethereum' });
    // alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));
    insufficientAccount = keyring.addFromSeed(hexToU8a(EMPTY_ACCT_PRIVATE_KEY));

    insufficientAccountSigner = new Wallet(EMPTY_ACCT_PRIVATE_KEY).connect(jsonProvider); // 'development' seed

    const xrpTokenAddress = assetIdToERC20ContractAddress(NATIVE_TOKEN_ID);
    xrpToken = new Contract(xrpTokenAddress, ERC20_ABI, insufficientAccountSigner);    
    const feeTokenAssetId = 1124;
    feeToken = new Contract(assetIdToERC20ContractAddress(feeTokenAssetId), ERC20_ABI, insufficientAccountSigner);
  });

  it('Cannot pay fees with non-native, preferred token if low asset balance', async () => {
    await loadFixture(setup);

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpToken.balanceOf(insufficientAccount.address),
      feeToken.balanceOf(insufficientAccount.address),
    ]);

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    let iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 10_000_000_000;
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, insufficientAccountSigner);
    const nonce = await insufficientAccountSigner.getTransactionCount();
    const chainId = 3999;
    const maxPriorityFeePerGas = 0; // 1_500_000_000 = '0x59682f00'
    const gasLimit = 23316; // '0x5b14' = 23316;
    const maxFeePerGas = 30_001_500_000_0000; // 30_001_500_000_000 = '0x1b4944c00f00'  
    const unsignedTx = { // eip1559 tx
      type: 2,
      from: insufficientAccount.address,
      to: FEE_PROXY_ADDRESS,
      nonce,
      data: feeProxy.interface.encodeFunctionData("callWithFeePreferences", [
        feeToken.address,
        maxFeePaymentInToken,
        feeToken.address,
        transferInput,
      ]),
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      chainId,
    };
    
    await insufficientAccountSigner.signTransaction(unsignedTx);
    const tx = await insufficientAccountSigner.sendTransaction(unsignedTx);
    console.log('waiting for tx rejection')
    await sleep(7000);
    let didContainError = false;
    // Expect system.ExtrinsicFailed to signal ModuleError of evm pallet
    await executeForPreviousEvent(api, { method: 'ExtrinsicFailed' }, 2, async (eventData) => {
      if (eventData.event.data.dispatchError) {
        didContainError = true;
        // Expect error is emitted from EVM pallet, which is currently 27
        expect(eventData.event.data.dispatchError.index).to.equal('27')
        // Expect WithdrawFailed error at index 0x03000000(third error of EVM pallet)
        expect(eventData.event.data.dispatchError.error).to.equal('0x03000000')
      }
    });

    // The error should have been received in recent history
    expect(didContainError).to.be.true
  });
});
