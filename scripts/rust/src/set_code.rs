
use subxt::{
	ext::{
		sp_core::{
            ecdsa::Pair as SubxtPair,
            Pair as SubxtPairT
        },
	},
	SubstrateConfig,
	tx::{BaseExtrinsicParams, PairSigner, PlainTip},
	OnlineClient, PolkadotConfig,
};

use clap::Parser;
use std::{env, path::PathBuf};

use std::fs::File;
use std::io::Read;

/// Root network runtime upgrade script
// Be sure to run:
// subxt metadata -f bytes > metadata.scale
// To get latest chain metadata into required `metadata.scale` file

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Local path the runtime WASM file
    #[arg(short, long)]
    wasm_path: PathBuf,
}

// Generate an interface that we can use from the node's metadata.
#[subxt::subxt(runtime_metadata_path = "./metadata.scale")]
pub mod root_node {}

fn read_wasm_file(path: &PathBuf) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut wasm_bytes = Vec::new();
    file.read_to_end(&mut wasm_bytes)?;
    Ok(wasm_bytes)
}

pub async fn set_code() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Create a new API client, configured to talk to Polkadot nodes.
    let api = OnlineClient::<SubstrateConfig>::new().await?;

    // Read the wasm bytes from the file
    let wasm_bytes = read_wasm_file(&args.wasm_path)?;

    let acct = env::var("PORCINI_ROOT_KEY").expect("Env var `PORCINI_ROOT_KEY` should be set");
    let pair = SubxtPair::from_string(&acct, None).unwrap();
    let signer = PairSigner::new(pair);

    let set_code_tx = root_node::tx().system().set_code(wasm_bytes);

    let events = api
        .tx()
        .sign_and_submit_then_watch_default(&set_code_tx, &signer)
        .await?
        .wait_for_finalized_success()
        .await?;

    // Find a Transfer event and print it.
    let runtime_upgrade_event = events.find_first::<root_node::balances::events::Transfer>()?;
    if let Some(event) = runtime_upgrade_event {
        println!("Balance transfer success: {event:?}");
    }

    Ok(())
}