//! shared pallet types and traits
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
pub use frame_support::log as logger;
use frame_support::{
	dispatch::{DispatchError, DispatchResult},
	traits::fungibles::Transfer,
	weights::Weight,
	PalletId,
};
use scale_info::TypeInfo;
use sp_core::H160;

use seed_primitives::{
	ethy::{EventClaimId, EventProofId},
	AssetId, Balance, TokenId,
};

pub mod utils;

/// syntactic sugar for logging.
/// the caller must define a variable `LOG_TARGET = "<my-target>"`
#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		seed_pallet_common::logger::$level!(
			target: crate::LOG_TARGET,
			$patter $(, $values)*
		)
	};
}

/// Extended transfer functionality for assets
pub trait TransferExt: Transfer<Self::AccountId> {
	/// The ID type for an account in the system
	type AccountId;
	/// Perform a split transfer from `source` to many destinations
	fn split_transfer(
		who: &Self::AccountId,
		asset_id: AssetId,
		transfers: &[(Self::AccountId, Balance)],
	) -> DispatchResult;
}

/// Place, release, and spend holds on assets
pub trait Hold {
	/// The ID type for an account in the system
	type AccountId;

	/// Place a hold on some amount of assets of who.
	/// The assets will be unspendable until subsequent call to release.
	/// If a hold already exists, it will be increased by `amount`
	///
	/// * `pallet_id` - the pallet authorizing the hold
	/// * `who` - the account owner
	/// * `asset_id` - the asset Id to hold
	/// * `amount` - the amount to hold
	fn place_hold(
		pallet_id: PalletId,
		who: &Self::AccountId,
		asset_id: AssetId,
		amount: Balance,
	) -> DispatchResult;

	/// Release exactly `amount` of asset from `who`, or fail
	/// Requires a prior hold was placed.
	///
	/// * `pallet_id` - the pallet authorizing the hold
	/// * `who` - the account owner
	/// * `asset_id` - the asset Id to hold
	/// * `amount` - the amount to hold
	/// * ```beneficiary` - the address to receive the funds as free balance
	fn release_hold(
		pallet_id: PalletId,
		who: &Self::AccountId,
		asset_id: AssetId,
		amount: Balance,
	) -> DispatchResult;

	/// Spend some held amounts of asset from `who`, or fail
	/// Requires a prior hold was placed.
	///
	/// * `pallet_id` - the pallet authorizing the spend
	/// * `who` - the account owner
	/// * `asset_id` - the asset Id to hold
	/// * `spends` - a list of spends to make
	fn spend_hold(
		pallet_id: PalletId,
		who: &Self::AccountId,
		asset_id: AssetId,
		spends: &[(Self::AccountId, Balance)],
	) -> DispatchResult;
}

/// A trait providing methods for creating and managing assets.
pub trait CreateExt {
	type AccountId;

	/// Create a new asset and resturn an asset ID.
	fn create(owner: Self::AccountId) -> Result<AssetId, DispatchError>;
}

/// The interface that states whether an account owns a token
pub trait IsTokenOwner {
	type AccountId;

	/// Gets whether account owns NFT of TokenId
	fn is_owner(account: &Self::AccountId, token_id: &TokenId) -> bool;
}

/// The nft with the given token_id was transferred.
pub trait OnTransferSubscriber {
	/// The nft with the given token_id was transferred.
	fn on_nft_transfer(token_id: &TokenId);
}

/// Reports whether the current session is the final session in a staking era (pre-authority change)
pub trait FinalSessionTracker {
	/// Returns whether the next session is the final session of an era
	fn is_next_session_final() -> bool;
	/// Returns whether the active session is the final session of an era
	fn is_active_session_final() -> bool;
}

