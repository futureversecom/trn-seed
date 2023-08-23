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

#![cfg(test)]
use super::*;
use crate::mock::{ExtBuilder, StakingPayout, System, TestRuntime};
use frame_support::{assert_ok, storage::StorageValue, traits::Hooks};
use seed_primitives::{AccountId, AccountId20};
use sp_core::H160;
use sp_runtime::traits::AccountIdConversion;

fn alice() -> AccountId {
	AccountId20([1; 20])
}

#[test]
fn payout_period_id_increments() {
	ExtBuilder::default().build().execute_with(|| {});
}

#[test]
fn iterates_per_block_validators() {
	ExtBuilder::default().build().execute_with(|| {
		let block = System::block_number();
		StakingPayout::on_initialize(block);
		assert_eq!(CurrentValidatorIter::<TestRuntime>::get(), Some(alice()));
	});
}
