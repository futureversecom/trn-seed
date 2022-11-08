import {expect} from "chai";
import { Contract, Wallet, utils } from 'ethers';

import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { hexToU8a } from '@polkadot/util';
import { KeyringPair } from "@polkadot/keyring/types";
import { JsonRpcProvider } from "@ethersproject/providers";

import { executeForPreviousEvent, typedefs, sleep, ERC20_ABI, FEE_PROXY_ABI, FEE_PROXY_ADDRESS, GAS_TOKEN_ID, ALICE_PRIVATE_KEY, BOB_PRIVATE_KEY } from '../../utils';

const feeTokenAssetId = 1124;
const EMPTY_ACCT_PRIVATE_KEY = '0xf8d74108dbe199c4a6e4ef457046db37c325ba3f709b14cabfa1885663e4c589';

describe("Fee Preferences under low token pair liquidity", function () {

  let api: ApiPromise;
  let bob: KeyringPair;
  let emptyAccountSigner: Wallet;
  let feeToken: Contract;

  before(async () => {
    // Setup providers for jsonRPCs and WS
    const jsonProvider = new JsonRpcProvider(`http://localhost:9933`);
    const wsProvider = new WsProvider(`ws://localhost:9944`);  

    api = await ApiPromise.create({ provider: wsProvider, types: typedefs });

    const keyring = new Keyring({ type: 'ethereum' });
    bob = keyring.addFromSeed(hexToU8a(BOB_PRIVATE_KEY));
    const alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));
    const emptyAcct = keyring.addFromSeed(hexToU8a(EMPTY_ACCT_PRIVATE_KEY));
    emptyAccountSigner = new Wallet(EMPTY_ACCT_PRIVATE_KEY).connect(jsonProvider); // 'development' seed

    const txes = [
      api.tx.assetsExt.createAsset(),
      api.tx.assets.mint(feeTokenAssetId, alice.address, 2_000_000_000_000_000),
      api.tx.assets.mint(feeTokenAssetId, emptyAcct.address, 2_000_000_000_000_000),
      api.tx.dex.addLiquidity(
        feeTokenAssetId,
        GAS_TOKEN_ID,
          100_000,
          100_000,
          100_000,
          100_000,
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

  it('Fails to pay fees in non-native token if insufficient liquidity', async () => {
    // call `transfer` on erc20 token - via `callWithFeePreferences` precompile function
    const transferAmount = 1;
    let iface = new utils.Interface(ERC20_ABI);
    const transferInput = iface.encodeFunctionData("transfer", [bob.address, transferAmount]);

    const maxFeePaymentInToken = 10_000_000_000;
    const feeProxy = new Contract(FEE_PROXY_ADDRESS, FEE_PROXY_ABI, emptyAccountSigner);
    const nonce = await emptyAccountSigner.getTransactionCount();
    const chainId = 3999;
    const maxPriorityFeePerGas = 0; // 1_500_000_000 = '0x59682f00'
    const gasLimit = 23316; // '0x5b14' = 23316;
    const maxFeePerGas = 30_001_500_000_0000; // 30_001_500_000_000 = '0x1b4944c00f00'  
    const unsignedTx = { // eip1559 tx
      type: 2,
      from: emptyAccountSigner.address,
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
    
    await emptyAccountSigner.signTransaction(unsignedTx);
    await emptyAccountSigner.sendTransaction(unsignedTx);
    console.log('waiting for tx rejection')
    await sleep(4000);

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
