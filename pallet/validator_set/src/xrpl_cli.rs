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
use seed_primitives::XRP_HTTP_URI;
use tokio::spawn;
use tokio_tungstenite::tungstenite::Message;

pub struct XrplWebsocketClient;
#[async_trait]
impl BridgeXrplWebsocketApi for XrplWebsocketClient {
	/// Fetch transaction details for a challenged transaction hash
	/// Parameters:
	/// - `tx_hash`: The challenged transaction hash
	/// - `ledger_index`: The ledger index for challenged transaction
	/// - `call_id`: The unique call id to identify the requests
	async fn transaction_entry_request(
		tx_hash: XrplTxHash,
		ledger_index: Option<u32>,
		call_id: ChainCallId,
	) -> Result<(), BridgeRpcError> {
		let xrp_http_uri = get_xrp_http_uri()?;
		let client = AsyncWebsocketClient { url: xrp_http_uri };
		let (mut ws_stream, (sender, mut receiver)) = client.open().await.unwrap();

		spawn(async move {
			while let Some(msg) = receiver.next().await {
				assert!(msg.is_ok());
				receiver.close();
				break
			}
		});
		let ledger_index = match ledger_index {
			Some(li) => Some(get_static_str_ref!(li.to_string())),
			None => None,
		};
		let request = TransactionEntry {
			tx_hash: get_static_str_ref!(tx_hash.to_string()),
			id: Option::from(get_static_str_ref!(call_id.to_string())),
			ledger_hash: None,
			ledger_index,
			command: RequestMethod::AccountChannels,
		};
		let message = Message::Text(request.to_json());
		log!(trace, "💎 request: {:?}", message.clone());

		match client.send(&mut ws_stream, sender, message).await {
			Ok(_) => (),
			Err(_error) => (),
		}
		Ok(().into())
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
