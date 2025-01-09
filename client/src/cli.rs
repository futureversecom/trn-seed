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

use crate::{cli_opt, custom_commands::VerifyProofSigSubCommand};
use clap::ArgAction;

#[allow(missing_docs)]
#[derive(Debug, clap::Parser)]
#[group(skip)]
pub struct RunCmd {
	#[allow(missing_docs)]
	#[clap(flatten)]
	pub base: sc_cli::RunCmd,

	/// Sets the frontier backend type (KeyValue or Sql)
	#[arg(long, value_enum, ignore_case = true, default_value_t = cli_opt::FrontierBackendType::default())]
	pub frontier_backend_type: cli_opt::FrontierBackendType,

	// Sets the SQL backend's pool size.
	#[arg(long, default_value = "100")]
	pub frontier_sql_backend_pool_size: u32,

	/// Sets the SQL backend's query timeout in number of VM ops.
	#[arg(long, default_value = "10000000")]
	pub frontier_sql_backend_num_ops_timeout: u32,

	/// Sets the SQL backend's auxiliary thread limit.
	#[arg(long, default_value = "4")]
	pub frontier_sql_backend_thread_count: u32,

	/// Sets the SQL backend's query timeout in number of VM ops.
	/// Default value is 200MB.
	#[arg(long, default_value = "209715200")]
	pub frontier_sql_backend_cache_size: u64,

	/// Maximum number of logs in a query (EVM).
	#[clap(long, default_value = "10000")]
	pub max_past_logs: u32,

	/// Maximum fee history cache size (EVM).
	#[clap(long, default_value = "2048")]
	pub fee_history_limit: u64,

	/// Ethereum JSON-RPC client endpoint
	#[clap(long = "eth-http")]
	pub eth_http: Option<String>,

	/// XRP JSON-RPC client endpoint
	// NOTE - check flags works as expected.
	#[clap(long = "xrp-http")]
	pub xrp_http: Option<String>,

	/// Option to disable the ethy p2p protocol
	/// p2p protocol is enabled by default
	#[clap(
		long = "ethy-p2p",
		default_missing_value("true"),
		default_value("true"),
		action = ArgAction::Set,
	)]
	pub ethy_p2p: bool,
}

impl RunCmd {
	pub fn new_rpc_config(&self) -> cli_opt::RpcConfig {
		cli_opt::RpcConfig {
			frontier_backend_config: match self.frontier_backend_type {
				cli_opt::FrontierBackendType::KeyValue => cli_opt::FrontierBackendConfig::KeyValue,
				cli_opt::FrontierBackendType::Sql => cli_opt::FrontierBackendConfig::Sql {
					pool_size: self.frontier_sql_backend_pool_size,
					num_ops_timeout: self.frontier_sql_backend_num_ops_timeout,
					thread_count: self.frontier_sql_backend_thread_count,
					cache_size: self.frontier_sql_backend_cache_size,
				},
			},
		}
	}
}

#[derive(Debug, clap::Parser)]
pub struct Cli {
	#[clap(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub run: RunCmd,
}

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Subcommand {
	/// Key management cli utilities
	#[clap(subcommand)]
	Key(sc_cli::KeySubcommand),

	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(sc_cli::RevertCmd),

	/// Sub-commands concerned with benchmarking.
	#[clap(subcommand)]
	Benchmark(frame_benchmarking_cli::BenchmarkCmd),

	/// Try some command against runtime state.
	#[cfg(feature = "try-runtime")]
	TryRuntime(try_runtime_cli::TryRuntimeCmd),

	/// Try some command against runtime state. Note: `try-runtime` feature must be enabled.
	#[cfg(not(feature = "try-runtime"))]
	TryRuntime,

	/// Db meta columns information.
	ChainInfo(sc_cli::ChainInfoCmd),

	/// verify proof signatures
	#[clap(subcommand)]
	VerifyProofSig(VerifyProofSigSubCommand),
}
