import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a, u8aToHex } from "@polkadot/util";
import { expect } from "chai";
import { blake256 } from "codechain-primitives";
import { Wallet, utils as ethersUtils } from "ethers";
import { computePublicKey, keccak256 } from "ethers/lib/utils";
import { encode, encodeForSigning } from "ripple-binary-codec";
import { deriveAddress, sign } from "ripple-keypairs";
import Web3 from "web3";
import * as AccountLib from "xrpl-accountlib";

import {
  ALICE_PRIVATE_KEY,
  ALITH_PRIVATE_KEY,
  GAS_TOKEN_ID,
  NodeProcess,
  finalizeTx,
  getNextAssetId,
  getPrefixLength,
  rpcs,
  startNode,
  stringToHex,
  typedefs,
} from "../../common";

const SPENDER_TYPE = {
  Grantor: "GRANTOR",
  Grantee: "GRANTEE",
};

describe("Sylo Actions", () => {
  let node: NodeProcess;
  let api: ApiPromise;
  let alith: KeyringPair;
  let alice: KeyringPair;
  let keyring: Keyring;
  let genesisHash: string;
  let feeTokenAssetId: number;

  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs, rpc: rpcs });
    genesisHash = api.genesisHash.toHex().slice(2);

    keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));

    feeTokenAssetId = await getNextAssetId(api);

    // add liquidity for XRP/SYLO token and set up user funds
    const txs = [
      api.tx.assetsExt.createAsset("sylo", "SYLO", 18, 1, alith.address),
      api.tx.assets.mint(feeTokenAssetId, alith.address, 2_000_000_000_000_000),
      api.tx.assets.mint(feeTokenAssetId, alice.address, 2_000_000_000_000_000),
      api.tx.dex.addLiquidity(
        feeTokenAssetId,
        GAS_TOKEN_ID,
        100_000_000_000,
        100_000_000_000,
        100_000_000_000,
        100_000_000_000,
        null,
        null,
      ),
    ];
    await finalizeTx(alith, api.tx.utility.batch(txs));

    // set payment asset
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.syloDataVerification.setPaymentAsset(feeTokenAssetId)));
  });

  after(async () => node.stop());

  it("can grant transact permission and transact", async () => {
    const grantor = alith;
    const grantee = alice;

    const allowedCalls = [["*", "*"]];

    await finalizeTx(
      grantor,
      api.tx.syloActionPermissions.grantTransactPermission(
        grantee.address,
        SPENDER_TYPE.Grantee,
        null,
        allowedCalls,
        null,
      ),
    );

    // Verify the permission was granted
    const permissionRecord = await api.query.syloActionPermissions.transactPermissions(
      grantor.address,
      grantee.address,
    );
    expect((permissionRecord as any).isSome).to.be.true;

    // test some calls
    const calls = [
      api.tx.system.remark("Hello, Sylo!"),
      api.tx.assetsExt.transfer(feeTokenAssetId, alith.address, 100, true),
      api.tx.dex.addLiquidity(feeTokenAssetId, GAS_TOKEN_ID, 100, 100, 100, 100, null, null),
    ];

    for (const call of calls) {
      await finalizeTx(grantee, api.tx.syloActionPermissions.transact(grantor.address, call));
    }

    // test using batch call
    await finalizeTx(grantee, api.tx.syloActionPermissions.transact(grantor.address, api.tx.utility.batch(calls)));
  });

  it("can grant with futurepass and transact", async () => {
    const grantor = alith;

    // Create a futurepass for the grantor
    await finalizeTx(alith, api.tx.futurepass.create(grantor.address));

    // Get the futurepass address
    const futurepassAddress = (await api.query.futurepass.holders(grantor.address)).toString();
    await finalizeTx(alith, api.tx.assets.transfer(feeTokenAssetId, futurepassAddress, 100_000_000_000));

    const grantee = alice;

    const allowedCalls = [["*", "*"]];

    // grant using futurepass
    await finalizeTx(
      grantor,
      api.tx.futurepass.proxyExtrinsic(
        futurepassAddress,
        api.tx.syloActionPermissions.grantTransactPermission(
          grantee.address,
          SPENDER_TYPE.Grantee,
          null, // spending_balance
          allowedCalls,
          null, // expiry
        ),
      ),
    );

    // Verify the permission was granted by the futurepass
    const permissionRecord = await api.query.syloActionPermissions.transactPermissions(
      futurepassAddress,
      grantee.address,
    );
    expect((permissionRecord as any).isSome).to.be.true;

    // test transact
    await finalizeTx(
      grantee,
      api.tx.syloActionPermissions.transact(futurepassAddress, api.tx.system.remark("Hello, Sylo!")),
    );
  });

  it("can grant to futurepass and transact", async () => {
    const grantor = alith;
    const grantee = alice;

    // Create a futurepass for the grantee
    await finalizeTx(alith, api.tx.futurepass.create(grantee.address));

    // Get the futurepass address
    const futurepassAddress = (await api.query.futurepass.holders(grantee.address)).toString();
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, futurepassAddress, 100_000_000_000));

    const allowedCalls = [["*", "*"]];

    // grant to futurepass
    await finalizeTx(
      grantor,
      api.tx.syloActionPermissions.grantTransactPermission(
        futurepassAddress,
        SPENDER_TYPE.Grantee,
        null, // spending_balance
        allowedCalls,
        null, // expiry
      ),
    );

    // test transact using futurepass proxy_extrinsic
    await finalizeTx(
      grantee,
      api.tx.futurepass.proxyExtrinsic(
        futurepassAddress,
        api.tx.syloActionPermissions.transact(grantor.address, api.tx.system.remark("Hello, Sylo!")),
      ),
    );
  });

  it("can transact with fee proxy", async () => {
    const grantor = alith;
    const grantee = alice;

    await finalizeTx(
      grantor,
      api.tx.syloActionPermissions.grantTransactPermission(
        grantee.address,
        SPENDER_TYPE.Grantee,
        null, // spending_balance
        [["*", "*"]],
        null, // expiry
      ),
    );

    const tokenBalanceBefore =
      ((await api.query.assets.account(feeTokenAssetId, grantee.address)).toJSON() as any)?.balance ?? 0;

    await finalizeTx(
      grantee,
      api.tx.feeProxy.callWithFeePreferences(
        feeTokenAssetId,
        100_000_000_000,
        api.tx.syloActionPermissions.transact(grantor.address, api.tx.system.remark("Hello, Sylo!")),
      ),
    );

    const tokenBalanceAfter =
      ((await api.query.assets.account(feeTokenAssetId, grantee.address)).toJSON() as any)?.balance ?? 0;

    expect(tokenBalanceAfter).to.be.lessThan(tokenBalanceBefore);
  });

  it("can transact with fee proxy and futurepass", async () => {
    const grantor = alith;
    const grantee = alice;

    // fund the futurepass address
    const futurepassAddress = (await api.query.futurepass.holders(grantee.address)).toString();
    await finalizeTx(alith, api.tx.assets.transfer(feeTokenAssetId, futurepassAddress, 100_000_000));

    const tokenBalanceBefore =
      ((await api.query.assets.account(feeTokenAssetId, futurepassAddress)).toJSON() as any)?.balance ?? 0;

    await finalizeTx(
      grantee,
      api.tx.feeProxy.callWithFeePreferences(
        feeTokenAssetId,
        100_000_000_000,
        api.tx.futurepass.proxyExtrinsic(
          futurepassAddress,
          api.tx.syloActionPermissions.transact(grantor.address, api.tx.system.remark("Hello, Sylo!")),
        ),
      ),
    );

    const tokenBalanceAfter =
      ((await api.query.assets.account(feeTokenAssetId, futurepassAddress)).toJSON() as any)?.balance ?? 0;

    expect(tokenBalanceAfter).to.be.lessThan(tokenBalanceBefore);
  });

  it("can transact using xrpl", async () => {
    const user = Wallet.createRandom();
    const publicKey = computePublicKey(user.publicKey, true);

    // fund the user account to pay for tx fees
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 100_000_000_000));

    // grant transact permission to xrpl account
    await finalizeTx(
      alith,
      api.tx.syloActionPermissions.grantTransactPermission(
        user.address,
        SPENDER_TYPE.Grantee,
        null,
        [["*", "*"]],
        null,
      ),
    );

    const call = api.tx.syloActionPermissions.transact(alith.address, api.tx.system.remark("Hello, Sylo!"));

    const hashedExtrinsicWithoutPrefix = blake256(call.toHex().slice(getPrefixLength(call))).toString();
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xamanJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: publicKey.slice(2),
      Account: deriveAddress(publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`${genesisHash}:0:${maxBlockNumber}:0:${hashedExtrinsicWithoutPrefix}`),
          },
        },
      ],
    };

    const message = encode(xamanJsonTx);
    const encodedSigningMessage = encodeForSigning(xamanJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    await new Promise<any[]>(async (resolve, reject) => {
      await api.tx.xrpl
        .transact(`0x${message}`, `0x${signature}`, call)
        .send(({ events = [], status }) => {
          if (status.isInBlock) resolve(events);
        })
        .catch(reject);
    });
  });

  it("can transact using xrpl and futurepass", async () => {
    const user = Wallet.createRandom();
    const publicKey = computePublicKey(user.publicKey, true);

    // Create a futurepass for the user
    await finalizeTx(alith, api.tx.futurepass.create(user.address));

    // Get the futurepass address
    const futurepassAddress = (await api.query.futurepass.holders(user.address)).toString();
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, futurepassAddress, 100_000_000_000));

    // grant transact permission to futurepass account
    await finalizeTx(
      alith,
      api.tx.syloActionPermissions.grantTransactPermission(
        futurepassAddress,
        SPENDER_TYPE.Grantee,
        null,
        [["*", "*"]],
        null,
      ),
    );

    // remark call that is wrapped in transact and proxy_extrinsic
    const call = api.tx.futurepass.proxyExtrinsic(
      futurepassAddress,
      api.tx.syloActionPermissions.transact(alith.address, api.tx.system.remark("Hello, Sylo!")),
    );

    const hashedExtrinsicWithoutPrefix = blake256(call.toHex().slice(getPrefixLength(call))).toString();
    const maxBlockNumber = +(await api.query.system.number()).toString() + 5;

    const xamanJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: publicKey.slice(2),
      Account: deriveAddress(publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            // remove `0x` from extrinsic hex string
            MemoData: stringToHex(`${genesisHash}:0:${maxBlockNumber}:0:${hashedExtrinsicWithoutPrefix}`),
          },
        },
      ],
    };

    const message = encode(xamanJsonTx);
    const encodedSigningMessage = encodeForSigning(xamanJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    await new Promise<any[]>(async (resolve, reject) => {
      await api.tx.xrpl
        .transact(`0x${message}`, `0x${signature}`, call)
        .send(({ events = [], status }) => {
          if (status.isInBlock) resolve(events);
        })
        .catch(reject);
    });
  });

  it("can transact with spender as grantor", async () => {
    const grantor = alith;
    const grantee = alice;

    const spendingBalance = 100_000_000_000;

    // update permission to use grantor as spender
    await finalizeTx(
      grantor,
      api.tx.syloActionPermissions.updateTransactPermission(
        grantee.address,
        SPENDER_TYPE.Grantor,
        spendingBalance,
        null,
        null,
      ),
    );

    const grantorBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, grantor.address)).toJSON() as any)?.balance ?? 0;

    await finalizeTx(
      grantee,
      api.tx.syloActionPermissions.transact(grantor.address, api.tx.system.remark("Hello, Sylo!")),
    );

    const grantorBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, grantor.address)).toJSON() as any)?.balance ?? 0;

    const record = await api.query.syloActionPermissions.transactPermissions(grantor.address, grantee.address);

    expect(grantorBalanceAfter).to.be.lessThan(grantorBalanceBefore);
    expect((record.toJSON() as any).spendingBalance).to.be.lessThan(spendingBalance);
  });

  it("can transact using futurepass with spender as grantor", async () => {
    const grantor = alith;
    const grantee = alice;

    // Get the futurepass address
    const futurepassAddress = (await api.query.futurepass.holders(grantee.address)).toString();
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, futurepassAddress, 100_000_000_000));

    const spendingBalance = 100_000_000_000;

    // update permission to use grantor as spender
    await finalizeTx(
      grantor,
      api.tx.syloActionPermissions.updateTransactPermission(
        futurepassAddress,
        SPENDER_TYPE.Grantor,
        spendingBalance,
        null,
        null,
      ),
    );

    const grantorBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, grantor.address)).toJSON() as any)?.balance ?? 0;

    await finalizeTx(
      grantee,
      api.tx.futurepass.proxyExtrinsic(
        futurepassAddress,
        api.tx.syloActionPermissions.transact(grantor.address, api.tx.system.remark("Hello, Sylo!")),
      ),
    );

    const grantorBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, grantor.address)).toJSON() as any)?.balance ?? 0;

    const record = await api.query.syloActionPermissions.transactPermissions(grantor.address, grantee.address);

    expect(grantorBalanceAfter).to.be.lessThan(grantorBalanceBefore);
    expect((record.toJSON() as any).spendingBalance).to.be.lessThan(spendingBalance);
  });

  it("can transact with fee-proxy and spender as grantor", async () => {
    const grantor = alith;
    const grantee = alice;

    const tokenBalanceBefore =
      ((await api.query.assets.account(feeTokenAssetId, grantor.address)).toJSON() as any)?.balance ?? 0;

    await finalizeTx(
      grantee,
      api.tx.feeProxy.callWithFeePreferences(
        feeTokenAssetId,
        100_000_000_000,
        api.tx.syloActionPermissions.transact(grantor.address, api.tx.system.remark("Hello, Sylo!")),
      ),
    );

    const tokenBalanceAfter =
      ((await api.query.assets.account(feeTokenAssetId, grantor.address)).toJSON() as any)?.balance ?? 0;

    expect(tokenBalanceAfter).to.be.lessThan(tokenBalanceBefore);
  });

  it("can accept transact permission using eip191", async () => {
    const grantor = Wallet.createRandom();

    const grantee = alith;

    const allowedCalls = [["system", "remark"]];

    const permissionToken = {
      grantee: grantee.address,
      futurepass: null,
      spender: SPENDER_TYPE.Grantee,
      spending_balance: null,
      allowed_calls: allowedCalls,
      expiry: null,
      nonce: u8aToHex(ethersUtils.randomBytes(32)),
    };

    // Serialize the TransactPermissionToken object
    const serializedToken = api.registry
      .createType("PalletSyloActionPermissionsTransactPermissionToken", permissionToken)
      .toU8a();

    // Sign the serialized token
    const signature = await grantor.signMessage(serializedToken);

    const eip191Sig = { EIP191: signature };

    // Submit the accept_transact_permission extrinsic
    await finalizeTx(
      grantee,
      api.tx.syloActionPermissions.acceptTransactPermission(grantor.address, permissionToken, eip191Sig),
    );

    // Verify the permission was accepted
    const permissionRecord = await api.query.syloActionPermissions.transactPermissions(
      grantor.address,
      grantee.address,
    );
    expect((permissionRecord as any).isSome).to.be.true;

    // Test calling transact permission
    const remark = "Hello, Sylo!";
    const tx = api.tx.syloActionPermissions.transact(grantor.address, api.tx.system.remark(remark));

    await finalizeTx(grantee, tx);
  });

  it("can accept transact permission using xrpl signature - ecdsa", async () => {
    const grantor = alice;
    const grantee = alith;

    const allowedCalls = [["system", "remark"]];

    const permissionToken = {
      grantee: grantee.address,
      futurepass: null,
      spender: SPENDER_TYPE.Grantee,
      spending_balance: null,
      allowed_calls: allowedCalls,
      expiry: null,
      nonce: u8aToHex(ethersUtils.randomBytes(32)),
    };

    // Serialize the TransactPermissionToken object
    const serializedToken = api.registry
      .createType("PalletSyloActionPermissionsTransactPermissionToken", permissionToken)
      .toU8a();

    // Generate XRPL transaction
    const publicKey = computePublicKey(grantor.publicKey, true);
    const xamanJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: publicKey.slice(2),
      Account: deriveAddress(publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            MemoData: u8aToHex(serializedToken).substring(2),
          },
        },
      ],
    };

    // Sign XRPL transaction
    const encodedSigningMessage = encodeForSigning(xamanJsonTx);
    const signature = sign(encodedSigningMessage, ALICE_PRIVATE_KEY.slice(2));

    const xrplSig = {
      XRPL: {
        encodedMsg: `0x${encode(xamanJsonTx)}`,
        signature: `0x${signature}`,
      },
    };

    // Submit the accept_transact_permission extrinsic
    await finalizeTx(
      grantee,
      api.tx.syloActionPermissions.acceptTransactPermission(grantor.address, permissionToken, xrplSig),
    );

    // Verify the permission was accepted
    const permissionRecord = await api.query.syloActionPermissions.transactPermissions(
      grantor.address,
      grantee.address,
    );
    expect((permissionRecord as any).isSome).to.be.true;
  });

  it("can accept transact permission using xrpl signature - ed25519", async () => {
    // create ed25519 account with eoa
    const importedAccount = AccountLib.derive.familySeed("sEdS4rAgVysUtD5Zmm9F8i8uJBGik4K");
    const signerInstance = AccountLib.derive.privatekey(importedAccount.keypair.privateKey!);
    const publicKey = computePublicKey(`0x${signerInstance.keypair.publicKey!}`, true);
    const grantor = Web3.utils.toChecksumAddress(
      // remove "ED" prefix from public key to compute EOA
      // keccak hash produces 32 bytes (64 chars) - take last 20 bytes (40 chars)
      // remove "0x" prefix from keccak hash output (2 chars)
      // get last 20 bytes of the keccak hash output (12 bytes - 24 chars)
      "0x" + keccak256(hexToU8a(`0x${publicKey.slice(4)}`)).slice(26),
    );

    const grantee = alith;

    const allowedCalls = [["system", "remark"]];

    const permissionToken = {
      grantee: grantee.address,
      futurepass: null,
      spender: SPENDER_TYPE.Grantee,
      spending_balance: null,
      allowed_calls: allowedCalls,
      expiry: null,
      nonce: u8aToHex(ethersUtils.randomBytes(32)),
    };

    // Serialize the TransactPermissionToken object
    const serializedToken = api.registry
      .createType("PalletSyloActionPermissionsTransactPermissionToken", permissionToken)
      .toU8a();

    const xamanJsonTx = {
      AccountTxnID: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580",
      SigningPubKey: publicKey.slice(2),
      Account: deriveAddress(publicKey.slice(2)),
      Memos: [
        {
          Memo: {
            MemoType: stringToHex("extrinsic"),
            MemoData: u8aToHex(serializedToken).substring(2),
          },
        },
      ],
    };

    // Sign XRPL transaction
    const encodedSigningMessage = encodeForSigning(xamanJsonTx);
    const signature = sign(encodedSigningMessage, signerInstance.keypair.privateKey);

    const xrplSig = {
      XRPL: {
        encodedMsg: `0x${encode(xamanJsonTx)}`,
        signature: `0x${signature}`,
      },
    };

    // Submit the accept_transact_permission extrinsic
    await finalizeTx(grantee, api.tx.syloActionPermissions.acceptTransactPermission(grantor, permissionToken, xrplSig));

    // Verify the permission was accepted
    const permissionRecord = await api.query.syloActionPermissions.transactPermissions(grantor, grantee.address);
    expect((permissionRecord as any).isSome).to.be.true;
  });

  it("can accept transact permission with grantor's futurepass", async () => {
    const grantor = Wallet.createRandom();

    // Create a futurepass for the grantor
    await finalizeTx(alith, api.tx.futurepass.create(grantor.address));

    // Get the futurepass address
    const futurepassAddress = (await api.query.futurepass.holders(grantor.address)).toString();

    // Define the permission token
    const allowedCalls = [["system", "remark"]];
    const permissionToken = {
      grantee: alith.address,
      futurepass: futurepassAddress,
      spender: SPENDER_TYPE.Grantee,
      spending_balance: null,
      allowed_calls: allowedCalls,
      expiry: null,
      nonce: u8aToHex(ethersUtils.randomBytes(32)),
    };

    // Serialize the TransactPermissionToken object
    const serializedToken = api.registry
      .createType("PalletSyloActionPermissionsTransactPermissionToken", permissionToken)
      .toU8a();

    // Sign the serialized token
    const signature = await grantor.signMessage(serializedToken);

    // accept the transact permission
    await finalizeTx(
      alith,
      api.tx.syloActionPermissions.acceptTransactPermission(grantor.address, permissionToken, {
        EIP191: signature,
      }),
    );

    // Verify the permission was granted by the futurepass
    const permissionRecord = await api.query.syloActionPermissions.transactPermissions(
      futurepassAddress,
      alith.address,
    );
    expect((permissionRecord as any).isSome).to.be.true;

    // Test calling transact permission on behalf of the futurepass
    const remark = "Hello, Sylo!";
    const tx = api.tx.syloActionPermissions.transact(futurepassAddress, api.tx.system.remark(remark));

    await finalizeTx(alith, tx);
  });
});
