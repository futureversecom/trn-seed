// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

//! shared pallet types and traits
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
pub use frame_support::log as logger;
use frame_support::{
	dispatch::{DispatchError, DispatchResult, GetCallMetadata},
	sp_runtime::{traits::AccountIdConversion, Perbill},
	traits::{fungibles::Mutate, Get},
	weights::{constants::RocksDbWeight as DbWeight, Weight},
	PalletId,
};
use frame_system::Config;
use scale_info::TypeInfo;
use seed_primitives::xrpl::Xls20TokenId;
use seed_primitives::{
	ethy::{EventClaimId, EventProofId},
	AccountId, AssetId, Balance, CollectionUuid, CrossChainCompatibility, MetadataScheme,
	OriginChain, RoyaltiesSchedule, SerialNumber, TokenCount, TokenId, TokenLockReason,
	WeightedDispatchResult,
};
use sp_core::{bounded::BoundedVec, H160, U256};
use sp_std::{fmt::Debug, vec::Vec};

#[cfg(feature = "std")]
pub mod test_utils;
pub mod utils;
#[cfg(feature = "std")]
pub use test_utils::test_prelude;

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
pub trait TransferExt: Mutate<Self::AccountId> {
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

	/// Create a new asset and return created asset ID.
	fn create(
		owner: &Self::AccountId,
		min_balance: Option<Balance>,
	) -> Result<AssetId, DispatchError>;

	/// Create a new asset with metadata and return created asset ID.
	fn create_with_metadata(
		owner: &Self::AccountId,
		name: Vec<u8>,
		symbol: Vec<u8>,
		decimals: u8,
		min_balance: Option<Balance>,
	) -> Result<AssetId, DispatchError>;
}

pub trait InspectExt {
	/// Check if the asset exists
	fn exists(asset_id: AssetId) -> bool;
}

/// The nft with the given token_id was transferred.
pub trait OnTransferSubscriber {
	/// The nft with the given token_id was transferred.
	fn on_nft_transfer(token_id: &TokenId);
}

impl OnTransferSubscriber for () {
	fn on_nft_transfer(_token_id: &TokenId) {}
}

/// Subscriber for when a new asset or nft is created
pub trait OnNewAssetSubscriber<RuntimeId> {
	/// The nft with the given token_id was transferred.
	fn on_asset_create(runtime_id: RuntimeId, precompile_prefix: &[u8; 4]);
}

impl<RuntimeId> OnNewAssetSubscriber<RuntimeId> for () {
	fn on_asset_create(_runtime_id: RuntimeId, _precompile_prefix: &[u8; 4]) {}
}

/// Reports whether the current session is the final session in a staking era (pre-authority change)
pub trait FinalSessionTracker {
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
/// ```ignore
/// impl EthereumEventRouter for (A,B,C)
/// where
/// 	A: EthereumEventSubscriber,
/// 	B: EthereumEventSubscriber,
/// 	C: EthereumEventSubscriber,
/// {
/// 	fn route(destination, source, data) -> EventRouterResult {
/// 		match destination {
/// 			A::Destination => A::on_event(source, data).map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err))),
/// 			B::Destination => B::on_event(source, data).map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err))),
/// 			C::Destination => C::on_event(source, data).map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err))),
/// 			 _ => Err((Weight::zero(), EventRouterError::NoReceiver)),
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

/// Handle verified Ethereum events (implemented by handler pallet)
pub trait EthereumEventSubscriber {
	/// The destination address of this subscriber (doubles as the source address for sent messages)
	type Address: Get<PalletId>;
	/// The source address that we restrict incoming messages from
	type SourceAddress: Get<H160>;

	/// The destination/source address getter function
	fn address() -> H160 {
		Self::Address::get().into_account_truncating()
	}

	/// process an incoming event from Ethereum
	/// Verifies source address then calls on_event
	fn process_event(source: &H160, data: &[u8]) -> WeightedDispatchResult {
		let verify_weight = Self::verify_source(source)?;
		let on_event_weight = Self::on_event(source, data)?;
		Ok(verify_weight.saturating_add(on_event_weight))
	}

	/// Verifies the source address
	/// Allows pallets to restrict the source based on individual requirements
	/// Default implementation compares source with SourceAddress
	fn verify_source(source: &H160) -> WeightedDispatchResult {
		if source != &Self::SourceAddress::get() {
			Err((
				DbWeight::get().reads(1u64),
				DispatchError::Other("Invalid source address").into(),
			))
		} else {
			Ok(DbWeight::get().reads(1u64))
		}
	}

