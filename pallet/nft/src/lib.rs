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

#![cfg_attr(not(feature = "std"), no_std)]
//! # NFT Module
//!
//! Provides the basic creation and management of dynamic NFTs (created at runtime).
//!
//! Intended to be used "as is" by dapps and provide basic NFT feature set for smart contracts
//! to extend.
//!
//! *Collection*:
//! Collection are a grouping of tokens- equivalent to an ERC721 contract
//!
//! *Tokens*:
//!  Individual tokens within a collection. Globally identifiable by a tuple of (collection, serial
//! number)

use frame_support::{
	dispatch::Dispatchable,
	ensure,
	traits::{fungibles::Mutate, Get},
	transactional, PalletId,
};
use seed_pallet_common::{
	utils::{CollectionUtilityFlags, PublicMintInformation, TokenUtilityFlags as TokenFlags},
	NFIRequest, OnNewAssetSubscriber, OnTransferSubscriber, Xls20MintRequest,
};
use seed_primitives::{
	AssetId, Balance, CollectionUuid, CrossChainCompatibility, MetadataScheme, OriginChain,
	ParachainId, RoyaltiesSchedule, SerialNumber, TokenCount, TokenId, TokenLockReason,
	MAX_COLLECTION_ENTITLEMENTS,
};
use sp_runtime::{
	traits::{AccountIdConversion, One, Zero},
	DispatchResult,
};
use sp_std::prelude::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(feature = "std")]
pub mod test_utils;
#[cfg(test)]
mod tests;
pub mod weights;

pub use weights::WeightInfo;

mod impls;
pub mod traits;
mod types;

pub use pallet::*;
pub use types::*;

/// The maximum amount of owned tokens to be returned by the RPC
pub const MAX_OWNED_TOKENS_LIMIT: u16 = 1000;
/// The logging target for this module
pub(crate) const LOG_TARGET: &str = "nft";

