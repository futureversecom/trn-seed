/* Copyright 2021 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */

use super::*;
use frame_support::{
	migration,
	traits::{GetStorageVersion, PalletInfoAccess, StorageVersion},
	weights::{constants::RocksDbWeight as DbWeight, Weight},
	IterableStorageDoubleMap, IterableStorageMap, StorageHasher, StorageMap, Twox64Concat,
};
use sp_runtime::traits::Zero;

#[allow(dead_code)]
pub mod v0_storage {}

pub fn try_migrate<T: Config>() -> Weight {
	let current = Pallet::<T>::current_storage_version();
	let onchain = Pallet::<T>::on_chain_storage_version();
	log::info!("Running migration with current storage version {current:?} / onchain {onchain:?}");
	if onchain == 0 {
		StorageVersion::new(1).put::<Pallet<T>>();
		let mut weight: Weight = 0;
		weight += move_to_eth_bridge_pallet::<T>();
		weight += move_to_validator_set_pallet::<T>();
		weight += move_to_ethy_pallet::<T>();
		clear_storage_prefixes();

		weight
	} else {
		Zero::zero()
	}
}

fn move_to_eth_bridge_pallet<T: Config>() -> Weight {
	let mut weight: Weight = 0;
	// direct move to pallet-eth-bridge
	// no migration is required since former pallet-ethy's name was EthBridge in the runtime.
	// following storage items have been moved to pallet-eth-bridge without any modification
	// ChallengerAccount
	// ChallengePeriod
	// ContractAddress
	// EventBlockConfirmations
	// EventNotarizations
	// PendingEventClaims
	// PendingClaimChallenges
	// PendingClaimStatus
	// ProcessedMessageIds
	// MessagesValidAt
	// NextEthCallId
	// Relayer
	// EthCallRequests
	// EthCallNotarizations
	// EthCallNotarizationsAggregated
	// EthCallRequestInfo

	// following has been moved to pallet-eth-bridge and name changed
	// RelayerPaidBond -> RelayerBond
	{
		let stored_data: Vec<_> = migration::storage_key_iter::<T::AccountId, u128, Twox64Concat>(
			b"EthBridge",
			b"RelayerPaidBond",
		)
		.collect();

		for (account, bond) in stored_data.clone().clone() {
			migration::put_storage_value(
				b"EthBridge",
				b"RelayerBond",
				&Twox64Concat::hash(&account.encode()),
				bond,
			);
		}
		weight += DbWeight::get()
			.reads_writes(stored_data.len() as Weight + 1, stored_data.len() as Weight + 1);
	}
	weight
}

fn move_to_validator_set_pallet<T: Config>() -> Weight {
	let mut weight: Weight = 0;
	// direct move to pallet-validator-set
	migration::move_storage_from_pallet(b"NextNotaryKeys", b"EthBridge", b"ValidatorSet");
	migration::move_storage_from_pallet(b"NotaryKeys", b"EthBridge", b"ValidatorSet");
	migration::move_storage_from_pallet(b"NotaryXrplKeys", b"EthBridge", b"ValidatorSet");
	migration::move_storage_from_pallet(b"XrplDoorSigners", b"EthBridge", b"ValidatorSet");
	migration::move_storage_from_pallet(b"NotarySetId", b"EthBridge", b"ValidatorSet");

	// following has been moved to pallet-validator-set and name changed
	// NextAuthorityChange -> NextValidatorSetChangeBlock
	{
		let stored_data: Vec<(_, T::BlockNumber)> =
			migration::storage_iter(b"EthBridge", b"NextAuthorityChange").collect();

		for (_, block_number) in stored_data.clone() {
			migration::put_storage_value(
				b"ValidatorSet",
				b"NextValidatorSetChangeBlock",
				b"",
				block_number,
			);
		}
		weight += DbWeight::get()
			.reads_writes(stored_data.len() as Weight + 1, stored_data.len() as Weight + 1);
	}
	weight
}