	/// Notify subscriber about a event received from Ethereum
	/// - `source` the sender address on Ethereum
	/// - `data` the Ethereum ABI encoded event data
	fn on_event(source: &H160, data: &[u8]) -> WeightedDispatchResult;
}

/// Interface for an Ethereum event bridge
/// Generates proof of events for the remote
/// chain
pub trait EthereumBridge {
	/// Send an event via the bridge for relaying to Ethereum
	///
	/// `source` the (pseudo) address of the pallet that submitted the event
	/// `destination` address on Ethereum
	/// `message` data
	///
	/// Returns a unique event proofId on success
	fn send_event(
		source: &H160,
		destination: &H160,
		message: &[u8],
	) -> Result<EventProofId, DispatchError>;
}

/// Interface from xrpl-bridge to ethy
pub trait XrplBridgeToEthyAdapter<AuthorityId> {
	/// Request ethy generate a signature for the given tx data
	fn sign_xrpl_transaction(tx_data: &[u8]) -> Result<EventProofId, DispatchError>;
	/// Return the current set of Ethy validators
	fn validators() -> Vec<AuthorityId>;
	/// Return the current set of xrp validators
	fn xrp_validators() -> Vec<AuthorityId>;
}

/// Interface from ethy to xrpl-bridge
pub trait EthyToXrplBridgeAdapter<AccountId> {
	/// Request xrpl-bridge to submit signer_list_set.
	fn submit_signer_list_set_request(
		_: Vec<(AccountId, u16)>,
	) -> Result<Vec<EventProofId>, DispatchError>;
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

pub trait Xls20MintRequest {
	type AccountId;

	fn request_xls20_mint(
		who: &Self::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: Vec<SerialNumber>,
		metadata_scheme: MetadataScheme,
	) -> DispatchResult;
}

impl Xls20MintRequest for () {
	type AccountId = AccountId;
	fn request_xls20_mint(
		_who: &Self::AccountId,
		_collection_id: CollectionUuid,
		_serial_numbers: Vec<SerialNumber>,
		_metadata_scheme: MetadataScheme,
	) -> DispatchResult {
		Ok(())
	}
}

/// Interface for the XLS20 pallet
pub trait Xls20Ext {
	type AccountId;

	fn deposit_xls20_token(
		receiver: &Self::AccountId,
		xls20_token_id: Xls20TokenId,
	) -> WeightedDispatchResult;

	fn get_xls20_token_id(token_id: TokenId) -> Option<Xls20TokenId>;
}

impl Xls20Ext for () {
	type AccountId = AccountId;

	fn deposit_xls20_token(
		_receiver: &Self::AccountId,
		_xls20_token_id: Xls20TokenId,
	) -> WeightedDispatchResult {
		Ok(Weight::zero())
	}

	fn get_xls20_token_id(_token_id: TokenId) -> Option<Xls20TokenId> {
		None
	}
}

/// NFT Minter trait allows minting of Bridged NFTs that originate on other chains
pub trait NFTMinter {
	type AccountId;

	/// Mint bridged tokens from other chain
	fn mint_bridged_nft(
		owner: &Self::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: Vec<SerialNumber>,
	) -> WeightedDispatchResult;
}

impl NFTMinter for () {
	type AccountId = AccountId;

	fn mint_bridged_nft(
		_owner: &Self::AccountId,
		_collection_id: CollectionUuid,
		_serial_numbers: Vec<SerialNumber>,
	) -> WeightedDispatchResult {
		Ok(Weight::zero())
	}
}

pub trait NFIRequest {
	type AccountId;

	fn request(
		who: &Self::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: Vec<SerialNumber>,
	) -> DispatchResult;

	fn on_burn(token_id: TokenId);
}

impl NFIRequest for () {
	type AccountId = AccountId;

	fn request(
		_who: &Self::AccountId,
		_collection_id: CollectionUuid,
		_serial_numbers: Vec<SerialNumber>,
	) -> DispatchResult {
		Ok(())
	}

	fn on_burn(_token_id: TokenId) {}
}

pub trait FeeConfig {
	fn evm_base_fee_per_gas() -> U256;
	fn weight_multiplier() -> Perbill;
	fn length_multiplier() -> Balance;
}

impl FeeConfig for () {
	fn evm_base_fee_per_gas() -> U256 {
		// Floor network base fee per gas
		// set the same values as the mainnet. 7,500 GWEI.
		// This will result a transfer tx costs 0.0000075*21000 = 0.1575 XRP
		U256::from(7_500_000_000_000u128)
	}
	fn weight_multiplier() -> Perbill {
		Perbill::from_parts(100_000)
	} // 0.01%

	fn length_multiplier() -> Balance {
		Balance::from(350u32)
	}
}

// Code used for futurepass V2
pub trait AccountProxy<AccountId> {
	fn primary_proxy(who: &AccountId) -> Option<AccountId>;
}

pub trait MaintenanceCheck<T: frame_system::Config>
where
	<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
{
	/// Checks whether the call is paused
	fn call_paused(call: &<T as frame_system::Config>::RuntimeCall) -> bool;
}

pub trait MaintenanceCheckEVM<T: frame_system::Config> {
	/// Checks whether an ethereum transaction can be executed
	/// returns true if the transaction is valid
	fn validate_evm_call(signer: &<T as frame_system::Config>::AccountId, target: &H160) -> bool;
	/// Checks whether an ethereum transaction can be executed
	/// returns true if the transaction is valid
	fn validate_evm_create(signer: &<T as frame_system::Config>::AccountId) -> bool;
}

impl<T: frame_system::Config> MaintenanceCheckEVM<T> for () {
	fn validate_evm_call(_signer: &<T as frame_system::Config>::AccountId, _target: &H160) -> bool {
		true
	}

