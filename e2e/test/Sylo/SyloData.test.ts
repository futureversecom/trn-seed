import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a, u8aToHex } from "@polkadot/util";
import { blake2AsHex } from "@polkadot/util-crypto";
import { Doughnut, PayloadVersion, SignatureVersion, Topping } from "@therootnetwork/doughnut-nodejs";
import { expect } from "chai";
import { blake256 } from "codechain-primitives";
import { Wallet, utils as ethersUtils } from "ethers";
import { computePublicKey } from "ethers/lib/utils";
import { xit } from "mocha";
import { encode, encodeForSigning } from "ripple-binary-codec";
import { deriveAddress, sign } from "ripple-keypairs";

import {
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

const PROXY_TYPE = {
  Any: 1,
};

const DATA_PERMISSION = {
  VIEW: "VIEW",
  MODIFY: "MODIFY",
  DISTRIBUTE: "DISTRIBUTE",
};

const TRN_PERMISSION_DOMAIN: string = "trn";

describe("Sylo Gas Costs", () => {
  let node: NodeProcess;
  let api: ApiPromise;
  let keyring: Keyring;
  let alith: KeyringPair;
  let userPrivateKey: string;
  let user: KeyringPair;
  let provider: JsonRpcProvider;
  let genesisHash: string;
  let feeTokenAssetId: number;

  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs, rpc: rpcs });
    genesisHash = api.genesisHash.toHex().slice(2);

    provider = new JsonRpcProvider(`http://127.0.0.1:${node.rpcPort}`);

    keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));
  });

  beforeEach(async () => {
    userPrivateKey = Wallet.createRandom().privateKey;
    user = keyring.addFromSeed(hexToU8a(userPrivateKey));

    feeTokenAssetId = await getNextAssetId(api);

    // add liquidity for XRP/SYLO token and set up user funds
    const txs = [
      api.tx.assetsExt.createAsset("sylo", "SYLO", 18, 1, alith.address),
      api.tx.assets.mint(feeTokenAssetId, alith.address, 2_000_000_000_000_000),
      api.tx.assets.mint(feeTokenAssetId, user.address, 2_000_000_000_000_000),
      api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 1), // avoids xrp balance increase due to preservation rules
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

    console.log("liquidity setup complete...");
  });

  after(async () => node.stop());

  // A set of sylo extrinsics to test, where each extrinsic should be paid for
  // using sylo tokens
  const createSyloExtrinsics = (api: ApiPromise) => [
    api.tx.syloDataVerification.registerResolver("id", ["endpoint"]),
    api.tx.syloDataVerification.updateResolver("id", ["endpoint-2"]),
    api.tx.syloDataVerification.deregisterResolver("id"),
    api.tx.syloDataVerification.createValidationRecord(
      "data-id",
      [{ method: "sylo-resolver", identifier: "id" }],
      "data-type",
      ["tag"],
      "0x0000000000000000000000000000000000000000000000000000000000000000",
    ),
    api.tx.syloDataVerification.addValidationRecordEntry(
      user.address,
      "data-id",
      "0x0000000000000000000000000000000000000000000000000000000000000000",
    ),
    api.tx.syloDataVerification.updateValidationRecord(
      "data-id",
      [{ method: "sylo-resolver", identifier: "id-2" }],
      "data-type-2",
      ["tag-2"],
    ),
    api.tx.syloDataVerification.deleteValidationRecord("data-id"),

    api.tx.syloDataVerification.createValidationRecord(
      "data-id",
      [{ method: "sylo-resolver", identifier: "id" }],
      "data-type",
      ["tag"],
      "0x0000000000000000000000000000000000000000000000000000000000000000",
    ),
    api.tx.syloDataPermissions.grantDataPermissions(
      user.address,
      alith.address,
      ["data-id"],
      DATA_PERMISSION.VIEW,
      null,
      false,
    ),
    api.tx.syloDataPermissions.revokeDataPermission(user.address, 0, alith.address, "data-id"),
    api.tx.syloDataPermissions.grantTaggedPermissions(alith.address, DATA_PERMISSION.VIEW, [], null, false),
    api.tx.syloDataPermissions.revokeTaggedPermission(alith.address, 1),
    api.tx.syloDataPermissions.grantPermissionReference(alith.address, "data-id"),
    api.tx.syloDataPermissions.revokePermissionReference(alith.address),
  ];

  it("can submit sylo extrinsic and pay with sylo tokens", async () => {
    const calls = createSyloExtrinsics(api);

    for (const call of calls) {
      console.log("testing call", call.meta.name.toString());

      const userXRPBalanceBefore =
        ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
      const userSyloBalanceBefore =
        ((await api.query.assets.account(feeTokenAssetId, user.address)).toJSON() as any)?.balance ?? 0;

      await finalizeTx(user, call);

      // verify balances updated
      const userXRPBalanceAfter =
        ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
      const userSyloBalanceAfter =
        ((await api.query.assets.account(feeTokenAssetId, user.address)).toJSON() as any)?.balance ?? 0;

      expect(userXRPBalanceAfter).to.be.eq(userXRPBalanceBefore);
      expect(userSyloBalanceAfter).to.be.lessThan(userSyloBalanceBefore);
    }
  });

  it("can submit sylo extrinsic with futurepass", async () => {
    // create a random user A
    const userPrivateKey = Wallet.createRandom().privateKey;
    const user: KeyringPair = keyring.addFromSeed(hexToU8a(userPrivateKey));

    // create a futurepass for user
    await finalizeTx(alith, api.tx.futurepass.create(user.address));

    // fund the futurepass account
    const futurepassAddress = (await api.query.futurepass.holders(user.address)).toString();
    await finalizeTx(alith, api.tx.assets.transfer(feeTokenAssetId, futurepassAddress, 100_000_000)); // gas
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, futurepassAddress, 1)); // preservation rules

    const calls = createSyloExtrinsics(api);

    for (const call of calls) {
      console.log("testing call", call.meta.name.toString());

      const userXRPBalanceBefore =
        ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
      const userSyloBalanceBefore =
        ((await api.query.assets.account(feeTokenAssetId, user.address)).toJSON() as any)?.balance ?? 0;

      const fpXRPBalanceBefore =
        ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
      const fpSyloBalanceBefore =
        ((await api.query.assets.account(feeTokenAssetId, futurepassAddress)).toJSON() as any)?.balance ?? 0;

      const futurepassCall = api.tx.futurepass.proxyExtrinsic(futurepassAddress, call);

      await finalizeTx(user, futurepassCall);

      const userXRPBalanceAfter =
        ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
      const userSyloBalanceAfter =
        ((await api.query.assets.account(feeTokenAssetId, user.address)).toJSON() as any)?.balance ?? 0;

      const fpXRPBalanceAfter =
        ((await api.query.assets.account(GAS_TOKEN_ID, futurepassAddress)).toJSON() as any)?.balance ?? 0;
      const fpSyloBalanceAfter =
        ((await api.query.assets.account(feeTokenAssetId, futurepassAddress)).toJSON() as any)?.balance ?? 0;

      // validate the futurepass's token balance has decreased, and the user's asset
      // balance remains the same
      expect(userXRPBalanceAfter).to.be.eq(userXRPBalanceBefore);
      expect(userSyloBalanceAfter).to.be.eq(userSyloBalanceBefore);

      expect(fpXRPBalanceAfter).to.be.eq(fpXRPBalanceBefore);
      expect(fpSyloBalanceAfter).to.be.lt(fpSyloBalanceBefore);
    }
  });

  it("can submit sylo extrinsic with proxy", async () => {
    // create a random user A
    const user: KeyringPair = keyring.addFromSeed(hexToU8a(Wallet.createRandom().privateKey));

    // create a futurepass for user
    await finalizeTx(alith, api.tx.futurepass.create(user.address));

    const delegateWallet = Wallet.createRandom();
    const delegate = keyring.addFromSeed(hexToU8a(delegateWallet.privateKey));

    const futurepassAddress = (await api.query.futurepass.holders(user.address)).toString();

    // fund user to create fp
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 10_000_000));
    await finalizeTx(alith, api.tx.balances.transfer(user.address, 10_000_000));

    // fund delegate for extrinsics
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, delegate.address, 1));
    await finalizeTx(alith, api.tx.assets.transfer(feeTokenAssetId, delegate.address, 10_000_000_000));

    const deadline = (await provider.getBlockNumber()) + 20;
    const message = ethersUtils
      .solidityKeccak256(
        ["address", "address", "uint8", "uint32"],
        [futurepassAddress, delegate.address, PROXY_TYPE.Any, deadline],
      )
      .substring(2);
    const signature = await delegateWallet.signMessage(message);

    // register a delegate for futurepass (only way to create a proxy)
    await finalizeTx(
      user,
      api.tx.futurepass.registerDelegateWithSignature(
        futurepassAddress,
        delegate.address,
        PROXY_TYPE.Any,
        deadline,
        signature,
      ),
    );

    const delegateXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, delegate.address)).toJSON() as any)?.balance ?? 0;
    const delegateSyloBalanceBefore =
      ((await api.query.assets.account(feeTokenAssetId, delegate.address)).toJSON() as any)?.balance ?? 0;

    await finalizeTx(
      delegate,
      api.tx.proxy.proxy(futurepassAddress, null, api.tx.syloDataVerification.registerResolver("test-proxy", [])),
    );

    // verify balances updated
    const delegateXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, delegate.address)).toJSON() as any)?.balance ?? 0;
    const delegateSyloBalanceAfter =
      ((await api.query.assets.account(feeTokenAssetId, delegate.address)).toJSON() as any)?.balance ?? 0;

    expect(delegateXRPBalanceAfter).to.be.eq(delegateXRPBalanceBefore);
    expect(delegateSyloBalanceAfter).to.be.lessThan(delegateSyloBalanceBefore);
  });

  it("can submit sylo extrinsic with xrpl", async () => {
    const user = Wallet.createRandom();
    const publicKey = computePublicKey(user.publicKey, true);

    // fund the user account to pay for tx fees
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 1));
    await finalizeTx(alith, api.tx.assets.transfer(feeTokenAssetId, user.address, 10_000_000_000));

    const call = createSyloExtrinsics(api)[0];

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

    // sign xaman tx
    const message = encode(xamanJsonTx);
    const encodedSigningMessage = encodeForSigning(xamanJsonTx);
    const signature = sign(encodedSigningMessage, user.privateKey.slice(2));

    const userXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const userSyloBalanceBefore =
      ((await api.query.assets.account(feeTokenAssetId, user.address)).toJSON() as any)?.balance ?? 0;

    // execute xaman tx extrinsic
    await new Promise<any[]>(async (resolve) => {
      await api.tx.xrpl.transact(`0x${message}`, `0x${signature}`, call).send(({ events = [], status }) => {
        if (status.isInBlock) resolve(events);
      });
    });

    const userXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const userSyloBalanceAfter =
      ((await api.query.assets.account(feeTokenAssetId, user.address)).toJSON() as any)?.balance ?? 0;

    expect(userXRPBalanceAfter).to.be.eq(userXRPBalanceBefore);
    expect(userSyloBalanceAfter).to.be.lessThan(userSyloBalanceBefore);
  });

  it("can submit sylo extrinsics in batch call", async () => {
    const calls = createSyloExtrinsics(api);

    const userXRPBalanceBefore =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const userSyloBalanceBefore =
      ((await api.query.assets.account(feeTokenAssetId, user.address)).toJSON() as any)?.balance ?? 0;

    await finalizeTx(user, api.tx.utility.batch(calls));

    const userXRPBalanceAfter =
      ((await api.query.assets.account(GAS_TOKEN_ID, user.address)).toJSON() as any)?.balance ?? 0;
    const userSyloBalanceAfter =
      ((await api.query.assets.account(feeTokenAssetId, user.address)).toJSON() as any)?.balance ?? 0;

    expect(userXRPBalanceAfter).to.be.eq(userXRPBalanceBefore);
    expect(userSyloBalanceAfter).to.be.lessThan(userSyloBalanceBefore);
  });

  // Failures to pay for extrinsics will hang, so failures tests are disabled.
  // Enable and run these tests manually to verify fee swap behaviour.
  xit("fails to submit without sylo tokens available", async () => {
    // create a new user
    const userPrivateKey = Wallet.createRandom().privateKey;
    const user = keyring.addFromSeed(hexToU8a(userPrivateKey));

    // ensure user has enough xrp to submit regular extrinsics
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 100_000_000));

    await finalizeTx(user, api.tx.syloDataVerification.registerResolver("id", ["endpoint"]));
  });

  xit("fails to submit when wrapping sylo exstrinsic in fee-proxy call", async () => {
    // create a new user
    const userPrivateKey = Wallet.createRandom().privateKey;
    const user = keyring.addFromSeed(hexToU8a(userPrivateKey));

    // ensure user has enough xrp to submit regular extrinsics
    await finalizeTx(alith, api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 100_000_000));

    const syloCall = api.tx.syloDataVerification.registerResolver("id", ["endpoint"]);

    const next_fee_token_id = 2148;

    // add liquidity for XRP/SYLO token and set up user funds
    const txs = [
      api.tx.assetsExt.createAsset("sylo-new", "SYLO-NEW", 18, 1, alith.address),
      api.tx.assets.mint(next_fee_token_id, user.address, 2_000_000_000_000_000),
      api.tx.dex.addLiquidity(
        next_fee_token_id,
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

    const maxTokenPayment = 5_000_000;

    await finalizeTx(user, api.tx.feeProxy.callWithFeePreferences(next_fee_token_id, maxTokenPayment, syloCall));
  });

  xit("fails to submit sylo extrinsic in mixed batch of calls", async () => {
    const calls = createSyloExtrinsics(api);

    await finalizeTx(user, api.tx.utility.batch([api.tx.system.remark("hello"), ...calls])).catch(console.log);
  });

  it("fails to submit sylo extrinsic with doughnuts", async () => {
    // create a doughnut
    const holderPrivateKey = Wallet.createRandom().privateKey;
    const holder: KeyringPair = keyring.addFromSeed(hexToU8a(holderPrivateKey));
    const issuerPubkey = user.publicKey;
    const holderPubkey = holder.publicKey;
    const feeMode = 0;
    const expiry = 100000;
    const notBefore = 0;

    const doughnut = new Doughnut(PayloadVersion.V1, issuerPubkey, holderPubkey, feeMode, expiry, notBefore);

    const module = [
      {
        name: "Balances",
        block_cooldown: 0,
        methods: [
          {
            name: "transfer",
            block_cooldown: 0,
            constraints: null,
          },
        ],
      },
    ];

    const topping = new Topping(module);

    // Add to trn topping
    doughnut.addTopping(TRN_PERMISSION_DOMAIN, topping.encode());

    // Sign the doughnut
    const userWallet = await new Wallet(userPrivateKey);
    const ethHash = blake2AsHex(doughnut.payload());
    const ethSlice = Buffer.from(ethHash.slice(2), "hex");
    const issuerSig = await userWallet.signMessage(ethSlice);
    const sigUint8 = Buffer.from(issuerSig.slice(2), "hex");
    doughnut.addSignature(sigUint8, SignatureVersion.EIP191);

    // Verify that the doughnut is valid
    const verified = doughnut.verify(holderPubkey, 5);
    expect(verified).to.be.equal(true);

    // Encode the doughnut
    const encodedDoughnut = doughnut.encode();
    const doughnutHex = u8aToHex(encodedDoughnut);

    // whitelist the holder.
    await finalizeTx(alith, api.tx.sudo.sudo(api.tx.doughnut.updateWhitelistedHolders(holder.address, true)));

    const tip = 0;
    const genesis_hash = await api.rpc.chain.getBlockHash(0);

    const call = createSyloExtrinsics(api)[0];

    const nonce = ((await api.query.system.account(holder.address)).toJSON() as any)?.nonce;
    const tx = await api.tx.doughnut.transact(call, doughnutHex, nonce, genesis_hash, tip, "");
    const txU8a = tx.toU8a(true).slice(2);
    const txHex = u8aToHex(txU8a);
    const holderWallet = await new Wallet(holderPrivateKey);
    const txHash = blake2AsHex(txHex);
    const txSlice = Buffer.from(txHash.slice(2), "hex");
    const holderSig = await holderWallet.signMessage(txSlice);

    const doughnutErr = await new Promise((resolve) => {
      api.tx.doughnut
        .transact(call, doughnutHex, nonce, genesis_hash, tip, holderSig)
        .send(({ internalError, dispatchError }) => {
          if (internalError) {
            return resolve(internalError);
          }

          if (dispatchError && !dispatchError.isModule) {
            return resolve(dispatchError.toJSON());
          }

          if (dispatchError && dispatchError.isModule) {
            const { section, name, docs } = dispatchError.registry.findMetaError(dispatchError.asModule);

            return resolve({ section, name, docs });
          }
        });
    });

    console.error("doughtnut err:", doughnutErr);
  });
});

