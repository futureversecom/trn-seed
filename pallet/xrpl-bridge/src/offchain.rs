use scale_info::prelude::{format, string::String};
use serde_json::{json, to_vec};
use sp_core::H512;
use sp_runtime::offchain::{http, http::Request, Duration};
use sp_std::{vec, vec::Vec};

const XRPL_ENDPOINT: &str = "https://s1.ripple.com:51234/";

pub fn get_xrpl_block_data(xrpl_block_hash: H512, ledger_index: u64) -> Result<(), http::Error> {
	let hash: String = String::from_utf8(xrpl_block_hash.as_bytes().to_vec()).unwrap();
	let body = rpc_body("transaction_entry", &hash, ledger_index);
	make_rpc_call(XRPL_ENDPOINT, body)
}

// Build RPC body for getting tx by ledger
fn rpc_body(method: &str, tx_hash: &str, ledger_index: u64) -> Vec<u8> {
	let body = json!({
		"method": method,
		"params": [
			{
				"tx_hash":  format!("{}", tx_hash),
				"ledger_index": ledger_index
			}
		]
	});
	to_vec(&body).unwrap()
}

fn make_rpc_call(url: &str, body: Vec<u8>) -> Result<(), http::Error> {
	let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
	let pending = Request::new(url)
		.method(http::Method::Post)
		.body(vec![body])
		.add_header("Content-Type", "application/json")
		.send()
		.unwrap();

	let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
	log::info!("response {:?}", response);

	// Let's check the status code before we proceed to reading the response.
	if response.code != 200 {
		log::warn!("Unexpected status code: {}", response.code);
		return Err(http::Error::Unknown)
	}

	let body = response.body().collect::<Vec<u8>>();
	// Create a str slice from the body.
	let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
		log::warn!("No UTF8 body");
		http::Error::Unknown
	})?;
	// TODO: Return and give parsed values depending on what is needed from XRPLs
	Ok(())
}
