//! # Pallet Echo
//!
//! A simple utility pallet for testing Ethereum bridging.
//! Users can call the ping() extrinsic which will submit an event to Ethereum
//! The pallet will subscribe to EthereumEventSubscriber so it can verify that the ping was received
//! on Ethereum
#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

use frame_support::{pallet_prelude::*, PalletId};
use frame_system::pallet_prelude::*;
use seed_pallet_common::{EthereumBridge, EthereumEventSubscriber, OnEventResult};
use seed_primitives::{ethy::EventProofId, AccountId};
use sp_core::H160;
use sp_std::prelude::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		/// The system event type
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;
		/// The EthereumBridge interface for sending messages to the bridge
		type EthereumBridge: EthereumBridge;
		/// This pallet's Id, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event {
		/// A message was sent to Ethereum
		Ping { source: H160, destination: H160, message: Vec<u8>, event_proof_id: EventProofId },
		/// A response was received from Ethereum
		Pong { source: H160, data: Vec<u8> },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Ping extrinsic sends an event to the bridge containing a message
		#[pallet::weight(0)]
		pub fn ping(origin: OriginFor<T>, destination: H160, message: Vec<u8>) -> DispatchResult {
			let source: H160 = ensure_signed(origin)?.into();

			// Send event to Ethereum
			let event_proof_id =
				T::EthereumBridge::send_event(&source, &destination, message.as_slice())?;

			// Deposit runtime event
			Self::deposit_event(Event::Ping { source, destination, message, event_proof_id });
			Ok(())
		}
	}
}

// Implement Subscriber to receive events from Ethereum
impl<T: Config> EthereumEventSubscriber for Pallet<T> {
	type DestinationAddress = T::PalletId;

	fn on_event(source: &H160, data: &[u8]) -> OnEventResult {
		// Deposit runtime event to notify that an event was received
		Self::deposit_event(Event::Pong { source: *source, data: data.to_vec() });
		Ok(0)
	}
}