describe.only("Sylo RPC", () => {
  let node: NodeProcess;
  let api: ApiPromise;
  let keyring: Keyring;
  let alith: KeyringPair;
  let userPrivateKey: string;
  let user: KeyringPair;
  let feeTokenAssetId: number;

  before(async () => {
    node = await startNode();

    const wsProvider = new WsProvider(`ws://127.0.0.1:${node.rpcPort}`);
    api = await ApiPromise.create({ provider: wsProvider, types: typedefs, rpc: rpcs });

    keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    userPrivateKey = Wallet.createRandom().privateKey;
    user = keyring.addFromSeed(hexToU8a(userPrivateKey));

    feeTokenAssetId = await getNextAssetId(api);

    // add liquidity for XRP/SYLO token and set up user funds
    const txs = [
      api.tx.assetsExt.createAsset("sylo", "SYLO", 18, 1, alith.address),
      api.tx.assets.mint(feeTokenAssetId, alith.address, 2_000_000_000_000_000),
      api.tx.assets.mint(feeTokenAssetId, user.address, 2_000_000_000_000_000),
      api.tx.assets.transfer(GAS_TOKEN_ID, user.address, 1), // avoids xrp balance increase due to preservation rules
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

    console.log("liquidity setup complete...");
  });

  it("getPermissions returns correctly if no permission granted", async () => {
    const res = await (api.rpc as any).syloDataPermissions.getPermissions(user.address, alith.address, ["data-id"]);

    expect(res.toJSON()).to.deep.equal({
      Ok: {
        permissions: [["data-id", []]],
        permission_reference: null,
      },
    });
  });

  it("getPermissions returns onchain permissions if permission granted", async () => {
    await finalizeTx(
      user,
      api.tx.syloDataVerification.createValidationRecord(
        "data-id",
        [],
        "data-type",
        [],
        "0x0000000000000000000000000000000000000000000000000000000000000000",
      ),
    );

    // grant data permission
    await finalizeTx(
      user,
      api.tx.syloDataPermissions.grantDataPermissions(
        user.address,
        alith.address,
        ["data-id"],
        DATA_PERMISSION.VIEW,
        null,
        false,
      ),
    );

    const res = await (api.rpc as any).syloDataPermissions.getPermissions(user.address, alith.address, ["data-id"]);

    expect(res.toJSON()).to.deep.equal({
      Ok: {
        permissions: [["data-id", ["VIEW"]]],
        permission_reference: null,
      },
    });
  });

  it("getPermissions returns onchain permission if tagged granted", async () => {
    await finalizeTx(
      user,
      api.tx.syloDataVerification.createValidationRecord(
        "data-id-2",
        [{ method: "sylo-resolver", identifier: "id" }],
        "data-type",
        ["tag"],
        "0x0000000000000000000000000000000000000000000000000000000000000000",
      ),
    );

    // grant data permission
    await finalizeTx(
      user,
      api.tx.syloDataPermissions.grantTaggedPermissions(alith.address, DATA_PERMISSION.MODIFY, ["tag"], null, false),
    );

    const res = await (api.rpc as any).syloDataPermissions.getPermissions(user.address, alith.address, ["data-id-2"]);

    expect(res.toJSON()).to.deep.equal({
      Ok: {
        permissions: [["data-id-2", ["MODIFY"]]],
        permission_reference: null,
      },
    });
  });

  it("getPermissions correctly returns multiple permissions", async () => {
    await finalizeTx(
      user,
      api.tx.syloDataVerification.createValidationRecord(
        "data-id-3",
        [{ method: "sylo-resolver", identifier: "id" }],
        "data-type",
        ["tag"],
        "0x0000000000000000000000000000000000000000000000000000000000000000",
      ),
    );

    // grant data permission
    await finalizeTx(
      user,
      api.tx.syloDataPermissions.grantDataPermissions(
        user.address,
        alith.address,
        ["data-id-3"],
        DATA_PERMISSION.VIEW,
        null,
        false,
      ),
    );

    // grant tagged permissions
    await finalizeTx(
      user,
      api.tx.syloDataPermissions.grantTaggedPermissions(user.address, DATA_PERMISSION.DISTRIBUTE, ["tag"], null, false),
    );

    const res = await (api.rpc as any).syloDataPermissions.getPermissions(user.address, alith.address, ["data-id-3"]);

    expect(res.toJSON()).to.deep.equal({
      Ok: {
        permissions: [["data-id-3", ["VIEW", "MODIFY"]]],
        permission_reference: null,
      },
    });

    console.log(user.address, alith.address);
  });

  it("getPermissions can query for multiple data ids", async () => {
    const res = await (api.rpc as any).syloDataPermissions.getPermissions(user.address, alith.address, [
      "data-id",
      "data-id-2",
      "data-id-3",
    ]);

    expect(res.toJSON()).to.deep.equal({
      Ok: {
        permissions: [
          ["data-id", ["VIEW"]],
          ["data-id-2", ["MODIFY"]],
          ["data-id-3", ["VIEW", "MODIFY"]],
        ],
        permission_reference: null,
      },
    });
  });

  it("getPermissions returns permission reference if it exists", async () => {
    await finalizeTx(user, api.tx.syloDataVerification.registerResolver("permission-resolver", ["endpoint"]));

    // create offchain permission record
    await finalizeTx(
      user,
      api.tx.syloDataVerification.createValidationRecord(
        "offchain-permission",
        [{ method: "sylo-data", identifier: "permission-resolver" }],
        "data-type",
        ["tag"],
        "0x0000000000000000000000000000000000000000000000000000000000000000",
      ),
    );

    // grant data permission
    await finalizeTx(user, api.tx.syloDataPermissions.grantPermissionReference(alith.address, "offchain-permission"));

    const res = await (api.rpc as any).syloDataPermissions.getPermissions(user.address, alith.address, []);

    expect(res.toJSON()).to.deep.equal({
      Ok: {
        permissions: [],
        permission_reference: {
          permission_record_id: "offchain-permission",
          resolvers: [["did:sylo-data:permission-resolver", ["endpoint"]]],
        },
      },
    });
  });

  it("getPermissions returns error if data id is too large", async () => {
    const dataId = Array(1000).fill("a").concat();

    const res = await (api.rpc as any).syloDataPermissions.getPermissions(user.address, alith.address, [dataId]);

    expect(res.toJSON().Err).to.not.be.null;
  });
});
