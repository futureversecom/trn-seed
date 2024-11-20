// Copyright 2019-2022 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonbeam.  If not, see <http://www.gnu.org/licenses/>.

/// Available frontier backend types.
#[derive(Debug, Copy, Clone, Default, clap::ValueEnum)]
pub enum FrontierBackendType {
	/// Either RocksDb or ParityDb as per inherited from the global backend settings.
	#[default]
	KeyValue,
	/// Sql database with custom log indexing.
	Sql,
}

/// Defines the frontier backend configuration.
#[derive(Default)]
pub enum FrontierBackendConfig {
	#[default]
	KeyValue,
	Sql {
		pool_size: u32,
		num_ops_timeout: u32,
		thread_count: u32,
		cache_size: u64,
	},
}

pub struct RpcConfig {
	pub frontier_backend_config: FrontierBackendConfig,
}
