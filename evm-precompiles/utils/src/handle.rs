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

use crate::{data::EvmDataReader, modifier::FunctionModifier, revert::MayRevert, EvmResult};
use fp_evm::{Log, PrecompileHandle};

pub trait PrecompileHandleExt: PrecompileHandle {
	/// Record cost of a log manually.
	/// This can be useful to record log costs early when their content have static size.
	fn record_log_costs_manual(&mut self, topics: usize, data_len: usize) -> EvmResult;

	/// Record cost of logs.
	fn record_log_costs(&mut self, logs: &[&Log]) -> EvmResult;

	/// Check that a function call is compatible with the context it is
	/// called into.
	fn check_function_modifier(&self, modifier: FunctionModifier) -> MayRevert;

	/// Read the selector from the input data.
	fn read_selector<T>(&self) -> MayRevert<T>
	where
		T: num_enum::TryFromPrimitive<Primitive = u32>;

	/// Returns a reader of the input, skipping the selector.
	fn read_after_selector(&self) -> MayRevert<EvmDataReader>;
}

impl<T: PrecompileHandle> PrecompileHandleExt for T {
	/// Record cost of a log manualy.
	/// This can be useful to record log costs early when their content have static size.
	fn record_log_costs_manual(&mut self, topics: usize, data_len: usize) -> EvmResult {
		self.record_cost(crate::costs::log_costs(topics, data_len)?)?;

		Ok(())
	}

	/// Record cost of logs.
	fn record_log_costs(&mut self, logs: &[&Log]) -> EvmResult {
		for log in logs {
			self.record_log_costs_manual(log.topics.len(), log.data.len())?;
		}

		Ok(())
	}

	/// Check that a function call is compatible with the context it is
	/// called into.
	fn check_function_modifier(&self, modifier: FunctionModifier) -> MayRevert {
		crate::modifier::check_function_modifier(self.context(), self.is_static(), modifier)
	}

	/// Read the selector from the input data.
	fn read_selector<S>(&self) -> MayRevert<S>
	where
		S: num_enum::TryFromPrimitive<Primitive = u32>,
	{
		EvmDataReader::read_selector(self.input())
	}

	/// Returns a reader of the input, skipping the selector.
	fn read_after_selector(&self) -> MayRevert<EvmDataReader> {
		EvmDataReader::new_skip_selector(self.input())
	}
}
