//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use futures::{future, StreamExt};

use fc_consensus::FrontierBlockImport;
use fc_db::Backend as FrontierBackend;
use fc_mapping_sync::{MappingSyncWorker, SyncStrategy};
use fc_rpc::{EthTask, OverrideHandle};
use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use sc_cli::SubstrateCli;
use sc_client_api::{Backend, BlockBackend, BlockchainEvents, ExecutorProvider};
use sc_consensus_babe::{self, SlotProportion};
pub use sc_executor::NativeElseWasmExecutor;
use sc_finality_grandpa::SharedVoterState;
use sc_keystore::LocalKeystore;
use sc_service::{error::Error as ServiceError, BasePath, Configuration, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_runtime::{offchain::OffchainStorage, traits::Block as BlockT};

use std::{
	collections::BTreeMap,
	path::PathBuf,
	sync::{Arc, Mutex},
	time::Duration,
};

use seed_primitives::{ethy::ETH_HTTP_URI, opaque::Block, XRP_HTTP_URI};
use seed_runtime::{self, RuntimeApi};

use crate::cli::Cli;

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
	sc_finality_grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>;
/// BABE block import handler (wraps `FullGrandpaBlockImport`)
type FullBabeBlockImport =
	sc_consensus_babe::BabeBlockImport<Block, FullClient, FullGrandpaBlockImport>;
/// Seed block import handler Frontier(Babe(Grandpa)))
type SeedBlockImport = FrontierBlockImport<Block, FullBabeBlockImport, FullClient>;
pub type ImportSetup = (
	SeedBlockImport,
	sc_finality_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
	sc_consensus_babe::BabeLink<Block>,
);

pub(crate) fn db_config_dir(config: &Configuration) -> PathBuf {
	config
		.base_path
		.as_ref()
		.map(|base_path| base_path.config_dir(config.chain_spec.id()))
		.unwrap_or_else(|| {
			BasePath::from_project("", "", &Cli::executable_name())
				.config_dir(config.chain_spec.id())
		})
}

pub fn new_partial(
	config: &Configuration,
	cli: &Cli,
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
			Arc<FrontierBackend<Block>>,
			Option<FilterPool>,
			(FeeHistoryCache, FeeHistoryCacheLimit),
		),
	>,
	ServiceError,
> {
	if config.keystore_remote.is_some() {
		return Err(ServiceError::Other("Remote Keystores are not supported.".into()));
	}

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

	log::info!("got telemetry");

	let executor = NativeElseWasmExecutor::<ExecutorDispatch>::new(
		config.wasm_method,
		config.default_heap_pages,
		config.max_runtime_instances,
		config.runtime_cache_size,
	);

	log::info!("got executor");

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;

	log::info!("got client, backend, keystore_container, task_manager");

	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	log::info!("spawned telemetry task");

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	log::info!("longest chain selected");

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	log::info!("got transaction pool");

	let frontier_backend =
		Arc::new(FrontierBackend::open(&config.database, &db_config_dir(config))?);

	log::info!("got frontier backend");

	let filter_pool: Option<FilterPool> = Some(Arc::new(Mutex::new(BTreeMap::new())));
	let fee_history_cache: FeeHistoryCache = Arc::new(Mutex::new(BTreeMap::new()));
	let fee_history_cache_limit: FeeHistoryCacheLimit = cli.run.fee_history_limit;

	let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import(
		client.clone(),
		&(client.clone() as Arc<_>),
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	log::info!("got grandpa block import");

	let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
		sc_consensus_babe::Config::get(&*client)?,
		grandpa_block_import.clone(),
		client.clone(),
	)?;

	log::info!("got babe block import");

	let frontier_block_import = FrontierBlockImport::new(
		babe_block_import.clone(),
		client.clone(),
		frontier_backend.clone(),
	);

	log::info!("got frontier block import");

	let slot_duration = babe_link.config().slot_duration();

	log::info!("got babe slot duration");

	let import_queue = sc_consensus_babe::import_queue(
		babe_link.clone(),
		frontier_block_import.clone(),
		Some(Box::new(grandpa_block_import)),
		client.clone(),
		select_chain.clone(),
		move |_, ()| async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

			let slot =
				sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					*timestamp,
					slot_duration,
				);

			let uncles =
				sp_authorship::InherentDataProvider::<<Block as BlockT>::Header>::check_inherents();

			Ok((timestamp, slot, uncles))
		},
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
		sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	log::info!("got import queue");

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
		),
	})
}

