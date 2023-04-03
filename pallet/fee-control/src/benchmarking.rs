// /* Copyright 2019-2021 Centrality Investments Limited
// *
// * Licensed under the LGPL, Version 3.0 (the "License");
// * you may not use this file except in compliance with the License.
// * Unless required by applicable law or agreed to in writing, software
// * distributed under the License is distributed on an "AS IS" BASIS,
// * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// * See the License for the specific language governing permissions and
// * limitations under the License.
// * You may obtain a copy of the License at the root of this project source code,
// * or at:
// * https://centrality.ai/licenses/gplv3.txt
// * https://centrality.ai/licenses/lgplv3.txt
// */
#![cfg(feature = "runtime-benchmarks")]

use super::*;

#[allow(unused_imports)]
use crate::Pallet as FeeControl;

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;
use sp_core::U256;

benchmarks! {
	set_evm_base_fee {
	}: _(RawOrigin::Root, U256::one())

	set_extrinsic_weight_to_fee_factor {
	}: _(RawOrigin::Root, Perbill::one())
}

impl_benchmark_test_suite!(FeeControl, crate::mock::TestExt::default().build(), crate::mock::Test);
