import { ApiPromise, Keyring } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";

import { ALITH_PRIVATE_KEY, GAS_TOKEN_ID } from "../common";
import { usingPlaygrounds } from "../common/FutureverseHelper";

const config = {
  relayUrl: process.env.RELAY_URL || "ws://127.0.0.1:9911",
  rootAUrl: process.env.ROOT_A_URL || "ws://127.0.0.1:9951",
  rootAParaId: process.env.ROOT_A_PARA_ID || 1000,
  rootBUrl: process.env.ROOT_B_URL || "ws://127.0.0.1:9961",
  rootBParaId: process.env.ROOT_A_PARA_ID || 1001,
};

const Assets = {
  Root: 1,
  XRP: 2,
};

describe("Futurepass Precompile", function () {
  let alith: KeyringPair;
  beforeEach(async () => {
    const keyring = new Keyring({ type: "ethereum" });
    alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

    // Root parachain A setup
    await usingPlaygrounds(config.rootAUrl, async (api) => {
      const destination = {
        V2: {
          parents: 0,
          interior: {
            X1: {
              GeneralKey: "FOOA",
            },
          },
        },
      };

      const metadata = {
        name: "Foo Token at Para A",
        symbol: "FOOA",
        decimals: 6,
        existentialDeposit: 0,
        location: destination,
        addtional: { feePerSecond: 10000000n },
      };
      const assetId = 1;
      const amount = 1000000000;

      // sudo assetRegistry.registerAsset
      await registerAsset(api.getApi(), alith, metadata, assetId);
      // sudo tokens.setBalance
      // sudo balances.forceSetBalance
      await fundAccount(api.getApi(), alith, alith.address, amount);
    });
    // Root parachain B setup
    await usingPlaygrounds(config.rootBUrl, async (api) => {
      const destination = {
        V2: {
          parents: 1,
          interior: {
            X1: {
              GeneralKey: "FOOA",
            },
          },
        },
      };

      const metadata = {
        name: "Foo Token at Para B",
        symbol: "FOOB",
        decimals: 6,
        existentialDeposit: 0,
        location: destination,
        addtional: { feePerSecond: 10000000n },
      };
      const assetId = 1;
      const amount = 1000000000;
      // sudo assetRegistry.registerAsset
      await registerAsset(api.getApi(), alith, metadata, assetId);
      // sudo tokens.setBalance
      // sudo balances.forceSetBalance
      await fundAccount(api.getApi(), alith, alith.address, amount);
    });
  });

  it("[XCM] Integration test: Transfer ROOT to the other parachain", async () => {
    await usingPlaygrounds(config.rootAUrl, async (api) => {
      const destination = {
        V2: {
          parents: 1,
          interior: {
            X2: [
              { Parachain: config.rootAParaId },
              {
                AccountKey20: {
                  network: "Any",
                  id: alith.addressRaw,
                },
              },
            ],
          },
        },
      };

      const currencyId = Assets.Root;
      const amount = 100000000;

      // xtokens.transfer
      await xTokensTransfer(api.getApi(), alith, currencyId, amount, destination);
    });
  });
});

function xTokensTransfer(
  api: ApiPromise,
  keyring: KeyringPair,
  currencyId: number,
  amount: number,
  destination: any,
): Promise<void> {
  console.log("[DEBUG]:Start to transfer", amount, "ROOT to", destination);
  return new Promise<void>((resolve) => {
    api.tx.utility
      .batch([api.tx.xTokens.transfer(currencyId, amount, destination, "Unlimited")])
      .signAndSend(keyring, ({ status }) => {
        if (status.isInBlock) {
          console.log("[DEBUG]:Transferred", amount, "ROOT to", destination);
          resolve();
        }
      });
  });
}

function fundAccount(api: ApiPromise, keyring: KeyringPair, address: string, amount: number): Promise<void> {
  console.log("[DEBUG]:Start to fund account", address, "with", amount, "GAS");
  return new Promise<void>((resolve) => {
    api.tx.utility
      .batch([
        api.tx.sudo.sudo(api.tx.tokens.setBalance(address, GAS_TOKEN_ID, amount, 0)),
        api.tx.sudo.sudo(api.tx.balances.forceSetBalance(address, amount)),
      ])
      .signAndSend(keyring, ({ status }) => {
        if (status.isInBlock) {
          console.log("[DEBUG]:Funded account", address, "with", amount, "GAS");
          resolve();
        }
      });
  });
}

function registerAsset(api: ApiPromise, keyring: KeyringPair, metadata: any, assetId: any): Promise<void> {
  console.log("[DEBUG]:Start to register asset", assetId);
  return new Promise<void>((resolve) => {
    api.tx.utility
      .batch([api.tx.sudo.sudo(api.tx.assetRegistry.registerAsset(metadata, assetId))])
      .signAndSend(keyring, ({ status }) => {
        if (status.isInBlock) {
          console.log("[DEBUG]:Registered asset", assetId);
          resolve();
        }
      });
  });
}
