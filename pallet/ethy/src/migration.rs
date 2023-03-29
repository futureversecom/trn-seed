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
	traits::{GetStorageVersion, StorageVersion},
	weights::{constants::RocksDbWeight as DbWeight, Weight},
	StorageHasher, Twox64Concat,
};
use log::warn;
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
		let Some(stored_data) = migration::get_storage_value::<BlockNumber>(
			b"EthBridge",
			b"NextAuthorityChange",
			b"",
		) else {
			warn!("Old Ethy migration - NextAuthorityChange not found.");
			return weight
		};
		migration::put_storage_value(
			b"ValidatorSet",
			b"NextValidatorSetChangeBlock",
			b"",
			stored_data,
		);
		weight += DbWeight::get().reads_writes(1 as Weight, 1 as Weight);
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
		if let Some(stored_data) =
			migration::get_storage_value::<u8>(b"EthBridge", b"DelayedEventProofsPerBlock", b"")
		{
			migration::put_storage_value(
				b"Ethy",
				b"DelayedProofRequestsPerBlock",
				b"",
				stored_data,
			);
			weight += DbWeight::get().reads_writes(1 as Weight, 1 as Weight);
		} else {
			warn!("Old Ethy migration - DelayedEventProofsPerBlock not found.");
		};
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
		if let Some(stored_data) =
			migration::get_storage_value::<bool>(b"EthBridge", b"BridgePaused", b"")
		{
			migration::put_storage_value(b"Ethy", b"EthyState", b"", stored_data);
			weight += DbWeight::get().reads_writes(1 as Weight, 1 as Weight);
		} else {
			warn!("Old Ethy migration - BridgePaused not found.");
		};
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

#[allow(dead_code)]
mod storage_v0 {
	use super::*;
	use codec::{Decode, Encode};
	use ethereum_types::H256;
	use frame_support::pallet_prelude::TypeInfo;

	/// Possible outcomes from attempting to verify an Ethereum event claim
	#[derive(Decode, Encode, Debug, PartialEq, Clone, TypeInfo)]
	pub enum EventClaimResult {
		/// It's valid
		Valid,
		/// Couldn't request data from the Eth client
		DataProviderErr,
		/// The eth tx is marked failed
		TxStatusFailed,
		/// The transaction recipient was not the expected contract
		UnexpectedContractAddress,
		/// The expected tx logs were not present
		NoTxLogs,
		/// Not enough block confirmations yet
		NotEnoughConfirmations,
		/// Tx event logs indicated this claim does not match the event
		UnexpectedData,
		/// The Tx Receipt was not present
		NoTxReceipt,
		/// The event source did not match the tx receipt `to` field
		UnexpectedSource,
	}

	/// The ethereum address data type
	pub type EthAddress = seed_primitives::EthAddress;
	/// The ethereum transaction hash type
	pub type EthHash = H256;
	pub type EthCallId = u64;

	#[derive(Debug, Default, Clone, PartialEq, Eq, Decode, Encode, TypeInfo)]
	/// Info required to claim an Ethereum event
	pub struct EventClaim {
		/// The Ethereum transaction hash which caused the event
		pub tx_hash: EthHash,
		/// The source address (contract) which posted the event
		pub source: EthAddress,
		/// The destination address (contract) which should receive the event
		/// It may be symbolic, mapping to a pallet vs. a deployed contract
		pub destination: EthAddress,
		/// The Ethereum ABI encoded event data as logged on Ethereum
		pub data: Vec<u8>,
	}

	#[derive(Decode, Encode, Debug, PartialEq, Clone, TypeInfo)]
	pub enum EventClaimStatus {
		/// The event is awaiting processing after the challenge period
		Pending,
		/// The event has been challenged and is awaiting notarization
		Challenged,
		/// The event has been challenged and has been proven to be valid
		/// This event will now be processed after the challenge period
		ProvenValid,
	}

