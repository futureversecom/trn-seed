#![cfg(feature = "runtime-benchmarks")]

use super::*;
#[cfg(test)]
use crate::mock::MockValidatorSetAdapter;
use crate::Pallet as EthBridge;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_noop, assert_ok, assert_storage_noop, traits::fungibles::Mutate};
use frame_system::RawOrigin;
use sp_std::prelude::*;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

pub fn transfer_funds<T: Config>(account: &T::AccountId, amount: Balance) {
	let asset_id = T::NativeAssetId::get();
	assert_ok!(T::MultiCurrency::mint_into(asset_id, account.into(), (amount as u32).into(),));
}

pub fn setup_relayer<T: Config>(relayer: T::AccountId) {
	let relayer_bond = T::RelayerBond::get();
	transfer_funds::<T>(&relayer, relayer_bond);
	assert_ok!(EthBridge::<T>::deposit_relayer_bond(RawOrigin::Signed(relayer.clone()).into()));
	assert_ok!(EthBridge::<T>::set_relayer(RawOrigin::Root.into(), relayer));
}

/// Ethereum ABI encode an event message according to the 1.5 standard
pub fn encode_event_message(
	event_id: EventClaimId,
	source: H160,
	destination: H160,
	message: &[u8],
) -> Vec<u8> {
	ethabi::encode(&[
		Token::Uint(event_id.into()),
		Token::Address(source),
		Token::Address(destination),
		Token::Bytes(message.to_vec()),
	])
}

benchmarks! {
	set_relayer {
		// make sure relayer bond is already paid
		// let's make alice the relayer
		let relayer = account::<T>("Alice");
		let relayer_bond = T::RelayerBond::get();
		transfer_funds::<T>(&relayer, relayer_bond);

		assert_ok!(EthBridge::<T>::deposit_relayer_bond(RawOrigin::Signed(relayer).into()));
		assert_eq!(EthBridge::<T>::relayer_bond(relayer), relayer_bond);
		assert_eq!(Relayer::<T>::get(), None);

	}: _(RawOrigin::Root, AccountId::from(relayer))
	verify {
		assert_eq!(Relayer::<T>::get(), Some(relayer));
	}

	deposit_relayer_bond {
		let relayer = account::<T>("Alice");
		let relayer_bond = T::RelayerBond::get();
		transfer_funds::<T>(&relayer, relayer_bond);
		assert_eq!(EthBridge::<T>::relayer_bond(relayer), Balance::default());
	}: _(RawOrigin::Signed(relayer))
	verify {
		assert_eq!(EthBridge::<T>::relayer_bond(relayer), relayer_bond);
	}

	withdraw_relayer_bond {
		let relayer = account::<T>("Alice");
		let relayer_bond = T::RelayerBond::get();
		transfer_funds::<T>(&relayer, relayer_bond);
		assert_ok!(EthBridge::<T>::deposit_relayer_bond(RawOrigin::Signed(relayer).into()));
		assert_eq!(EthBridge::<T>::relayer_bond(relayer), relayer_bond);
	}: _(RawOrigin::Signed(relayer))
	verify {
		assert_eq!(EthBridge::<T>::relayer_bond(relayer), Balance::default());
	}

	set_event_block_confirmations {
		let event_block_confirmations = 10_u64;
		assert_eq!(EventBlockConfirmations::<T>::get(), 3_u64); // default value
	}: _(RawOrigin::Root, event_block_confirmations)
	verify {
		assert_eq!(EventBlockConfirmations::<T>::get(), event_block_confirmations);
	}

	set_challenge_period {
		let challenge_period = T::BlockNumber::from(10_u32);
		assert_eq!(ChallengePeriod::<T>::get(), 150_u32.into()); // default value
	}: _(RawOrigin::Root, challenge_period)
	verify {
		assert_eq!(ChallengePeriod::<T>::get(), challenge_period);
	}

	set_contract_address {
		let contract_address = H160::from_low_u64_be(1);
		assert_eq!(ContractAddress::<T>::get(), H160::default());
	}: _(RawOrigin::Root, contract_address)
	verify {
		assert_eq!(ContractAddress::<T>::get(), contract_address);
	}

	submit_event {
		// set the relayer
		let relayer = account::<T>("Alice");
		setup_relayer::<T>(relayer);
		let tx_hash = H256::from_low_u64_be(33);
		let (event_id, source, destination, message) =
			(1_u64, H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
		let event_data = encode_event_message(event_id, source, destination, message);
	}: _(RawOrigin::Signed(relayer), tx_hash, event_data)
	verify {
		let event_claim = EventClaim {
			tx_hash,
			source: source,
			destination: destination,
			data: message.to_vec(),
		};
		assert_eq!(PendingEventClaims::<T>::get(event_id), Some(event_claim));
	}

	submit_challenge {
		// Add claim
		let challenger = account::<T>("Alice");
		let challenger_bond = T::ChallengeBond::get();
		transfer_funds::<T>(&challenger, challenger_bond);
		let event_id = 1;
		let event_claim = EventClaim {
			tx_hash: H256::default(),
			source: EthAddress::default(),
			destination: EthAddress::default(),
			data: Vec::<u8>::default(),
		};

		PendingEventClaims::<T>::insert(event_id, &event_claim);
		PendingClaimStatus::<T>::insert(event_id, EventClaimStatus::Pending);

	}: _(RawOrigin::Signed(challenger), event_id)
	verify {
		assert_eq!(PendingClaimStatus::<T>::get(event_id), Some(EventClaimStatus::Challenged));
	}

	submit_notarization {
		let relayer = H160::from_low_u64_be(1);
		let challenger = H160::from_low_u64_be(2);
		let validators = vec![AuthorityId::generate_pair(None), AuthorityId::generate_pair(None)];
		let event_id = 1;
		setup_relayer::<T>(relayer.into());
		transfer_funds::<T>(&challenger.into(), T::ChallengeBond::get());
		// set the validators to the mock db
		#[cfg(test)]
		{
			MockValidatorSetAdapter::add_to_validator_set(&validators[0]);
			MockValidatorSetAdapter::add_to_validator_set(&validators[0]);
		}

		let event_claim = EventClaim {
			tx_hash: H256::default(),
			source: EthAddress::default(),
			destination: EthAddress::default(),
			data: Vec::<u8>::default(),
		};
		PendingEventClaims::<T>::insert(event_id, &event_claim);
		PendingClaimStatus::<T>::insert(event_id, EventClaimStatus::Pending);
		assert_ok!(EthBridge::<T>::submit_challenge(RawOrigin::Signed(challenger.into()).into(), event_id));
		PendingClaimStatus::<T>::insert(event_id, EventClaimStatus::Challenged);

		let notarization_payload = NotarizationPayload::Event {
			event_claim_id: event_id,
			authority_index: 0, // signed by first validator
			result: EventClaimResult::Valid,
		};
		let signature = validators[0].sign(&notarization_payload.encode()).ok_or("couldn't make signature")?;

	}: _(RawOrigin::None, notarization_payload, signature)
	verify {
		assert_eq!(EventNotarizations::<T>::get(event_id, validators[0].clone()), Some(EventClaimResult::Valid));
	}
}

impl_benchmark_test_suite!(
	EthBridge,
	crate::mock::ExtBuilder::default().with_keystore().build(),
	crate::mock::TestRuntime
);
