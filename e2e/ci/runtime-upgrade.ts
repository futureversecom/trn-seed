import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { hexToU8a } from "@polkadot/util";

import { ALITH_PRIVATE_KEY, rpcs, typedefs } from "../common";

// Import the API & Provider and some utility functions

const fs = require("fs");

async function main() {
  // Initialise the provider to connect to the dev node
  const provider = new WsProvider("wss://forkednet.au.cicd.rootnet.app/ws");

  // Create the API and wait until ready (optional provider passed through)
  const api = await ApiPromise.create({ provider, types: typedefs, rpc: rpcs });

  const keyring = new Keyring({ type: "ethereum" });
  const alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

  // Retrieve the runtime to upgrade
  const code = fs.readFileSync("../output/seed_runtime.compact.compressed.wasm").toString("hex");
  const proposal = api.tx.system.setCode(`0x${code}`);

  console.log(`Upgrading from ${alith.address}, ${code.length / 2} bytes`);

  // Perform the actual chain upgrade via the sudo module
  api.tx.sudo.sudoUncheckedWeight(proposal, 0).signAndSend(alith, ({ events = [], status }) => {
    console.log("Proposal status:", status.type);

    if (status.isInBlock) {
      events.forEach(function (e) {
        e.event.data.forEach(function (d) {
          if (d.err) {
            print(d.err.module.error);
          }
        });
      });

      console.log("You have just upgraded your chain");
      console.log("Included at block hash", status.asInBlock.toHex());
      console.log("Events:");
      console.log(JSON.stringify(events, null, 2));
    } else if (status.isFinalized) {
      console.log("Finalized block hash", status.asFinalized.toHex());

      process.exit(0);
    }
  });
}

main().catch((error) => {
  console.error(error);
  process.exit(-1);
});
