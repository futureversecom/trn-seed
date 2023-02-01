#![cfg(test)]
use crate::mock::{EVMChainId, Event, ExtBuilder, Origin, System, ALICE};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};

#[test]
fn default_chain_id() {
	ExtBuilder::default().build().execute_with(|| {
		let chain_id = EVMChainId::chain_id();
		assert_eq!(chain_id, 3999);
	});
}

#[test]
fn update_chain_id() {
	ExtBuilder::default().build().execute_with(|| {
		// normal user cannot update chain id
		assert_noop!(EVMChainId::set_chain_id(Origin::signed(ALICE), 1234), BadOrigin);
		assert_eq!(EVMChainId::chain_id(), 3999); // chain id is not updated

		// root user can update chain id
		assert_ok!(EVMChainId::set_chain_id(Origin::root().into(), 1234));
		assert_eq!(EVMChainId::chain_id(), 1234); // chain id is updated

		System::assert_last_event(Event::EVMChainId(crate::Event::ChainIdSet(1234)));
	});
}
