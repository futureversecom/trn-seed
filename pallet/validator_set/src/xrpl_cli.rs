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
	tokio::AsyncWebsocketClient,
};

use crate::{
	xrpl_types::{BridgeRpcError, BridgeXrplWebsocketApi, XrplTxHash},
	ChainCallId,
};
use codec::alloc::string::String;
use futures::StreamExt;
use scale_info::prelude::string::ToString;
use seed_pallet_common::{get_static_str_ref, log};
use seed_primitives::{
	xrpl::{LedgerIndex, XrpTransaction},
	XRP_HTTP_URI,
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
					Ok(m) => tx.send(is_valid_xrp_transaction(m, xrp_transaction)).await.unwrap(),
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
			command: RequestMethod::AccountChannels,
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
) -> Result<XrplTxHash, BridgeRpcError> {
	let data = msg.to_string();
	let x = XrpTransaction {
		transaction_hash: Default::default(),
		transaction: Default::default(),
		timestamp: 0,
	};
	Ok(xrp_transaction.transaction_hash)
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
