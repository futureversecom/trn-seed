#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as ValidatorSet;
use codec::EncodeLike;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;
use seed_primitives::{ethy::crypto::AuthorityId, AccountId};
use sp_core::crypto::ByteArray;
use sp_std::prelude::*;

benchmarks! {
	where_clause { where AuthorityId: EncodeLike<T::EthyId>,  T::EthyId: From<AuthorityId> }
	finalise_validator_set_change {
		let validator_keys = vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
		];
		let default_account = AccountId::default();
		let next_validator_keys = vec![
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
		];
		let xrpl_door_signers = vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
		];
		NotaryKeys::<T>::put(validator_keys);
		NextNotaryKeys::<T>::put(next_validator_keys);
		for signer in xrpl_door_signers {
			XrplDoorSigners::<T>::insert(signer, true);
		}
		// pallet should be in validator_set_change_in_progress
		ValidatorSet::<T>::start_validator_set_change();
		assert_eq!(ValidatorsChangeInProgress::<T>::get(), true);
	}: _(RawOrigin::None, NextNotaryKeys::<T>::get())
	verify {
		assert_eq!(ValidatorsChangeInProgress::<T>::get(), false);
	}

	set_xrpl_door_signers {
		let xrpl_door_signers = vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap().into(),
		];
	}: _(RawOrigin::Root, xrpl_door_signers.clone())
	verify {
		assert_eq!(ValidatorSet::<T>::xrpl_door_signers(xrpl_door_signers[0].clone()), true);
	}
}

impl_benchmark_test_suite!(
	ValidatorSet,
	crate::mock::ExtBuilder::default().build(),
	crate::mock::TestRuntime
);
