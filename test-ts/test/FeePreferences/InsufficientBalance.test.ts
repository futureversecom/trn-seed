// Call an EVM transaction with fee preferences for an account that has zero native token balance, ensuring that the preferred asset with liquidity is spent instead

import {expect} from "chai";
import { Contract, Wallet, utils } from 'ethers';

import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { hexToU8a } from '@polkadot/util';
import { KeyringPair } from "@polkadot/keyring/types";
import { JsonRpcProvider } from "@ethersproject/providers";

import { executeForPreviousEvent, typedefs, sleep, assetIdToERC20ContractAddress, NATIVE_TOKEN_ID, ERC20_ABI, FEE_PROXY_ABI, FEE_PROXY_ADDRESS } from '../../util/index'

describe("Fee Preferences in low asset balance scenario", function () {
  const ALICE_PRIVATE_KEY = '0xcb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854';
  const BOB_PRIVATE_KEY = '0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf';
  const EMPTY_ACCT_PRIVATE_KEY = '0xf8d74108dbe199c4a6e4ef457046db37c325ba3f709b14cabfa1885663e4c589';

  const feeTokenAssetId = 1124;
  const xrpTokenId = 2;

  let bob: KeyringPair;
  let insufficientAccount: KeyringPair;
  let insufficientAccountSigner: Wallet;
  let xrpToken: Contract;
  let feeToken: Contract;
  let api: ApiPromise;

  // Setup api instance and keyring wallet addresses for alice and bob
  before(async () => {
    // Setup providers for jsonRPCs and WS
    const jsonProvider = new JsonRpcProvider(`http://localhost:9933`);
    const wsProvider = new WsProvider(`ws://localhost:9944`);  
    const keyring = new Keyring({ type: 'ethereum' });
    // alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));
    insufficientAccount = keyring.addFromSeed(hexToU8a(EMPTY_ACCT_PRIVATE_KEY));
    insufficientAccountSigner = new Wallet(EMPTY_ACCT_PRIVATE_KEY).connect(jsonProvider); // 'development' seed

    const xrpTokenAddress = assetIdToERC20ContractAddress(NATIVE_TOKEN_ID);
    xrpToken = new Contract(xrpTokenAddress, ERC20_ABI, insufficientAccountSigner);    
    feeToken = new Contract(assetIdToERC20ContractAddress(feeTokenAssetId), ERC20_ABI, insufficientAccountSigner);

    const alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));
  
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
    
    const txes = [
      api.tx.assetsExt.createAsset(),
      api.tx.assets.mint(feeTokenAssetId, alice.address, 2_000_000_000_000_000),
      api.tx.assets.mint(feeTokenAssetId, insufficientAccount.address, 2_000),
      api.tx.dex.addLiquidity(
        feeTokenAssetId,
        xrpTokenId,
          100_000_000_000,
          100_000_000_000,
          100_000_000_000,
          100_000_000_000,
        0,
      )
    ];

    await new Promise<void>((resolve) => {
      api.tx.utility
        .batch(txes)
        .signAndSend(alice, ({ status }) => {
          if (status.isInBlock) {
            console.log(`setup block hash: ${status.asInBlock}`);
            resolve();
          }
        })
      });
  });

  it('Cannot pay fees with non-native, preferred token if low asset balance', async () => {
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
    await executeForPreviousEvent(api, { method: 'ExtrinsicFailed', section: 'system' }, 2, async (event) => {
      if ('dispatchError' in event.data) {
        didContainError = true;
        // Expect error is emitted from EVM pallet, which is currently 27
        expect(event.data.dispatchError.index).to.equal('27')
        // Expect WithdrawFailed error at index 0x03000000(third error of EVM pallet)
        expect(event.data.dispatchError.error).to.equal('0x03000000')
      }
    });

    // The error should have been received in recent history
    expect(didContainError).to.be.true
  });
});
