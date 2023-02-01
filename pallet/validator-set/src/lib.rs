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
#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use core::default::Default;
use ethabi::Token;
use frame_support::{
	codec::{Decode, MaxEncodedLen},
	ensure, fail,
	pallet_prelude::DispatchResult,
	traits::{
		schedule::{Anon, DispatchTime},
		Get, OneSessionHandler,
	},
	weights::{constants::RocksDbWeight as DbWeight, Weight},
	BoundedVec, PalletId,
};
use frame_system::{ensure_none, pallet_prelude::OriginFor};
use log::{debug, error, info, trace, warn};
pub use pallet::*;
use pallet_ethy::types::Log;
use seed_pallet_common::{
	ethy::{
		EthereumBridgeAdapter,
		State::{Active, Paused},
		XRPLBridgeAdapter,
	},
	log,
	validator_set::{ValidatorSetChangeHandler, ValidatorSetChangeInfo, ValidatorSetInterface},
	EthereumBridge, EthereumEventSubscriber, FinalSessionTracker as FinalSessionTrackerT,
};
use seed_primitives::{
	ethy::{
		ConsensusLog, EventProofId, ValidatorSet as ValidatorSetS, ValidatorSetId, ETHY_ENGINE_ID,
	},
	CollectionUuid, EthyEcdsaToEthereum, EthyEcdsaToXRPLAccountId, SerialNumber,
};
use sp_core::{H160, U256};
use sp_runtime::{
	traits::{AccountIdConversion, Saturating},
	DigestItem, DispatchError, Percent, SaturatedConversion,
};
use sp_std::{boxed::Box, vec, vec::Vec};

