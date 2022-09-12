import { loadFixture } from "@nomicfoundation/hardhat-network-helpers";
import {expect} from "chai";
import { Contract, Wallet, utils } from 'ethers';
import web3 from 'web3';

import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { hexToU8a } from '@polkadot/util';
import { KeyringPair } from "@polkadot/keyring/types";
import { JsonRpcProvider } from "@ethersproject/providers";

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

const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

const assetIdToERC20ContractAddress = (assetId: string | Number): string => {
  const asset_id_hex = (+assetId).toString(16).padStart(8, '0');
  return web3.utils.toChecksumAddress(`0xCCCCCCCC${asset_id_hex}000000000000000000000000`);
}

const nextAssetIdToAssetUUID = (assetId: string | Number): Number => {
  const assetIdBin = (+assetId).toString(2).padStart(22, '0')
  const parachainIdBin = (100).toString(2).padStart(10, '0')
  return parseInt(assetIdBin + parachainIdBin, 2);
}

describe("ERC20 Precompile", function () {
  const ALICE_PRIVATE_KEY = '0xcb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854';
  const BOB_PRIVATE_KEY = '0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf';

  let api: ApiPromise;
  let alice: KeyringPair;
  let bob: KeyringPair;
  let aliceSigner: Wallet;
  let xrpToken: Contract;
  let feeToken: Contract;

  async function setup() {
    console.log('running setup...');

    const xrpTokenId = +api.consts.assetsExt.nativeAssetId.toString();
    const feeTokenAssetId = 1124;

    await api.tx.assetsExt.createAsset().signAndSend(alice);

    await sleep(4000);
    console.log('created asset: 1124');

    // mint some tokens
    await api.tx.assets.mint(feeTokenAssetId, alice.address, 2_000_000_000_000_000).signAndSend(alice);

    await sleep(4000);
    console.log('minted asset: 1124');

    // add liquidity for XRP<->token
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
    console.log(`added liquidity for assets: ${xrpTokenId}<->${feeTokenAssetId}`);

    // batch all transactions
    // console.log(api.tx)

    // await api.tx.utility.batchAll([
    //   api.tx.assetsExt.createAsset(),
    //   api.tx.assets.mint(feeTokenAssetId, alice.address, 2_000_000_000),
    //   api.tx.dex.addLiquidity(
    //     feeTokenAssetId,
    //     xrpTokenId,
    //     1_000_000_000,
    //     1_000_000_000,
    //     1_000_000_000,
    //     1_000_000_000,
    //     0,
    //   ),
    // ]).signAndSend(alice);
  }

  // Setup api instance and keyring wallet addresses for alice and bob
  beforeEach(async () => {
    // Setup providers for jsonRPCs and WS
    const jsonProvider = new JsonRpcProvider(`http://localhost:9933`);
   
    // simple RPC logger to log requests and responses (once)
    const rpcLogger: { [key: string]: { [key: string]: any } } = {};
    jsonProvider.on('debug', (info: any) => {
      if (info.action === 'request') {
        rpcLogger[info.request.id] = { request: info.request };
      }
      if (info.action === 'response') {
        rpcLogger[info.request.id] = { ...rpcLogger[info.request.id], response: info.response};
        console.log(rpcLogger[info.request.id]);
      }
    });

    const wsProvider = new WsProvider(`ws://localhost:9944`);

    const keyring = new Keyring({ type: 'ethereum' });
    alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));

    aliceSigner = new Wallet(ALICE_PRIVATE_KEY).connect(jsonProvider); // 'development' seed

    const xrpTokenId = 2;
    const xrpTokenAddress = assetIdToERC20ContractAddress(xrpTokenId);
    xrpToken = new Contract(xrpTokenAddress, ERC20_ABI, aliceSigner);

    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    console.log('connected to api.')
    
    // const feeTokenAssetId = nextAssetIdToAssetUUID((await api.query.assetsExt.nextAssetId()).toString());
    const feeTokenAssetId = 1124;

    feeToken = new Contract(assetIdToERC20ContractAddress(feeTokenAssetId), ERC20_ABI, aliceSigner);
  });

  it('Pays fees in non-native token', async () => {
    await loadFixture(setup);

    // get token balances
    const [xrpBalance, tokenBalance] = await Promise.all([
      xrpToken.balanceOf(alice.address),
      feeToken.balanceOf(alice.address),
    ]);
    console.table({
      xrpBalance: { ...xrpBalance, num: xrpBalance.toString()},
      tokenBalance: { ...tokenBalance, num: tokenBalance.toString()},
    });

    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    let iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 10_000_000_000;
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, aliceSigner);

    // call `callWithFeePreferences` on fee proxy
    const tx = await feeProxy.connect(aliceSigner)
      .callWithFeePreferences(feeToken.address, maxFeePaymentInToken, feeToken.address, transferInput, {
        gasLimit: 150_000_000,
      });
    // console.log(tx);
    console.table({
      maxPriorityFeePerGas: { ...tx.maxPriorityFeePerGas, value: tx.maxPriorityFeePerGas.toNumber() },
      maxFeePerGas: { ...tx.maxFeePerGas, value: tx.maxFeePerGas.toNumber() },
      gasPrice: { ...tx.gasPrice },
      gasLimit: { ...tx.gasLimit, value: tx.gasLimit.toNumber() },
    });

    await tx.wait();
    
    // check updated balances
    const [xrpBalanceUpdated, tokenBalanceUpdated] = await Promise.all([
      xrpToken.balanceOf(alice.address),
      feeToken.balanceOf(alice.address),
    ]);
    console.table({
      xrpBalanceUpdated: { ...xrpBalanceUpdated, num: xrpBalanceUpdated.toString() },
      tokenBalanceUpdated: { ...tokenBalanceUpdated, num: tokenBalanceUpdated.toString() },
    });
    
    console.log(`XRP balance difference: ${xrpBalanceUpdated.sub(xrpBalance).toString()}`);
    console.log(`Token balance difference: ${tokenBalanceUpdated.sub(tokenBalance).toString()}`);
    
    // verify XRP balance has not changed (payment made in non-native token)
    expect(xrpBalanceUpdated.sub(xrpBalance).toString()).to.equal('0');
  });
});
