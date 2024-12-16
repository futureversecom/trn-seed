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

use super::*;
use hex::encode;
use mock::{Dex, RuntimeEvent as MockEvent, RuntimeOrigin, System, Test, TestExt};
use seed_pallet_common::test_prelude::*;
use sp_arithmetic::helpers_128bit::sqrt;
use std::str::FromStr;

#[test]
fn test_run() {
	TestExt.build().execute_with(|| assert_eq!(1, 1));
}
