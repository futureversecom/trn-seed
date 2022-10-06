#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod xrpl_cli;
mod xrpl_impls;
mod xrpl_types;

use crate::xrpl_types::{
	BridgeXrplWebsocketApi, ChainCallId, CheckedChainCallRequest, CheckedChainCallResult,
	EventClaim, EventClaimResult, EventProofInfo, NotarizationPayload,
};
use frame_support::{
	pallet_prelude::*,
	traits::{OneSessionHandler, UnixTime, ValidatorSet as ValidatorSetT},
	PalletId,
};
use frame_system::{offchain::CreateSignedTransaction, pallet_prelude::*};
use hex_literal::hex;
pub use pallet::*;
use pallet_xrpl_bridge::XrplBridgeCall;
use seed_pallet_common::{log, FinalSessionTracker as FinalSessionTrackerT};
use seed_primitives::{
	ethy::EthyChainId,
	validator::{EventClaimId, EventProofId, ValidatorSet},
};
use sp_core::H160;
use sp_runtime::{
	traits::Saturating, BoundToRuntimeAppPublic, Percent, RuntimeAppPublic, SaturatedConversion,
};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

pub type ValidatorIdOf<T> = <T as Config>::ValidatorId;
pub(crate) const LOG_TARGET: &str = "validator_set";
pub const ENGINE_ID: sp_runtime::ConsensusEngineId = *b"EGN-";
/// The type to sign and send transactions.
pub const UNSIGNED_TXS_PRIORITY: u64 = 100;
/// Max notarization claims to attempt per block/OCW invocation
pub const CLAIMS_PER_BLOCK: usize = 1;
/// Max eth_call checks to attempt per block/OCW invocation
pub const CALLS_PER_BLOCK: usize = 1;
/// The solidity selector of bridge events
/// i.e. output of `keccak256('SubmitEvent(address,address,bytes)')` /
/// `0f8885c9654c5901d61d2eae1fa5d11a67f9b8fca77146d5109bc7be00f4472a`
pub const SUBMIT_BRIDGE_EVENT_SELECTOR: [u8; 32] =
	hex!("0f8885c9654c5901d61d2eae1fa5d11a67f9b8fca77146d5109bc7be00f4472a");

