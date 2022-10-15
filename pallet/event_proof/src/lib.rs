#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

mod eth_types;
mod impls;
mod types;

use frame_support::{pallet_prelude::*, weights::constants::RocksDbWeight as DbWeight};

use frame_system::offchain::CreateSignedTransaction;
pub use pallet::*;
use seed_primitives::validator::EventProofId;

pub const ENGINE_ID: sp_runtime::ConsensusEngineId = *b"EGN-";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::types::SigningRequest;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn bridge_paused)]
	/// Whether the bridge is paused (e.g. during validator transitions or by governance)
	pub type BridgePaused<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::type_value]
	pub fn DefaultDelayedEventProofsPerBlock() -> u8 {
		5_u8
	}

	#[pallet::storage]
	#[pallet::getter(fn delayed_event_proofs_per_block)]
	/// The maximum number of delayed events that can be processed in on_initialize()
	pub type DelayedEventProofsPerBlock<T: Config> =
		StorageValue<_, u8, ValueQuery, DefaultDelayedEventProofsPerBlock>;

	#[pallet::storage]
	#[pallet::getter(fn pending_event_proofs)]
	/// Queued event proofs to be processed once bridge has been re-enabled
	pub type PendingEventProofs<T: Config> =
		StorageMap<_, Twox64Concat, EventProofId, SigningRequest>;

	#[pallet::storage]
	#[pallet::getter(fn next_event_proof_id)]
	/// Id of the next event proof
	pub type NextEventProofId<T: Config> = StorageValue<_, EventProofId, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Generating event proof delayed as bridge is paused
		ProofDelayed(EventProofId),
		/// An event proof has been sent for signing by gadget
		EventSend { event_proof_id: EventProofId, signing_request: SigningRequest },
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
			let mut consumed_weight = DbWeight::get().reads(2 as Weight);
			if PendingEventProofs::<T>::iter().next().is_some() && !Self::bridge_paused() {
				let max_delayed_events = Self::delayed_event_proofs_per_block();
				consumed_weight = consumed_weight.saturating_add(
					DbWeight::get().reads(1 as Weight) +
						max_delayed_events as Weight * DbWeight::get().writes(2 as Weight),
				);
				for (event_proof_id, signing_request) in
					PendingEventProofs::<T>::iter().take(max_delayed_events as usize)
				{
					Self::do_request_event_proof(event_proof_id, signing_request);
					PendingEventProofs::<T>::remove(event_proof_id);
				}
			}
			consumed_weight
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}