	fn validate_evm_create(_signer: &<T as Config>::AccountId) -> bool {
		true
	}
}

/// Generic trait to validate extrinsics satisfy some condition
pub trait ExtrinsicChecker {
	type Call;
	type Extra;
	type Result;
	fn check_extrinsic(call: &Self::Call, extra: &Self::Extra) -> Self::Result;
}

pub trait NFTExt {
	type AccountId: Debug + PartialEq + Clone;
	type StringLimit: Get<u32>;

	/// Mint a token in a specified collection
	fn do_mint(
		origin: Self::AccountId,
		collection_id: CollectionUuid,
		quantity: TokenCount,
		token_owner: Option<Self::AccountId>,
	) -> DispatchResult;

	/// Transfer a token from origin to new_owner
	fn do_transfer(
		origin: &Self::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: Vec<SerialNumber>,
		new_owner: &Self::AccountId,
	) -> DispatchResult;

	/// Create a new collection
	fn do_create_collection(
		owner: Self::AccountId,
		name: BoundedVec<u8, Self::StringLimit>,
		initial_issuance: TokenCount,
		max_issuance: Option<TokenCount>,
		token_owner: Option<Self::AccountId>,
		metadata_scheme: MetadataScheme,
		royalties_schedule: Option<RoyaltiesSchedule<Self::AccountId>>,
		origin_chain: OriginChain,
		cross_chain_compatibility: CrossChainCompatibility,
	) -> Result<CollectionUuid, DispatchError>;

	/// Returns Some(token_owner) for a token if the owner exists
	fn get_token_owner(token_id: &TokenId) -> Option<Self::AccountId>;

	/// Returns collection current issuance and max issuance
	fn get_collection_issuance(
		collection_id: CollectionUuid,
	) -> Result<(TokenCount, Option<TokenCount>), DispatchError>;

	/// Returns the collection public mint information
	fn get_public_mint_info(
		collection_id: CollectionUuid,
	) -> Result<utils::PublicMintInformation, DispatchError>;

	/// Transfers the ownership of a collection to the new owner
	fn transfer_collection_ownership(
		who: Self::AccountId,
		collection_id: CollectionUuid,
		new_owner: Self::AccountId,
	) -> DispatchResult;

	/// Return the RoyaltiesSchedule if it exists for a collection
	/// Returns an error if the collection does not exist
	fn get_royalties_schedule(
		collection_id: CollectionUuid,
	) -> Result<Option<RoyaltiesSchedule<Self::AccountId>>, DispatchError>;

	/// Enable XLS20 compatibility for a collection
	/// who must be collection owner
	fn enable_xls20_compatibility(
		who: Self::AccountId,
		collection_id: CollectionUuid,
	) -> DispatchResult;

	/// Returns the next collection_uuid
	fn next_collection_uuid() -> Result<CollectionUuid, DispatchError>;

	/// Increments the collection_uuid
	fn increment_collection_uuid() -> DispatchResult;

	/// Returns the token lock status of a token
	fn get_token_lock(token_id: TokenId) -> Option<TokenLockReason>;

	/// Sets the token lock status of a token
	/// who must own the token
	fn set_token_lock(
		token_id: TokenId,
		lock_reason: TokenLockReason,
		who: Self::AccountId,
	) -> DispatchResult;

	/// Remove a token lock without performing checks
	fn remove_token_lock(token_id: TokenId);

	fn get_collection_owner(
		collection_id: CollectionUuid,
	) -> Result<Self::AccountId, DispatchError>;
}

pub trait SFTExt {
	type AccountId: Debug + PartialEq + Clone;

	fn do_transfer(
		origin: Self::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: Vec<(SerialNumber, Balance)>,
		new_owner: Self::AccountId,
	) -> DispatchResult;

	fn reserve_balance(token_id: TokenId, amount: Balance, who: &Self::AccountId)
		-> DispatchResult;

	fn free_reserved_balance(
		token_id: TokenId,
		amount: Balance,
		who: &Self::AccountId,
	) -> DispatchResult;

	fn get_royalties_schedule(
		collection_id: CollectionUuid,
	) -> Result<Option<RoyaltiesSchedule<Self::AccountId>>, DispatchError>;

	fn get_collection_owner(
		collection_id: CollectionUuid,
	) -> Result<Self::AccountId, DispatchError>;

	fn token_exists(token_id: TokenId) -> bool;
}

// Migrator trait to be implemented by the migration pallet. Can be used to determine whether a
// migration is in progress
pub trait Migrator {
	fn ensure_migrated() -> DispatchResult;
}

impl Migrator for () {
	fn ensure_migrated() -> DispatchResult {
		Ok(())
	}
}
