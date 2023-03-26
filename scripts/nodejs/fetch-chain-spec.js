/**
 * This script is a modified version of the fork-off-substrate procedure written maxsam4
 * Link: https://github.com/maxsam4/fork-off-substrate
 *
 * It establishes a connection with a local node and in parts gets the whole storage from it.
 * Once the storage is stored, a dev chain specification is build and then that spec is
 * populated with the storage data. In the process of populating the new chain spec, certain
 * modules are skipped, either fully or partially, like the System module or Babe module.
 *
 * By default this script runs a local node and then establishes a connection to it. If that's
 * not the desired behavior you can set the env variable `NETWORK` to `local` and it won't
 * start a new node.
 *
 * The fetched storage data and the generated chain specification are stored inside the ./data/
 * folder.
 *
 */
const fs = require('fs');
const path = require('path');
const chalk = require('chalk');
const cliProgress = require('cli-progress');
require("dotenv").config();
const { ApiPromise } = require('@polkadot/api');
const { HttpProvider } = require('@polkadot/rpc-provider');
const { xxhashAsHex } = require('@polkadot/util-crypto');
const execFileSync = require('child_process').execFileSync;
const execSync = require('child_process').execSync;
const { spawn } = require('child_process');
const binaryPath = path.join(__dirname, 'data', 'binary');
const forkedSpecPath = path.join(__dirname, 'data', 'fork.json');
const storagePath = path.join(__dirname, 'data', 'storage.json');

// Using http endpoint since substrate's Ws endpoint has a size limit.
const provider = new HttpProvider('http://127.0.0.1:9933')
// The storage download will be split into 256^chunksLevel chunks.
const chunksLevel = 1;
const totalChunks = Math.pow(256, chunksLevel);
const network = process.env.NETWORK || "porcini";
const chain_path = "./data/" + network + ".json";

let chunksFetched = 0;
let separator = false;
const progressBar = new cliProgress.SingleBar({}, cliProgress.Presets.shades_classic);

let prefixes = ['0x26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9' /* System.Account */, '0x3a636f6465' /* Runtime */];
const skippedModulesPrefix = ['System', 'Session', 'Babe', 'Grandpa', 'GrandpaFinality', 'FinalityTracker', 'Authorship'];
let child;

const sleep = (time) => new Promise(resolve => setTimeout(resolve, time));

const startNode = async () => {
  console.log("Starting node...");

  child = spawn('./data/binary', ["--chain", chain_path, "--tmp", "--sync", "warp", "--rpc-max-response-size", "1000", "--rpc-max-request-size", "1000"]);

  child.stdout.on('data', data => {
    console.log(`stdout:\n${data}`);
  });

  child.stderr.on('data', data => {
    console.error(`stderr: ${data}`);
  });

  child.on('close', (code) => {
    console.log(`child process exited with code ${code}`);
  });

  console.log("Waiting for node to get up and running...");
  await sleep(40000);
}

const stopNode = async () => {
  console.log("Stopping node...");
  child.kill();
  await sleep(1000);
}

const fetchChunks = async (prefix, levelsRemaining, stream, at) => {
  if (levelsRemaining <= 0) {
    const pairs = await provider.send('state_getPairs', [prefix, at]);
    if (pairs.length > 0) {
      separator ? stream.write(",") : separator = true;
      stream.write(JSON.stringify(pairs).slice(1, -1));
    }
    progressBar.update(++chunksFetched);
    return;
  }

  // Async fetch the last level
  if (levelsRemaining == 1) {
    let promises = [];
    for (let i = 0; i < 256; i++) {
      promises.push(fetchChunks(prefix + i.toString(16).padStart(2, "0"), levelsRemaining - 1, stream, at));
    }
    await Promise.all(promises);
  } else {
    for (let i = 0; i < 256; i++) {
      await fetchChunks(prefix + i.toString(16).padStart(2, "0"), levelsRemaining - 1, stream, at);
    }
  }
}

const generateChainSpec = async () => {
  if (!fs.existsSync(binaryPath)) {
    console.log(chalk.red('Binary missing. Please copy the binary of your substrate node to the data folder and rename the binary to "binary"'));
    process.exit(1);
  }
  execFileSync('chmod', ['+x', binaryPath]);

  let api;
  console.log(chalk.green('We are intentionally using the HTTP endpoint. If you see any warnings about that, please ignore them.'));
  api = await ApiPromise.create({ provider });

  if (fs.existsSync(storagePath)) {
    console.log(chalk.yellow('Reusing cached storage. Delete ./data/storage.json and rerun the script if you want to fetch latest storage'));
  } else {
    // Download state of original chain
    console.log(chalk.green('Fetching current state of the live chain. Please wait, it can take a while depending on the size of your chain.'));
    let at = (await api.rpc.chain.getBlockHash()).toString();
    progressBar.start(totalChunks, 0);
    const stream = fs.createWriteStream(storagePath, { flags: 'a' });
    stream.write("[");
    await fetchChunks("0x", chunksLevel, stream, at);
    stream.write("]");
    stream.end();
    progressBar.stop();
  }

  const metadata = await api.rpc.state.getMetadata();

  // Populate the prefixes array
  const modules = metadata.asLatest.pallets;
  modules.forEach((module) => {
    if (module.storage) {
      const name = module.name.toHuman();
      if (!skippedModulesPrefix.includes(name)) {
        prefixes.push(xxhashAsHex(module.name, 128));
      }
    }
  });

  // Generate chain spec for original and forked chains
  execSync(binaryPath + ` build-spec --chain dev --disable-default-bootnode --raw > ` + forkedSpecPath);

  let storage = JSON.parse(fs.readFileSync(storagePath, 'utf8'));
  let forkedSpec = JSON.parse(fs.readFileSync(forkedSpecPath, 'utf8'));

  let sudo_key = forkedSpec.genesis.raw.top['0x5c0d1176a568c1f92944340dbfed9e9c530ebca703c85910e7164cb7d1c9e47b']; // Alith's key

  // Modify chain name and id
  forkedSpec.name = forkedSpec.name + ' Fork :) ';
  forkedSpec.id = forkedSpec.id + " fork :) ";

  // Grab the items to be moved, then iterate through and insert into storage
  storage
    .filter((i) => prefixes.some((prefix) => i[0].startsWith(prefix)))
    .forEach(([key, value]) => (forkedSpec.genesis.raw.top[key] = value));

  // Delete System.LastRuntimeUpgrade to ensure that the on_runtime_upgrade event is triggered
  delete forkedSpec.genesis.raw.top['0x26aa394eea5630e07c48ae0c9558cef7f9cce9c888469bb1a0dceaa129672ef8'];

  // To prevent the validator set from changing mid-test, set Staking.ForceEra to ForceNone ('0x02')
  forkedSpec.genesis.raw.top['0x5f3e4907f716ac89b6347d15ececedcaf7dad0317324aecae8744b87fc95f2f3'] = '0x02';

  // Set sudo key to Alith
  forkedSpec.genesis.raw.top['0x5c0d1176a568c1f92944340dbfed9e9c530ebca703c85910e7164cb7d1c9e47b'] = sudo_key;

  fs.writeFileSync(forkedSpecPath, JSON.stringify(forkedSpec, null, 4));

  console.log('Forked genesis generated successfully. Find it at ./data/fork.json');
}

async function main() {
  if (network !== "local") {
    await startNode();
  }

  await generateChainSpec();

  if (network !== "local") {
    await stopNode();
  }

  process.exit();
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
