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
use crate::mock::{ExtBuilder, TestRuntime};
use frame_support::{assert_ok, storage::StorageValue};
use seed_primitives::AccountId;
use sp_core::H160;
use sp_runtime::traits::AccountIdConversion;

#[test]
fn payout_period_id_increments() {
	ExtBuilder::default().build().execute_with(|| {});
}