#[derive(Eq, Clone, Copy, Encode, Decode, Debug, TypeInfo, PartialEq)]
pub enum EventRouterError {
	/// Failed during processing
	FailedProcessing(DispatchError),
	/// Message had no configured receiver (check destination address)
	NoReceiver,
}
/// Event router result with consumed weight
pub type EventRouterResult = Result<Weight, (Weight, EventRouterError)>;
/// Routes verified Ethereum messages to handler pallets
///
/// ```no_run
/// impl EthereumEventRouter for (A,B,C)
/// where:
/// 	A: EthereumEventSubscriber,
/// 	B: EthereumEventSubscriber,
/// 	C: EthereumEventSubscriber,
/// {
/// 	fn route(destination, source, data) -> EventRouterResult {
/// 		match source {
/// 			A::Destination => A::on_event(source, data).map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err))),
/// 			B::Destination => B::on_event(source, data).map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err))),
/// 			C::Destination => C::on_event(source, data).map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err))),
/// 			 _ => Err((0, EventRouterError::NoReceiver)),
/// 		}
/// 	}
/// }
/// ```
pub trait EthereumEventRouter {
	/// Route an event to a handler at `destination`
	/// - `source` the sender address on Ethereum
	/// - `destination` the intended handler (pseudo) address
	/// - `data` the Ethereum ABI encoded event data
	fn route(source: &H160, destination: &H160, data: &[u8]) -> EventRouterResult;
}

/// Result of processing an event by an `EthereumEventSubscriber`
pub type OnEventResult = Result<Weight, (Weight, DispatchError)>;
/// Handle verified Ethereum events (implemented by handler pallet)
pub trait EthereumEventSubscriber {
	/// Notify subscriber about a event received from Ethereum
	/// - `source` the sender address on Ethereum
	/// - `data` the Ethereum ABI encoded event data
	fn on_event(source: &H160, data: &[u8]) -> OnEventResult;
}

/// Interface for an Ethereum event bridge
/// Generates proof of events for the remote
/// chain
pub trait EthereumBridge {
	/// Submit an event to the bridge for relaying to Ethereum
	///
	/// `source` the (pseudo) address of the pallet that submitted the event
	/// `destination` address on Ethereum
	/// `message` data
	///
	/// Returns a unique event proofId on success
	fn submit_event(
		source: &H160,
		destination: &H160,
		message: &[u8],
	) -> Result<EventProofId, DispatchError>;
}

#[derive(Encode, Decode, Debug, PartialEq, TypeInfo)]
pub enum EthCallFailure {
	/// Return data exceeds limit
	ReturnDataExceedsLimit,
	/// Return data was empty
	ReturnDataEmpty,
	/// Failure due to some internal reason
	Internal,
}
/// Verifies correctness of state on Ethereum i.e. by issuing `eth_call`s
pub trait EthCallOracle {
	/// EVM address type
	type Address;
	/// Identifies call requests
	type CallId;
	/// Performs an `eth_call` on address `target` with `input` at (or near) `block_hint`
	///
	/// Returns a call Id for subscribers (impl `EthCallOracleSubscriber`)
	fn checked_eth_call(
		target: &Self::Address,
		input: &[u8],
		timestamp: u64,
		block_hint: u64,
		max_block_look_behind: u64,
	) -> Self::CallId;
}

impl EthCallOracle for () {
	type Address = H160;
	type CallId = u64;
	fn checked_eth_call(
		_target: &Self::Address,
		_input: &[u8],
		_timestamp: u64,
		_block_hint: u64,
		_max_block_look_behind: u64,
	) -> Self::CallId {
		0_u64
	}
}

/// Subscribes to verified ethereum state
pub trait EthCallOracleSubscriber {
	/// Identifies requests
	type CallId;
	/// Receives verified details about prior `EthCallOracle::checked_eth_call` requests upon their
	/// successful completion
	fn on_eth_call_complete(
		call_id: Self::CallId,
		return_data: &[u8; 32],
		block_number: u64,
		block_timestamp: u64,
	);
	/// Error callback failed for some internal reason `EthCallOracle::checked_eth_call`
	fn on_eth_call_failed(call_id: Self::CallId, reason: EthCallFailure);
}

impl EthCallOracleSubscriber for () {
	type CallId = EventClaimId;
	fn on_eth_call_complete(
		_call_id: Self::CallId,
		_return_data: &[u8; 32],
		_block_number: u64,
		_block_timestamp: u64,
	) {
	}
	/// Error callback failed for some internal reason `EthCallOracle::checked_eth_call`
	fn on_eth_call_failed(_call_id: Self::CallId, _reason: EthCallFailure) {}
}
