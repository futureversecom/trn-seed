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

use crate::{Runtime, Weight, XRPLBridge};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
use frame_system::pallet_prelude::BlockNumberFor;

#[allow(unused_imports)]
use sp_runtime::DispatchError;
#[allow(unused_imports)]
use sp_std::vec::Vec;

pub struct Upgrade;

impl OnRuntimeUpgrade for Upgrade {
	fn on_runtime_upgrade() -> Weight {
		let current = XRPLBridge::current_storage_version();
		let onchain = XRPLBridge::on_chain_storage_version();
		log::info!(target: "Migration", "XRPLBridge: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		if onchain != 4 {
			log::info!(
				target: "Migration",
				"XRPLBridge: No migration was done, This migration should be on top of storage version 4. Migration code needs to be removed."
			);
			return weight;
		}

		log::info!(target: "Migration", "XRPLBridge: Migrating from on-chain version {onchain:?} to on-chain version {current:?}.");
		weight += v5::migrate::<Runtime>();
		StorageVersion::new(5).put::<XRPLBridge>();
		log::info!(target: "Migration", "XRPLBridge: Migration successfully completed.");
		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		log::info!(target: "Migration", "XRPLBridge: Upgrade to v5 Pre Upgrade.");
		let onchain = XRPLBridge::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 5 {
			return Ok(Vec::new());
		}
		assert_eq!(onchain, 4);

		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
		log::info!(target: "Migration", "XRPLBridge: Upgrade to v5 Post Upgrade.");
		let current = XRPLBridge::current_storage_version();
		let onchain = XRPLBridge::on_chain_storage_version();
		assert_eq!(current, 5);
		assert_eq!(onchain, 5);
		Ok(())
	}
}

#[allow(dead_code)]
#[allow(unused_imports)]
pub mod v5 {
	use super::*;
	use crate::migrations::Value;

	use frame_support::weights::Weight;
	use pallet_xrpl_bridge::{
		DoorAddress, DoorTicketSequence, DoorTicketSequenceParams, DoorTicketSequenceParamsNext,
		DoorTxFee, PaymentDelay, TicketSequenceThresholdReachedEmitted,
	};

	use pallet_xrpl_bridge::types::{XRPLDoorAccount, XrplTicketSequenceParams};
	use seed_primitives::xrpl::{XrplAccountId, XrplTxTicketSequence};
	use seed_primitives::Balance;
	use sp_core::{Get, H160};

