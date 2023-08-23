import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { hexToU8a } from "@polkadot/util";
import { readFileSync } from "fs";

const ALITH_PRIVATE_KEY =
  "0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133";

async function main() {
  // Initialise the provider to connect to the dev node
  const provider = new WsProvider("wss://forkednet.au.cicd.rootnet.app/ws");

  // Create the API and wait until ready (optional provider passed through)
  const api = await ApiPromise.create({ provider });

  const keyring = new Keyring({ type: "ethereum" });
  const alith = keyring.addFromSeed(hexToU8a(ALITH_PRIVATE_KEY));

  // Retrieve the runtime to upgrade
  const code = readFileSync(
    "./runtime-wasm/seed_runtime.compact.compressed.wasm"
  ).toString("hex");
  const proposal = api.tx.system.setCode(`0x${code}`);

  console.log(`Upgrading from ${alith.address}, ${code.length / 2} bytes`);

  // Perform the actual chain upgrade via the sudo module
  let errorOccurred = false;
  api.tx.sudo
    .sudoUncheckedWeight(proposal, 0)
    .signAndSend(alith, ({ events = [], status }) => {
      console.log("Proposal status:", status.type);

      if (status.isInBlock) {
        // Check if error happens during the upgrade
        events.forEach(function (e) {
          e.event.data.forEach(function (d) {
            if (d.toString().indexOf("err") >= 0) {
              console.error("Error occurs during the runtime upgrade. ");
              console.error("Error details: ");
              console.error(d.toHuman());
              errorOccurred = true;
            }
          });
        });

        console.log("Included at block hash", status.asInBlock.toHex());
      } else if (status.isFinalized) {
        console.log("Finalized block hash", status.asFinalized.toHex());
        if (errorOccurred) {
          throw new Error(
            "Runtime upgrade failed. Please check the error log."
          );
        } else {
          console.log("You have just upgraded your chain");
        }
      }
    });
}

main().catch((error) => {
  console.error(error);
  process.exit(-1);
});
