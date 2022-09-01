//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]
use std::sync::Arc;

use jsonrpsee::RpcModule;
// Substrate
use sc_client_api::{
	backend::{AuxStore, Backend, StateBackend, StorageProvider},
	client::BlockchainEvents,
};
use sc_network::NetworkService;
use sc_rpc::SubscriptionTaskExecutor;
use sc_rpc_api::DenyUnsafe;
use sc_service::TransactionPool;
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_runtime::traits::BlakeTwo256;
// Frontier
use fc_rpc::{EthBlockDataCacheTask, OverrideHandle, RuntimeApiStorageOverride};
use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};

// Runtime
use ethy_gadget::notification::EthyEventProofStream;
use ethy_gadget_rpc::{EthyApiServer, EthyRpcHandler};
use seed_primitives::{opaque::Block, AccountId, Balance, Hash, Index};

/// Extra RPC deps for Ethy
pub struct EthyDeps {
	/// Receives notifications about event proofs from Ethy.
	pub event_proof_stream: EthyEventProofStream,
	/// Executor to drive the subscription manager in the Ethy RPC handler.
	pub subscription_executor: SubscriptionTaskExecutor,
}

/// Full client dependencies.
pub struct FullDeps<C, P, A: ChainApi> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Graph pool instance.
	pub graph: Arc<Pool<A>>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// The Node authority flag
	pub is_authority: bool,
	/// Network service
	pub network: Arc<NetworkService<Block, Hash>>,
	/// EthFilterApi pool.
	pub filter_pool: Option<FilterPool>,
	/// Backend.
	pub backend: Arc<fc_db::Backend<Block>>,
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
}

pub fn overrides_handle<C, BE>(client: Arc<C>) -> Arc<OverrideHandle<Block>>
where
	C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
	C: Send + Sync + 'static,
	C::Api: sp_api::ApiExt<Block>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>
		+ fp_rpc::ConvertTransactionRuntimeApi<Block>,
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
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
pub fn create_full<C, P, BE, A>(
	deps: FullDeps<C, P, A>,
	subscription_task_executor: SubscriptionTaskExecutor,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
	C: BlockchainEvents<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
	C: Send + Sync + 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
	C::Api: BlockBuilder<Block>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	P: TransactionPool<Block = Block> + 'static,
	A: ChainApi<Block = Block> + 'static,
{
	use fc_rpc::{
		Eth, EthApiServer, EthFilter, EthFilterApiServer, EthPubSub, EthPubSubApiServer, Net,
		NetApiServer, Web3, Web3ApiServer,
	};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};

	let mut io = RpcModule::new(());
	let FullDeps {
		client,
		pool,
		graph,
		is_authority,
		deny_unsafe,
		network,
		filter_pool,
		backend,
		max_past_logs,
		fee_history_cache,
		fee_history_cache_limit,
		overrides,
		block_data_cache,
		ethy,
	} = deps;

	// Substrate RPCs
	io.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
	io.merge(TransactionPayment::new(client.clone()).into_rpc())?;
	io.merge(
		EthyRpcHandler::new(ethy.event_proof_stream, ethy.subscription_executor, client.clone())
			.into_rpc(),
	)?;

	// Ethereum compatible RPCs
	io.merge(
		Eth::new(
			client.clone(),
			pool.clone(),
			graph,
			Some(seed_runtime::TransactionConverter),
			network.clone(),
			Default::default(), // signers
			overrides.clone(),
			backend.clone(),
			is_authority,
			block_data_cache.clone(),
			fee_history_cache,
			fee_history_cache_limit,
			10,
		)
		.into_rpc(),
	)?;

	if let Some(filter_pool) = filter_pool {
		io.merge(
			EthFilter::new(
				client.clone(),
				backend,
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
			network.clone(),
			subscription_task_executor,
			overrides,
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