	pub fn migrate<T: frame_system::Config + pallet_xrpl_bridge::Config>() -> Weight {
		log::info!(target: "Migration", "XRPLBridge: migrating multi door support");
		let mut weight: Weight = Weight::zero();

		// DoorTicketSequence
		weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().reads(1));
		if let Some(door_ticket_sequence) =
			Value::unsafe_storage_get::<XrplTxTicketSequence>(b"XRPLBridge", b"DoorTicketSequence")
		{
			weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().writes(2));
			Value::unsafe_clear(b"XRPLBridge", b"DoorTicketSequence");
			DoorTicketSequence::<T>::insert(XRPLDoorAccount::Main, door_ticket_sequence);
		}
		// DoorTicketSequenceParams
		weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().reads(1));
		if let Some(door_ticket_sequence_params) = Value::unsafe_storage_get::<
			XrplTicketSequenceParams,
		>(b"XRPLBridge", b"DoorTicketSequenceParams")
		{
			weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().writes(2));
			Value::unsafe_clear(b"XRPLBridge", b"DoorTicketSequenceParams");
			DoorTicketSequenceParams::<T>::insert(
				XRPLDoorAccount::Main,
				door_ticket_sequence_params,
			);
		}
		// DoorTicketSequenceParamsNext
		weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().reads(1));
		if let Some(door_ticket_sequence_params_next) =
			Value::unsafe_storage_get::<XrplTicketSequenceParams>(
				b"XRPLBridge",
				b"DoorTicketSequenceParamsNext",
			) {
			weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().writes(2));
			Value::unsafe_clear(b"XRPLBridge", b"DoorTicketSequenceParamsNext");
			DoorTicketSequenceParamsNext::<T>::insert(
				XRPLDoorAccount::Main,
				door_ticket_sequence_params_next,
			);
		}
		// TicketSequenceThresholdReachedEmitted
		weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().reads(1));
		if let Some(ticket_sequence_threshold_reach_emitted) = Value::unsafe_storage_get::<bool>(
			b"XRPLBridge",
			b"TicketSequenceThresholdReachedEmitted",
		) {
			weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().writes(2));
			Value::unsafe_clear(b"XRPLBridge", b"TicketSequenceThresholdReachedEmitted");
			TicketSequenceThresholdReachedEmitted::<T>::insert(
				XRPLDoorAccount::Main,
				ticket_sequence_threshold_reach_emitted,
			);
		}
		// DoorTxFee
		weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().reads(1));
		if let Some(door_tx_fee) = Value::unsafe_storage_get::<u64>(b"XRPLBridge", b"DoorTxFee") {
			weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().writes(2));
			Value::unsafe_clear(b"XRPLBridge", b"DoorTxFee");
			DoorTxFee::<T>::insert(XRPLDoorAccount::Main, door_tx_fee);
		}
		// DoorAddress
		weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().reads(1));
		if let Some(door_address) =
			Value::unsafe_storage_get::<XrplAccountId>(b"XRPLBridge", b"DoorAddress")
		{
			weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().writes(2));
			Value::unsafe_clear(b"XRPLBridge", b"DoorAddress");
			DoorAddress::<T>::insert(XRPLDoorAccount::Main, door_address);
		}

		log::info!(target: "Migration", "XRPLBridge: migrating multi door support successful");
		weight
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;

		#[test]
		fn migrate_with_existing_values() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(4).put::<XRPLBridge>();
				let door_ticket_sequence: XrplTxTicketSequence = 10;
				let door_ticket_sequence_params =
					XrplTicketSequenceParams { start_sequence: 10, bucket_size: 20 };
				let door_ticket_sequence_params_next =
					XrplTicketSequenceParams { start_sequence: 50, bucket_size: 100 };
				let ticket_sequence_threshold_reach_emitted = true;
				let door_tx_fee = 100;
				let door_address = H160::from_low_u64_be(5);

				Value::unsafe_storage_put::<XrplTxTicketSequence>(
					b"XRPLBridge",
					b"DoorTicketSequence",
					door_ticket_sequence,
				);
				Value::unsafe_storage_put::<XrplTicketSequenceParams>(
					b"XRPLBridge",
					b"DoorTicketSequenceParams",
					door_ticket_sequence_params.clone(),
				);
				Value::unsafe_storage_put::<XrplTicketSequenceParams>(
					b"XRPLBridge",
					b"DoorTicketSequenceParamsNext",
					door_ticket_sequence_params_next.clone(),
				);
				Value::unsafe_storage_put::<bool>(
					b"XRPLBridge",
					b"TicketSequenceThresholdReachedEmitted",
					ticket_sequence_threshold_reach_emitted,
				);
				Value::unsafe_storage_put::<u64>(b"XRPLBridge", b"DoorTxFee", door_tx_fee);
				Value::unsafe_storage_put::<XrplAccountId>(
					b"XRPLBridge",
					b"DoorAddress",
					door_address,
				);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(XRPLBridge::on_chain_storage_version(), 5);
				assert_eq!(
					DoorTicketSequence::<Runtime>::get(XRPLDoorAccount::Main),
					door_ticket_sequence
				);
				assert_eq!(
					Value::unsafe_storage_get::<XrplTxTicketSequence>(
						b"XRPLBridge",
						b"DoorTicketSequence",
					),
					None
				);
				assert_eq!(
					DoorTicketSequenceParams::<Runtime>::get(XRPLDoorAccount::Main),
					door_ticket_sequence_params
				);
				assert_eq!(
					Value::unsafe_storage_get::<XrplTicketSequenceParams>(
						b"XRPLBridge",
						b"DoorTicketSequenceParams",
					),
					None
				);
				assert_eq!(
					DoorTicketSequenceParamsNext::<Runtime>::get(XRPLDoorAccount::Main),
					door_ticket_sequence_params_next
				);
				assert_eq!(
					Value::unsafe_storage_get::<XrplTicketSequenceParams>(
						b"XRPLBridge",
						b"DoorTicketSequenceParamsNext",
					),
					None
				);
				assert_eq!(
					TicketSequenceThresholdReachedEmitted::<Runtime>::get(XRPLDoorAccount::Main),
					ticket_sequence_threshold_reach_emitted
				);
				assert_eq!(
					Value::unsafe_storage_get::<bool>(
						b"XRPLBridge",
						b"TicketSequenceThresholdReachedEmitted",
					),
					None
				);
				assert_eq!(DoorTxFee::<Runtime>::get(XRPLDoorAccount::Main), door_tx_fee);
				assert_eq!(Value::unsafe_storage_get::<u64>(b"XRPLBridge", b"DoorTxFee",), None);
				assert_eq!(DoorAddress::<Runtime>::get(XRPLDoorAccount::Main), Some(door_address));
				assert_eq!(
					Value::unsafe_storage_get::<XrplAccountId>(b"XRPLBridge", b"DoorAddress",),
					None
				);
			});
		}

		#[test]
		fn migrate_with_no_current_data() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(4).put::<XRPLBridge>();

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(XRPLBridge::on_chain_storage_version(), 5);
				assert_eq!(DoorTicketSequence::<Runtime>::get(XRPLDoorAccount::Main), 0);
				let ticket_sequence_params_default =
					XrplTicketSequenceParams { start_sequence: 0_u32, bucket_size: 0_u32 };
				assert_eq!(
					DoorTicketSequenceParams::<Runtime>::get(XRPLDoorAccount::Main),
					ticket_sequence_params_default
				);
				assert_eq!(
					DoorTicketSequenceParamsNext::<Runtime>::get(XRPLDoorAccount::Main),
					ticket_sequence_params_default
				);
				assert_eq!(
					TicketSequenceThresholdReachedEmitted::<Runtime>::get(XRPLDoorAccount::Main),
					false
				);
				assert_eq!(DoorTxFee::<Runtime>::get(XRPLDoorAccount::Main), 1_000_000); // default value
				assert_eq!(DoorAddress::<Runtime>::get(XRPLDoorAccount::Main), None);
			});
		}
	}
}
