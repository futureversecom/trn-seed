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

use super::Event;
use crate::{
	mock::{RuntimeEvent as MockEvent, *},
};
use frame_support::{
	assert_ok,
	dispatch::{DispatchClass, GetDispatchInfo},
	traits::fungibles::Mutate,
};
use frame_system::{limits::BlockWeights, RawOrigin};
use pallet_transaction_payment::ChargeTransactionPayment;
use seed_pallet_common::CreateExt;
use sp_core::U256;
use sp_runtime::{traits::SignedExtension, Perbill};
use seed_pallet_common::test_prelude::*;

#[test]
fn set_length_multiplier_works() {
	TestExt::<Test>::default().build().execute_with(|| {

	});
}
