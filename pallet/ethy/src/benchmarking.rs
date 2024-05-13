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

use ethabi::Token;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, traits::fungibles::Mutate};
use frame_system::RawOrigin;
use sp_core::crypto::ByteArray;

use crate::Pallet as EthBridge;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

fn encode_event_message(
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
	where_clause { where <T as Config>::EthyId: ByteArray}

	set_xrpl_door_signers {
		let p in 1 .. (T::MaxNewSigners::get() as u32 - 1);
		let mut new_signers = vec![];
		for i in 0..p {
			// Generate random signer
			let slice = [i as u8; 33];
			let new_signer = T::EthyId::from_slice(&slice).unwrap();
			new_signers.push(new_signer);
		}
		let new_signers_joined = new_signers.clone().into_iter().map(|x| (x, true)).collect::<Vec<_>>();
	}: _(RawOrigin::Root, new_signers_joined.clone())
	verify {
		for signer in new_signers {
			assert_eq!(XrplDoorSigners::<T>::get(&signer), true);
		}
	}

	set_relayer {
		let relayer = account::<T>("//Alice");
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get().into(), &relayer, T::RelayerBond::get()));
		assert_ok!(EthBridge::<T>::deposit_relayer_bond(origin::<T>(&relayer).into()));
	}: _(RawOrigin::Root, relayer.clone())
	verify {
		assert_eq!(Relayer::<T>::get().unwrap(), relayer);
	}

	deposit_relayer_bond {
		let relayer = account::<T>("//Alice");
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get().into(), &relayer, T::RelayerBond::get()));
	}: _(origin::<T>(&relayer))
	verify {
		assert_eq!(RelayerPaidBond::<T>::get(relayer), T::RelayerBond::get());
	}

	withdraw_relayer_bond {
		let relayer = account::<T>("//Alice");
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get().into(), &relayer, T::RelayerBond::get()));
		assert_ok!(EthBridge::<T>::deposit_relayer_bond(origin::<T>(&relayer).into()));
	}: _(origin::<T>(&relayer))
	verify {
		assert_eq!(RelayerPaidBond::<T>::get(relayer), 0);
	}

	set_event_block_confirmations {
		let confirmations: u64 = 123;
	}: _(RawOrigin::Root, confirmations)
	verify {
		assert_eq!(EventBlockConfirmations::<T>::get(), confirmations);
	}

	set_delayed_event_proofs_per_block {
		let count: u8 = 123;
	}: _(RawOrigin::Root, count)
	verify {
		assert_eq!(DelayedEventProofsPerBlock::<T>::get(), count);
	}

	set_challenge_period {
		let blocks: T::BlockNumber = T::BlockNumber::from(100_u32);
	}: _(RawOrigin::Root, blocks)
	verify {
		assert_eq!(ChallengePeriod::<T>::get(), blocks);
	}

	set_contract_address {
		let contract_address = H160::from_low_u64_be(123);
	}: _(RawOrigin::Root, contract_address)
	verify {
		assert_eq!(ContractAddress::<T>::get(), contract_address);
	}

	set_bridge_paused {
		let paused = true;
		// Sanity check
		assert_eq!(BridgePaused::<T>::get(), !paused);
	}: _(RawOrigin::Root, paused)
	verify {
		assert_eq!(BridgePaused::<T>::get(), paused);
	}

	finalise_authorities_change {
		let next_keys = vec![
			T::EthyId::from_slice(
				hex!("03e2161ca58ac2f2fa7dfd9f6980fdda1059b467e375ee78cdd5749dc058c0b2c9")
					.as_slice(),
			).unwrap(),
		];
		let next_notary_keys = WeakBoundedVec::try_from(next_keys.clone()).unwrap();
	}: _(RawOrigin::None, next_notary_keys.clone())
	verify {
		assert_eq!(NotaryKeys::<T>::get(), next_notary_keys);
	}

	remove_missing_event_id {
		let range = (2,5);
		MissedMessageIds::<T>::put(vec![1,2,3,4,5,6]);
	}: _(RawOrigin::Root, range)
	verify {
		assert_eq!(MissedMessageIds::<T>::get(), vec![1,6]);
	}

	submit_missing_event {
		let relayer = account::<T>("//Alice");
		Relayer::<T>::put(&relayer);
		let tx_hash: H256 = EthHash::from_low_u64_be(33);
		let (event_id, source, destination, message) =
			(1_u64, H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
		let event_data = encode_event_message(event_id, source, destination, message);
		MissedMessageIds::<T>::put(vec![1]);
	}: _(origin::<T>(&relayer), tx_hash, event_data)
	verify {
		let process_at = <frame_system::Pallet<T>>::block_number() + ChallengePeriod::<T>::get();
		assert_eq!(MessagesValidAt::<T>::get(process_at).into_inner(), [event_id]);
	}

	submit_event {
		let relayer = account::<T>("//Alice");
		Relayer::<T>::put(&relayer);
		let tx_hash: H256 = EthHash::from_low_u64_be(33);
		let (event_id, source, destination, message) =
			(1_u64, H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
		let event_data = encode_event_message(event_id, source, destination, message);
	}: _(origin::<T>(&relayer), tx_hash, event_data)
	verify {
		let process_at = <frame_system::Pallet<T>>::block_number() + ChallengePeriod::<T>::get();
		assert_eq!(MessagesValidAt::<T>::get(process_at).into_inner(), [event_id]);
	}

	submit_challenge {
		let challenger = account::<T>("//Bob");
		let relayer = account::<T>("//Alice");
		Relayer::<T>::put(&relayer);
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get().into(), &challenger, T::ChallengeBond::get()));

		let tx_hash: H256 = EthHash::from_low_u64_be(33);
		let (event_id, source, destination, message) =
			(1_u64, H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
		let event_data = encode_event_message(event_id, source, destination, message);
		assert_ok!(EthBridge::<T>::submit_event(origin::<T>(&relayer).into(), tx_hash, event_data));
	}: _(origin::<T>(&challenger), event_id)
	verify {
		assert_eq!(
			PendingClaimStatus::<T>::get(event_id),
			Some(EventClaimStatus::Challenged)
		);
	}

	submit_notarization {
		let challenger = account::<T>("//Bob");
		let relayer = account::<T>("//Alice");
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get().into(), &relayer, T::RelayerBond::get()));
		assert_ok!(EthBridge::<T>::deposit_relayer_bond(origin::<T>(&relayer).into()));
		Relayer::<T>::put(&relayer);

		let tx_hash: H256 = EthHash::from_low_u64_be(33);
		let (event_id, source, destination, message) =
			(1_u64, H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
		let event_data = encode_event_message(event_id, source, destination, message);

		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get().into(), &challenger, T::ChallengeBond::get()));
		assert_ok!(EthBridge::<T>::submit_event(origin::<T>(&relayer).into(), tx_hash, event_data));
		assert_ok!(EthBridge::<T>::submit_challenge(origin::<T>(&challenger).into(), event_id));

		let result = EventClaimResult::Valid;
		let authority_index: u16 = 0;
		let notary_key = T::EthyId::from_slice(
				hex!("03e2161ca58ac2f2fa7dfd9f6980fdda1059b467e375ee78cdd5749dc058c0b2c9")
					.as_slice(),
			).unwrap();
		let notary_keys = vec![notary_key.clone()];
		let notary_keys = WeakBoundedVec::try_from(notary_keys.clone()).unwrap();
		NotaryKeys::<T>::put(notary_keys);

		let payload = NotarizationPayload::Event { event_claim_id: event_id, result, authority_index };
		let key = T::EthyId::generate_pair(None);
		let signature = key.sign(&payload.encode()).unwrap();

	}: _(RawOrigin::None, payload, signature)
	verify {
		assert_eq!(PendingClaimChallenges::<T>::get(), vec![]);
	}

	handle_authorities_change {
		let notary_key = T::EthyId::from_slice(
				hex!("03e2161ca58ac2f2fa7dfd9f6980fdda1059b467e375ee78cdd5749dc058c0b2c9")
					.as_slice(),
			).unwrap();
		let notary_keys = vec![notary_key.clone()];
		let notary_keys = WeakBoundedVec::try_from(notary_keys.clone()).unwrap();
		NotaryKeys::<T>::put(notary_keys);

		let next_notary_key = T::EthyId::from_slice(
				hex!("04e2161ca58ac2f2fa7dfd9f6980fdda1059b467e375ee78cdd5749dc058c0b2c0")
					.as_slice(),
			).unwrap();
		let next_notary_keys = vec![next_notary_key.clone()];
		let next_notary_keys = WeakBoundedVec::try_from(next_notary_keys.clone()).unwrap();
		NextNotaryKeys::<T>::put(next_notary_keys);

		NextAuthorityChange::<T>::put(T::BlockNumber::default());
		BridgePaused::<T>::put(false);
	}: {crate::Pallet::<T>::handle_authorities_change()}
	verify {
		assert!(BridgePaused::<T>::get());
		assert_eq!(NextAuthorityChange::<T>::get(), None);
	}
}

impl_benchmark_test_suite!(
	EthBridge,
	crate::mock::ExtBuilder::default().with_keystore().build(),
	crate::mock::Test
);