type AccountOf<T> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Allowed origins to add/removw the validator
		type ApproveOrigin: EnsureOrigin<Self::Origin>;
		/// Knows the active authority set (validator stash addresses)
		type AuthoritySet: ValidatorSetT<Self::AccountId, ValidatorId = Self::AccountId>;
		/// To call Xrpl bridge for challenged transactions
		type XrplBridgeCall: XrplBridgeCall<AccountOf<Self>>;
		/// The identifier type for an authority in this module (i.e. active validator session key)
		/// 33 byte ECDSA public key
		type ValidatorId: Member
			+ Parameter
			+ AsRef<[u8]>
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize;
		/// Reports the final session of na eras
		type FinalSessionTracker: FinalSessionTrackerT;
		/// The pallet bridge address (destination for incoming messages, source for outgoing)
		type BridgePalletId: Get<PalletId>;
		// The duration in blocks of one epoch
		type EpochDuration: Get<u64>;
		/// The bridge contract address
		type BridgeContractAddress: Get<H160>;
		/// The threshold of notarizations required to approve an event
		type NotarizationThreshold: Get<Percent>;
		/// Provides an api for Remote Chain JSON-RPC request/responses to the bridged network
		type ChainWebsocketClient: BridgeXrplWebsocketApi;

		/// Unix time
		type UnixTime: UnixTime;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn notary_keys)]
	/// List of all the validators
	pub type NotaryKeys<T: Config> = StorageValue<_, Vec<T::ValidatorId>, ValueQuery>;

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn next_notary_keys)]
	/// Next List of all the validators
	pub type NextNotaryKeys<T: Config> = StorageValue<_, Vec<T::ValidatorId>, ValueQuery>;

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn next_authority_change)]
	/// The block in which we process the next authority change
	pub type NextAuthorityChange<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn bridge_paused)]
	/// Current validators set id
	pub type BridgePaused<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn notary_set_id)]
	/// Current validators set id
	pub type NotarySetId<T: Config> = StorageValue<_, EventProofId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn notary_set_proof_id)]
	/// Current validators set id
	pub type NotarySetProofId<T: Config> = StorageValue<_, EventProofId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn next_event_proof_id)]
	/// Current validators set id
	pub type NextEventProofId<T: Config> = StorageValue<_, EventProofId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pending_event_proofs)]
	/// Queued event proofs to be processed once bridge has been re-enabled
	pub type PendingEventProofs<T: Config> =
		StorageMap<_, Twox64Concat, EventProofId, EventProofInfo>;

	#[pallet::storage]
	#[pallet::getter(fn pending_claim_challenges)]
	/// Queued event proofs to be processed once bridge has been re-enabled
	pub type PendingClaimChallenges<T: Config> = StorageValue<_, Vec<EventClaimId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn chain_call_requests)]
	/// Queue of pending Chain CallOracle requests
	pub type ChainCallRequests<T: Config> = StorageValue<_, Vec<ChainCallId>, ValueQuery>;

	#[pallet::type_value]
	pub fn DefaultNextChainCallId() -> u64 {
		0_u64
	}

	#[pallet::storage]
	#[pallet::getter(fn next_chain_call_id)]
	/// Subscription Id for Call requests
	pub type NextChainCallId<T: Config> =
		StorageValue<_, ChainCallId, ValueQuery, DefaultNextChainCallId>;

	#[pallet::storage]
	#[pallet::getter(fn event_deadline_seconds)]
	/// Events cannot be claimed after this time (seconds)
	pub type EventDeadlineSeconds<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn chain_call_request_info)]
	/// Queue of pending Chain CallOracle requests
	pub type ChainCallRequestInfo<T: Config> =
		StorageMap<_, Twox64Concat, ChainCallId, CheckedChainCallRequest>;

	#[pallet::storage]
	#[pallet::getter(fn pending_event_claims)]
	/// Queued event claims, can be challenged within challenge period
	pub type PendingEventClaims<T: Config> = StorageMap<_, Twox64Concat, EventClaimId, EventClaim>;

	#[pallet::storage]
	#[pallet::getter(fn event_notarizations)]
	/// Notarizations for queued events
	/// Either: None = no notarization exists OR Some(yay/nay)
	pub type EventNotarizations<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EventClaimId,
		Twox64Concat,
		T::ValidatorId,
		EventClaimResult,
	>;

	#[pallet::storage]
	#[pallet::getter(fn chain_call_notarizations)]
	/// Chain CallOracle notarizations keyed by (Id, Notary)
	pub type ChainCallNotarizations<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		ChainCallId,
		Twox64Concat,
		T::ValidatorId,
		CheckedChainCallResult,
	>;

	#[pallet::storage]
	#[pallet::getter(fn chain_call_notarizations_aggregated)]
	/// map from Chain CallOracle notarizations to an aggregated count
	pub type ChainCallNotarizationsAggregated<T: Config> =
		StorageMap<_, Twox64Concat, ChainCallId, BTreeMap<CheckedChainCallResult, u32>>;

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn white_list_validators)]
	/// List of all the white list validators
	pub type WhiteListValidators<T: Config> = StorageMap<_, Twox64Concat, T::ValidatorId, bool>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New validator added
		/// parameters. [who]
		ValidatorAdded(T::ValidatorId),

		/// Validator removed
		/// parameters. [who]
		ValidatorRemoved(T::ValidatorId),
		/// A notary (validator) set change is in motion (event_id, new_validator_set_id)
		/// A proof for the change will be generated with the given `event_id`
		AuthoritySetChange(EventProofId, u64),
		/// Generating event proof delayed as bridge is paused
		ProofDelayed(EventProofId),
		/// An event proof has been sent for signing by ethy-gadget
		EventSend { event_proof_id: EventProofId, chain_id: EthyChainId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Validator is already in the validator set.
		DuplicateValidator,
		/// Validator not found in the validator set.
		ValidatorNotFound,
		/// The bridge is paused pending validator set changes (once every era / 24 hours)
		/// It will reactive after ~10 minutes
		BridgePaused,
		/// Error returned when making unsigned transactions with signed payloads in off-chain
		/// worker
		OffchainUnsignedTxSignedPayload,
		/// Claim was invalid e.g. not properly ABI encoded
		InvalidClaim,
		/// A notarization was invalid
		InvalidNotarization,
		/// Some internal operation failed
		Internal,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			Self::schedule_requests()
		}

		fn offchain_worker(block_number: T::BlockNumber) {
			log!(trace, "💎 entering off-chain worker: {:?}", block_number);
			log!(trace, "💎 active notaries: {:?}", Self::notary_keys());

			// this passes if flag `--validator` set, not necessarily in the active set
			if !sp_io::offchain::is_validator() {
				log!(info, "💎 not a validator, exiting");
				return
			}

			// check a local key exists for a valid bridge notary
			if let Some((active_key, authority_index)) = Self::find_active_validator_key() {
				// check enough validators have active notary keys
				let supports = NotaryKeys::<T>::decode_len().unwrap_or(0);
				let needed = T::NotarizationThreshold::get();
				// TODO: check every session change not block
				if Percent::from_rational(supports, T::AuthoritySet::validators().len()) < needed {
					log!(
						info,
						"💎 waiting for validator support to activate bridge: {:?}/{:?}",
						supports,
						needed
					);
					return
				}
				// validate challenges
				Self::do_call_validate_challenge_ocw(&active_key, authority_index);
			} else {
				log!(trace, "💎 not an active validator, exiting");
			}

			log!(trace, "💎 exiting off-chain worker");
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		/// The initial validator set.
		pub initial_validators: Vec<T::ValidatorId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { initial_validators: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_validators(&self.initial_validators);
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add a new validator to the set.
		/// New validator's session keys should be set in Session pallet before
		/// calling this.
		/// Emits an event on success else error
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(0,1))]
		pub fn add_validator(
			origin: OriginFor<T>,
			validator_id: T::ValidatorId,
		) -> DispatchResultWithPostInfo {
			// Check if the sender is an approved origin or not
			T::ApproveOrigin::ensure_origin(origin)?;
			if <WhiteListValidators<T>>::contains_key(&validator_id) {
				Err(Error::<T>::DuplicateValidator.into())
			} else {
				<WhiteListValidators<T>>::insert(&validator_id, true);
				Self::deposit_event(Event::ValidatorAdded(validator_id));
				Ok(().into())
			}
		}

		/// Remove a validator from the set.
		/// Emits an event on success else error
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn remove_validator(
			origin: OriginFor<T>,
			validator_id: T::ValidatorId,
		) -> DispatchResultWithPostInfo {
			// Check if the sender is an approved origin or not
			T::ApproveOrigin::ensure_origin(origin)?;
			if <WhiteListValidators<T>>::contains_key(&validator_id) {
				<WhiteListValidators<T>>::remove(&validator_id);
				Self::deposit_event(Event::ValidatorRemoved(validator_id));
				Ok(().into())
			} else {
				Err(Error::<T>::ValidatorNotFound.into())
			}
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		/// Internal only
		/// Validators will submit inherents with their notarization vote for a given claim
		pub fn submit_notarization(
			origin: OriginFor<T>,
			payload: NotarizationPayload,
			signature: <<T as Config>::ValidatorId as RuntimeAppPublic>::Signature,
		) -> DispatchResult {
			let _ = ensure_none(origin)?;

			// we don't need to verify the signature here because it has been verified in
			// `validate_unsigned` function when sending out the unsigned tx.
			let authority_index = payload.authority_index() as usize;
			let notary_keys = Self::notary_keys();
			let notary_public_key = match notary_keys.get(authority_index) {
				Some(id) => id,
				None => return Err(Error::<T>::InvalidNotarization.into()),
			};

			match payload {
				NotarizationPayload::Call { call_id, result, .. } =>
					Self::handle_call_notarization(call_id, result, notary_public_key),
				_ => Ok(()),
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	#[allow(dead_code)]
	fn initialize_validators(validator_list: &[T::ValidatorId]) {
		// Add the initial validator_list to WhiteList Validators.
		for validator in validator_list {
			<WhiteListValidators<T>>::insert(validator, true);
		}
	}

	// To check if the given member is validator or not
	pub fn is_validator(validator_id: &T::ValidatorId) -> bool {
		WhiteListValidators::<T>::get(validator_id).unwrap_or(false)
	}

	fn update_validators(validator_list: Vec<T::ValidatorId>) {
		// Filter validator_list from WhiteList Validators.
		let mut validators: Vec<T::ValidatorId> = Vec::new();
		for validator in validator_list {
			if <WhiteListValidators<T>>::contains_key(&validator) {
				validators.push(validator);
			}
		}
		<NotaryKeys<T>>::put(validators);
	}
}

impl<T: Config> ValidatorWhiteList<ValidatorIdOf<T>> for Pallet<T> {
	fn is_validator(validator_id: &T::ValidatorId) -> bool {
		Self::is_validator(validator_id)
	}
}

pub trait ValidatorWhiteList<ValidatorId> {
	fn is_validator(validator_id: &ValidatorId) -> bool;
}

impl<T: Config> BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::ValidatorId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::ValidatorId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::ValidatorId)>,
	{
		let keys = validators.map(|x| x.1).collect::<Vec<_>>();
		if !keys.is_empty() {
			assert!(
				NotaryKeys::<T>::decode_len().is_none(),
				"ValidatorList are already initialized!"
			);
			Self::update_validators(keys);
		}
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, _validators: I, queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::ValidatorId)>,
	{
		if T::FinalSessionTracker::is_active_session_final() {
			// Store the keys for usage next session
			let next_queued_authorities = queued_validators.map(|(_, k)| k).collect::<Vec<_>>();
			<NextNotaryKeys<T>>::put(next_queued_authorities);

			// Next authority change is 5 minutes before this session ends
			// (Just before the start of the next epoch)
			// next_block = current_block + epoch_duration - 75 (5 minutes in blocks)
			let epoch_duration: u32 = T::EpochDuration::get().saturated_into();
			let next_block: T::BlockNumber = <frame_system::Pallet<T>>::block_number()
				.saturating_add(epoch_duration.saturating_sub(75_u32).into());
			<NextAuthorityChange<T>>::put(next_block);
		}
	}

	/// A notification for end of the session.
	///
	/// Note it is triggered before any [`SessionManager::end_session`] handlers,
	/// so we can still affect the validator set.
	fn on_before_session_ending() {
		// Re-activate the bridge, allowing claims & proofs again
		if T::FinalSessionTracker::is_active_session_final() {
			log!(trace, "💎 session & era ending, set new validator keys");
			// A proof should've been generated now so we can reactivate the bridge with the new
			// validator set
			<BridgePaused<T>>::kill();
			// Time to update the bridge validator keys.
			let next_notary_keys = NextNotaryKeys::<T>::take();
			// Store the new keys and increment the validator set id
			// Next notary keys should be unset, until populated by new session logic
			Self::update_validators(next_notary_keys);
			<NotarySetId<T>>::mutate(|next_set_id| *next_set_id = next_set_id.wrapping_add(1));
		}
	}

	fn on_disabled(_i: u32) {}
}
