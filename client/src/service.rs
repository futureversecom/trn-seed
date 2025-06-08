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

//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use fc_consensus::FrontierBlockImport;
use fc_db::{Backend as FrontierBackend, DatabaseSource};
use fc_mapping_sync::{kv::MappingSyncWorker, SyncStrategy};
use fc_rpc::{EthTask, OverrideHandle};
use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use futures::{future, FutureExt, StreamExt};
use sc_client_api::{
	AuxStore, Backend, BlockBackend, BlockchainEvents, StateBackend, StorageProvider,
};
use sc_consensus_babe::{self, BabeWorkerHandle, SlotProportion};
use sc_consensus_grandpa::SharedVoterState;
pub use sc_executor::NativeElseWasmExecutor;
use sc_network_sync::SyncingService;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager, WarpSyncParams};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_runtime::{offchain::OffchainStorage, traits::BlakeTwo256};
use std::{
	collections::BTreeMap,
	path::Path,
	sync::{Arc, Mutex},
	time::Duration,
};

use seed_primitives::{ethy::ETH_HTTP_URI, opaque::Block, XRP_HTTP_URI};
use seed_runtime::{self, RuntimeApi};

use crate::{
	cli::Cli,
	cli_opt::{FrontierBackendConfig, RpcConfig},
	consensus_data_providers::BabeConsensusDataProvider,
};

// Our native executor instance.
pub struct ExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for ExecutorDispatch {
	/// Only enable the benchmarking host functions when we actually want to benchmark.
	#[cfg(feature = "runtime-benchmarks")]
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;
	/// Otherwise we only use the default Substrate host functions.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type ExtendHostFunctions = ();

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		seed_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		seed_runtime::native_version()
	}
}

pub(crate) type FullClient =
	sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
/// Grandpa block import handler
type FullGrandpaBlockImport =
	sc_consensus_grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>;
/// BABE block import handler (wraps `FullGrandpaBlockImport`)
type FullBabeBlockImport =
	sc_consensus_babe::BabeBlockImport<Block, FullClient, FullGrandpaBlockImport>;
/// Seed block import handler Frontier(Babe(Grandpa)))
type SeedBlockImport = FrontierBlockImport<Block, FullBabeBlockImport, FullClient>;
pub type ImportSetup = (
	SeedBlockImport,
	sc_consensus_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
	sc_consensus_babe::BabeLink<Block>,
);

pub fn frontier_database_dir(config: &Configuration, path: &str) -> std::path::PathBuf {
	config.base_path.config_dir(config.chain_spec.id()).join("frontier").join(path)
}

pub fn open_frontier_backend<C, BE>(
	client: Arc<C>,
	config: &Configuration,
	rpc_config: &RpcConfig,
) -> Result<fc_db::Backend<Block>, String>
where
	C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
	C: Send + Sync + 'static,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
{
	let frontier_backend = match rpc_config.frontier_backend_config {
		FrontierBackendConfig::KeyValue => {
			fc_db::Backend::KeyValue(fc_db::kv::Backend::<Block>::new(
				client,
				&fc_db::kv::DatabaseSettings {
					source: match config.database {
						DatabaseSource::RocksDb { .. } => DatabaseSource::RocksDb {
							path: frontier_database_dir(config, "db"),
							cache_size: 0,
						},
						DatabaseSource::ParityDb { .. } => DatabaseSource::ParityDb {
							path: frontier_database_dir(config, "paritydb"),
						},
						DatabaseSource::Auto { .. } => DatabaseSource::Auto {
							rocksdb_path: frontier_database_dir(config, "db"),
							paritydb_path: frontier_database_dir(config, "paritydb"),
							cache_size: 0,
						},
						_ => {
							return Err(
								"Supported db sources: `rocksdb` | `paritydb` | `auto`".to_string()
							)
						},
					},
				},
			)?)
		},
		FrontierBackendConfig::Sql { pool_size, num_ops_timeout, thread_count, cache_size } => {
			let overrides = crate::rpc::overrides_handle(client.clone());
			let sqlite_db_path = frontier_database_dir(config, "sql");
			std::fs::create_dir_all(&sqlite_db_path).expect("failed creating sql db directory");
			let backend = futures::executor::block_on(fc_db::sql::Backend::new(
				fc_db::sql::BackendConfig::Sqlite(fc_db::sql::SqliteBackendConfig {
					path: Path::new("sqlite:///")
						.join(sqlite_db_path)
						.join("frontier.db3")
						.to_str()
						.expect("frontier sql path error"),
					create_if_missing: true,
					thread_count,
					cache_size,
				}),
				pool_size,
				std::num::NonZeroU32::new(num_ops_timeout),
				overrides.clone(),
			))
			.unwrap_or_else(|err| panic!("failed creating sql backend: {:?}", err));
			fc_db::Backend::Sql(backend)
		},
	};
	Ok(frontier_backend)
}