pub(crate) const LOG_TARGET: &str = "validator-set";
pub(crate) const SCHEDULER_PRIORITY: u8 = 63;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet, pallet_prelude::*, traits::schedule::Anon, transactional};
	use frame_system::{ensure_signed, offchain::CreateSignedTransaction, pallet_prelude::*};
	use seed_pallet_common::{
		ethy::{EthereumBridgeAdapter, XRPLBridgeAdapter},
		validator_set::{ValidatorSetChangeHandler, ValidatorSetChangeInfo},
	};
	use seed_primitives::{AccountId, ValidatorId};
	use sp_runtime::RuntimeAppPublic;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountId = AccountId> + CreateSignedTransaction<Call<Self>>
	{
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Pallet Id
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// The identifier type for a validator in this module (i.e. active validator session key)
		/// 33 byte secp256k1 public key
		type EthyId: Member
			+ Parameter
			+ AsRef<[u8]>
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;
		/// The duration in blocks of one epoch
		#[pallet::constant]
		type EpochDuration: Get<u64>;
		/// Length of time the ethy will be paused while the validator set changes
		#[pallet::constant]
		type ValidatorChangeDelay: Get<Self::BlockNumber>;
		/// Reports the final session of na eras
		type FinalSessionTracker: FinalSessionTrackerT;
		/// The Scheduler.
		type Scheduler: Anon<Self::BlockNumber, <Self as Config>::Call, Self::PalletsOrigin>;
		/// The runtime call type.
		type Call: From<Call<Self>>;
		/// Overarching type of all pallets origins.
		type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;
		/// Max Xrpl notary (validator) public keys
		#[pallet::constant]
		type MaxXrplKeys: Get<u8>;
		/// ethy adapter
		type EthyAdapter: ValidatorSetChangeHandler<Self::EthyId>;
		/// XRPL Bridge adapter
		type XRPLBridgeAdapter: XRPLBridgeAdapter<Self::EthyId>;
		/// Eth Bridge adapter
		type EthBridgeAdapter: EthereumBridgeAdapter;
		/// Max amount of new signers that can be set an in extrinsic
		type MaxNewSigners: Get<u8>; // TODO(surangap): Update this with #419
	}

	#[pallet::storage]
	#[pallet::getter(fn next_notary_keys)]
	/// Scheduled notary (validator) public keys for the next session
	pub type NextNotaryKeys<T: Config> = StorageValue<_, Vec<T::EthyId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn notary_keys)]
	/// Active notary (validator) public keys
	pub type NotaryKeys<T: Config> = StorageValue<_, Vec<T::EthyId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn notary_xrpl_keys)]
	/// Active xrpl notary (validator) public keys
	pub type NotaryXrplKeys<T: Config> = StorageValue<_, Vec<T::EthyId>, ValueQuery>;

	/// The current validator set id
	#[pallet::storage]
	#[pallet::getter(fn notary_set_id)]
	pub type NotarySetId<T> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn next_validator_set_change_block)]
	/// The block in which we process the next validator set change
	pub type NextValidatorSetChangeBlock<T: Config> = StorageValue<_, T::BlockNumber>;

	#[pallet::storage]
	#[pallet::getter(fn validators_change_in_progress)]
	/// Flag to indicate whether a validator set change is in progress
	pub type ValidatorsChangeInProgress<T> = StorageValue<_, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn xrpl_door_signers)]
	/// XRPL Door Signers set by sudo (white listed validators)
	pub type XrplDoorSigners<T: Config> = StorageMap<_, Twox64Concat, T::EthyId, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn notary_set_proof_id)]
	/// The event proof Id generated by the previous validator set to notarize the current full
	/// validator set. Useful for syncing the latest proof to Ethereum
	pub type NotarySetProofId<T> = StorageValue<_, EventProofId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn xrpl_notary_set_proof_id)]
	/// The event proof Id generated by the previous xrpl validator set to notarize the current xrpl
	/// validator set. Useful for syncing the latest proof to Xrpl
	pub type XrplNotarySetProofId<T> = StorageValue<_, EventProofId, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub xrp_door_signers: Vec<T::EthyId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { xrp_door_signers: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for new_signer in self.xrp_door_signers.iter() {
				XrplDoorSigners::<T>::insert(new_signer, true);
			}
			// set the NotaryXrplKeys as well
			let genesis_xrpl_keys = NotaryKeys::<T>::get()
				.into_iter()
				.filter(|validator| XrplDoorSigners::<T>::get(validator))
				.map(|validator| -> T::EthyId { validator.clone() })
				.take(T::MaxXrplKeys::get().into())
				.collect::<Vec<_>>();
			NotaryXrplKeys::<T>::put(genesis_xrpl_keys);
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Someone tried to set a greater amount of validators than allowed
		MaxNewSignersExceeded,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Validator set change successful
		ValidatorSetChangeFinalizeSuccess { validator_set_id: ValidatorSetId },
		/// Validator set change failed
		ValidatorSetChangeFinalizeFailed { validator_set_id: ValidatorSetId },
		/// Validator set change finalize scheduling failed
		ValidatorSetFinalizeSchedulingFailed { scheduled_at: T::BlockNumber },
		/// XRPL notary keys update failed
		XRPLNotaryKeysUpdateFailed { validator_set_id: ValidatorSetId },
		/// Xrpl Door signers are set
		XrplDoorSignersSet,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: T::BlockNumber) -> Weight {
			// Handle change in validators, ValidatorChangeDelay(5 minutes) times before the end of
			// an era
			let mut weight = 0 as Weight;
			if Some(block_number) == Self::next_validator_set_change_block() {
				weight += Self::start_validator_set_change();
			}
			weight += DbWeight::get().reads(1 as Weight);
			weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160>,
	{
		/// Finalises the validator set change
		/// Called internally after force new era
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn finalise_validator_set_change(
			origin: OriginFor<T>,
			next_notary_keys: Vec<T::EthyId>,
		) -> DispatchResult {
			ensure_none(origin)?;
			Self::do_finalise_validator_set_change(next_notary_keys)
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		/// Set new XRPL door signers
		pub fn set_xrpl_door_signers(
			origin: OriginFor<T>,
			new_signers: Vec<T::EthyId>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(
				(new_signers.len() as u8) < T::MaxNewSigners::get(),
				Error::<T>::MaxNewSignersExceeded
			);
			// TODO(surangap): To be changed with #419
			for new_signer in new_signers.iter() {
				XrplDoorSigners::<T>::insert(new_signer, true);
			}
			Self::update_xrpl_notary_keys(&Self::notary_keys());
			Self::deposit_event(Event::<T>::XrplDoorSignersSet);
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub(crate) fn update_xrpl_notary_keys(
		validator_list: &Vec<T::EthyId>,
	) -> Result<(), DispatchError> {
		let validators = Self::get_xrpl_notary_keys(validator_list);
		<NotaryXrplKeys<T>>::put(&validators);
		Ok(())
	}

	/// Iterate through the given validator_list and extracts the first number of MaxXrplKeys that
	/// are in the XrplDoorSigners
	pub(crate) fn get_xrpl_notary_keys(validator_list: &Vec<T::EthyId>) -> Vec<T::EthyId> {
		validator_list
			.into_iter()
			.filter(|validator| XrplDoorSigners::<T>::get(validator))
			.map(|validator| -> T::EthyId { validator.clone() })
			.take(T::MaxXrplKeys::get().into())
			.collect()
	}

	/// Starts changing the validator set
	/// This will be called ValidatorChangeDelay(5) minutes before the end of an era when a natural
	/// era change happens. It should give the bridge enough time to update the keys at the remote
	/// side/blockchain This could also be called at the end of an era when doing a forced era. In
	/// this case the actual Notary keys update will be delayed by ValidatorChangeDelay to give the
	/// bridge enough time to update the remote side
	pub(crate) fn start_validator_set_change() -> Weight {
		let next_keys = NextNotaryKeys::<T>::get();
		let next_validator_set_id = Self::notary_set_id().wrapping_add(1);
		debug!(
			target: LOG_TARGET,
			"handling validator set change for validator set id {:?}", next_validator_set_id
		);

		let info = ValidatorSetChangeInfo {
			current_validator_set_id: Self::notary_set_id(),
			current_validator_set: Self::notary_keys(),
			next_validator_set_id,
			next_validator_set: next_keys,
		};
		// let know the ethy
		T::EthyAdapter::validator_set_change_in_progress(info);
		ValidatorsChangeInProgress::<T>::put(true);
		DbWeight::get().reads(4) + DbWeight::get().writes(1)
	}

	/// Finalise validator set changes
	pub fn do_finalise_validator_set_change(
		next_notary_keys: Vec<T::EthyId>,
	) -> Result<(), DispatchError> {
		info!(target: LOG_TARGET, "finalise validator set change");
		// A proof should've been generated now, we can finalize the validator set change and signal
		// resume operations
		ValidatorsChangeInProgress::<T>::kill();
		// Store the new keys and increment the validator set id
		// Next notary keys should be unset, until populated by new session logic
		<NotaryKeys<T>>::put(&next_notary_keys);
		Self::update_xrpl_notary_keys(&next_notary_keys)?;
		NotarySetId::<T>::mutate(|next_set_id| *next_set_id = next_set_id.wrapping_add(1));
		// Inform ethy
		T::EthyAdapter::validator_set_change_finalized(ValidatorSetChangeInfo {
			current_validator_set_id: Self::notary_set_id(),
			current_validator_set: Self::notary_keys(),
			..Default::default()
		});
		Ok(())
	}

	/// return the validator set that governs the Eth bridge
	pub fn get_eth_validator_set() -> ValidatorSetS<T::EthyId> {
		let validator_keys = NotaryKeys::<T>::get();
		ValidatorSetS::<T::EthyId> {
			proof_threshold: T::EthBridgeAdapter::get_notarization_threshold()
				.mul_ceil(validator_keys.len() as u32),
			validators: validator_keys,
			id: NotarySetId::<T>::get(),
		}
	}

	/// return the validator set that governs the Xrpl bridge
	pub fn get_xrpl_validator_set() -> ValidatorSetS<T::EthyId> {
		let validator_keys = NotaryXrplKeys::<T>::get();
		ValidatorSetS::<T::EthyId> {
			proof_threshold: validator_keys.len().saturating_sub(1) as u32, /* tolerate 1 missing
			                                                                 * witness */
			validators: validator_keys,
			id: NotarySetId::<T>::get(),
		}
	}
}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::EthyId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::EthyId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::EthyId)>,
	{
		let keys = validators.map(|x| x.1).collect::<Vec<_>>();
		if keys.is_empty() {
			// no work, return
			return
		}
		assert!(NotaryKeys::<T>::decode_len().is_none(), "NotaryKeys are already initialized!");
		NotaryKeys::<T>::put(&keys);
		if let Err(e) = Self::update_xrpl_notary_keys(&keys) {
			Self::deposit_event(Event::<T>::XRPLNotaryKeysUpdateFailed {
				validator_set_id: Self::notary_set_id().wrapping_add(1).into(),
			});
			error!(
				target: LOG_TARGET,
				"Update XRPL notary keys failed. error: {:?}",
				Into::<&str>::into(e)
			);
		}
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, _validators: I, queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::EthyId)>,
	{
		// TODO(surangap): check and make use of _changed

		// Store the keys for usage next session
		let next_queued_validators = queued_validators.map(|(_, k)| k).collect::<Vec<_>>();
		<NextNotaryKeys<T>>::put(next_queued_validators);

		if T::FinalSessionTracker::is_active_session_final() {
			// Next authority change is AuthorityChangeDelay(5 minutes) before this session ends
			// (Just before the start of the next epoch)
			let epoch_duration: T::BlockNumber = T::EpochDuration::get().saturated_into();
			let next_change_block: T::BlockNumber = <frame_system::Pallet<T>>::block_number()
				.saturating_add(epoch_duration.saturating_sub(T::ValidatorChangeDelay::get()));
			<NextValidatorSetChangeBlock<T>>::put(next_change_block);
		}
	}

	/// A notification for end of the session.
	///
	/// Note it is triggered before any [`SessionManager::end_session`] handlers,
	/// so we can still affect the validator set.
	fn on_before_session_ending() {
		if !T::FinalSessionTracker::is_active_session_final() {
			// no work here. return
			return
		}
		// Get the next_notary_keys for the next era
		let next_notary_keys = NextNotaryKeys::<T>::get();
		if !Self::validators_change_in_progress() {
			// The validator set change hasn't been started yet
			// This could be due to a new era being forced before the final session
			Self::start_validator_set_change();
			// Schedule the finalizing of the new validator set into the future to give the ethy a
			// sufficient time
			let scheduled_block =
				<frame_system::Pallet<T>>::block_number() + T::ValidatorChangeDelay::get();
			if T::Scheduler::schedule(
				DispatchTime::At(scheduled_block),
				None,
				SCHEDULER_PRIORITY,
				frame_system::RawOrigin::None.into(),
				Call::finalise_validator_set_change { next_notary_keys }.into(),
			)
			.is_err()
			{
				// The scheduler failed for some reason, throw an event and log
				Self::deposit_event(Event::<T>::ValidatorSetFinalizeSchedulingFailed {
					scheduled_at: scheduled_block,
				});
				error!(target: LOG_TARGET, "Scheduling finalize validator set change failed");
			}
		} else {
			// validators change is in progress already, finalise the changes
			match Self::do_finalise_validator_set_change(next_notary_keys) {
				Ok(_) => {
					Self::deposit_event(Event::<T>::ValidatorSetChangeFinalizeSuccess {
						validator_set_id: Self::notary_set_id(),
					});
					info!(
						target: LOG_TARGET,
						"Validator set change finalize successful. set Id: {:?}",
						Self::notary_set_id()
					);
				},
				Err(e) => {
					Self::deposit_event(Event::<T>::ValidatorSetChangeFinalizeFailed {
						validator_set_id: Self::notary_set_id(),
					});
					error!(
						target: LOG_TARGET,
						"Validator set change finalize failed. set Id: {:?}, error: {:?}",
						Self::notary_set_id(),
						Into::<&str>::into(e)
					);
				},
			}
		}
	}
	fn on_disabled(_i: u32) {}
}

impl<T: Config> ValidatorSetInterface<T::EthyId> for Pallet<T> {
	fn get_validator_set_id() -> ValidatorSetId {
		NotarySetId::<T>::get()
	}

	fn get_validator_set() -> Vec<T::EthyId> {
		NotaryKeys::<T>::get()
	}

	fn get_next_validator_set() -> Vec<T::EthyId> {
		NextNotaryKeys::<T>::get()
	}

	fn get_xrpl_validator_set() -> Vec<T::EthyId> {
		NotaryXrplKeys::<T>::get()
	}

	fn get_xrpl_notary_keys(validator_list: &Vec<T::EthyId>) -> Vec<T::EthyId> {
		Self::get_xrpl_notary_keys(validator_list)
	}
}
