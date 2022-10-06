/* Copyright 2021-2022 Centrality Investments Limited
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
use async_trait::async_trait;
use sp_runtime::offchain::StorageKind;
use sp_std::prelude::*;
use xrpl::{
	models::{Model, RequestMethod, TransactionEntry},
	serde_json,
	tokio::AsyncWebsocketClient,
};

use crate::{
	xrpl_types::{BridgeRpcError, BridgeXrplWebsocketApi, XrplAddress, XrplTxHash},
	ChainCallId,
};
use codec::alloc::string::String;
use futures::StreamExt;
use scale_info::prelude::string::ToString;
use seed_pallet_common::{get_static_str_ref, log};
use seed_primitives::{
	xrpl::{LedgerIndex, XrpTransaction, XrplTxData},
	Balance, XRP_HTTP_URI,
};
use tokio::{
	spawn,
	sync::{mpsc, mpsc::Receiver},
};
use tokio_tungstenite::tungstenite::Message;

pub struct XrplWebsocketClient;
#[async_trait]
impl BridgeXrplWebsocketApi for XrplWebsocketClient {
	/// Fetch transaction details for a challenged transaction hash
	/// Parameters:
	/// - `xrp_transaction`: The challenged transaction details
	/// - `ledger_index`: The ledger index for challenged transaction
	/// - `call_id`: The unique call id to identify the requests
	async fn transaction_entry_request(
		xrp_transaction: XrpTransaction,
		ledger_index: LedgerIndex,
		call_id: ChainCallId,
	) -> Result<Receiver<Result<XrplTxHash, BridgeRpcError>>, BridgeRpcError> {
		let xrp_http_uri = get_xrp_http_uri()?;
		let client = AsyncWebsocketClient { url: xrp_http_uri };
		let (mut ws_stream, (sender, mut receiver)) = client.open().await.unwrap();
		let (tx, rx) = mpsc::channel(4);
		spawn(async move {
			while let Some(msg) = receiver.next().await {
				assert!(msg.is_ok());
				match msg {
					Ok(m) => tx
						.send(is_valid_xrp_transaction(m, xrp_transaction, ledger_index))
						.await
						.unwrap(),
					Err(_e) => tx.send(Err(BridgeRpcError::HttpFetch)).await.unwrap(),
				}
				receiver.close();
				break
			}
		});
		let request = TransactionEntry {
			tx_hash: get_static_str_ref!(xrp_transaction.transaction_hash.to_string()),
			id: Option::from(get_static_str_ref!(call_id.to_string())),
			ledger_hash: None,
			ledger_index: Option::from(get_static_str_ref!(ledger_index.to_string())),
			command: RequestMethod::TransactionEntry,
		};
		let message = Message::Text(request.to_json());
		log!(trace, "💎 request: {:?}", message.clone());

		match client.send(&mut ws_stream, sender, message).await {
			Ok(_) => (),
			Err(_error) => (),
		}
		Ok(rx)
	}
}

pub fn is_valid_xrp_transaction(
	msg: Message,
	xrp_transaction: XrpTransaction,
	ledger_index: LedgerIndex,
) -> Result<XrplTxHash, BridgeRpcError> {
	let response: serde_json::Value = match serde_json::from_str(&msg.to_string()) {
		Ok(v) => v,
		Err(_) => return Err(BridgeRpcError::InvalidTransaction("Json Parse Failed".to_string())),
	};
	match response["status"].as_str() {
		Some("success") => {
			let li: LedgerIndex =
				match response["result"]["ledger_index"].clone().to_string().parse::<LedgerIndex>()
				{
					Ok(v) => v,
					Err(_) =>
						return Err(BridgeRpcError::InvalidTransaction(
							"LedgerIndex Parse Failed".to_string(),
						)),
				};
			let validated: bool =
				match response["result"]["validated"].clone().to_string().parse::<bool>() {
					Ok(v) => v,
					Err(_) =>
						return Err(BridgeRpcError::InvalidTransaction(
							"validated Parse Failed".to_string(),
						)),
				};
			let memos: XrplAddress = match response["result"]["tx_json"]["Memos"].clone().as_str() {
				Some(v) => XrplAddress::from_slice(v.as_bytes()),
				None =>
					return Err(BridgeRpcError::InvalidTransaction("Memos Parse Failed".to_string())),
			};
			let tx_amount: Balance = match response["result"]["tx_json"]["Amount"]
				.clone()
				.to_string()
				.parse::<Balance>()
			{
				Ok(v) => v,
				Err(_) =>
					return Err(BridgeRpcError::InvalidTransaction(
						"Amount Parse Failed".to_string(),
					)),
			};
			let transaction_hash: XrplTxHash = match response["result"]["tx_json"]["hash"]
				.clone()
				.as_str()
			{
				Some(v) => XrplTxHash::from_slice(v.as_bytes()),
				None =>
					return Err(BridgeRpcError::InvalidTransaction("hash Parse Failed".to_string())),
			};
			if ledger_index.ne(&li) {
				return Err(BridgeRpcError::InvalidTransaction("ledger_index Mismatch".to_string()))
			}
			if !validated {
				return Err(BridgeRpcError::InvalidTransaction("not validated".to_string()))
			}
			if transaction_hash.ne(&xrp_transaction.transaction_hash) {
				return Err(BridgeRpcError::InvalidTransaction(
					"transaction_hash Mismatch".to_string(),
				))
			}
			match xrp_transaction.transaction {
				XrplTxData::Payment { amount, address } => {
					if amount.ne(&tx_amount) {
						return Err(BridgeRpcError::InvalidTransaction(
							"amount Mismatch".to_string(),
						))
					}
					if address.ne(&memos) {
						return Err(BridgeRpcError::InvalidTransaction(
							"address Mismatch".to_string(),
						))
					}
				},
				XrplTxData::CurrencyPayment { .. } => {},
				XrplTxData::Xls20 => {},
			}
			Ok(transaction_hash)
		},
		_ => Err(BridgeRpcError::InvalidJSON),
	}
}

pub fn get_xrp_http_uri() -> Result<&'static str, BridgeRpcError> {
	let xrp_http_uri = if let Some(value) =
		sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &XRP_HTTP_URI)
	{
		value
	} else {
		log!(error, "💎 Xrp http uri is not configured! set --eth-http=<value> on start up");
		return Err(BridgeRpcError::OcwConfig)
	};
	let xrp_http_uri = match String::from_utf8(xrp_http_uri) {
		Ok(uri) => uri,
		Err(_) => return Err(BridgeRpcError::OcwConfig),
	};
	let xrp_http_uri = core::str::from_utf8(get_static_str_ref!(xrp_http_uri).as_ref())
		.map_err(|_| BridgeRpcError::OcwConfig)?;
	Ok(xrp_http_uri)
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::H160;

	#[test]
	fn test_is_valid_xrp_transaction_success() {
		let xrpl_tx = XrpTransaction {
			transaction_hash: XrplTxHash::from_slice(
				b"C53ECF838647FA5A4C780377025FEC7999AB4182590510CA461444B207AB74A9",
			),
			transaction: XrplTxData::Payment {
				amount: 20160 as Balance,
				address: H160::from_slice(b"6490B68F1116BFE87DDC"),
			},
			timestamp: 0,
		};
		let response = r#"{
								"result": {
									"ledger_hash": "793E56131D8D4ABFB27FA383BFC44F2978B046E023FF46C588D7E0C874C2472A",
									"ledger_index": 56865245,
									"metadata": {},
									"tx_json": {
									  "Account": "rhhh49pFH96roGyuC4E5P4CHaNjS1k8gzM",
									  "Fee": "12",
									  "Flags": 0,
									  "LastLedgerSequence": 56865248,
									  "OfferSequence": 5037708,
									  "Sequence": 5037710,
									  "SigningPubKey": "03B51A3EDF70E4098DA7FB053A01C5A6A0A163A30ED1445F14F87C7C3295FCB3BE",
									  "TakerGets": "15000000000",
									  "TakerPays": {
										"currency": "CNY",
										"issuer": "rKiCet8SdvWxPXnAgYarFUXMh1zCPz432Y",
										"value": "20160.75"
									  },
									  "TransactionType": "OfferCreate",
									  "TxnSignature": "3045022100A5023A0E64923616FCDB6D664F569644C7C9D1895772F986CD6B981B515B02A00220530C973E9A8395BC6FE2484948D2751F6B030FC7FB8575D1BFB406368AD554D9",
									  "Memos": "6490B68F1116BFE87DDC",
									  "Amount": 20160,
									  "hash": "C53ECF838647FA5A4C780377025FEC7999AB4182590510CA461444B207AB74A9"
									},
									"validated": true
								},
								"status": "success",
								"type": "response"
							}"#;
		let response_msg = Message::text(response);
		let result = is_valid_xrp_transaction(response_msg, xrpl_tx, 56865245);
		assert!(result.is_ok());
		assert_eq!(
			result.unwrap(),
			XrplTxHash::from_slice(
				b"C53ECF838647FA5A4C780377025FEC7999AB4182590510CA461444B207AB74A9"
			)
		);
	}

	#[test]
	fn test_is_valid_xrp_transaction_failure() {
		let xrpl_tx = XrpTransaction {
			transaction_hash: Default::default(),
			transaction: Default::default(),
			timestamp: 0,
		};
		let response = "{\"error\":\"transactionNotFound\",\"ledger_hash\":\"A79AF319CFD103F3619D03BED6A5425B8244BD2D8A3E202DBB9101716B1A8C0A\",\"ledger_index\":74879866,\"request\":{\"command\":\"transaction_entry\",\"ledger_index\":\"74879866\",\"tx_hash\":\"6498E5CAEE95FE3E9D9E02D923A63D4A42B83CDAB15AC73CF956DD7BAFFA27AB\"},\"status\":\"error\",\"type\":\"response\",\"validated\":true}";
		let response_msg = Message::text(response);
		let result = is_valid_xrp_transaction(response_msg, xrpl_tx, 74879866);
		assert!(result.is_err());
		assert_eq!(result, Err(BridgeRpcError::InvalidJSON));
	}
}