pub fn new_partial(
	config: &Configuration,
	cli: &Cli,
	rpc_config: &RpcConfig,
) -> Result<
	sc_service::PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		sc_consensus::DefaultImportQueue<Block, FullClient>,
		sc_transaction_pool::FullPool<Block, FullClient>,
		(
			Option<Telemetry>,
			ImportSetup,
			FrontierBackend<Block>,
			Option<FilterPool>,
			(FeeHistoryCache, FeeHistoryCacheLimit),
			BabeWorkerHandle<Block>,
		),
	>,
	ServiceError,
> {
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor = NativeElseWasmExecutor::<ExecutorDispatch>::new(
		config.wasm_method,
		config.default_heap_pages,
		config.max_runtime_instances,
		config.runtime_cache_size,
	);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;

	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});
	let select_chain = sc_consensus::LongestChain::new(backend.clone());
	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let frontier_backend = open_frontier_backend(client.clone(), config, rpc_config)?;

	let filter_pool: Option<FilterPool> = Some(Arc::new(Mutex::new(BTreeMap::new())));
	let fee_history_cache: FeeHistoryCache = Arc::new(Mutex::new(BTreeMap::new()));
	let fee_history_cache_limit: FeeHistoryCacheLimit = cli.run.fee_history_limit;

	let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
		client.clone(),
		&(client.clone() as Arc<_>),
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
		sc_consensus_babe::configuration(&*client)?,
		grandpa_block_import.clone(),
		client.clone(),
	)?;

	let frontier_block_import = FrontierBlockImport::new(babe_block_import.clone(), client.clone());

	let slot_duration = babe_link.config().slot_duration();

	let (import_queue, babe_worker_handle) =
		sc_consensus_babe::import_queue(sc_consensus_babe::ImportQueueParams {
			link: babe_link.clone(),
			block_import: frontier_block_import.clone(),
			justification_import: Some(Box::new(grandpa_block_import)),
			client: client.clone(),
			select_chain: select_chain.clone(),
			create_inherent_data_providers: move |_, ()| async move {
				let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

				let slot = sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
				*timestamp,
				slot_duration,
			);

				Ok((slot, timestamp))
			},
			spawner: &task_manager.spawn_essential_handle(),
			registry: config.prometheus_registry(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool.clone()),
		})?;

	let import_setup = (frontier_block_import, grandpa_link, babe_link);

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (
			telemetry,
			import_setup,
			frontier_backend,
			filter_pool,
			(fee_history_cache, fee_history_cache_limit),
			babe_worker_handle,
		),
	})
}

