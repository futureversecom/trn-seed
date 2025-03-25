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
use mock::{RuntimeEvent as MockEvent, SyloPermissions, System, Test};
use seed_pallet_common::test_prelude::*;

mod transact {
	use super::*;

	#[test]
	fn transact_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let call = mock::RuntimeCall::System(frame_system::Call::remark {
				remark: Default::default(),
			});

			assert_ok!(SyloPermissions::transact(
				RawOrigin::Signed(grantee.clone()).into(),
				grantor.clone(),
				Box::new(call.clone())
			));
		});
	}
}
