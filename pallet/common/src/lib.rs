//! shared pallet types and traits
#![cfg_attr(not(feature = "std"), no_std)]

pub use frame_support::log as logger;
use frame_support::{
	dispatch::DispatchResult, pallet_prelude::DispatchError, traits::fungibles::Transfer, PalletId,
};
use root_primitives::{AssetId, Balance, TokenId};

pub mod utils;

/// syntactic sugar for logging.
/// the caller must define a variable `LOG_TARGET = "<my-target>"`
#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		root_pallet_common::logger::$level!(
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
