/* Copyright 2019-2021 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */
use crate::{Config, Error, helpers::{XrpTransaction, XrplTxData}};
use scale_info::prelude::{format, string::String};
use seed_primitives::{xrpl::{LedgerIndex, XrplTxHash}, AccountId, Balance};
use serde_json::{json, to_vec};
use sp_core::{H512, H160};
use sp_runtime::offchain::{http, http::Request, Duration};
use sp_std::{vec, vec::Vec};
// use serde::{Serialize, Deserialize};

const XRPL_ENDPOINT: &str = "https://s1.ripple.com:51234/";
// const XRPL_ENDPOINT: &str = "https://webhook.site/22df0ca3-bc4c-4161-b7f2-976fc22536e6";



#[derive(Debug, Clone)]
pub struct TransactionEntryResponse {
	pub result: Option<TransactionEntryResponseResult>,
	pub status: String,
	pub r#type: String,
	pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TransactionEntryResponseResult {
	pub ledger_hash: String,
	pub ledger_index: u64,
	pub tx_json: Payment,
	pub validated: bool,
}

#[derive(Debug, Clone)]
pub struct Payment {
	pub account: String,
	pub amount: String, // https://xrpl.org/basic-data-types.html#specifying-currency-amounts
	pub hash: String,
	pub memos: Option<Vec<Memo>>,
}

#[derive(Debug, Clone)]
pub struct Memo {
	pub memo_type: String,
	pub memo_data: String,
}

pub fn get_xrpl_tx_data<T: Config>(xrpl_block_hash: H512) -> Result<Vec<u8>, Error<T>>  {
	let hash: String = String::from_utf8(xrpl_block_hash.as_bytes().to_vec())
		.map_err(|_| Error::<T>::CantParseXrplBlockHash)?;

	let body = rpc_body("tx", &hash);
	make_rpc_call(XRPL_ENDPOINT, body)
}

// Build RPC body for XRPL `tx` RPC method
fn rpc_body(method: &str, tx_hash: &str) -> Vec<u8> {
	let body = json!({
		"method": method,
		"params": [
			{
				"transaction": format!("{}", "BE75DA0263A223FD629A9CFD83471AFD776E7A69E7D7972643A99976D8ABE0EF"),
				"binary": false
			}
		]
	});
	to_vec(&body).unwrap()
}

fn make_rpc_call<T: Config>(url: &str, body: Vec<u8>) -> Result<Vec<u8>, Error<T>> {
	let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
	let pending = Request::new(url)
		.method(http::Method::Post)
		.body(vec![body])
		.add_header("Content-Type", "application/json")
		.send()
		.unwrap();

	let response = pending
		.try_wait(deadline)
		.map_err(|_| Error::DeadlineReached)?
		.map_err(|_| Error::HttpTimeout)?;

	// Let's check the status code before we proceed to reading the response.
	if response.code != 200 {
		log::warn!("Unexpected status code: {}", response.code);
		log::warn!("Unexpected status code: {:?}", response);
		return Err(Error::UnexpectedStatusCode)
	}

	let body = response.body().collect::<Vec<u8>>();
	Ok(body)
}

pub fn is_valid_xrp_transaction<T: Config>(
	msg: &str,
	// xrp_transaction: XrpTransaction,
	// ledger_index: LedgerIndex,
// ) -> Result<XrplTxHash, Error<T>> {
) {
	let response = lite_json::parse_json(&msg).unwrap();
	let result_object = response.as_object().unwrap();

	let account = result_object
		.into_iter()
		.find(|(k, val)| {
			let parsed = val.as_object().unwrap();
			// Pull values out of JSON fields
			let (account_val, signing_pub_key_val, transaction_type_val, txn_signature_val, hash_val) = parsed.into_iter()
    .fold((None, None, None, None, None), |(account_val, signing_pub_key_val, transaction_type_val, txn_signature_val, hash_val), (key, val)| {
        match key.iter().collect::<String>().as_str() {
            "Account" => (Some(account_val.unwrap_or(val.as_string().unwrap())), signing_pub_key_val, transaction_type_val, txn_signature_val, hash_val),
            "SigningPubKey" => (account_val, Some(signing_pub_key_val.unwrap_or(val.as_string().unwrap())), transaction_type_val, txn_signature_val, hash_val),
            "TransactionType" => (account_val, signing_pub_key_val, Some(transaction_type_val.unwrap_or(val.as_string().unwrap())), txn_signature_val, hash_val),
            "TxnSignature" => (account_val, signing_pub_key_val, transaction_type_val, Some(txn_signature_val.unwrap_or(val.as_string().unwrap())), hash_val),
            "hash" => (account_val, signing_pub_key_val, transaction_type_val, txn_signature_val, Some(hash_val.unwrap_or(val.as_string().unwrap()))),
            _ => (account_val, signing_pub_key_val, transaction_type_val, txn_signature_val, hash_val)
        }
    });

	// Do things, verification
	true
	});
}