fn move_to_ethy_pallet<T: Config>() -> Weight {
	let mut weight: Weight = 0;
	// direct move to ethy-pallet
	migration::move_storage_from_pallet(b"NextEventProofId", b"EthBridge", b"Ethy");
	migration::move_storage_from_pallet(b"NotarySetProofId", b"EthBridge", b"Ethy");
	migration::move_storage_from_pallet(b"XrplNotarySetProofId", b"EthBridge", b"Ethy");

	// following has been moved to pallet-ethy and name changed
	// DelayedEventProofsPerBlock -> DelayedProofRequestsPerBlock
	{
		let stored_data: Vec<(_, u8)> =
			migration::storage_iter(b"EthBridge", b"DelayedEventProofsPerBlock").collect();

		for (_, blocks) in stored_data.clone() {
			migration::put_storage_value(b"Ethy", b"DelayedProofRequestsPerBlock", b"", blocks);
		}
		weight += DbWeight::get()
			.reads_writes(stored_data.len() as Weight + 1, stored_data.len() as Weight + 1);
	}
	// PendingEventProofs -> PendingProofRequests
	{
		let stored_data: Vec<_> = migration::storage_key_iter::<
			u64,
			EthySigningRequest,
			Twox64Concat,
		>(b"EthBridge", b"PendingEventProofs")
		.collect();

		for (proof_id, request) in stored_data.clone() {
			migration::put_storage_value(
				b"Ethy",
				b"PendingProofRequests",
				&Twox64Concat::hash(&proof_id.encode()),
				request,
			);
		}
		weight += DbWeight::get()
			.reads_writes(stored_data.len() as Weight + 1, stored_data.len() as Weight + 1);
	}
	// BridgePaused -> EthyState
	{
		let stored_data: Vec<(_, bool)> =
			migration::storage_iter(b"EthBridge", b"BridgePaused").collect();

		for (_, paused) in stored_data.clone() {
			let state = if paused { State::Paused } else { State::Active };
			migration::put_storage_value(b"Ethy", b"EthyState", b"", state);
		}
		weight += DbWeight::get()
			.reads_writes(stored_data.len() as Weight + 1, stored_data.len() as Weight + 1);
	}
	weight
}

fn clear_storage_prefixes() {
	// The following items needs to be deleted from the storage
	// RelayerPaidBond
	let res = frame_support::migration::clear_storage_prefix(
		b"EthBridge",
		b"RelayerPaidBond",
		b"",
		None,
		None,
	);
	if res.maybe_cursor.is_some() {
		log::error!("RelayerPaidBond storage item removal was not completed");
	} else {
		log::info!("RelayerPaidBond storage item successfully removed")
	};
	// NextAuthorityChange
	let res = frame_support::migration::clear_storage_prefix(
		b"EthBridge",
		b"NextAuthorityChange",
		b"",
		None,
		None,
	);
	if res.maybe_cursor.is_some() {
		log::error!("NextAuthorityChange storage item removal was not completed");
	} else {
		log::info!("NextAuthorityChange storage item successfully removed")
	};
	// AuthoritiesChangedThisEra
	let res = frame_support::migration::clear_storage_prefix(
		b"EthBridge",
		b"AuthoritiesChangedThisEra",
		b"",
		None,
		None,
	);
	if res.maybe_cursor.is_some() {
		log::error!("AuthoritiesChangedThisEra storage item removal was not completed");
	} else {
		log::info!("AuthoritiesChangedThisEra storage item successfully removed")
	};
	// BridgePaused
	let res = frame_support::migration::clear_storage_prefix(
		b"EthBridge",
		b"BridgePaused",
		b"",
		None,
		None,
	);
	if res.maybe_cursor.is_some() {
		log::error!("BridgePaused storage item removal was not completed");
	} else {
		log::info!("BridgePaused storage item successfully removed")
	};
	// DelayedEventProofsPerBlock
	let res = frame_support::migration::clear_storage_prefix(
		b"EthBridge",
		b"DelayedEventProofsPerBlock",
		b"",
		None,
		None,
	);
	if res.maybe_cursor.is_some() {
		log::error!("DelayedEventProofsPerBlock storage item removal was not completed");
	} else {
		log::info!("DelayedEventProofsPerBlock storage item successfully removed")
	};
	// PendingEventProofs
	let res = frame_support::migration::clear_storage_prefix(
		b"EthBridge",
		b"PendingEventProofs",
		b"",
		None,
		None,
	);
	if res.maybe_cursor.is_some() {
		log::error!("PendingEventProofs storage item removal was not completed");
	} else {
		log::info!("PendingEventProofs storage item successfully removed")
	};
}
