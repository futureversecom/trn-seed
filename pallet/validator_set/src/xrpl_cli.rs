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
use sp_runtime::offchain::StorageKind;
use sp_std::prelude::*;

#[cfg(not(feature = "std"))]
use sp_std::alloc::string::ToString;
#[cfg(std)]
use std::string::ToString;
use xrpl::models::{Model, RequestMethod, Tx};
use xrpl::serde_json::Value::String;
use xrpl::tokio::AsyncWebsocketClient;

use seed_pallet_common::log;
use seed_primitives::XRP_HTTP_URI;
use crate::xrpl_types::{BridgeRpcError, BridgeXrplWebsocketApi, XrplTxHash};

/// Provides minimal ethereum RPC queries for eth bridge protocol
pub struct XrplWebsocketClient;
#[async_trait]
impl BridgeXrplWebsocketApi for XrplWebsocketClient {
    async fn xrpl_call(hash: XrplTxHash, ledger_index: Option<u32>) -> Result<Vec<u8>, BridgeRpcError> {
        let xrp_http_uri = if let Some(value) =
        sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &XRP_HTTP_URI)
        {
            value
        } else {
            log!(error, "💎 Xrp http uri is not configured! set --eth-http=<value> on start up");
            return Err(BridgeRpcError::OcwConfig)
        };
        let xrp_http_uri =
            core::str::from_utf8(&xrp_http_uri).map_err(|_| BridgeRpcError::OcwConfig)?;

        let client = AsyncWebsocketClient {
            url: xrp_http_uri,
        };

        let request = Tx {
            id: None,
            binary: Some(false),
            min_ledger: ledger_index,
            max_ledger: ledger_index,
            command: RequestMethod::AccountChannels
        };
        let message = Message::Text(request.to_json());
        log!(trace, "💎 request: {:?}", message.clone());
        assert!(client.request(message).await.is_ok());
        log!(trace, "💎 request: {:?}", request);
        //let message = Message::Text(request.to_json());
    }
}