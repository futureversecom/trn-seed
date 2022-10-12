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
	xrpl_types::{BridgeRpcError, BridgeXrplWebsocketApi, TransactionEntryResponse, XrplTxHash},
	ChainCallId, H160,
};
use codec::alloc::string::String;
use futures::StreamExt;
use scale_info::prelude::string::ToString;
use seed_pallet_common::{get_lifetime_str_ref, log};
use seed_primitives::{
	xrpl::{LedgerIndex, XrpTransaction, XrplTxData},
	AccountId, Balance, XRP_HTTP_URI,
};
use tokio::{
	spawn,
	sync::{mpsc, mpsc::Receiver},
};
use tokio_tungstenite::tungstenite::Message;

pub struct XrplWebsocketClient;
#[async_trait]
impl<'a> BridgeXrplWebsocketApi for XrplWebsocketClient {
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
			tx_hash: get_lifetime_str_ref!('static, xrp_transaction.transaction_hash.to_string()),
			id: Option::from(get_lifetime_str_ref!('static, call_id.to_string())),
			ledger_hash: None,
			ledger_index: Option::from(get_lifetime_str_ref!('static, ledger_index.to_string())),
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
	let response: TransactionEntryResponse = match serde_json::from_str(&msg.to_string()) {
		Ok(v) => v,
		Err(_) => return Err(BridgeRpcError::InvalidTransaction("Json Parse Failed".to_string())),
	};

	match response.status.as_str() {
		"success" => {
			let result = match response.result {
				Some(r) => r,
				None => return Err(BridgeRpcError::InvalidJSON),
			};
			let li: LedgerIndex = result.ledger_index as LedgerIndex;
			let validated: bool = result.validated;
			// https://centralitydev.atlassian.net/wiki/spaces/FUT/pages/2255781889/RIP+2+XRPL+Bridge#XRPL--%3E-Root-Payments
			let root_address: AccountId = match result.tx_json.memos {
				Some(memos) => {
					let hex_address = match hex::decode(&memos[0].memo_data[6..]) {
						Ok(val) => {
							if val.len() != 20 {
								return Err(BridgeRpcError::InvalidTransaction(
									"XrplAddress extraction from Memo Failed".to_string(),
								))
							}
							val
						},
						Err(_) =>
							return Err(BridgeRpcError::InvalidTransaction(
								"XrplAddress extraction from Memo Failed".to_string(),
							)),
					};
					AccountId::from(H160::from_slice(&hex_address))
				},
				None =>
					return Err(BridgeRpcError::InvalidTransaction(
						"XrplAddress extraction from Memo Failed".to_string(),
					)),
			};
			let tx_amount: Balance = match result.tx_json.amount.parse::<Balance>() {
				Ok(v) => v,
				Err(_) =>
					return Err(BridgeRpcError::InvalidTransaction(
						"Amount Parse Failed".to_string(),
					)),
			};
			let transaction_hash: XrplTxHash =
				XrplTxHash::from_slice(result.tx_json.hash.as_bytes());

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
					if AccountId::from(address).ne(&root_address) {
						return Err(BridgeRpcError::InvalidTransaction(
							"xrpl address Mismatch".to_string(),
						))
					}
				},
				XrplTxData::CurrencyPayment { .. } => {},
				XrplTxData::Xls20 => {},
			}
			Ok(transaction_hash)
		},
		"error" => match response.error {
			Some(e) => Err(BridgeRpcError::InvalidTransaction(e)),
			_ => Err(BridgeRpcError::InvalidJSON),
		},
		_ => Err(BridgeRpcError::InvalidJSON),
	}
}

pub fn get_xrp_http_uri<'a>() -> Result<&'a str, BridgeRpcError> {
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
	let xrp_http_uri = core::str::from_utf8(get_lifetime_str_ref!('a, xrp_http_uri).as_ref())
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
				b"353E4FA8FA6B891B4CBA2B9A823285095CC2EE41E10B81FA69C0266382256B00",
			),
			transaction: XrplTxData::Payment {
				amount: 1653600 as Balance,
				address: H160::from_slice(b"6490B68F1116BFE87DDC"),
			},
			timestamp: 0,
		};
		let response = "{\"result\":{\"ledger_hash\":\"304DE877B7E36422EC34B3E3E8218B9A6F2D909916D8ECEFFF5512FB289A6D24\",\"ledger_index\":74897485,\"metadata\":{\"AffectedNodes\":[{\"ModifiedNode\":{\"FinalFields\":{\"Balance\":{\"currency\":\"CHP\",\"issuer\":\"rrrrrrrrrrrrrrrrrrrrBZbvji\",\"value\":\"-1761.1119\"},\"Flags\":2228224,\"HighLimit\":{\"currency\":\"CHP\",\"issuer\":\"r4ECXyK6fgYcJ4kZQKr8A3BkgsCse5s9Vo\",\"value\":\"100000000\"},\"HighNode\":\"0\",\"LowLimit\":{\"currency\":\"CHP\",\"issuer\":\"rhFNUEAKyXZmJHsnfJvH8hM12Ydk2icEof\",\"value\":\"0\"},\"LowNode\":\"426\"},\"LedgerEntryType\":\"RippleState\",\"LedgerIndex\":\"016C25C3A9AB04379EEE02374AF59964A040B457BDF1CE5839DE3BB07D63E7D0\",\"PreviousFields\":{\"Balance\":{\"currency\":\"CHP\",\"issuer\":\"rrrrrrrrrrrrrrrrrrrrBZbvji\",\"value\":\"-1762.7655\"}},\"PreviousTxnID\":\"7B4E3140251FF660DECB10143FB62CBFDE58B57671577FF420E016E010D9D956\",\"PreviousTxnLgrSeq\":74897483}},{\"ModifiedNode\":{\"FinalFields\":{\"Balance\":{\"currency\":\"CHP\",\"issuer\":\"rrrrrrrrrrrrrrrrrrrrBZbvji\",\"value\":\"-54.2628651343126\"},\"Flags\":2228224,\"HighLimit\":{\"currency\":\"CHP\",\"issuer\":\"rPDt1KAmLkZU5yTY8GgiNe9ZDJo6R5moJh\",\"value\":\"100000000\"},\"HighNode\":\"0\",\"LowLimit\":{\"currency\":\"CHP\",\"issuer\":\"rhFNUEAKyXZmJHsnfJvH8hM12Ydk2icEof\",\"value\":\"0\"},\"LowNode\":\"425\"},\"LedgerEntryType\":\"RippleState\",\"LedgerIndex\":\"2EA8A716E36FC131D017E14547055C1A4EBC6AC14660E7D5BABC0AEA68AA0EDD\",\"PreviousFields\":{\"Balance\":{\"currency\":\"CHP\",\"issuer\":\"rrrrrrrrrrrrrrrrrrrrBZbvji\",\"value\":\"-52.6092651343126\"}},\"PreviousTxnID\":\"293E48CE236DB58D62029B1A1E753766C7B1624818D7E1DE55CFDABF115E9027\",\"PreviousTxnLgrSeq\":74897442}},{\"ModifiedNode\":{\"FinalFields\":{\"Account\":\"r4ECXyK6fgYcJ4kZQKr8A3BkgsCse5s9Vo\",\"Balance\":\"269934645\",\"Flags\":0,\"OwnerCount\":1,\"Sequence\":74799456},\"LedgerEntryType\":\"AccountRoot\",\"LedgerIndex\":\"9CC2F2943D07735FDF80ABC79EEC729E921513E8DB3A8C7DB52AB121BDE88ABF\",\"PreviousFields\":{\"Balance\":\"269934657\",\"Sequence\":74799455},\"PreviousTxnID\":\"890F08C28C54B394D2F97EF331645173BAC246D44C6866C6E11D89917F6D5A1E\",\"PreviousTxnLgrSeq\":74897483}}],\"TransactionIndex\":15,\"TransactionResult\":\"tesSUCCESS\"},\"tx_json\":{\"Account\":\"r4ECXyK6fgYcJ4kZQKr8A3BkgsCse5s9Vo\",\"Amount\":\"1653600\",\"Destination\":\"rPDt1KAmLkZU5yTY8GgiNe9ZDJo6R5moJh\",\"Fee\":\"12\",\"Flags\":2147483648,\"LastLedgerSequence\":74897488,\"SendMax\":{\"currency\":\"CHP\",\"issuer\":\"rhFNUEAKyXZmJHsnfJvH8hM12Ydk2icEof\",\"value\":\"1.6536\"},\"Sequence\":74799455,\"SigningPubKey\":\"ED1164235CCB3CC3CEB6EA1EE0FD390BCA574A1626D3B58CA18CA0DD2105CD5C10\",\"TransactionType\":\"Payment\",\"TxnSignature\":\"37BC5B126754A41F1DAAB36008D610A10C89F920F5454BCFD8A690CD4D144C6F480D69CDF5716292BD65B596016214DC1D59ECCAFA7888A3A4E43DA0B0FB1600\",\"hash\":\"353E4FA8FA6B891B4CBA2B9A823285095CC2EE41E10B81FA69C0266382256B00\",\"Memos\":[{\"MemoType\":\"root-network-bridge\",\"MemoData\":\"0100643634393042363846313131364246453837444443\"}]},\"validated\":true},\"status\":\"success\",\"type\":\"response\"}";
		let response_msg = Message::text(response);
		let result = is_valid_xrp_transaction(response_msg, xrpl_tx, 74897485);
		assert!(result.is_ok());
		assert_eq!(
			result.unwrap(),
			XrplTxHash::from_slice(
				b"353E4FA8FA6B891B4CBA2B9A823285095CC2EE41E10B81FA69C0266382256B00"
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
		assert_eq!(result, Err(BridgeRpcError::InvalidTransaction("transactionNotFound".into())));
	}
}