#[frame_support::pallet]
pub mod pallet {
	use super::{DispatchResult, *};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use seed_pallet_common::utils::TokenBurnAuthority;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(5);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		_phantom: sp_std::marker::PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			NextCollectionId::<T>::put(1_u32);
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The overarching call type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ From<frame_system::Call<Self>>;
		/// Max tokens that a collection can contain
		type MaxTokensPerCollection: Get<u32>;
		/// Max quantity of NFTs that can be minted in one transaction
		type MintLimit: Get<u32>;
		/// Handler for when an NFT has been transferred
		type OnTransferSubscription: OnTransferSubscriber;
		/// Handler for when an NFT collection has been created
		type OnNewAssetSubscription: OnNewAssetSubscriber<CollectionUuid>;
		/// Handles a multi-currency fungible asset system
		type MultiCurrency: Mutate<Self::AccountId, Balance = Balance, AssetId = AssetId>;
		/// This pallet's Id, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// The parachain_id being used by this parachain
		type ParachainId: Get<ParachainId>;
		/// The maximum length of a collection name, stored on-chain
		#[pallet::constant]
		type StringLimit: Get<u32>;
		/// Provides the public call to weight mapping
		type WeightInfo: WeightInfo;
		/// Interface for sending XLS20 mint requests
		type Xls20MintRequest: Xls20MintRequest<AccountId = Self::AccountId>;
		/// Interface for requesting extra meta storage items
		type NFIRequest: NFIRequest<AccountId = Self::AccountId>;
	}

	/// Map from collection to its information
	#[pallet::storage]
	pub type CollectionInfo<T: Config> = StorageMap<
		_,
		Twox64Concat,
		CollectionUuid,
		CollectionInformation<T::AccountId, T::MaxTokensPerCollection, T::StringLimit>,
	>;

	/// Map from collection to its public minting information
	#[pallet::storage]
	pub type PublicMintInfo<T: Config> =
		StorageMap<_, Twox64Concat, CollectionUuid, PublicMintInformation>;

	/// The next available incrementing collection id
	#[pallet::storage]
	pub type NextCollectionId<T> = StorageValue<_, u32, ValueQuery>;

	/// Map from a token to lock status if any
	#[pallet::storage]
	pub type TokenLocks<T> = StorageMap<_, Twox64Concat, TokenId, TokenLockReason>;

	/// Map from a collection to additional utility flags
	#[pallet::storage]
	pub type UtilityFlags<T> =
		StorageMap<_, Twox64Concat, CollectionUuid, CollectionUtilityFlags, ValueQuery>;

	/// Map from a token_id to transferable and burn authority flags
	#[pallet::storage]
	pub type TokenUtilityFlags<T> = StorageMap<_, Twox64Concat, TokenId, TokenFlags, ValueQuery>;

	/// The next available incrementing issuance id
	#[pallet::storage]
	pub type NextPendingIssuanceId<T> =
		StorageMap<_, Twox64Concat, CollectionUuid, u32, ValueQuery>;

	// Map from a collection id and issuance id to a pending issuance
	#[pallet::storage]
	pub type PendingIssuances<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		CollectionUuid,
		Twox64Concat,
		u32,
		PendingIssuance<T::AccountId>,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new collection of tokens was created
		CollectionCreate {
			collection_uuid: CollectionUuid,
			initial_issuance: TokenCount,
			max_issuance: Option<TokenCount>,
			collection_owner: T::AccountId,
			metadata_scheme: MetadataScheme,
			name: Vec<u8>,
			royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
			origin_chain: OriginChain,
			compatibility: CrossChainCompatibility,
		},
		/// Public minting was enabled/disabled for a collection
		PublicMintToggle { collection_id: CollectionUuid, enabled: bool },
		/// Token(s) were minted
		Mint {
			collection_id: CollectionUuid,
			start: SerialNumber,
			end: SerialNumber,
			owner: T::AccountId,
		},
		/// Payment was made to cover a public mint
		MintFeePaid {
			who: T::AccountId,
			collection_id: CollectionUuid,
			payment_asset: AssetId,
			payment_amount: Balance,
			token_count: TokenCount,
		},
		/// A mint price was set for a collection
		MintPriceSet {
			collection_id: CollectionUuid,
			payment_asset: Option<AssetId>,
			mint_price: Option<Balance>,
		},
		/// Token(s) were bridged
		BridgedMint {
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
			owner: T::AccountId,
		},
		/// A new owner was set
		OwnerSet { collection_id: CollectionUuid, new_owner: T::AccountId },
		/// Max issuance was set
		MaxIssuanceSet { collection_id: CollectionUuid, max_issuance: TokenCount },
		/// Base URI was set
		BaseUriSet { collection_id: CollectionUuid, base_uri: Vec<u8> },
		/// Name was set
		NameSet { collection_id: CollectionUuid, name: BoundedVec<u8, T::StringLimit> },
		/// Royalties schedule was set
		RoyaltiesScheduleSet {
			collection_id: CollectionUuid,
			royalties_schedule: RoyaltiesSchedule<T::AccountId>,
		},
		/// A token was transferred
		Transfer {
			previous_owner: T::AccountId,
			collection_id: CollectionUuid,
			serial_numbers: Vec<SerialNumber>,
			new_owner: T::AccountId,
		},
		/// A token was burned
		Burn {
			token_owner: T::AccountId,
			collection_id: CollectionUuid,
			serial_number: SerialNumber,
		},
		/// Collection has been claimed
		CollectionClaimed { account: T::AccountId, collection_id: CollectionUuid },
		/// Utility flags were set for a collection
		UtilityFlagsSet { collection_id: CollectionUuid, utility_flags: CollectionUtilityFlags },
		/// Token transferable flag was set
		TokenTransferableFlagSet { token_id: TokenId, transferable: bool },
		/// A pending issuance for a soulbound token has been created
		PendingIssuanceCreated {
			collection_id: CollectionUuid,
			issuance_id: u32,
			token_owner: T::AccountId,
			burn_authority: TokenBurnAuthority,
		},
		/// A soulbound token was successfully issued
		Issued {
			from: T::AccountId,
			to: T::AccountId,
			token_id: TokenId,
			burn_auth: TokenBurnAuthority,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Given collection name is invalid (invalid utf-8, too long, empty)
		CollectionNameInvalid,
		/// No more Ids are available, they've been exhausted
		NoAvailableIds,
		/// Origin does not own the NFT
		NotTokenOwner,
		/// The token does not exist
		NoToken,
		/// Origin is not the collection owner and is not permitted to perform the operation
		NotCollectionOwner,
		/// This collection has not allowed public minting
		PublicMintDisabled,
		/// Cannot operate on a listed NFT
		TokenLocked,
		/// Total royalties would exceed 100% of sale or an empty vec is supplied
		RoyaltiesInvalid,
		/// The collection does not exist
		NoCollectionFound,
		/// The metadata path is invalid (non-utf8 or empty)
		InvalidMetadataPath,
		/// The caller can not be the new owner
		InvalidNewOwner,
		/// The number of tokens have exceeded the max tokens allowed
		TokenLimitExceeded,
		/// The quantity exceeds the max tokens per mint limit
		MintLimitExceeded,
		/// Max issuance needs to be greater than 0 and initial_issuance
		/// Cannot exceed MaxTokensPerCollection
		InvalidMaxIssuance,
		/// The max issuance has already been set and can't be changed
		MaxIssuanceAlreadySet,
		/// The collection max issuance has been reached and no more tokens can be minted
		MaxIssuanceReached,
		/// Attemped to mint a token that was bridged from a different chain
		AttemptedMintOnBridgedToken,
		/// Cannot claim already claimed collections
		CannotClaimNonClaimableCollections,
		/// Only Root originated NFTs that are not XLS-20 compatible can have their metadata updated
		CannotUpdateMetadata,
		/// Initial issuance on XLS-20 compatible collections must be zero
		InitialIssuanceNotZero,
		/// Total issuance of collection must be zero to add xls20 compatibility
		CollectionIssuanceNotZero,
		/// Token(s) blocked from minting during the bridging process
		BlockedMint,
		/// Minting has been disabled for tokens within this collection
		MintUtilityBlocked,
		/// Transfer has been disabled for tokens within this collection
		TransferUtilityBlocked,
		/// Burning has been disabled for tokens within this collection
		BurnUtilityBlocked,
		/// Attempted to accept an issuance that does not exist, or is not
		/// set for the caller
		InvalidPendingIssuance,
		/// Attempted to update the token utility flags for a soulbound token
		CannotUpdateTokenUtility,
		/// Attempted to burn a token from an account that does not adhere to
		/// the token's burn authority
		InvalidBurnAuthority,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::claim_unowned_collection())]
		/// Bridged collections from Ethereum will initially lack an owner. These collections will
		/// be assigned to the pallet. This allows for claiming those collections assuming they were
		/// assigned to the pallet
		pub fn claim_unowned_collection(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			new_owner: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;

			CollectionInfo::<T>::try_mutate(collection_id, |maybe_collection| -> DispatchResult {
				let collection = maybe_collection.as_mut().ok_or(Error::<T>::NoCollectionFound)?;
				ensure!(
					collection.owner == Self::account_id(),
					Error::<T>::CannotClaimNonClaimableCollections
				);

				collection.owner = new_owner.clone();
				Ok(())
			})?;
			let event = Event::<T>::CollectionClaimed { account: new_owner, collection_id };
			Self::deposit_event(event);

			Ok(())
		}

		/// Set the owner of a collection
		/// Caller must be the current collection owner
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_owner())]
		pub fn set_owner(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_set_owner(who, collection_id, new_owner)
		}

		/// Set the max issuance of a collection
		/// Caller must be the current collection owner
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_max_issuance())]
		pub fn set_max_issuance(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			max_issuance: TokenCount,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut collection_info =
				<CollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			ensure!(!max_issuance.is_zero(), Error::<T>::InvalidMaxIssuance);
			ensure!(collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);
			ensure!(collection_info.max_issuance.is_none(), Error::<T>::MaxIssuanceAlreadySet);
			ensure!(
				collection_info.collection_issuance <= max_issuance,
				Error::<T>::InvalidMaxIssuance
			);

			collection_info.max_issuance = Some(max_issuance);
			<CollectionInfo<T>>::insert(collection_id, collection_info);
			Self::deposit_event(Event::<T>::MaxIssuanceSet { collection_id, max_issuance });
			Ok(())
		}

		/// Set the base URI of a collection
		/// Caller must be the current collection owner
		/// Collection must originate on TRN and not be XLS-20 compatible
		/// XLS-20 metadata is immutable so we must respect that on our chain as well
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::set_base_uri())]
		pub fn set_base_uri(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			base_uri: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut collection_info =
				<CollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			ensure!(collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);
			ensure!(
				!collection_info.cross_chain_compatibility.xrpl,
				Error::<T>::CannotUpdateMetadata
			);
			ensure!(
				collection_info.origin_chain == OriginChain::Root,
				Error::<T>::CannotUpdateMetadata
			);

			collection_info.metadata_scheme = base_uri
				.clone()
				.as_slice()
				.try_into()
				.map_err(|_| Error::<T>::InvalidMetadataPath)?;

			<CollectionInfo<T>>::insert(collection_id, collection_info);
			Self::deposit_event(Event::<T>::BaseUriSet { collection_id, base_uri });
			Ok(())
		}

		/// Create a new collection
		/// Additional tokens can be minted via `mint_additional`
		///
		/// `name` - the name of the collection
		/// `initial_issuance` - number of tokens to mint now
		/// `max_issuance` - maximum number of tokens allowed in collection
		/// `token_owner` - the token owner, defaults to the caller
		/// `metadata_scheme` - The off-chain metadata referencing scheme for tokens in this
		/// `royalties_schedule` - defacto royalties plan for secondary sales, this will
		/// apply to all tokens in the collection by default.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::create_collection())]
		#[transactional]
		pub fn create_collection(
			origin: OriginFor<T>,
			name: BoundedVec<u8, T::StringLimit>,
			initial_issuance: TokenCount,
			max_issuance: Option<TokenCount>,
			token_owner: Option<T::AccountId>,
			metadata_scheme: MetadataScheme,
			royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
			cross_chain_compatibility: CrossChainCompatibility,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_create_collection(
				who,
				name,
				initial_issuance,
				max_issuance,
				token_owner,
				metadata_scheme,
				royalties_schedule,
				OriginChain::Root,
				cross_chain_compatibility,
			)?;
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::toggle_public_mint())]
		pub fn toggle_public_mint(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			enabled: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let collection_info =
				<CollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			// Only the owner can make this call
			ensure!(collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);

			// Get public mint info and set enabled flag
			let mut public_mint_info = <PublicMintInfo<T>>::get(collection_id).unwrap_or_default();
			public_mint_info.enabled = enabled;

			if public_mint_info == PublicMintInformation::default() {
				// If the pricing details are None, and enabled is false
				// Remove the storage entry
				<PublicMintInfo<T>>::remove(collection_id);
			} else {
				// Otherwise, update the storage
				<PublicMintInfo<T>>::insert(collection_id, public_mint_info);
			}

			Self::deposit_event(Event::<T>::PublicMintToggle { collection_id, enabled });
			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::set_mint_fee())]
		pub fn set_mint_fee(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			pricing_details: Option<(AssetId, Balance)>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let collection_info =
				<CollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			// Only the owner can make this call
			ensure!(collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);

			// Get the existing public mint info if it exists
			let mut public_mint_info = <PublicMintInfo<T>>::get(collection_id).unwrap_or_default();
			public_mint_info.pricing_details = pricing_details;

			if public_mint_info == PublicMintInformation::default() {
				// If the pricing details are None, and enabled is false
				// Remove the storage entry
				<PublicMintInfo<T>>::remove(collection_id);
			} else {
				// Otherwise, update the storage
				<PublicMintInfo<T>>::insert(collection_id, public_mint_info);
			}

			// Extract payment asset and mint price for clearer event logging
			let (payment_asset, mint_price) = match pricing_details {
				Some((asset, price)) => (Some(asset), Some(price)),
				None => (None, None),
			};

			Self::deposit_event(Event::<T>::MintPriceSet {
				collection_id,
				payment_asset,
				mint_price,
			});
			Ok(())
		}

		/// Mint tokens for an existing collection
		///
		/// `collection_id` - the collection to mint tokens in
		/// `quantity` - how many tokens to mint
		/// `token_owner` - the token owner, defaults to the caller if unspecified
		/// Caller must be the collection owner
		/// -----------
		/// Weight is O(N) where N is `quantity`
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::mint())]
		#[transactional]
		pub fn mint(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			quantity: TokenCount,
			token_owner: Option<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(quantity <= T::MintLimit::get(), Error::<T>::MintLimitExceeded);
			// minting flag must be enabled on the collection
			ensure!(<UtilityFlags<T>>::get(collection_id).mintable, Error::<T>::MintUtilityBlocked);

			let mut collection_info =
				<CollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			let public_mint_info = <PublicMintInfo<T>>::get(collection_id).unwrap_or_default();

			// Perform pre mint checks
			let serial_numbers =
				Self::pre_mint(&who, quantity, &collection_info, public_mint_info.enabled)?;
			let owner = token_owner.unwrap_or(who.clone());
			let xls20_compatible = collection_info.cross_chain_compatibility.xrpl;
			let metadata_scheme = collection_info.metadata_scheme.clone();

			// Increment next serial number
			let next_serial_number = collection_info.next_serial_number;
			collection_info.next_serial_number =
				next_serial_number.checked_add(quantity).ok_or(Error::<T>::NoAvailableIds)?;

			// Only charge mint fee if public mint enabled and caller is not collection owner
			if public_mint_info.enabled && !collection_info.is_collection_owner(&who) {
				// Charge the mint fee for the mint
				Self::charge_mint_fee(
					&who,
					collection_id,
					&collection_info.owner,
					public_mint_info,
					quantity,
				)?;
			}

			// Perform the mint and update storage
			Self::do_mint(collection_id, collection_info, &owner, &serial_numbers)?;

			// Check if this collection is XLS-20 compatible
			if xls20_compatible {
				// Pay XLS20 mint fee and send requests
				T::Xls20MintRequest::request_xls20_mint(
					&who,
					collection_id,
					serial_numbers.clone().into_inner(),
					metadata_scheme,
				)?;
			}

			// Request NFI storage if enabled
			T::NFIRequest::request(&who, collection_id, serial_numbers.clone().into_inner())?;

			// throw event, listing starting and endpoint token ids (sequential mint)
			Self::deposit_event(Event::<T>::Mint {
				collection_id,
				start: *serial_numbers.first().ok_or(Error::<T>::NoToken)?,
				end: *serial_numbers.last().ok_or(Error::<T>::NoToken)?,
				owner,
			});
			Ok(())
		}

		/// Transfer ownership of an NFT
		/// Caller must be the token owner
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::transfer(serial_numbers.len() as u32))]
		#[transactional]
		pub fn transfer(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Self::do_transfer(collection_id, serial_numbers, &who, &new_owner)
		}

		/// Burn a token 🔥
		///
		/// Caller must be the token owner
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::burn())]
		#[transactional]
		pub fn burn(origin: OriginFor<T>, token_id: TokenId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let (collection_id, serial_number) = token_id;

			Self::do_burn(&who, collection_id, serial_number)?;
			Self::deposit_event(Event::<T>::Burn {
				token_owner: who,
				collection_id,
				serial_number,
			});
			Ok(())
		}

		/// Set the name of a collection
		/// Caller must be the current collection owner
		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::set_name())]
		pub fn set_name(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			name: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut collection_info =
				<CollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			ensure!(collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);

			ensure!(!name.is_empty(), Error::<T>::CollectionNameInvalid);
			ensure!(core::str::from_utf8(&name).is_ok(), Error::<T>::CollectionNameInvalid);
			collection_info.name = name.clone();

			<CollectionInfo<T>>::insert(collection_id, collection_info);
			Self::deposit_event(Event::<T>::NameSet { collection_id, name });
			Ok(())
		}

		/// Set the royalties schedule of a collection
		/// Caller must be the current collection owner
		#[pallet::call_index(11)]
		#[pallet::weight(T::WeightInfo::set_royalties_schedule())]
		pub fn set_royalties_schedule(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			royalties_schedule: RoyaltiesSchedule<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut collection_info =
				<CollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			ensure!(collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);

			// Check that the entitlements are less than MAX_ENTITLEMENTS - 2
			// This is because when the token is listed, two more entitlements will be added
			// for the network fee and marketplace fee
			ensure!(
				royalties_schedule.entitlements.len() <= MAX_COLLECTION_ENTITLEMENTS as usize,
				Error::<T>::RoyaltiesInvalid
			);
			ensure!(royalties_schedule.validate(), Error::<T>::RoyaltiesInvalid);

			collection_info.royalties_schedule = Some(royalties_schedule.clone());

			<CollectionInfo<T>>::insert(collection_id, collection_info);
			Self::deposit_event(Event::<T>::RoyaltiesScheduleSet {
				collection_id,
				royalties_schedule,
			});
			Ok(())
		}

		/// Set utility flags of a collection. This allows restricting certain operations on a
		/// collection such as transfer, burn or mint
		#[pallet::call_index(12)]
		#[pallet::weight(T::WeightInfo::set_utility_flags())]
		#[transactional]
		pub fn set_utility_flags(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			utility_flags: CollectionUtilityFlags,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let collection_info =
				<CollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			ensure!(collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);

			if utility_flags == CollectionUtilityFlags::default() {
				// If the utility flags are default, remove the storage entry
				<UtilityFlags<T>>::remove(collection_id);
			} else {
				// Otherwise, update the storage
				<UtilityFlags<T>>::insert(collection_id, utility_flags);
			}

			Self::deposit_event(Event::<T>::UtilityFlagsSet { collection_id, utility_flags });
			Ok(())
		}

		/// Set transferable flag on a token, allowing or disallowing transfers
		/// Caller must be the collection owner
		#[pallet::call_index(13)]
		#[pallet::weight(T::WeightInfo::set_token_transferable_flag())]
		#[transactional]
		pub fn set_token_transferable_flag(
			origin: OriginFor<T>,
			token_id: TokenId,
			transferable: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let collection_info =
				<CollectionInfo<T>>::get(token_id.0).ok_or(Error::<T>::NoCollectionFound)?;
			ensure!(collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);

			// Check if the token exists
			ensure!(collection_info.token_exists(token_id.1), Error::<T>::NoToken);

			ensure!(
				<TokenUtilityFlags<T>>::get(token_id).burn_authority.is_none(),
				Error::<T>::CannotUpdateTokenUtility
			);

			TokenUtilityFlags::<T>::mutate(token_id, |flags| {
				flags.transferable = transferable;
			});

			Self::deposit_event(Event::<T>::TokenTransferableFlagSet { token_id, transferable });
			Ok(())
		}

		/// Issue a soulbound token. The issuance will be pending until the
		/// token owner accepts the issuance.
		#[pallet::call_index(14)]
		#[pallet::weight(1_000)]
		#[transactional]
		pub fn issue(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			quantity: TokenCount,
			token_owner: T::AccountId,
			burn_authority: TokenBurnAuthority,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let collection_info =
				<CollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			// Only the owner can make this call
			ensure!(collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);

			for _ in 0..quantity {
				let issuance_id = <NextPendingIssuanceId<T>>::get(collection_id);

				<PendingIssuances<T>>::insert(
					collection_id,
					issuance_id,
					PendingIssuance { token_owner: token_owner.clone(), burn_authority },
				);

				Self::deposit_event(Event::<T>::PendingIssuanceCreated {
					collection_id,
					issuance_id,
					token_owner: token_owner.clone(),
					burn_authority,
				});

				<NextPendingIssuanceId<T>>::mutate(collection_id, |i| *i += u32::one());
			}

			Ok(())
		}

		/// Accept the issuance of a soulbound token.
		#[pallet::call_index(15)]
		#[pallet::weight(1_000)]
		#[transactional]
		pub fn accept_issuance(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			issuance_id: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let pending_issuance = <PendingIssuances<T>>::get(collection_id, issuance_id)
				.ok_or(Error::<T>::InvalidPendingIssuance)?;

			if pending_issuance.token_owner != who {
				Err(Error::<T>::InvalidPendingIssuance)?;
			}

			let quantity = 1;

			let mut collection_info =
				<CollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;

			// Perform pre mint checks
			// Note: We validate this mint as if it was being performed
			// by the owner.
			let serial_numbers =
				Self::pre_mint(&collection_info.owner, quantity, &collection_info, false)?;
			let xls20_compatible = collection_info.cross_chain_compatibility.xrpl;
			let metadata_scheme = collection_info.metadata_scheme.clone();

			// Increment next serial number
			let next_serial_number = collection_info.next_serial_number;
			collection_info.next_serial_number =
				next_serial_number.checked_add(quantity).ok_or(Error::<T>::NoAvailableIds)?;

			let owner = collection_info.owner.clone();

			// Perform the mint and update storage
			Self::do_mint(collection_id, collection_info, &who, &serial_numbers)?;

			// Check if this collection is XLS-20 compatible
			if xls20_compatible {
				// Pay XLS20 mint fee and send requests
				T::Xls20MintRequest::request_xls20_mint(
					&who,
					collection_id,
					serial_numbers.clone().into_inner(),
					metadata_scheme,
				)?;
			}

			// Request NFI storage if enabled
			T::NFIRequest::request(&who, collection_id, serial_numbers.clone().into_inner())?;

			let token_id = (collection_id, next_serial_number);

			// Set the utility flags for the token
			TokenUtilityFlags::<T>::mutate(token_id, |flags| {
				flags.transferable = false;
				flags.burn_authority = Some(pending_issuance.burn_authority);
			});

			Self::deposit_event(Event::<T>::Issued {
				from: owner,
				to: who,
				token_id,
				burn_auth: pending_issuance.burn_authority,
			});

			Ok(())
		}
	}
}

impl<T: Config> From<TokenOwnershipError> for Error<T> {
	fn from(val: TokenOwnershipError) -> Error<T> {
		match val {
			TokenOwnershipError::TokenLimitExceeded => Error::<T>::TokenLimitExceeded,
		}
	}
}
