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
use crate::{
	Config, Error,
};
use lite_json::JsonValue;
use scale_info::prelude::{format, string::String};
use serde_json::{json, to_vec};
use sp_core::H512;
use sp_runtime::offchain::{http, http::Request, Duration};
use sp_std::{vec, vec::Vec};

const XRPL_ENDPOINT: &str = "https://s1.ripple.com:51234/";

pub fn get_xrpl_tx_data<T: Config>(xrpl_block_hash: H512) -> Result<Vec<u8>, Error<T>> {
	let hash: String = String::from_utf8(xrpl_block_hash.as_bytes().to_vec())
		.map_err(|_| Error::<T>::CantParseXrplBlockHash)?;

	let body = rpc_body("tx", &hash);
	let xrpl_response_data = make_rpc_call::<T>(XRPL_ENDPOINT, body);
	let utf8_version = String::from_utf8(xrpl_response_data.unwrap()).unwrap();
	let response = lite_json::parse_json(&utf8_version).unwrap();
	let result_object = response.as_object().unwrap();

	Ok(vec![])
}

// Build RPC body for XRPL `tx` RPC method
fn rpc_body(method: &str, tx_hash: &str) -> Vec<u8> {
	let body = json!({
		"method": method,
		"params": [
			{
				"transaction": format!("{}", tx_hash),
				// "transaction": format!("{}", "C53ECF838647FA5A4C780377025FEC7999AB4182590510CA461444B207AB74A9"),
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

#[derive(Debug, Clone)]
struct XrplResults {
	Account: String,
	SigningPubKey: String,
	TransactionType: String,
	TxnSignature: String,
	hash: String,
	Destination: String,
	Sequence: u64,
	Amount: String,
	Fee: String,
}

impl XrplResults {
	fn default() -> Self {
		XrplResults {
			Account: String::from(""),
			SigningPubKey: String::from(""),
			TransactionType: String::from(""),
			TxnSignature: String::from(""),
			hash: String::from(""),
			Destination: String::from(""),
			Sequence: 0,
			Amount: String::from(""),
			Fee: String::from(""),
		}
	}

	fn from_json(xrpl_json: &[(sp_application_crypto::Vec<char>, JsonValue)]) {
		// Traverse first field: "result" because it always has what we want
		let outer_json = xrpl_json.into_iter();

		// Expect a top-level field "result"
		let inner_json: &JsonValue = &outer_json
			.filter_map(|i| {
				let key_word: String = i.0.clone().iter().collect::<String>().into();
				let expected_field = "result";

				if matches!(key_word, expected_field) {
					return Some(i.clone().1)
				}
				None
			})
			.collect::<Vec<JsonValue>>()[0];

		let mut init = Self::default();

		inner_json.as_object().into_iter().fold(init, |mut t, v| {
			v.iter().for_each(|inner_json_key| {
				let keystr: String = inner_json_key.0.clone().iter().collect();
				let value: String = inner_json_key.1.clone().as_string().unwrap().iter().collect();

				if &keystr == "Account" {
					t.Account = value.clone();
				}

				if &keystr == "TxnSignature" {
					t.TxnSignature = value.clone();
				}

				if &keystr == "SigningPubKey" {
					t.SigningPubKey = value.clone();
				}
				if &keystr == "TransactionType" {
					t.TransactionType = value.clone();
				}
				if &keystr == "TxnSignature" {
					t.TxnSignature = value.clone();
				}
				if &keystr == "hash" {
					t.hash = value.clone();
				}
				if &keystr == "Destination" {
					t.Destination = value.clone();
				}
				if &keystr == "Sequence" {
					let value: u64 = inner_json_key.1.clone().as_number().unwrap().integer;
					t.Sequence = value.clone();
				}
				if &keystr == "Amount" {
					t.Amount = value.clone();
				}
				if &keystr == "Fee" {
					t.Fee = value.clone();
				}
			});

			t
		});
	}
}
