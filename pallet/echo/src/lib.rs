//! # Pallet Echo
//!
//! A simple utility pallet for testing Ethereum bridging.
//! Users can call the ping() extrinsic which will submit an event to Ethereum
//! The pallet will subscribe to EthereumEventSubscriber so it can verify that the ping was received
//! on Ethereum
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use ethabi::{ParamType, Token};
use frame_support::{
	pallet_prelude::*,
	sp_runtime::{traits::One, SaturatedConversion},
	PalletId,
};
use frame_system::pallet_prelude::*;
use seed_pallet_common::{EthereumBridge, EthereumEventSubscriber, OnEventResult};
use seed_primitives::{ethy::EventProofId, AccountId};
use sp_core::H160;
use sp_std::prelude::*;

// Value used to show that the origin of the ping is from this pallet
const PING: u8 = 0;
// Value used to show that the origin of the ping is from Ethereum
const PONG: u8 = 1;

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

	/// The next available offer_id
	#[pallet::storage]
	#[pallet::getter(fn next_session_id)]
	pub type NextSessionId<T> = StorageValue<_, u64, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event {
		/// A ping message was sent to Ethereum
		PingSent { session_id: u64, source: H160, destination: H160, event_proof_id: EventProofId },
		/// A pong response was received from Ethereum
		PongReceived { session_id: u64, source: H160, data: Vec<u8> },
		/// A ping was received from Ethereum
		PingReceived { session_id: u64, source: H160, data: Vec<u8> },
		/// A pong message was sent to Ethereum
		PongSent { session_id: u64, source: H160, destination: H160, event_proof_id: EventProofId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// There are no remaining session ids
		NoAvailableIds,
		/// Invalid ping_or_pong parameter, must be 0 or 1
		InvalidParameter,
		/// The abi received does not match the encoding scheme
		InvalidAbiEncoding,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Ping extrinsic sends an event to the bridge containing a message
		#[pallet::weight(0)]
		pub fn ping(origin: OriginFor<T>, destination: H160) -> DispatchResult {
			let source: H160 = ensure_signed(origin)?.into();

			// Get session id and ensure within u64 bounds
			let session_id = Self::next_session_id();
			ensure!(session_id.checked_add(u64::one()).is_some(), Error::<T>::NoAvailableIds);

			// Encode the message, the first value as 0 states that the event was sent from this
			// pallet The second  value is an incrementing session_id to distinguish events
			let message = ethabi::encode(&[
				Token::Uint(PING.into()),
				Token::Uint(session_id.into()),
				Token::Address(destination),
			]);

			// Send event to Ethereum
			let event_proof_id =
				T::EthereumBridge::send_event(&source, &destination, message.as_slice())?;

			// Increment sessionId
			<NextSessionId<T>>::mutate(|i| *i += 1);

			// Deposit runtime event
			Self::deposit_event(Event::PingSent {
				session_id,
				source,
				destination,
				event_proof_id,
			});
			Ok(())
		}
	}
}

// Implement Subscriber to receive events from Ethereum
impl<T: Config> EthereumEventSubscriber for Pallet<T> {
	type Address = T::PalletId;

	fn on_event(source: &H160, data: &[u8]) -> OnEventResult {
		let abi_decoded = match ethabi::decode(
			&[ParamType::Uint(64), ParamType::Uint(64), ParamType::Address],
			data,
		) {
			Ok(abi) => abi,
			Err(_) => return Err((0, Error::<T>::InvalidAbiEncoding.into())),
		};

		if let [Token::Uint(ping_or_pong), Token::Uint(session_id), Token::Address(destination)] =
			abi_decoded.as_slice()
		{
			let ping_or_pong: u8 = (*ping_or_pong).saturated_into();
			let session_id: u64 = (*session_id).saturated_into();
			let destination: H160 = (*destination).into();

			// Check whether event is a pong or a ping from Ethereum
			match ping_or_pong {
				PING => {
					// Pong was received from Ethereum
					Self::deposit_event(Event::PongReceived {
						session_id,
						source: *source,
						data: data.to_vec(),
					});
					Ok(0)
				},
				PONG => {
					// Ping was received from Ethereum
					Self::deposit_event(Event::PingReceived {
						session_id,
						source: *source,
						data: data.to_vec(),
					});

					// Encode response data
					let message = ethabi::encode(&[
						Token::Uint(PONG.into()),
						Token::Uint(session_id.into()),
						Token::Address(destination),
					]);
					// Send pong response event to Ethereum
					let event_proof_id = match T::EthereumBridge::send_event(
						&source,
						&destination,
						message.as_slice(),
					) {
						Ok(event_id) => event_id,
						Err(e) => return Err((0, e)),
					};

					Self::deposit_event(Event::PongSent {
						session_id,
						source: *source,
						destination,
						event_proof_id,
					});
					Ok(0)
				},
				_ => Err((0, Error::<T>::InvalidParameter.into())),
			}
		} else {
			Ok(0)
		}
	}
}