/// Builds a new service for a full client.
pub fn new_full(
	config: Configuration,
	cli: &Cli,
	rpc_config: &RpcConfig,
) -> Result<TaskManager, ServiceError> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other:
			(
				mut telemetry,
				import_setup,
				frontier_backend,
				filter_pool,
				(fee_history_cache, fee_history_cache_limit),
				babe_worker_handle,
			),
	} = new_partial(&config, cli, rpc_config)?;

	let hwbench = (true)
		.then_some(config.database.path().map(|database_path| {
			let _ = std::fs::create_dir_all(&database_path);
			sc_sysinfo::gather_hwbench(Some(database_path))
		}))
		.flatten();

	if let Some(hwbench) = hwbench {
		sc_sysinfo::print_hwbench(&hwbench);
	}

	// Set eth http bridge config
	// the config is stored into the offchain context where it can
	// be accessed later by the crml-eth-bridge offchain worker.
	if let Some(ref eth_http_uri) = cli.run.eth_http {
		backend
			.offchain_storage()
			.expect("Failed to retrieve offchain storage handle")
			.set(sp_core::offchain::STORAGE_PREFIX, &ETH_HTTP_URI, eth_http_uri.as_bytes());
	}

	if let Some(ref xrp_http_uri) = cli.run.xrp_http {
		backend
			.offchain_storage()
			.expect("Failed to retrieve offchain storage handle")
			.set(sp_core::offchain::STORAGE_PREFIX, &XRP_HTTP_URI, xrp_http_uri.as_bytes())
	}

	let mut net_config = sc_network::config::FullNetworkConfiguration::new(&config.network);

	// register grandpa p2p protocol
	let grandpa_protocol_name = sc_consensus_grandpa::protocol_standard_name(
		&client.block_hash(0).ok().flatten().expect("Genesis block exists; qed"),
		&config.chain_spec,
	);
	net_config.add_notification_protocol(sc_consensus_grandpa::grandpa_peers_set_config(
		grandpa_protocol_name.clone(),
	));

	// register ethy p2p protocol
	let genesis_hash = client.block_hash(0).ok().flatten().expect("Genesis block exists; qed");
	let ethy_protocol_name = ethy_gadget::protocol_standard_name(&genesis_hash, &config.chain_spec);
	if cli.run.ethy_p2p {
		net_config.add_notification_protocol(ethy_gadget::ethy_peers_set_config(
			ethy_protocol_name.clone(),
		));
	}

	let warp_sync = Arc::new(sc_consensus_grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		import_setup.1.shared_authority_set().clone(),
		Vec::default(),
	));

	let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_params: Some(WarpSyncParams::WithProvider(warp_sync)),
		})?;

	if config.offchain_worker.enabled {
		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-work",
			sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
				runtime_api_provider: client.clone(),
				keystore: Some(keystore_container.keystore()),
				offchain_db: backend.offchain_storage(),
				transaction_pool: Some(OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				)),
				network_provider: network.clone(),
				is_validator: config.role.is_authority(),
				enable_http_requests: true,
				custom_extensions: move |_| vec![],
			})
			.run(client.clone(), task_manager.spawn_handle())
			.boxed(),
		);
	}

	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks: Option<()> = None;
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();
	let overrides = crate::rpc::overrides_handle(client.clone());

	let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
		task_manager.spawn_handle(),
		overrides.clone(),
		50,
		50,
		prometheus_registry.clone(),
	));

	let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
		fc_mapping_sync::EthereumBlockNotification<Block>,
	> = Default::default();
	let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

	let (event_proof_sender, event_proof_stream) =
		ethy_gadget::notification::EthyEventProofStream::channel();

	let (block_import, grandpa_link, babe_link) = import_setup;
	let rpc_extensions_builder = {
		let client = client.clone();
		let backend = backend.clone();
		let pool = transaction_pool.clone();
		let select_chain = select_chain.clone();
		let keystore = keystore_container.keystore();
		let is_authority = role.is_authority();
		let network = network.clone();
		let sync_service = sync_service.clone();
		let filter_pool = filter_pool.clone();
		let frontier_backend = frontier_backend.clone();
		let overrides = overrides.clone();
		let fee_history_cache = fee_history_cache.clone();
		let max_past_logs = cli.run.max_past_logs;
		let babe_config = babe_link.config().clone();
		let pubsub_notification_sinks = pubsub_notification_sinks.clone();

		let justification_stream = grandpa_link.justification_stream();
		let shared_authority_set = grandpa_link.shared_authority_set().clone();
		let finality_proof_provider = sc_consensus_grandpa::FinalityProofProvider::new_for_service(
			backend.clone(),
			Some(shared_authority_set.clone()),
		);
		let shared_voter_state = sc_consensus_grandpa::SharedVoterState::empty();

		move |deny_unsafe, subscription_task_executor: sc_rpc::SubscriptionTaskExecutor| {
			let deps = crate::rpc::FullDeps {
				frontier_backend: match frontier_backend.clone() {
					fc_db::Backend::KeyValue(b) => Arc::new(b),
					fc_db::Backend::Sql(b) => Arc::new(b),
				},
				block_data_cache: block_data_cache.clone(),
				client: client.clone(),
				fee_history_cache: fee_history_cache.clone(),
				fee_history_cache_limit,
				filter_pool: filter_pool.clone(),
				graph: pool.pool().clone(),
				select_chain: select_chain.clone(),
				is_authority,
				max_past_logs,
				network: network.clone(),
				overrides: overrides.clone(),
				pool: pool.clone(),
				deny_unsafe,
				babe: crate::rpc::BabeDeps {
					babe_config: babe_config.clone(),
					babe_worker_handle: babe_worker_handle.clone(),
					keystore: keystore.clone(),
				},
				ethy: crate::rpc::EthyDeps {
					event_proof_stream: event_proof_stream.clone(),
					subscription_executor: subscription_task_executor.clone(),
				},
				grandpa: crate::rpc::GrandpaDeps {
					shared_voter_state: shared_voter_state.clone(),
					shared_authority_set: shared_authority_set.clone(),
					justification_stream: justification_stream.clone(),
					subscription_executor: subscription_task_executor.clone(),
					finality_provider: finality_proof_provider.clone(),
				},
				syncing_service: sync_service.clone(),
				eth_forced_parent_hashes: None,
			};

			crate::rpc::create_full(
				deps,
				subscription_task_executor,
				pubsub_notification_sinks.clone(),
				Box::new(BabeConsensusDataProvider::new()),
			)
			.map_err(Into::into)
		}
	};

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network: network.clone(),
		client: client.clone(),
		keystore: keystore_container.keystore(),
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_builder: Box::new(rpc_extensions_builder),
		backend: backend.clone(),
		system_rpc_tx,
		config,
		telemetry: telemetry.as_mut(),
		sync_service: sync_service.clone(),
		tx_handler_controller,
	})?;

	spawn_frontier_tasks(
		&task_manager,
		client.clone(),
		backend.clone(),
		frontier_backend.clone(),
		filter_pool,
		overrides,
		fee_history_cache,
		fee_history_cache_limit,
		sync_service.clone(),
		pubsub_notification_sinks.clone(),
	);

	if role.is_authority() {
		let proposer_factory = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let client_clone = client.clone();
		let slot_duration = babe_link.config().slot_duration();
		let babe_config = sc_consensus_babe::BabeParams {
			keystore: keystore_container.keystore(),
			client: client.clone(),
			select_chain,
			env: proposer_factory,
			block_import: block_import.clone(),
			sync_oracle: sync_service.clone(),
			justification_sync_link: sync_service.clone(),
			create_inherent_data_providers: move |parent, ()| {
				let client_clone = client_clone.clone();
				async move {
					let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

					let slot = sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
						*timestamp,
						slot_duration,
					);

					// NOTE - check if we can remove this
					let storage_proof =
						sp_transaction_storage_proof::registration::new_data_provider(
							&*client_clone,
							&parent,
						)?;

					Ok((slot, timestamp, storage_proof))
				}
			},
			force_authoring,
			backoff_authoring_blocks,
			babe_link,
			block_proposal_slot_portion: SlotProportion::new(0.5),
			max_block_proposal_slot_portion: None,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		let babe = sc_consensus_babe::start_babe(babe_config)?;

		task_manager.spawn_essential_handle().spawn_blocking(
			"babe-proposer",
			Some("block-authoring"),
			babe,
		);
	}

	// if the node isn't actively participating in consensus then it doesn't
	// need a keystore, regardless of which protocol we use below.
	let keystore = if role.is_authority() { Some(keystore_container.keystore()) } else { None };

	if cli.run.ethy_p2p {
		let ethy_params = ethy_gadget::EthyParams {
			client: client.clone(),
			backend,
			runtime: client.clone(),
			key_store: keystore.clone(),
			network: network.clone(),
			event_proof_sender,
			prometheus_registry: prometheus_registry.clone(),
			protocol_name: ethy_protocol_name,
			sync_service: sync_service.clone(),
			_phantom: std::marker::PhantomData,
		};

		// Start the ETHY bridge gadget.
		task_manager.spawn_essential_handle().spawn_blocking(
			"ethy-gadget",
			None,
			ethy_gadget::start_ethy_gadget::<_, _, _, _, _, _>(ethy_params),
		);
	}

	let grandpa_config = sc_consensus_grandpa::Config {
		gossip_duration: Duration::from_millis(333),
		justification_period: 512,
		name: Some(name),
		observer_enabled: false,
		keystore,
		local_role: role,
		telemetry: telemetry.as_ref().map(|x| x.handle()),
		protocol_name: grandpa_protocol_name,
	};

	if enable_grandpa {
		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_config = sc_consensus_grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network,
			sync: sync_service.clone(),
			voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state: SharedVoterState::empty(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			sc_consensus_grandpa::run_grandpa_voter(grandpa_config)?,
		);
	}

	network_starter.start_network();
	Ok(task_manager)
}

