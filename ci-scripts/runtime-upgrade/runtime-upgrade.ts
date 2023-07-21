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
  const code = readFileSync("./seed_runtime.compact.compressed.wasm").toString(
    "hex"
  );
  const proposal = api.tx.system.setCode(`0x${code}`);

  console.log(`Upgrading from ${alith.address}, ${code.length / 2} bytes`);

  // Perform the actual chain upgrade via the sudo module
  api.tx.sudo
    .sudoUncheckedWeight(proposal, 0)
    .signAndSend(alith, ({ events = [], status }) => {
      console.log("Proposal status:", status.type);

      if (status.isInBlock) {
        events.forEach(function (e) {
          e.event.data.forEach(function (d) {
            if (d) {
              console.log(d);
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