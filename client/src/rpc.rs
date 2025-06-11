// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::{collections::BTreeMap, sync::Arc};

use fp_rpc::EthereumRuntimeRPCApi;
use seed_runtime::RuntimeEvent;

use jsonrpsee::RpcModule;
// Substrate
use sc_client_api::{
	backend::{AuxStore, Backend, StateBackend, StorageProvider},
	client::BlockchainEvents,
};
use sc_consensus_grandpa::{
	FinalityProofProvider, GrandpaJustificationStream, SharedAuthoritySet, SharedVoterState,
};
use sc_network::NetworkService;
use sc_rpc::SubscriptionTaskExecutor;
use sc_rpc_api::DenyUnsafe;
use sc_service::TransactionPool;
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::{CallApiAt, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_blockchain::{
	Backend as BlockchainBackend, Error as BlockChainError, HeaderBackend, HeaderMetadata,
};
use sp_consensus::SelectChain;
use sp_consensus_babe::BabeApi;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block as BlockT;
use precompile_utils::
	constants::ERC20_PRECOMPILE_ADDRESS_PREFIX;

// Frontier
use fc_rpc::{
	pending::ConsensusDataProvider, EthBlockDataCacheTask, OverrideHandle,
	RuntimeApiStorageOverride,
};
use fc_rpc_core::types::{BlockNumber as BlockNo, FeeHistoryCache, FeeHistoryCacheLimit, Filter, FilterPool};
use frame_system::Event;
use sc_network_sync::SyncingService;
use sp_core::{H160, H256};
use sp_transaction_storage_proof::IndexedBody;

// Runtime
use ethy_gadget::notification::EthyEventProofStream;
use ethy_gadget_rpc::{EthyApiServer, EthyRpcHandler};
use seed_primitives::{ethy::EthyApi, opaque::Block, AccountId, Balance, BlockNumber, Hash, Nonce, AssetId};
use seed_runtime::Runtime;

/// Extra RPC deps for Ethy
pub struct EthyDeps {
	/// Receives notifications about event proofs from Ethy.
	pub event_proof_stream: EthyEventProofStream,
	/// Executor to drive the subscription manager in the Ethy RPC handler.
	pub subscription_executor: SubscriptionTaskExecutor,
}

/// Extra dependencies for BABE.
pub struct BabeDeps {
	/// BABE protocol config.
	pub babe_config: sc_consensus_babe::BabeConfiguration,
	/// A handle to the BABE worker for issuing requests.
	pub babe_worker_handle: sc_consensus_babe::BabeWorkerHandle<Block>,
	/// The keystore that manages the keys of the node.
	pub keystore: KeystorePtr,
}

/// Extra dependencies for GRANDPA
pub struct GrandpaDeps<B> {
	/// Voting round info.
	pub shared_voter_state: SharedVoterState,
	/// Authority set info.
	pub shared_authority_set: SharedAuthoritySet<Hash, BlockNumber>,
	/// Receives notifications about justification events from Grandpa.
	pub justification_stream: GrandpaJustificationStream<Block>,
	/// Executor to drive the subscription manager in the Grandpa RPC handler.
	pub subscription_executor: SubscriptionTaskExecutor,
	/// Finality proof provider.
	pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Full client dependencies.
pub struct FullDeps<C, P, A: ChainApi, BE, SC> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Graph pool instance.
	pub graph: Arc<Pool<A>>,
	/// The SelectChain Strategy
	pub select_chain: SC,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// The Node authority flag
	pub is_authority: bool,
	/// Network service
	pub network: Arc<NetworkService<Block, Hash>>,
	/// EthFilterApi pool.
	pub filter_pool: Option<FilterPool>,
	/// Frontier Backend.
	pub frontier_backend: Arc<dyn fc_db::BackendReader<Block> + Send + Sync>,
	/// Maximum number of logs in a query.
	pub max_past_logs: u32,
	/// Fee history cache.
	pub fee_history_cache: FeeHistoryCache,
	/// Maximum fee history cache size.
	pub fee_history_cache_limit: FeeHistoryCacheLimit,
	/// Ethereum data access overrides.
	pub overrides: Arc<OverrideHandle<Block>>,
	/// Cache for Ethereum block data.
	pub block_data_cache: Arc<EthBlockDataCacheTask<Block>>,
	/// Ethy specific dependencies.
	pub ethy: EthyDeps,
	/// BABE specific dependencies.
	pub babe: BabeDeps,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps<BE>,
	/// Chain syncing service
	pub syncing_service: Arc<SyncingService<Block>>,
	/// Mandated parent hashes for a given block hash.
	pub eth_forced_parent_hashes: Option<BTreeMap<H256, H256>>,
}

pub fn overrides_handle<B, C, BE>(client: Arc<C>) -> Arc<OverrideHandle<B>>
where
	B: BlockT,
	C: ProvideRuntimeApi<B>,
	C::Api: EthereumRuntimeRPCApi<B>,
	C: HeaderBackend<B> + StorageProvider<B, BE> + 'static,
	BE: Backend<B> + 'static,
{
	// NB: the following is used to redefine storage schema after certain blocks
	// on live chains
	// let mut overrides_map = BTreeMap::new();
	// overrides_map.insert(
	// 	EthereumStorageSchema::V3,
	// 	Box::new(SchemaV3Override::new(client.clone()))
	// 		as Box<dyn StorageOverride<_> + Send + Sync>,
	// );

	Arc::new(OverrideHandle {
		schemas: Default::default(),
		fallback: Box::new(RuntimeApiStorageOverride::new(client)),
	})
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, A, BE, SC>(
	deps: FullDeps<C, P, A, BE, SC>,
	subscription_task_executor: SubscriptionTaskExecutor,
	pubsub_notification_sinks: Arc<
		fc_mapping_sync::EthereumBlockNotificationSinks<
			fc_mapping_sync::EthereumBlockNotification<Block>,
		>,
	>,
	pending_consensus_data_provider: Box<dyn ConsensusDataProvider<Block>>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	A: ChainApi<Block = Block> + 'static,
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<sp_runtime::traits::BlakeTwo256>,
	BE::Blockchain: BlockchainBackend<Block>,
	C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
	C: BlockchainEvents<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
	C: Send + Sync + 'static,
	C: CallApiAt<Block>,
	C: IndexedBody<Block>,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: BabeApi<Block>,
	C::Api: BlockBuilder<Block>,
	C::Api: EthyApi<Block>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	C::Api: pallet_dex_rpc::DexRuntimeApi<Block, Runtime>,
	C::Api: pallet_nft_rpc::NftRuntimeApi<Block, AccountId, Runtime>,
	C::Api: pallet_sft_rpc::SftRuntimeApi<Block, Runtime>,
	C::Api: pallet_assets_ext_rpc::AssetsExtRuntimeApi<Block, AccountId>,
	C::Api: pallet_sylo_data_permissions_rpc::SyloDataPermissionsRuntimeApi<Block, AccountId>,
	P: TransactionPool<Block = Block> + 'static,
	SC: SelectChain<Block> + 'static,
{
	use fc_rpc::{
		Eth, EthApiServer, EthFilter, EthFilterApiServer, EthPubSub, EthPubSubApiServer, Net,
		NetApiServer, Web3, Web3ApiServer,
	};
	use pallet_assets_ext_rpc::{AssetsExt, AssetsExtApiServer};
	use pallet_dex_rpc::{Dex, DexApiServer};
	use pallet_nft_rpc::{Nft, NftApiServer};
	use pallet_sft_rpc::{Sft, SftApiServer};
	use pallet_sylo_data_permissions_rpc::{SyloDataPermissions, SyloDataPermissionsApiServer};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use sc_consensus_babe_rpc::{Babe, BabeApiServer};
	use sc_consensus_grandpa_rpc::{Grandpa, GrandpaApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};

	let mut io = RpcModule::new(());
	let FullDeps {
		client,
		pool,
		graph,
		select_chain,
		is_authority,
		deny_unsafe,
		network,
		filter_pool,
		frontier_backend,
		max_past_logs,
		fee_history_cache,
		fee_history_cache_limit,
		overrides,
		block_data_cache,
		ethy,
		babe,
		grandpa,
		syncing_service,
		eth_forced_parent_hashes,
	} = deps;

	let BabeDeps { babe_config, babe_worker_handle, keystore } = babe;

	let client_clone = client.clone();
	let slot_duration = babe_config.slot_duration();

	let pending_create_inherent_data_providers = move |parent, _| {
		let client_clone = client_clone.clone();
		async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

			let slot = sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
				*timestamp,
				slot_duration,
			);

			// NOTE - check if we can remove this
			let storage_proof = sp_transaction_storage_proof::registration::new_data_provider(
				&*client_clone.clone(),
				&parent,
			)?;

			Ok((slot, timestamp, storage_proof))
		}
	};

	let GrandpaDeps {
		shared_voter_state,
		shared_authority_set,
		justification_stream,
		subscription_executor,
		finality_provider,
	} = grandpa;

	// Substrate RPCs
	io.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
	io.merge(TransactionPayment::new(client.clone()).into_rpc())?;
	io.merge(
		EthyRpcHandler::new(
			ethy.event_proof_stream,
			ethy.subscription_executor,
			client.clone(),
			client.clone(),
		)
		.into_rpc(),
	)?;
	io.merge(
		Babe::new(client.clone(), babe_worker_handle.clone(), keystore, select_chain, deny_unsafe)
			.into_rpc(),
	)?;
	io.merge(
		Grandpa::new(
			subscription_executor,
			shared_authority_set.clone(),
			shared_voter_state,
			justification_stream,
			finality_provider,
		)
		.into_rpc(),
	)?;

	// The Root Network RPCs
	io.merge(Dex::new(client.clone()).into_rpc())?;
	io.merge(Nft::new(client.clone()).into_rpc())?;
	io.merge(Sft::new(client.clone()).into_rpc())?;
	io.merge(AssetsExt::new(client.clone()).into_rpc())?;
	io.merge(SyloDataPermissions::new(client.clone()).into_rpc())?;

	// Ethereum compatible RPCs
	io.merge(
		Eth::new(
			client.clone(),
			pool.clone(),
			graph.clone(),
			Some(seed_runtime::TransactionConverter),
			syncing_service.clone(),
			Default::default(), // signers
			overrides.clone(),
			frontier_backend.clone(),
			is_authority,
			block_data_cache.clone(),
			fee_history_cache,
			fee_history_cache_limit,
			10,
			eth_forced_parent_hashes,
			pending_create_inherent_data_providers,
			Some(pending_consensus_data_provider),
		)
		.into_rpc(),
	)?;

	if let Some(filter_pool) = filter_pool {
		io.merge(
			EthFilter::new(
				client.clone(),
				frontier_backend,
				graph.clone(),
				filter_pool,
				500_usize, // max stored filters
				max_past_logs,
				block_data_cache,
			)
			.into_rpc(),
		)?;
	}

	io.merge(
		EthPubSub::new(
			pool,
			client.clone(),
			syncing_service.clone(),
			subscription_task_executor,
			overrides,
			pubsub_notification_sinks,
		)
		.into_rpc(),
	)?;

	io.merge(
		Net::new(
			client.clone(),
			network,
			// Whether to format the `peer_count` response as Hex (default) or not.
			true,
		)
		.into_rpc(),
	)?;

	io.merge(Web3::new(client).into_rpc())?;

	Ok(io)
}

pub trait EventCheckerT {
	// block_hash: H256,
	// 		block_number: BlockNumber,
	fn check_event(&self, events: RuntimeEvent, block_hash: H256, block_number: BlockNumber);

	fn create_asset_transfer_log(&self, token: H160, from: AccountId, to: AccountId, amount: u64, block_hash: H256, block_number: BlockNumber, log_index: u32);
}

pub struct EventChecker;

impl<Runtime> EventCheckerT<Runtime> for EventChecker {
	// type RuntimeEvent = RuntimeEvent;
	fn check_event(&self, events: RuntimeEvent, block_hash: H256, block_number: BlockNumber) {
		let mut logs = Vec::new();

		// // Resolve block range from filter
		// // let (from_block, to_block) = self.resolve_block_range(filter.from_block, filter.to_block).await?;
		// let from_block = filter.from_block.unwrap();
		// let to_block = filter.to_block.unwrap();
		//
		// // Process each block in the range
		// for block_number in from_block..=to_block {
		//
		// 	T::BlockHashMapping::block_hash(number.as_u32())
		// 	BlockNo::Num(block_number)
		// 	let block_hash = self.client.block_hash(block_number)
		// 		.ok_or_else(|| internal_err("Block hash not found"))?;
		//
		// 	// Get all events for this block
		// 	let events = self.client.runtime_api().events(block_hash)?;

			// Process asset transfer events
			for (index, event_record) in events.into_iter().enumerate() {

				// match event_record.event {
				// 	Some(idx) => claim_ids.drain(..idx - 1),
				// 	None => claim_ids.drain(..claim_ids.len() - 1), // we need the last element to remain
				// };
				// need to convert the following if else to switch
				if let RuntimeEvent::Assets(pallet_assets_ext::Event::<Runtime>::Transferred {
												asset_id,
												from,
												to,
												amount
											}) = event_record.event {
					// Get EVM address for this asset - using AssetAddress mapping from impls.rs

					let asset_address =
						Runtime::runtime_id_to_evm_id(asset_id, ERC20_PRECOMPILE_ADDRESS_PREFIX);



					// let asset_address = match AssetAddress::<Runtime>::asset_to_address(asset_id) {
					// 	Some(address) => address,
					// 	None => continue, // Skip assets without EVM address
					// };

					// Filter by address if specified
					// if let Some(addresses) = &filter.address {
					// 	if !addresses.contains(&asset_address) {
					// 		continue;
					// 	}
					// }

					// Create synthetic log
					let log = self.create_asset_transfer_log(
						asset_address,
						from,
						to,
						amount,
						block_hash,
						block_number,
						index as u32
					)?;

					logs.push(log);
				}
				// balances.Transfer
				else if let RuntimeEvent::Balances(pallet_balances::Event::<Runtime>::Transfer {
													   from,
													   to,
													   amount
												   }) = event_record.event {
					let asset_id = 1;
					// Get EVM address for this asset - using AssetAddress mapping from impls.rs
					let asset_address: H160 = <Runtime as ErcIdConversion<AssetId>>::runtime_id_to_evm_id(
						asset_id,
						ERC20_PRECOMPILE_ADDRESS_PREFIX,
					)
						.into();
					// let asset_address = match AssetAddress::<Runtime>::asset_to_address(asset_id) {
					// 	Some(address) => address,
					// 	None => continue, // Skip assets without EVM address
					// };

					// Filter by address if specified
					// if let Some(addresses) = &filter.address {
					// 	if !addresses.contains(&asset_address) {
					// 		continue;
					// 	}
					// }

					// Create synthetic log
					let log = self.create_asset_transfer_log(
						asset_address,
						from,
						to,
						amount,
						block_hash,
						block_number,
						index as u32
					)?;

					logs.push(log);
				}
				else if let RuntimeEvent::Assets(pallet_assets::Event::<Runtime>::Burned {
													 asset_id,
													 owner,
													 balance
												 }) = event_record.event {
					let from = owner;
					let to = "0x0000000000000000000000000000000000000000";
					// Get EVM address for this asset - using AssetAddress mapping from impls.rs
					let asset_address: H160 = <Runtime as ErcIdConversion<AssetId>>::runtime_id_to_evm_id(
						asset_id,
						ERC20_PRECOMPILE_ADDRESS_PREFIX,
					)
						.into();
					// let asset_address = match AssetAddress::<Runtime>::asset_to_address(asset_id) {
					// 	Some(address) => address,
					// 	None => continue, // Skip assets without EVM address
					// };

					// Filter by address if specified
					// if let Some(addresses) = &filter.address {
					// 	if !addresses.contains(&asset_address) {
					// 		continue;
					// 	}
					// }

					// Create synthetic log
					let log = self.create_asset_transfer_log(
						asset_address,
						from,
						to,
						balance,
						block_hash,
						block_number,
						index as u32
					)?;

					logs.push(log);
				}
				else if let RuntimeEvent::Assets(pallet_assets::Event::<Runtime>::Issued {
													 asset_id,
													 owner,
													 amount
												 }) = event_record.event {
					let to = owner;
					let from = "0x0000000000000000000000000000000000000000";
					// Get EVM address for this asset - using AssetAddress mapping from impls.rs
					let asset_address: H160 = <Runtime as ErcIdConversion<AssetId>>::runtime_id_to_evm_id(
						asset_id,
						ERC20_PRECOMPILE_ADDRESS_PREFIX,
					)
						.into();
					// let asset_address = match AssetAddress::<Runtime>::asset_to_address(asset_id) {
					// 	Some(address) => address,
					// 	None => continue, // Skip assets without EVM address
					// };

					// Filter by address if specified
					// if let Some(addresses) = &filter.address {
					// 	if !addresses.contains(&asset_address) {
					// 		continue;
					// 	}
					// }

					// Create synthetic log
					let log = self.create_asset_transfer_log(
						asset_address,
						from,
						to,
						amount,
						block_hash,
						block_number,
						index as u32
					)?;

					logs.push(log);
				}
				else if let RuntimeEvent::AssetsExt(pallet_assets_ext::Event::<Runtime>::InternalWithdraw { // gas fee paid
														asset_id,
														who,
														amount
													}) = event_record.event {
					let from = owner;
					let to = "0x0000000000000000000000000000000000000000";
					// Get EVM address for this asset - using AssetAddress mapping from impls.rs
					let asset_address: H160 = <Runtime as ErcIdConversion<AssetId>>::runtime_id_to_evm_id(
						asset_id,
						ERC20_PRECOMPILE_ADDRESS_PREFIX,
					)
						.into();
					// let asset_address = match AssetAddress::<Runtime>::asset_to_address(asset_id) {
					// 	Some(address) => address,
					// 	None => continue, // Skip assets without EVM address
					// };

					// Filter by address if specified
					// if let Some(addresses) = &filter.address {
					// 	if !addresses.contains(&asset_address) {
					// 		continue;
					// 	}
					// }

					// Create synthetic log
					let log = self.create_asset_transfer_log(
						asset_address,
						from,
						to,
						amount,
						block_hash,
						block_number,
						index as u32
					)?;

					logs.push(log);
				}
				else if let RuntimeEvent::AssetsExt(pallet_assets_ext::Event::<Runtime>::InternalWithdraw { // gas fee paid
														asset_id,
														who,
														amount
													}) = event_record.event {
					let from = who;
					let to = "0x0000000000000000000000000000000000000000";
					// Get EVM address for this asset - using AssetAddress mapping from impls.rs
					let asset_address: H160 = <Runtime as ErcIdConversion<AssetId>>::runtime_id_to_evm_id(
						asset_id,
						ERC20_PRECOMPILE_ADDRESS_PREFIX,
					)
						.into();
					// let asset_address = match AssetAddress::<Runtime>::asset_to_address(asset_id) {
					// 	Some(address) => address,
					// 	None => continue, // Skip assets without EVM address
					// };

					// Filter by address if specified
					// if let Some(addresses) = &filter.address {
					// 	if !addresses.contains(&asset_address) {
					// 		continue;
					// 	}
					// }

					// Create synthetic log
					let log = self.create_asset_transfer_log(
						asset_address,
						from,
						to,
						amount,
						block_hash,
						block_number,
						index as u32
					)?;

					logs.push(log);
				}
				else if let RuntimeEvent::AssetsExt(pallet_assets_ext::Event::<Runtime>::InternalDeposit { // gas fee paid
														asset_id,
														who,
														amount
													}) = event_record.event {
					let to = who;
					let from = "0x0000000000000000000000000000000000000000";
					// Get EVM address for this asset - using AssetAddress mapping from impls.rs
					let asset_address: H160 = <Runtime as ErcIdConversion<AssetId>>::runtime_id_to_evm_id(
						asset_id,
						ERC20_PRECOMPILE_ADDRESS_PREFIX,
					)
						.into();
					// let asset_address = match AssetAddress::<Runtime>::asset_to_address(asset_id) {
					// 	Some(address) => address,
					// 	None => continue, // Skip assets without EVM address
					// };

					// Filter by address if specified
					// if let Some(addresses) = &filter.address {
					// 	if !addresses.contains(&asset_address) {
					// 		continue;
					// 	}
					// }

					// Create synthetic log
					let log = self.create_asset_transfer_log(
						asset_address,
						from,
						to,
						amount,
						block_hash,
						block_number,
						index as u32
					)?;

					logs.push(log);
				}
			}
		// }

		// Ok(logs)
	}

	fn create_asset_transfer_log(
		&self,
		token: H160,
		from: AccountId,
		to: AccountId,
		amount: AssetBalance,
		block_hash: H256,
		block_number: BlockNumber,
		log_index: u32
	) -> RpcResult<Log> {
		// Convert Substrate accounts to EVM addresses using the provided mappings
		let from_address = Runtime::AddressMapping::into_account_id(from); // Runtime::AddressMapping::into_account_id(from);
		let to_address = Runtime::AddressMapping::into_account_id(to); // Runtime::AddressMapping::into_account_id(to);

		// ERC20 Transfer event signature: keccak256("Transfer(address,address,uint256)")
		let signature = H256::from_slice(
			&hex::decode("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef")
				.expect("Valid hex")
		);

		// Create event topics
		let mut topics = Vec::new();
		topics.push(signature);
		topics.push(H256::from(from_address.as_fixed_bytes()));
		topics.push(H256::from(to_address.as_fixed_bytes()));

		// Encode amount as bytes - using the proper balance type from pallet-assets-ext
		let mut data = vec![0u8; 32];
		// Convert AssetBalance to U256 - taking into account potential decimal differences
		let evm_amount = U256::from(amount.saturated_into::<u128>());
		evm_amount.to_big_endian(&mut data);

		// Create synthetic log
		let log = Log {
			address: token,
			topics,
			data: Bytes(data),
			block_hash: Some(block_hash),
			block_number: Some(block_number.into()),
			transaction_hash: None,
			transaction_index: None,
			log_index: Some(U256::from(log_index)),
			transaction_log_index: None,
			removed: false,
		};

		Ok(log)
	}
}