#[allow(clippy::too_many_arguments)]
fn spawn_frontier_tasks(
	task_manager: &TaskManager,
	client: Arc<FullClient>,
	backend: Arc<FullBackend>,
	frontier_backend: FrontierBackend<Block>,
	filter_pool: Option<FilterPool>,
	overrides: Arc<OverrideHandle<Block>>,
	fee_history_cache: FeeHistoryCache,
	fee_history_cache_limit: FeeHistoryCacheLimit,
	sync_service: Arc<SyncingService<Block>>,
	pubsub_notification_sinks: Arc<
		fc_mapping_sync::EthereumBlockNotificationSinks<
			fc_mapping_sync::EthereumBlockNotification<Block>,
		>,
	>,
) {
	// Maps emulated ethereum data to substrate native data.
	match frontier_backend {
		fc_db::Backend::KeyValue(b) => {
			task_manager.spawn_essential_handle().spawn(
				"frontier-mapping-sync-worker",
				None,
				MappingSyncWorker::new(
					client.import_notification_stream(),
					Duration::new(seed_runtime::constants::MILLISECS_PER_BLOCK / 1_000, 0),
					client.clone(),
					backend,
					overrides.clone(),
					Arc::new(b),
					3,
					0,
					SyncStrategy::Normal,
					sync_service,
					pubsub_notification_sinks,
				)
				.for_each(|()| future::ready(())),
			);
		},
		fc_db::Backend::Sql(b) => {
			task_manager.spawn_essential_handle().spawn_blocking(
				"frontier-mapping-sync-worker",
				None,
				fc_mapping_sync::sql::SyncWorker::run(
					client.clone(),
					backend,
					Arc::new(b),
					client.import_notification_stream(),
					fc_mapping_sync::sql::SyncWorkerConfig {
						read_notification_timeout: Duration::from_secs(10),
						check_indexed_blocks_interval: Duration::from_secs(60),
					},
					SyncStrategy::Normal,
					sync_service,
					pubsub_notification_sinks,
				),
			);
		},
	}

	// Spawn Frontier EthFilterApi maintenance task.
	if let Some(filter_pool) = filter_pool {
		// Each filter is allowed to stay in the pool for 100 blocks.
		const FILTER_RETAIN_THRESHOLD: u64 = 100;
		task_manager.spawn_essential_handle().spawn(
			"frontier-filter-pool",
			None,
			EthTask::filter_pool_task(client.clone(), filter_pool, FILTER_RETAIN_THRESHOLD),
		);
	}

	// Spawn Frontier FeeHistory cache maintenance task.
	task_manager.spawn_essential_handle().spawn(
		"frontier-fee-history",
		None,
		EthTask::fee_history_task(
			client.clone(),
			overrides,
			fee_history_cache,
			fee_history_cache_limit,
		),
	);
}
