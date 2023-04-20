#![cfg(test)]
use crate::mock::{Event, Origin, System, TestExt};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};

#[test]
fn default_chain_id() {
	TestExt::default().build().execute_with(|| {
		// TODO: Add tests
	});
}