	#[derive(Encode, Decode, Debug, Eq, PartialOrd, Ord, PartialEq, Copy, Clone, TypeInfo)]
	pub enum CheckedEthCallResult {
		/// returndata obtained, ethereum block number, ethereum timestamp
		Ok([u8; 32], u64, u64),
		/// returndata obtained, exceeds length limit
		ReturnDataExceedsLimit,
		/// returndata obtained, empty
		ReturnDataEmpty,
		/// Failed to retrieve all the required data from Ethereum
		DataProviderErr,
		/// Ethereum block number is invalid (0, max)
		InvalidEthBlock,
		/// Timestamps have desynced or are otherwise invalid
		InvalidTimestamp,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		migration::storage_v0::{
			CheckedEthCallResult, EthCallId, EthHash, EventClaim, EventClaimResult,
			EventClaimStatus,
		},
		mock::{AccountId, Block, ExtBuilder, TestRuntime},
	};
	use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};
	use seed_primitives::{Balance, EthAddress};
	use sp_core::{ByteArray, H160};
	use std::collections::BTreeMap;

	#[test]
	fn migrate_v0_to_v1_vanished() {
		ExtBuilder::default().build().execute_with(|| {
			assert_eq!(StorageVersion::get::<Pallet<TestRuntime>>(), 0);
			// Add values to v0 storage
			migration::put_storage_value(b"EthBridge", b"AuthoritiesChangedThisEra", b"", true);

			// Run upgrade
			<Pallet<TestRuntime> as OnRuntimeUpgrade>::on_runtime_upgrade();

			assert_eq!(StorageVersion::get::<Pallet<TestRuntime>>(), 1);

			// check deleted storage items
			// AuthoritiesChangedThisEra
			assert_eq!(
				migration::get_storage_value::<bool>(
					b"EthBridge",
					b"AuthoritiesChangedThisEra",
					b""
				),
				None
			);
			assert_eq!(
				migration::get_storage_value::<bool>(b"Ethy", b"AuthoritiesChangedThisEra", b""),
				None
			);
		});
	}

	#[test]
	fn migrate_v0_to_v1_moved_to_eth_bridge() {
		ExtBuilder::default().build().execute_with(|| {
			let challenger_account = H160::from_low_u64_be(2);

			assert_eq!(StorageVersion::get::<Pallet<TestRuntime>>(), 0);
			// Add values to v0 storage(old EthBridge)

			migration::put_storage_value(
				b"EthBridge",
				b"ChallengerAccount",
				&Twox64Concat::hash(&(1 as EventClaimId).encode()),
				(AccountId::from(challenger_account), 1 as Balance),
			);
			migration::put_storage_value(
				b"EthBridge",
				b"ChallengePeriod",
				b"",
				BlockNumber::from(150_u32),
			);
			let eth_contract_address = EthAddress::from_low_u64_be(10);
			migration::put_storage_value(
				b"EthBridge",
				b"ContractAddress",
				b"",
				eth_contract_address,
			);
			migration::put_storage_value(b"EthBridge", b"EventBlockConfirmations", b"", 3_u64);
			let event_claim_result = storage_v0::EventClaimResult::Valid;
			let authority_id = [1_u8; 33];
			let mut event_notarization_comb_key = Twox64Concat::hash(&(1 as EventClaimId).encode());
			event_notarization_comb_key.extend_from_slice(&Twox64Concat::hash(
				&AuthorityId::from_slice(&authority_id).encode(),
			));
			migration::put_storage_value(
				b"EthBridge",
				b"EventNotarizations",
				&event_notarization_comb_key,
				event_claim_result.clone(),
			);
			let event_claim = storage_v0::EventClaim {
				tx_hash: EthHash::from_low_u64_be(33),
				source: H160::from_low_u64_be(1),
				destination: H160::from_low_u64_be(2),
				data: vec![1_u8; 50],
			};
			migration::put_storage_value(
				b"EthBridge",
				b"PendingEventClaims",
				&Twox64Concat::hash(&(1 as EventClaimId).encode()),
				event_claim.clone(),
			);
			let pending_claim_challenges = vec![1 as EventClaimId, 2 as EventClaimId];
			migration::put_storage_value(
				b"EthBridge",
				b"PendingClaimChallenges",
				b"",
				pending_claim_challenges.clone(),
			);
			migration::put_storage_value(
				b"EthBridge",
				b"PendingClaimStatus",
				&Twox64Concat::hash(&(1 as EventProofId).encode()),
				storage_v0::EventClaimStatus::Pending,
			);
			let processed_mssge_ids = vec![1 as EventClaimId, 2 as EventClaimId];
			migration::put_storage_value(
				b"EthBridge",
				b"ProcessedMessageIds",
				b"",
				processed_mssge_ids.clone(),
			);
			let messages_valid_at = vec![1 as EventClaimId, 2 as EventClaimId];
			migration::put_storage_value(
				b"EthBridge",
				b"MessagesValidAt",
				&Twox64Concat::hash(&(1 as BlockNumber).encode()),
				messages_valid_at.clone(),
			);
			migration::put_storage_value(
				b"EthBridge",
				b"NextEthCallId",
				b"",
				1 as storage_v0::EthCallId,
			);
			let relayer_address = H160::from_low_u64_be(10);
			migration::put_storage_value(
				b"EthBridge",
				b"Relayer",
				b"",
				AccountId::from(relayer_address),
			);
			migration::put_storage_value(
				b"EthBridge",
				b"RelayerPaidBond",
				&Twox64Concat::hash(&AccountId::from(relayer_address).encode()),
				10 as Balance,
			);
			let eth_call_requests = vec![1 as storage_v0::EthCallId, 2 as storage_v0::EthCallId];
			migration::put_storage_value(
				b"EthBridge",
				b"EthCallRequests",
				b"",
				eth_call_requests.clone(),
			);
			let eth_call_result = storage_v0::CheckedEthCallResult::Ok([1_u8; 32], 100, 100);
			let authority_id = [1_u8; 33];
			let mut call_notarization_comb_key = Twox64Concat::hash(&(1 as EventClaimId).encode());
			call_notarization_comb_key.extend_from_slice(&Twox64Concat::hash(
				&AuthorityId::from_slice(&authority_id).encode(),
			));
			migration::put_storage_value(
				b"EthBridge",
				b"EthCallNotarizations",
				&call_notarization_comb_key,
				eth_call_result,
			);
			let mut eth_call_aggregated = BTreeMap::<storage_v0::CheckedEthCallResult, u32>::new();
			eth_call_aggregated.insert(eth_call_result, 5);
			eth_call_aggregated.insert(storage_v0::CheckedEthCallResult::InvalidEthBlock, 10);
			migration::put_storage_value(
				b"EthBridge",
				b"EthCallNotarizationsAggregated",
				&Twox64Concat::hash(&(1 as EthCallId).encode()),
				eth_call_aggregated.clone(),
			);
			migration::put_storage_value(
				b"EthBridge",
				b"EthCallRequestInfo",
				&Twox64Concat::hash(&(1 as EthCallId).encode()),
				eth_call_result,
			);

			// Run upgrade
			<Pallet<TestRuntime> as OnRuntimeUpgrade>::on_runtime_upgrade();
			assert_eq!(StorageVersion::get::<Pallet<TestRuntime>>(), 1);

			// moved items to pallet-eth-bridge should still be available via "EthBrige"
			// ChallengerAccount
			assert_eq!(
				migration::get_storage_value::<(AccountId, Balance)>(
					b"EthBridge",
					b"ChallengerAccount",
					&Twox64Concat::hash(&(1 as EventClaimId).encode())
				),
				Some((AccountId::from(challenger_account), 1 as Balance))
			);
			// ChallengePeriod
			assert_eq!(
				migration::get_storage_value::<BlockNumber>(b"EthBridge", b"ChallengePeriod", b"",),
				Some(BlockNumber::from(150_u32))
			);
			// ContractAddress
			assert_eq!(
				migration::get_storage_value::<EthAddress>(b"EthBridge", b"ContractAddress", b"",),
				Some(eth_contract_address)
			);
			// EventBlockConfirmations
			assert_eq!(
				migration::get_storage_value::<u64>(b"EthBridge", b"EventBlockConfirmations", b"",),
				Some(3_u64)
			);
			// EventNotarizations
			assert_eq!(
				migration::get_storage_value::<EventClaimResult>(
					b"EthBridge",
					b"EventNotarizations",
					&event_notarization_comb_key
				),
				Some(event_claim_result)
			);
			// PendingEventClaims
			assert_eq!(
				migration::get_storage_value::<EventClaim>(
					b"EthBridge",
					b"PendingEventClaims",
					&Twox64Concat::hash(&(1 as EventClaimId).encode())
				),
				Some(event_claim)
			);
			// PendingClaimChallenges
			assert_eq!(
				migration::get_storage_value::<Vec<EventClaimId>>(
					b"EthBridge",
					b"PendingClaimChallenges",
					b"",
				),
				Some(pending_claim_challenges)
			);
			// PendingClaimStatus
			assert_eq!(
				migration::get_storage_value::<EventClaimStatus>(
					b"EthBridge",
					b"PendingClaimStatus",
					&Twox64Concat::hash(&(1 as EventClaimId).encode())
				),
				Some(storage_v0::EventClaimStatus::Pending)
			);
			// ProcessedMessageIds
			assert_eq!(
				migration::get_storage_value::<Vec<EventClaimId>>(
					b"EthBridge",
					b"ProcessedMessageIds",
					b"",
				),
				Some(processed_mssge_ids)
			);
			// MessagesValidAt
			assert_eq!(
				migration::get_storage_value::<Vec<EventClaimId>>(
					b"EthBridge",
					b"MessagesValidAt",
					&Twox64Concat::hash(&(1 as BlockNumber).encode())
				),
				Some(messages_valid_at)
			);
			// NextEthCallId
			assert_eq!(
				migration::get_storage_value::<EthCallId>(b"EthBridge", b"NextEthCallId", b"",),
				Some(1 as storage_v0::EthCallId)
			);
			// Relayer
			assert_eq!(
				migration::get_storage_value::<AccountId>(b"EthBridge", b"Relayer", b"",),
				Some(AccountId::from(relayer_address))
			);
			// RelayerPaidBond -> RelayerBond
			assert_eq!(
				migration::get_storage_value::<Balance>(
					b"EthBridge",
					b"RelayerBond",
					&Twox64Concat::hash(&AccountId::from(relayer_address).encode())
				),
				Some(10 as Balance)
			);
			assert_eq!(
				migration::get_storage_value::<Balance>(
					b"EthBridge",
					b"RelayerPaidBond",
					&Twox64Concat::hash(&AccountId::from(relayer_address).encode())
				),
				None
			);
			// EthCallRequests
			assert_eq!(
				migration::get_storage_value::<Vec<EthCallId>>(
					b"EthBridge",
					b"EthCallRequests",
					b"",
				),
				Some(eth_call_requests)
			);
			// EthCallNotarizations
			assert_eq!(
				migration::get_storage_value::<CheckedEthCallResult>(
					b"EthBridge",
					b"EthCallNotarizations",
					&call_notarization_comb_key
				),
				Some(eth_call_result)
			);
			// EthCallNotarizationsAggregated
			assert_eq!(
				migration::get_storage_value::<BTreeMap<CheckedEthCallResult, u32>>(
					b"EthBridge",
					b"EthCallNotarizationsAggregated",
					&Twox64Concat::hash(&(1 as EthCallId).encode())
				),
				Some(eth_call_aggregated)
			);
			// EthCallRequestInfo
			assert_eq!(
				migration::get_storage_value::<CheckedEthCallResult>(
					b"EthBridge",
					b"EthCallRequestInfo",
					&Twox64Concat::hash(&(1 as EthCallId).encode())
				),
				Some(eth_call_result)
			);
		});
	}

	#[test]
	fn migrate_v0_to_v1_moved_to_validator_set() {
		ExtBuilder::default().build().execute_with(|| {
			assert_eq!(StorageVersion::get::<Pallet<TestRuntime>>(), 0);
			// Add values to v0 storage(old EthBridge)

			let next_notary_keys = vec![
				AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
				AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
			];
			migration::put_storage_value(
				b"EthBridge",
				b"NextNotaryKeys",
				b"",
				next_notary_keys.clone(),
			);
			let notary_keys = vec![
				AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
				AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
			];
			migration::put_storage_value(b"EthBridge", b"NotaryKeys", b"", notary_keys.clone());
			let notary_xrpl_keys = vec![AuthorityId::from_slice(&[1_u8; 33]).unwrap()];
			migration::put_storage_value(
				b"EthBridge",
				b"NotaryXrplKeys",
				b"",
				notary_xrpl_keys.clone(),
			);
			let xrpl_door_signers = notary_xrpl_keys[0].clone();
			migration::put_storage_value(
				b"EthBridge",
				b"XrplDoorSigners",
				&Twox64Concat::hash(&xrpl_door_signers.encode()),
				true,
			);
			let notary_set_id = 1_u64;
			migration::put_storage_value(b"EthBridge", b"NotarySetId", b"", notary_set_id);
			let next_authority_change = BlockNumber::from(10_u32);
			migration::put_storage_value(
				b"EthBridge",
				b"NextAuthorityChange",
				b"",
				next_authority_change,
			);

			// Run upgrade
			<Pallet<TestRuntime> as OnRuntimeUpgrade>::on_runtime_upgrade();
			assert_eq!(StorageVersion::get::<Pallet<TestRuntime>>(), 1);

			// moved items to pallet-validator-set should still be available via "ValidatorSet"
			// NextNotaryKeys
			assert_eq!(
				migration::get_storage_value::<Vec<AuthorityId>>(
					b"ValidatorSet",
					b"NextNotaryKeys",
					b"",
				),
				Some(next_notary_keys)
			);
			// NotaryKeys
			assert_eq!(
				migration::get_storage_value::<Vec<AuthorityId>>(
					b"ValidatorSet",
					b"NotaryKeys",
					b"",
				),
				Some(notary_keys)
			);
			// NotaryXrplKeys
			assert_eq!(
				migration::get_storage_value::<Vec<AuthorityId>>(
					b"ValidatorSet",
					b"NotaryXrplKeys",
					b"",
				),
				Some(notary_xrpl_keys)
			);
			// XrplDoorSigners
			assert_eq!(
				migration::get_storage_value::<bool>(
					b"ValidatorSet",
					b"XrplDoorSigners",
					&Twox64Concat::hash(&xrpl_door_signers.encode()),
				),
				Some(true)
			);
			// NotarySetId
			assert_eq!(
				migration::get_storage_value::<u64>(b"ValidatorSet", b"NotarySetId", b"",),
				Some(notary_set_id)
			);
			// NextAuthorityChange -> NextValidatorSetChangeBlock
			assert_eq!(
				migration::get_storage_value::<BlockNumber>(
					b"ValidatorSet",
					b"NextValidatorSetChangeBlock",
					b"",
				),
				Some(next_authority_change)
			);
			assert_eq!(
				migration::get_storage_value::<u32>(b"EthBridge", b"NextAuthorityChange", b"",),
				None
			);
		});
	}

	#[test]
	fn migrate_v0_to_v1_moved_to_ethy() {
		ExtBuilder::default().build().execute_with(|| {
			assert_eq!(StorageVersion::get::<Pallet<TestRuntime>>(), 0);
			// Add values to v0 storage(old EthBridge)
			migration::put_storage_value(b"EthBridge", b"BridgePaused", b"", false);
			let delayed_event_proofs_per_block = 5_u8;
			migration::put_storage_value(
				b"EthBridge",
				b"DelayedEventProofsPerBlock",
				b"",
				delayed_event_proofs_per_block,
			);
			let next_event_proof_id: EventProofId = 1;
			migration::put_storage_value(
				b"EthBridge",
				b"NextEventProofId",
				b"",
				next_event_proof_id,
			);
			let notary_set_proof_id: EventProofId = 1;
			migration::put_storage_value(
				b"EthBridge",
				b"NotarySetProofId",
				b"",
				notary_set_proof_id,
			);
			let xrpl_notary_set_proof_id: EventProofId = 1;
			migration::put_storage_value(
				b"EthBridge",
				b"XrplNotarySetProofId",
				b"",
				xrpl_notary_set_proof_id,
			);
			let pending_event_proof = EthySigningRequest::XrplTx(vec![1_u8; 50]);
			migration::put_storage_value(
				b"EthBridge",
				b"PendingEventProofs",
				&Twox64Concat::hash(&(1 as EventProofId).encode()),
				pending_event_proof.clone(),
			);

			// Run upgrade
			<Pallet<TestRuntime> as OnRuntimeUpgrade>::on_runtime_upgrade();
			assert_eq!(StorageVersion::get::<Pallet<TestRuntime>>(), 1);

			// moved items to pallet-ethy should still be available via "Ethy"
			// BridgePaused -> EthyState
			assert_eq!(
				migration::get_storage_value::<State>(b"Ethy", b"EthyState", b"",),
				Some(State::Active)
			);
			assert_eq!(
				migration::get_storage_value::<bool>(b"EthBrige", b"BridgePaused", b"",),
				None
			);
			// DelayedEventProofsPerBlock -> DelayedProofRequestsPerBlock
			assert_eq!(
				migration::get_storage_value::<u8>(b"Ethy", b"DelayedProofRequestsPerBlock", b"",),
				Some(delayed_event_proofs_per_block)
			);
			assert_eq!(
				migration::get_storage_value::<u8>(
					b"EthBrige",
					b"DelayedProofRequestsPerBlock",
					b"",
				),
				None
			);
			// NextEventProofId
			assert_eq!(
				migration::get_storage_value::<EventProofId>(b"Ethy", b"NextEventProofId", b"",),
				Some(next_event_proof_id)
			);
			assert_eq!(
				migration::get_storage_value::<EventProofId>(b"EthBrige", b"NextEventProofId", b"",),
				None
			);
			// NotarySetProofId
			assert_eq!(
				migration::get_storage_value::<EventProofId>(b"Ethy", b"NotarySetProofId", b"",),
				Some(notary_set_proof_id)
			);
			assert_eq!(
				migration::get_storage_value::<EventProofId>(b"EthBrige", b"NotarySetProofId", b"",),
				None
			);
			// XrplNotarySetProofId
			assert_eq!(
				migration::get_storage_value::<EventProofId>(b"Ethy", b"XrplNotarySetProofId", b"",),
				Some(xrpl_notary_set_proof_id)
			);
			assert_eq!(
				migration::get_storage_value::<EventProofId>(
					b"EthBrige",
					b"XrplNotarySetProofId",
					b"",
				),
				None
			);
			// PendingEventProofs ->PendingProofRequests
			assert_eq!(
				migration::get_storage_value::<EthySigningRequest>(
					b"Ethy",
					b"PendingProofRequests",
					&Twox64Concat::hash(&(1 as EventProofId).encode()),
				),
				Some(pending_event_proof)
			);
			assert_eq!(
				migration::get_storage_value::<EthySigningRequest>(
					b"EthBrige",
					b"PendingEventProofs",
					b"",
				),
				None
			);
		});
	}
}
