//! shared pallet types and traits
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use scale_info::TypeInfo;
pub use frame_support::log as logger;
use frame_support::{
	dispatch::{DispatchError, DispatchResult}, traits::fungibles::Transfer, PalletId,
};
use sp_core::{H160, H256};

use seed_primitives::{AssetId, Balance, TokenId, ethy::{EventClaimId, EventProofId}};

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
	/// Returns whether the next session the final session of an era
	/// (is_final, was_forced)
	fn is_next_session_final() -> (bool, bool);
	/// Returns whether the active session is the final session of an era
	fn is_active_session_final() -> bool;
}

/// Reward validators for bridge notarizations
pub trait NotarizationRewardHandler {
	/// Ubiquitous runtime AccountId type
	type AccountId;
	/// Note that the given account ID witnessed an eth-bridge claim
	fn reward_notary(notary: &Self::AccountId);
}

/// Subscription interface for Ethereum bridge event claims
#[impl_trait_for_tuples::impl_for_tuples(10)]
pub trait EthereumEventClaimSubscriber {
	/// Notify subscriber about a successful Ethereum event claim for the given event data
	/// Previously submitted via `request_event_claim`
	fn on_success(event_claim_id: EventClaimId, source_address: &H160, event_signature: &H256, event_data: &[u8]);
	/// Notify subscriber about a failed Ethereum event claim for the given event data
	/// Previously submitted via `request_event_claim`
	fn on_failure(event_claim_id: EventClaimId, source_address: &H160, event_signature: &H256, event_data: &[u8]);
}

/// Interface for an Ethereum event bridge
/// Verifies events happened on the remote chain (claims) and proves events happened to the remote chain (proofs)
pub trait EthereumEventBridge {
	/// Request verification of an event on the bridged Ethereum chain
	/// Returns a unique claim Id on success
	fn request_event_claim(
		source_address: &H160,
		event_signature: &H256,
		tx_hash: &H256,
		event_data: &[u8],
	) -> Result<EventClaimId, DispatchError>;
	/// Request an event proof is generated for the given message to be consumed by `destination_address` on the bridged Ethereum chain
	/// Returns a unique event proof Id on success
	fn request_event_proof(destination_address: &H160, message: &[u8]) -> Result<EventProofId, DispatchError>;
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
	/// Receives verified details about prior `EthCallOracle::checked_eth_call` requests upon their successful completion
	fn on_eth_call_complete(call_id: Self::CallId, return_data: &[u8; 32], block_number: u64, block_timestamp: u64);
	/// Error callback failed for some internal reason `EthCallOracle::checked_eth_call`
	fn on_eth_call_failed(call_id: Self::CallId, reason: EthCallFailure);
}
