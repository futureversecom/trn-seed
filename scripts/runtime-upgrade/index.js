// Import the API & Provider and some utility functions
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');

const fs = require('fs');

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
  // Initialise the provider to connect to the local node
  const provider = new WsProvider('ws://127.0.0.1:9944');

  // Create the API and wait until ready (optional provider passed through)
  const api = await ApiPromise.create({ provider, types });

  // Find the actual keypair in the keyring (if this is a changed value, the key
  // needs to be added to the keyring before - this assumes we have defaults, i.e.
  // Alice as the key - and this already exists on the test keyring)
  const keyring = new Keyring({ type: 'ethereum' });
  const alith = keyring.addFromUri('0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133');

  // Retrieve the runtime to upgrade
  const code = fs.readFileSync('./data/test.wasm').toString('hex');
  const proposal = api.tx.system.setCode(`0x${code}`) // For newer versions of Substrate
  const tx = api.tx.sudo.sudoUncheckedWeight(proposal, 0);

  console.log(`${code.length / 2} bytes`);

  // Perform the actual chain upgrade via the sudo module
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
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});