fn remote_keystore(_url: &String) -> Result<Arc<LocalKeystore>, &'static str> {
	// FIXME: here would the concrete keystore be built,
	//        must return a concrete type (NOT `LocalKeystore`) that
	//        implements `CryptoStore` and `SyncCryptoStore`
	Err("Remote Keystore not supported.")
}

/// Builds a new service for a full client.
pub fn new_full(mut config: Configuration, cli: &Cli) -> Result<TaskManager, ServiceError> {
	log::info!("new_full entered");
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		mut keystore_container,
		select_chain,
		transaction_pool,
		other:
			(
				mut telemetry,
				import_setup,
				frontier_backend,
				filter_pool,
				(fee_history_cache, fee_history_cache_limit),
			),
	} = new_partial(&config, cli)?;

	log::info!("Partial components initialized");

	// Set eth http bridge config
	// the config is stored into the offchain context where it can
	// be accessed later by the crml-eth-bridge offchain worker.
	if let Some(ref eth_http_uri) = cli.run.eth_http {
		backend.offchain_storage().unwrap().set(
			sp_core::offchain::STORAGE_PREFIX,
			&ETH_HTTP_URI,
			eth_http_uri.as_bytes(),
		);
	}

	log::info!("got eth http uri");

	if let Some(ref xrp_http_uri) = cli.run.xrp_http {
		backend.offchain_storage().unwrap().set(
			sp_core::offchain::STORAGE_PREFIX,
			&XRP_HTTP_URI,
			xrp_http_uri.as_bytes(),
		)
	}

	log::info!("got xrp http uri");

	if let Some(url) = &config.keystore_remote {
		match remote_keystore(url) {
			Ok(k) => keystore_container.set_remote_keystore(k),
			Err(e) => {
				return Err(ServiceError::Other(format!(
					"Error hooking up remote keystore for {}: {}",
					url, e
				)))
			},
		};
	}

	log::info!("got remote keystore interaction");

	let grandpa_protocol_name = sc_finality_grandpa::protocol_standard_name(
		&client.block_hash(0).ok().flatten().expect("Genesis block exists; qed"),
		&config.chain_spec,
	);

	log::info!("got grandpa protocol name");

	// register grandpa p2p protocol
	config
		.network
		.extra_sets
		.push(sc_finality_grandpa::grandpa_peers_set_config(grandpa_protocol_name.clone()));
	let warp_sync = Arc::new(sc_finality_grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		import_setup.1.shared_authority_set().clone(),
		Vec::default(),
	));

	log::info!("registered grandpa protocol");

	let genesis_hash = client.block_hash(0).ok().flatten().expect("Genesis block exists; qed");
	let ethy_protocol_name = ethy_gadget::protocol_standard_name(&genesis_hash, &config.chain_spec);
	config
		.network
		.extra_sets
		.push(ethy_gadget::ethy_peers_set_config(ethy_protocol_name.clone()));

	log::info!("got ethy protocol name");

	let (network, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync: Some(warp_sync),
		})?;

	log::info!("built network");

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	log::info!("offchain worker check/build");

	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks: Option<()> = None;
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();
	let overrides = crate::rpc::overrides_handle(client.clone());

	log::info!("got overrides handle");

	let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
		task_manager.spawn_handle(),
		overrides.clone(),
		50,
		50,
		prometheus_registry.clone(),
	));

	log::info!("got block data cache task");

	let (event_proof_sender, event_proof_stream) =
		ethy_gadget::notification::EthyEventProofStream::channel();

	log::info!("got ethy proof stream");

	let (block_import, grandpa_link, babe_link) = import_setup;

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let select_chain = select_chain.clone();
		let keystore = keystore_container.sync_keystore();
		let is_authority = role.is_authority();
		let network = network.clone();
		let filter_pool = filter_pool.clone();
		let frontier_backend = frontier_backend.clone();
		let overrides = overrides.clone();
		let fee_history_cache = fee_history_cache.clone();
		let max_past_logs = cli.run.max_past_logs;
		let babe_config = babe_link.config().clone();
		let shared_epoch_changes = babe_link.epoch_changes().clone();

		let justification_stream = grandpa_link.justification_stream();
		let shared_authority_set = grandpa_link.shared_authority_set().clone();
		let finality_proof_provider = sc_finality_grandpa::FinalityProofProvider::new_for_service(
			backend.clone(),
			Some(shared_authority_set.clone()),
		);

		log::info!("got finality proof provider");

		let shared_voter_state = sc_finality_grandpa::SharedVoterState::empty();

		move |deny_unsafe, subscription_task_executor: sc_rpc::SubscriptionTaskExecutor| {
			let deps = crate::rpc::FullDeps {
				backend: frontier_backend.clone(),
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
					shared_epoch_changes: shared_epoch_changes.clone(),
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
			};
			crate::rpc::create_full(deps, subscription_task_executor).map_err(Into::into)
		}
	};

	log::info!("built rpc extensions");

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network: network.clone(),
		client: client.clone(),
		keystore: keystore_container.sync_keystore(),
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_builder: Box::new(rpc_extensions_builder),
		backend: backend.clone(),
		system_rpc_tx,
		config,
		telemetry: telemetry.as_mut(),
	})?;

	log::info!("spawned rpc handlers");

	spawn_frontier_tasks(
		&task_manager,
		client.clone(),
		backend.clone(),
		frontier_backend,
		filter_pool,
		overrides,
		fee_history_cache,
		fee_history_cache_limit,
	);

	log::info!("spawned frontier tasks");

	if role.is_authority() {
		let proposer_factory = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let can_author_with =
			sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

		let client_clone = client.clone();
		let slot_duration = babe_link.config().slot_duration();
		let babe_config = sc_consensus_babe::BabeParams {
			keystore: keystore_container.sync_keystore(),
			client: client.clone(),
			select_chain,
			env: proposer_factory,
			block_import: block_import.clone(),
			sync_oracle: network.clone(),
			justification_sync_link: network.clone(),
			create_inherent_data_providers: move |parent, ()| {
				let client_clone = client_clone.clone();
				async move {
					let uncles = sc_consensus_uncles::create_uncles_inherent_data_provider(
						&*client_clone,
						parent,
					)?;

					let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

					let slot =
						sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							*timestamp,
							slot_duration,
						);

					let storage_proof =
						sp_transaction_storage_proof::registration::new_data_provider(
							&*client_clone,
							&parent,
						)?;

					Ok((timestamp, slot, uncles, storage_proof))
				}
			},
			force_authoring,
			backoff_authoring_blocks,
			babe_link,
			can_author_with,
			block_proposal_slot_portion: SlotProportion::new(0.5),
			max_block_proposal_slot_portion: None,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		log::info!("got babe config");

		let babe = sc_consensus_babe::start_babe(babe_config)?;

		log::info!("got babe task");

		task_manager.spawn_essential_handle().spawn_blocking(
			"babe-proposer",
			Some("block-authoring"),
			babe,
		);
		log::info!("spawned babe task");
	}

	// if the node isn't actively participating in consensus then it doesn't
	// need a keystore, regardless of which protocol we use below.
	let keystore =
		if role.is_authority() { Some(keystore_container.sync_keystore()) } else { None };

	let ethy_params = ethy_gadget::EthyParams {
		client: client.clone(),
		backend,
		runtime: client.clone(),
		key_store: keystore.clone(),
		network: network.clone(),
		event_proof_sender,
		prometheus_registry: prometheus_registry.clone(),
		protocol_name: ethy_protocol_name,
		_phantom: std::marker::PhantomData,
	};

	log::info!("got keystore, ethy params");

	// Start the ETHY bridge gadget.
	task_manager.spawn_essential_handle().spawn_blocking(
		"ethy-gadget",
		None,
		ethy_gadget::start_ethy_gadget::<_, _, _, _, _>(ethy_params),
	);

	log::info!("spawned ethy gadget");

	let grandpa_config = sc_finality_grandpa::Config {
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
		let grandpa_config = sc_finality_grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network,
			voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state: SharedVoterState::empty(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		log::info!("got grandpa config");

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			sc_finality_grandpa::run_grandpa_voter(grandpa_config)?,
		);
		log::info!("spawned grandpa config");
	}

	network_starter.start_network();
	log::info!("started network");
	Ok(task_manager)
}

fn spawn_frontier_tasks(
	task_manager: &TaskManager,
	client: Arc<FullClient>,
	backend: Arc<FullBackend>,
	frontier_backend: Arc<FrontierBackend<Block>>,
	filter_pool: Option<FilterPool>,
	overrides: Arc<OverrideHandle<Block>>,
	fee_history_cache: FeeHistoryCache,
	fee_history_cache_limit: FeeHistoryCacheLimit,
) {
	task_manager.spawn_essential_handle().spawn(
		"frontier-mapping-sync-worker",
		None,
		MappingSyncWorker::new(
			client.import_notification_stream(),
			Duration::new(seed_runtime::constants::MILLISECS_PER_BLOCK / 1_000, 0),
			client.clone(),
			backend,
			frontier_backend.clone(),
			3,
			0,
			SyncStrategy::Normal,
		)
		.for_each(|()| future::ready(())),
	);

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
