// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

#![cfg(test)]
use crate::mock::{EVMChainId, RuntimeEvent, RuntimeOrigin, System, TestExt, ALICE};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};

#[test]
fn default_chain_id() {
	TestExt::default().build().execute_with(|| {
		let chain_id = EVMChainId::chain_id();
		assert_eq!(chain_id, 7672);
	});
}

#[test]
fn update_chain_id() {
	TestExt::default().build().execute_with(|| {
		// normal user cannot update chain id
		assert_noop!(EVMChainId::set_chain_id(RuntimeOrigin::signed(ALICE), 1234), BadOrigin);
		assert_eq!(EVMChainId::chain_id(), 7672); // chain id is not updated

		// root user can update chain id
		assert_ok!(EVMChainId::set_chain_id(RuntimeOrigin::root().into(), 1234));
		assert_eq!(EVMChainId::chain_id(), 1234); // chain id is updated

		System::assert_last_event(RuntimeEvent::EVMChainId(crate::Event::ChainIdSet(1234)));
	});
}
