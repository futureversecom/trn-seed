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

use crate::{
	benchmarking, chain_spec,
	cli::{Cli, Subcommand},
	service,
};
use sc_cli::SubstrateCli;
use sc_service::PartialComponents;
use seed_runtime::Block;

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Seed 🌱".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/futureversecom/seed".into()
	}

	fn copyright_start_year() -> i32 {
		2017
	}

	fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"dev" => Box::new(chain_spec::dev_config()?),
			"porcini" => Box::new(chain_spec::porcini_config()?),
			"root" => Box::new(chain_spec::root_config()?),
			"" => return Err("Please specify which chain you want to run!".into()),
			path => {
				Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?)
			},
		})
	}
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let rpc_config = cli.run.new_rpc_config();
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, import_queue, .. } =
					service::new_partial(&config, &cli, &rpc_config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let rpc_config = cli.run.new_rpc_config();
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } =
					service::new_partial(&config, &cli, &rpc_config)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let rpc_config = cli.run.new_rpc_config();
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } =
					service::new_partial(&config, &cli, &rpc_config)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let rpc_config = cli.run.new_rpc_config();
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, import_queue, .. } =
					service::new_partial(&config, &cli, &rpc_config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let rpc_config = cli.run.new_rpc_config();
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, backend, .. } =
					service::new_partial(&config, &cli, &rpc_config)?;
				let aux_revert = Box::new(|client, _, blocks| {
					sc_consensus_grandpa::revert(client, blocks)?;
					Ok(())
				});
				Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
			})
		},
		Some(Subcommand::Benchmark(cmd)) => {
			use benchmarking::{inherent_benchmark_data, RemarkBuilder, TransferKeepAliveBuilder};
			use frame_benchmarking_cli::{BenchmarkCmd, ExtrinsicFactory};
			use seed_primitives::EthereumSigner;
			use seed_runtime::ExistentialDeposit;
			use sp_core::Pair;
			use sp_runtime::traits::IdentifyAccount;

			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| {
				// This switch needs to be in the client, since the client decides
				// which sub-commands it wants to support.
				match cmd {
					BenchmarkCmd::Pallet(cmd) => {
						if !cfg!(feature = "runtime-benchmarks") {
							return Err(
								"Runtime benchmarking wasn't enabled when building the node. \
							You can enable it with `--features runtime-benchmarks`."
									.into(),
							);
						}

						cmd.run::<Block, ()>(config)
					},
					BenchmarkCmd::Block(cmd) => {
						let rpc_config = cli.run.new_rpc_config();
						let PartialComponents { client, .. } =
							service::new_partial(&config, &cli, &rpc_config)?;
						cmd.run(client)
					},
					#[cfg(not(feature = "runtime-benchmarks"))]
					BenchmarkCmd::Storage(_) => Err(
						"Storage benchmarking can be enabled with `--features runtime-benchmarks`."
							.into(),
					),
					#[cfg(feature = "runtime-benchmarks")]
					BenchmarkCmd::Storage(cmd) => {
						let rpc_config = cli.run.new_rpc_config();
						let PartialComponents { client, backend, .. } =
							service::new_partial(&config, &cli, &rpc_config)?;
						let db = backend.expose_db();
						let storage = backend.expose_storage();

						cmd.run(config, client, db, storage)
					},
					BenchmarkCmd::Overhead(cmd) => {
						let rpc_config = cli.run.new_rpc_config();
						let PartialComponents { client, .. } =
							service::new_partial(&config, &cli, &rpc_config)?;
						let ext_builder = RemarkBuilder::new(client.clone());

						cmd.run(
							config,
							client,
							inherent_benchmark_data()?,
							Vec::new(),
							&ext_builder,
						)
					},
					BenchmarkCmd::Extrinsic(cmd) => {
						let rpc_config = cli.run.new_rpc_config();
						let PartialComponents { client, .. } =
							service::new_partial(&config, &cli, &rpc_config)?;
						// Register the *Remark* and *TKA* builders.
						let alice: sp_core::ecdsa::Pair =
							sp_core::ecdsa::Pair::from_string("//Alice", None)
								.expect("convert Pair from String");
						let alice = EthereumSigner::from(alice.public()).into_account();

						let ext_factory = ExtrinsicFactory(vec![
							Box::new(RemarkBuilder::new(client.clone())),
							Box::new(TransferKeepAliveBuilder::new(
								client.clone(),
								alice,
								ExistentialDeposit::get(),
							)),
						]);

						cmd.run(client, inherent_benchmark_data()?, Vec::new(), &ext_factory)
					},
					_ => panic!("Unsupported subcommand"),
				}
			})
		},
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(cmd)) => {
			use crate::service::ExecutorDispatch;
			use frame_benchmarking::frame_support::sp_io;
			use sc_executor::{sp_wasm_interface::ExtendedHostFunctions, NativeExecutionDispatch};
			use seed_runtime::constants::SLOT_DURATION;
			use try_runtime_cli::block_building_info::substrate_info;

			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				// we don't need any of the components of new_partial, just a runtime, or a task
				// manager to do `async_run`.
				let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
				let task_manager =
					sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
						.map_err(|e| sc_cli::Error::Service(sc_service::Error::Prometheus(e)))?;
				let info_provider = substrate_info(SLOT_DURATION);
				Ok((
					cmd.run::<Block, ExtendedHostFunctions<
						sp_io::SubstrateHostFunctions,
						<ExecutorDispatch as NativeExecutionDispatch>::ExtendHostFunctions,
					>, _>(Some(info_provider)),
					task_manager,
				))
			})
		},
		#[cfg(not(feature = "try-runtime"))]
		Some(Subcommand::TryRuntime) => Err("TryRuntime wasn't enabled when building the node. \
				You can enable it with `--features try-runtime`."
			.into()),
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run::<Block>(&config))
		},
		Some(Subcommand::VerifyProofSig(cmd)) => cmd.run(&cli),
		None => {
			let runner = cli.create_runner(&cli.run.base)?;
			runner.run_node_until_exit(|config| async move {
				let rpc_config = cli.run.new_rpc_config();
				service::new_full(config, &cli, &rpc_config).map_err(sc_cli::Error::Service)
			})
		},
	}
}
