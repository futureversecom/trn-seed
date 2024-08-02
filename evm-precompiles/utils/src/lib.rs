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

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod constants;
pub mod costs;
pub mod handle;
pub mod logs;
pub mod modifier;
pub mod precompile_set;
pub mod revert;
pub mod substrate;

#[cfg(feature = "testing")]
pub mod solidity;

#[cfg(feature = "testing")]
pub mod testing;

#[cfg(test)]
mod tests;

use crate::alloc::{borrow::ToOwned, vec::Vec};
use fp_evm::{ExitRevert, ExitSucceed, PrecompileFailure, PrecompileHandle, PrecompileOutput};

pub mod data;

pub use data::{Address, Bytes, EvmData, EvmDataReader, EvmDataWriter};
pub use fp_evm::Precompile;
pub use precompile_utils_macro::{generate_function_selector, keccak256};

/// Generated a `PrecompileFailure::Revert` with proper encoding for the output.
/// If the revert needs improved formatting such as backtraces, `Revert` type should
/// be used instead.
#[must_use]
pub fn revert(output: impl AsRef<[u8]>) -> PrecompileFailure {
	PrecompileFailure::Revert { exit_status: ExitRevert::Reverted, output: encoded_revert(output) }
}

pub fn encoded_revert(output: impl AsRef<[u8]>) -> Vec<u8> {
	EvmDataWriter::new_with_selector(revert::RevertSelector::Generic)
		.write::<Bytes>(Bytes(output.as_ref().to_owned()))
		.build()
}

#[must_use]
pub fn succeed(output: impl AsRef<[u8]>) -> PrecompileOutput {
	PrecompileOutput { exit_status: ExitSucceed::Returned, output: output.as_ref().to_owned() }
}

/// returns the first four bytes or zero if less than four bytes
pub fn get_selector(call_data: &[u8]) -> [u8; 4] {
	if call_data.len() < 4 {
		return [0_u8; 4];
	}

	call_data[0..4].try_into().unwrap_or_default()
}

/// Alias for Result returning an EVM precompile error.
pub type EvmResult<T = ()> = Result<T, PrecompileFailure>;

/// Trait similar to `fp_evm::Precompile` but with a `&self` parameter to manage some
/// state (this state is only kept in a single transaction and is lost afterward).
pub trait StatefulPrecompile {
	/// Instanciate the precompile.
	/// Will be called once when building the PrecompileSet at the start of each
	/// Ethereum transaction.
	fn new() -> Self;

	/// Execute the precompile with a reference to its state.
	fn execute(&self, handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput>;
}

/// Convert EVM addresses into Runtime Id identifiers and vice versa
pub trait ErcIdConversion<RuntimeId> {
	/// ID type used by EVM
	type EvmId;
	/// Get runtime Id from EVM id
	fn evm_id_to_runtime_id(
		evm_id: Self::EvmId,
		precompile_address_prefix: &[u8; 4],
	) -> Option<RuntimeId>;
	/// Get EVM id from runtime Id
	fn runtime_id_to_evm_id(
		runtime_id: RuntimeId,
		precompile_address_prefix: &[u8; 4],
	) -> Self::EvmId;
}

pub mod prelude {
	pub use crate::{
		data::{Address, BoundedBytes, BoundedVec, Bytes, EvmData, EvmDataReader, EvmDataWriter},
		handle::PrecompileHandleExt,
		logs::{log0, log1, log2, log3, log4, LogExt},
		modifier::{check_function_modifier, FunctionModifier},
		read_args, read_struct, revert,
		revert::{BacktraceExt, InjectBacktrace, MayRevert, Revert, RevertExt, RevertReason},
		substrate::{RuntimeHelper, TryDispatchError},
		succeed, ErcIdConversion, EvmResult, StatefulPrecompile,
	};
	pub use pallet_evm::PrecompileHandle;
	pub use precompile_utils_macro::{generate_function_selector, keccak256};
}
