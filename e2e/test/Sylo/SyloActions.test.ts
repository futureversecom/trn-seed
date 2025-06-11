import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a, u8aToHex } from "@polkadot/util";
import { expect } from "chai";
import { Wallet, utils as ethersUtils } from "ethers";
import { computePublicKey, keccak256 } from "ethers/lib/utils";
import { encode, encodeForSigning } from "ripple-binary-codec";
import { deriveAddress, sign } from "ripple-keypairs";
import Web3 from "web3";
import * as AccountLib from "xrpl-accountlib";

import {
  ALICE_PRIVATE_KEY,
  ALITH_PRIVATE_KEY,
  NodeProcess,
  finalizeTx,
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

  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs, rpc: rpcs });

    keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
    alice = keyring.addFromSeed(hexToU8a(ALICE_PRIVATE_KEY));
  });

  after(async () => node.stop());

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

  it.only("can accept transact permission using xrpl signature - ecdsa", async () => {
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

    console.log(alith.address, alice.address, xrplSig);

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
});
