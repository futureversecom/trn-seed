/**
 * This scripts establishes a connection to a local running node
 * and executes a runtime upgrade. The WASM file needs to be
 * stored inside the ./data/ folder and it needs to be called `runtime.wasm`
 *
 */
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const fs = require('fs');

const URI = process.env.URI || "ws://127.0.0.1:9944";
const SUDO_KEY = process.env.SUDO_KEY || "0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133";

const types = {
  AccountId: "EthereumAccountId",
  AccountId20: "EthereumAccountId",
  AccountId32: "EthereumAccountId",
  Address: "AccountId",
  LookupSource: "AccountId",
  Lookup0: "AccountId",
  EthereumSignature: {
    r: "H256",
    s: "H256",
    v: "U8",
  },
  ExtrinsicSignature: "EthereumSignature",
  SessionKeys: "([u8; 32], [u8; 32])",
};

async function main() {
  const provider = new WsProvider(URI);
  const api = await ApiPromise.create({ provider, types, throwOnConnect: true });

  const keyring = new Keyring({ type: 'ethereum' });
  const alith = keyring.addFromUri(SUDO_KEY);

  const code = fs.readFileSync('./data/runtime.wasm').toString('hex');
  const proposal = api.tx.system.setCode(`0x${code}`) // For newer versions of Substrate
  const tx = api.tx.sudo.sudoUncheckedWeight(proposal, 0);

  await new Promise((resolve, reject) => {
    tx.signAndSend(alith, ({ events = [], status }) => {
      console.log('Proposal status:', status.type);

      if (status.isInBlock) {
        console.error('You have just upgraded your chain');

        console.log('Included at block hash', status.asInBlock.toHex());
      } else if (status.isFinalized) {
        console.log('Finalized block hash', status.asFinalized.toHex());

        process.exit(0);
      }
    });
  });
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
