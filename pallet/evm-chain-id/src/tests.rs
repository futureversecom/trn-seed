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
use crate::mock::{EVMChainId, RuntimeEvent, RuntimeOrigin, System, Test, TestExt};
use crate::ChainId;
use seed_pallet_common::test_prelude::*;

#[test]
fn default_chain_id() {
	TestExt::default().build().execute_with(|| {
		let chain_id = ChainId::<Test>::get();
		assert_eq!(chain_id, 7672);
	});
}

#[test]
fn update_chain_id() {
	TestExt::default().build().execute_with(|| {
		// normal user cannot update chain id
		assert_noop!(EVMChainId::set_chain_id(RuntimeOrigin::signed(alice()), 1234), BadOrigin);
		assert_eq!(ChainId::<Test>::get(), 7672); // chain id is not updated

		// root user can update chain id
		assert_ok!(EVMChainId::set_chain_id(RuntimeOrigin::root().into(), 1234));
		assert_eq!(ChainId::<Test>::get(), 1234); // chain id is updated

		System::assert_last_event(RuntimeEvent::EVMChainId(crate::Event::ChainIdSet(1234)));
	});
}
