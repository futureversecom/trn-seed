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
#![recursion_limit = "256"]
//! # SFT Module

use frame_support::{traits::tokens::fungibles::Mutate, transactional, PalletId};
use seed_pallet_common::{
	utils::{CollectionUtilityFlags, PublicMintInformation, TokenUtilityFlags as TokenFlags},
	NFIRequest, NFTExt, OnNewAssetSubscriber, OnTransferSubscriber,
};
use seed_primitives::{
	AssetId, Balance, CollectionUuid, MetadataScheme, OriginChain, ParachainId, RoyaltiesSchedule,
	SerialNumber, TokenId,
};
use sp_runtime::{BoundedVec, DispatchResult};
use sp_std::prelude::*;

#[cfg(test)]
pub mod mock;
#[cfg(feature = "std")]
pub mod test_utils;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod impls;
pub mod types;
mod weights;

pub use weights::WeightInfo;

pub use pallet::*;
pub use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::{DispatchResult, *};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use seed_pallet_common::utils::TokenBurnAuthority;
	use seed_primitives::IssuanceId;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Handles a multi-currency fungible asset system
		type MultiCurrency: Mutate<Self::AccountId, AssetId = AssetId, Balance = Balance>;

		/// NFT Extension, used to retrieve nextCollectionUuid
		type NFTExt: NFTExt<AccountId = Self::AccountId>;
		/// Handler for when an SFT has been transferred
		type OnTransferSubscription: OnTransferSubscriber;
		/// Handler for when an SFT collection has been created
		type OnNewAssetSubscription: OnNewAssetSubscriber<CollectionUuid>;
		/// This pallet's Id, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// The parachain_id being used by this parachain
		type ParachainId: Get<ParachainId>;
		/// The maximum length of a collection or token name, stored on-chain
		#[pallet::constant]
		type StringLimit: Get<u32>;
		/// The maximum length of the stored additional data for a token
		#[pallet::constant]
		type MaxDataLength: Get<u32>;
		/// Provides the public call to weight mapping
		type WeightInfo: WeightInfo;
		/// Max tokens that a collection can contain
		#[pallet::constant]
		type MaxTokensPerSftCollection: Get<u32>;
		/// Max tokens that can be minted in one transaction
		#[pallet::constant]
		type MaxSerialsPerMint: Get<u32>;
		/// Max unique owners that can own an SFT token
		#[pallet::constant]
		type MaxOwnersPerSftToken: Get<u32>;
		/// Interface for requesting extra meta storage items
		type NFIRequest: NFIRequest<AccountId = Self::AccountId>;
		/// Max number of pending issuances for a collection
		type MaxSftPendingIssuances: Get<u32>;
	}

	/// Map from collection to its information
	#[pallet::storage]
	pub type SftCollectionInfo<T: Config> = StorageMap<
		_,
		Twox64Concat,
		CollectionUuid,
		SftCollectionInformation<T::AccountId, T::StringLimit>,
	>;

	/// Map from collection to its public minting information
	#[pallet::storage]
	pub type PublicMintInfo<T: Config> =
		StorageMap<_, Twox64Concat, TokenId, PublicMintInformation>;

	/// Map from a collection to additional utility flags
	#[pallet::storage]
	pub type UtilityFlags<T> =
		StorageMap<_, Twox64Concat, CollectionUuid, CollectionUtilityFlags, ValueQuery>;

	/// Map from a token_id to transferable and burn authority flags
	#[pallet::storage]
	pub type TokenUtilityFlags<T> = StorageMap<_, Twox64Concat, TokenId, TokenFlags, ValueQuery>;

	/// Map from a token_id to additional token data. Useful for assigning extra information
	/// to a token outside the collection metadata.
	#[pallet::storage]
	pub type AdditionalTokenData<T: Config> =
		StorageMap<_, Twox64Concat, TokenId, BoundedVec<u8, T::MaxDataLength>, ValueQuery>;

	/// Map from token to its token information, including ownership information
	#[pallet::storage]
	pub type TokenInfo<T: Config> = StorageMap<
		_,
		Twox64Concat,
		TokenId,
		SftTokenInformation<T::AccountId, T::StringLimit, T::MaxOwnersPerSftToken>,
	>;

	// Map from a collection id to a collection's pending issuances
	#[pallet::storage]
	pub type PendingIssuances<T: Config> = StorageMap<
		_,
		Twox64Concat,
		CollectionUuid,
		SftCollectionPendingIssuances<
			T::AccountId,
			T::MaxSerialsPerMint,
			T::MaxSftPendingIssuances,
		>,
		ValueQuery,
	>;

	/// The next available incrementing issuance ID, unique across all pending issuances
	#[pallet::storage]
	pub type NextIssuanceId<T> = StorageValue<_, IssuanceId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new collection of tokens was created
		CollectionCreate {
			collection_id: CollectionUuid,
			collection_owner: T::AccountId,
			metadata_scheme: MetadataScheme,
			name: BoundedVec<u8, T::StringLimit>,
			royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
			origin_chain: OriginChain,
		},
		/// Public minting was enabled/disabled for a collection
		PublicMintToggle {
			token_id: TokenId,
			enabled: bool,
		},
		/// Token(s) were minted
		Mint {
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxSerialsPerMint>,
			balances: BoundedVec<Balance, T::MaxSerialsPerMint>,
			owner: T::AccountId,
		},
		/// Payment was made to cover a public mint
		MintFeePaid {
			who: T::AccountId,
			token_id: TokenId,
			payment_asset: AssetId,
			payment_amount: Balance,
			token_count: Balance,
		},
		/// A mint price was set for a collection
		MintPriceSet {
			token_id: TokenId,
			payment_asset: Option<AssetId>,
			mint_price: Option<Balance>,
		},
		/// A new owner was set
		OwnerSet {
			collection_id: CollectionUuid,
			new_owner: T::AccountId,
		},
		/// Max issuance was set
		MaxIssuanceSet {
			token_id: TokenId,
			max_issuance: Balance,
		},
		/// Base URI was set
		BaseUriSet {
			collection_id: CollectionUuid,
			metadata_scheme: MetadataScheme,
		},
		/// Name was set
		NameSet {
			collection_id: CollectionUuid,
			collection_name: BoundedVec<u8, T::StringLimit>,
		},
		/// Token name was set
		TokenNameSet {
			token_id: TokenId,
			token_name: BoundedVec<u8, T::StringLimit>,
		},
		/// Royalties schedule was set
		RoyaltiesScheduleSet {
			collection_id: CollectionUuid,
			royalties_schedule: RoyaltiesSchedule<T::AccountId>,
		},
		/// A new token was created within a collection
		TokenCreate {
			token_id: TokenId,
			initial_issuance: Balance,
			max_issuance: Option<Balance>,
			token_name: BoundedVec<u8, T::StringLimit>,
			token_owner: T::AccountId,
		},
		/// A token was transferred
		Transfer {
			previous_owner: T::AccountId,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxSerialsPerMint>,
			balances: BoundedVec<Balance, T::MaxSerialsPerMint>,
			new_owner: T::AccountId,
		},
		/// A token was burned
		Burn {
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxSerialsPerMint>,
			balances: BoundedVec<Balance, T::MaxSerialsPerMint>,
			owner: T::AccountId,
		},
		/// Utility flags were set for a collection
		UtilityFlagsSet {
			collection_id: CollectionUuid,
			utility_flags: CollectionUtilityFlags,
		},
		/// Token transferable flag was set
		TokenTransferableFlagSet {
			token_id: TokenId,
			transferable: bool,
		},
		TokenBurnAuthoritySet {
			token_id: TokenId,
			burn_authority: TokenBurnAuthority,
		},
		/// A pending issuance for a soulbound token has been created
		PendingIssuanceCreated {
			collection_id: CollectionUuid,
			issuance_id: IssuanceId,
			serial_numbers: BoundedVec<SerialNumber, T::MaxSerialsPerMint>,
			balances: BoundedVec<Balance, T::MaxSerialsPerMint>,
			token_owner: T::AccountId,
		},
		/// Soulbound tokens were successfully issued
		Issued {
			token_owner: T::AccountId,
			serial_numbers: BoundedVec<SerialNumber, T::MaxSerialsPerMint>,
			balances: BoundedVec<Balance, T::MaxSerialsPerMint>,
		},
		/// Some additional data has been set for a token
		AdditionalDataSet {
			token_id: TokenId,
			additional_data: Option<BoundedVec<u8, T::MaxDataLength>>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Given collection or token name is invalid (invalid utf-8, empty)
		NameInvalid,
		/// The token does not exist
		NoToken,
		/// Origin is not the collection owner and is not permitted to perform the operation
		NotCollectionOwner,
		/// Total royalties would exceed 100% of sale or an empty vec is supplied
		RoyaltiesInvalid,
		/// The collection does not exist
		NoCollectionFound,
		/// The user does not own enough of this token to perform the operation
		InsufficientBalance,
		/// The specified quantity must be greater than 0
		InvalidQuantity,
		/// Max issuance needs to be greater than 0 and initial_issuance
		/// Cannot exceed MaxTokensPerCollection
		InvalidMaxIssuance,
		/// Caller can not be the new owner
		InvalidNewOwner,
		/// The additional data cannot be an empty vec
		InvalidAdditionalData,
		/// The max issuance has already been set and can't be changed
		MaxIssuanceAlreadySet,
		/// The collection max issuance has been reached and no more tokens can be minted
		MaxIssuanceReached,
		/// The max amount of owners per token has been reached
		MaxOwnersReached,
		/// The operation would cause a numeric overflow
		Overflow,
		/// This collection has not allowed public minting
		PublicMintDisabled,
		/// The number of tokens have exceeded the max tokens allowed
		TokenLimitExceeded,
		/// Minting has been disabled for tokens within this collection
		MintUtilityBlocked,
		/// Transfer has been disabled for tokens within this collection
		TransferUtilityBlocked,
		/// Burning has been disabled for tokens within this collection
		BurnUtilityBlocked,
		/// The burn authority for has already been and can't be changed
		BurnAuthorityAlreadySet,
		/// Attempted to set burn authority for a token that has already
		/// been issued
		TokenAlreadyIssued,
		/// The number of pending issuances has exceeded the max for a collection
		PendingIssuanceLimitExceeded,
		/// Attempted to issue a soulbound token where the burn authority
		/// has not been set
		NoBurnAuthority,
		/// Attempted to accept an issuance that does not exist, or is not
		/// set for the caller
		InvalidPendingIssuance,
		/// Attempted to update the token utility flags for a soulbound token
		CannotUpdateTokenUtility,
		/// Attempted to burn a token from an account that does not adhere to
		/// the token's burn authority
		InvalidBurnAuthority,
		/// The SerialNumbers attempting to be transferred are not unique
		SerialNumbersNotUnique,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a new collection to group multiple semi-fungible tokens
		/// Tokens can be created within the collection by calling create_token
		///
		/// `collection_name` - the name of the collection
		/// `collection_owner` - the collection owner, defaults to the caller
		/// `metadata_scheme` - The off-chain metadata referencing scheme for tokens in this
		/// `royalties_schedule` - defacto royalties plan for secondary sales, this will
		/// apply to all tokens in the collection by default.
		///
		/// The collectionUuid used to store the SFT CollectionInfo is retrieved from the NFT
		/// pallet. This is so that CollectionUuids are unique across all collections, regardless
		/// of if they are SFT or NFT collections.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_collection())]
		#[transactional]
		pub fn create_collection(
			origin: OriginFor<T>,
			collection_name: BoundedVec<u8, T::StringLimit>,
			collection_owner: Option<T::AccountId>,
			metadata_scheme: MetadataScheme,
			royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let owner = collection_owner.unwrap_or(who);
			let _ = Self::do_create_collection(
				owner,
				collection_name,
				metadata_scheme,
				royalties_schedule,
				OriginChain::Root,
			)?;
			Ok(())
		}

		/// Create additional tokens for an existing collection
		/// These tokens act similar to tokens within an ERC1155 contract
		/// Each token has individual issuance, max_issuance,
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::create_token())]
		#[transactional]
		pub fn create_token(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			token_name: BoundedVec<u8, T::StringLimit>,
			initial_issuance: Balance,
			max_issuance: Option<Balance>,
			token_owner: Option<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let _ = Self::do_create_token(
				who,
				collection_id,
				token_name,
				initial_issuance,
				max_issuance,
				token_owner,
			)?;
			Ok(())
		}

		/// Mints some balances into some serial numbers for an account
		/// This acts as a batch mint function and allows for multiple serial numbers and quantities
		/// to be passed in simultaneously.
		/// Must be called by the collection owner
		///
		/// `collection_id` - the SFT collection to mint into
		/// `serial_numbers` - A list of serial numbers to mint into
		/// `quantities` - A list of quantities to mint into each serial number
		/// `token_owner` - The owner of the tokens, defaults to the caller
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::mint())]
		#[transactional]
		pub fn mint(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
			token_owner: Option<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// tokens with burn authority set should only be minted
			// through issue/accept_soulbound_issuance
			ensure!(
				serial_numbers.iter().all(|(serial_number, _)| {
					TokenUtilityFlags::<T>::get((collection_id, serial_number))
						.burn_authority
						.is_none()
				}),
				Error::<T>::BurnAuthorityAlreadySet,
			);

			let collection_info =
				<SftCollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;

			Self::pre_mint(
				who.clone(),
				collection_id,
				collection_info.clone(),
				serial_numbers.clone(),
			)?;

			Self::do_mint(
				who.clone(),
				collection_id,
				collection_info,
				serial_numbers.clone(),
				token_owner.clone(),
			)?;

			let (serial_numbers, balances) = Self::unzip_serial_numbers(serial_numbers);
			Self::deposit_event(Event::<T>::Mint {
				collection_id,
				serial_numbers,
				balances,
				owner: token_owner.unwrap_or(who),
			});

			Ok(())
		}

		/// Transfer ownership of an SFT
		/// Caller must be the token owner
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::transfer(serial_numbers.len() as u32))]
		#[transactional]
		pub fn transfer(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_transfer(who, collection_id, serial_numbers, new_owner)
		}

		/// Burn a token 🔥
		///
		/// Caller must be the token owner
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::burn())]
		#[transactional]
		pub fn burn(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_burn(&who, &who, collection_id, serial_numbers)
		}

		/// Set the owner of a collection
		/// Caller must be the current collection owner
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::set_owner())]
		#[transactional]
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
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::set_max_issuance())]
		pub fn set_max_issuance(
			origin: OriginFor<T>,
			token_id: TokenId,
			max_issuance: Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_set_max_issuance(who, token_id, max_issuance)
		}

		/// Set the base URI of a collection (MetadataScheme)
		/// Caller must be the current collection owner
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::set_base_uri())]
		pub fn set_base_uri(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			metadata_scheme: MetadataScheme,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_set_base_uri(who, collection_id, metadata_scheme)
		}

		/// Set the name of a collection
		/// Caller must be the current collection owner
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::set_name())]
		pub fn set_name(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			collection_name: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_set_name(who, collection_id, collection_name)
		}

		/// Set the royalties schedule of a collection
		/// Caller must be the current collection owner
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::set_royalties_schedule())]
		pub fn set_royalties_schedule(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			royalties_schedule: RoyaltiesSchedule<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_set_royalties_schedule(who, collection_id, royalties_schedule)
		}

		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::toggle_public_mint())]
		pub fn toggle_public_mint(
			origin: OriginFor<T>,
			token_id: TokenId,
			enabled: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let collection_info =
				SftCollectionInfo::<T>::get(token_id.0).ok_or(Error::<T>::NoCollectionFound)?;
			// Caller must be collection_owner
			ensure!(collection_info.collection_owner == who, Error::<T>::NotCollectionOwner);

			// Get public mint info and set enabled flag
			let mut public_mint_info = <PublicMintInfo<T>>::get(token_id).unwrap_or_default();
			public_mint_info.enabled = enabled;

			if public_mint_info == PublicMintInformation::default() {
				// If the pricing details are None, and enabled is false
				// Remove the storage entry
				<PublicMintInfo<T>>::remove(token_id);
			} else {
				// Otherwise, update the storage
				<PublicMintInfo<T>>::insert(token_id, public_mint_info);
			}

			Self::deposit_event(Event::<T>::PublicMintToggle { token_id, enabled });
			Ok(())
		}

		#[pallet::call_index(11)]
		#[pallet::weight(T::WeightInfo::set_mint_fee())]
		pub fn set_mint_fee(
			origin: OriginFor<T>,
			token_id: TokenId,
			pricing_details: Option<(AssetId, Balance)>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let collection_info =
				<SftCollectionInfo<T>>::get(token_id.0).ok_or(Error::<T>::NoCollectionFound)?;
			// Only the owner can make this call
			ensure!(collection_info.collection_owner == who, Error::<T>::NotCollectionOwner);

			// Get the existing public mint info if it exists
			let mut public_mint_info = <PublicMintInfo<T>>::get(token_id).unwrap_or_default();
			public_mint_info.pricing_details = pricing_details;

			if public_mint_info == PublicMintInformation::default() {
				// If the pricing details are None, and enabled is false
				// Remove the storage entry
				<PublicMintInfo<T>>::remove(token_id);
			} else {
				// Otherwise, update the storage
				<PublicMintInfo<T>>::insert(token_id, public_mint_info);
			}

			// Extract payment asset and mint price for clearer event logging
			let (payment_asset, mint_price) = match pricing_details {
				Some((asset, price)) => (Some(asset), Some(price)),
				None => (None, None),
			};

			Self::deposit_event(Event::<T>::MintPriceSet { token_id, payment_asset, mint_price });
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
				<SftCollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			ensure!(collection_info.collection_owner == who, Error::<T>::NotCollectionOwner);

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

		/// Set the name of a collection
		/// Caller must be the current collection owner
		#[pallet::call_index(13)]
		#[pallet::weight(T::WeightInfo::set_token_name())]
		pub fn set_token_name(
			origin: OriginFor<T>,
			token_id: TokenId,
			token_name: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_set_token_name(who, token_id, token_name)
		}

		/// Set transferable flag on a token, allowing or disallowing transfers
		/// Caller must be the collection owner
		#[pallet::call_index(14)]
		#[pallet::weight(T::WeightInfo::set_token_transferable_flag())]
		#[transactional]
		pub fn set_token_transferable_flag(
			origin: OriginFor<T>,
			token_id: TokenId,
			transferable: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let collection_info =
				<SftCollectionInfo<T>>::get(token_id.0).ok_or(Error::<T>::NoCollectionFound)?;
			ensure!(collection_info.collection_owner == who, Error::<T>::NotCollectionOwner);

			// Check if the token exists
			ensure!(<TokenInfo<T>>::contains_key(token_id), Error::<T>::NoToken);

			TokenUtilityFlags::<T>::mutate(token_id, |flags| {
				flags.transferable = transferable;
			});

			Self::deposit_event(Event::<T>::TokenTransferableFlagSet { token_id, transferable });
			Ok(())
		}

		/// Set burn authority on a token. This value will be immutable after
		/// being set.
		/// Caller must be the collection owner.
		#[pallet::call_index(15)]
		#[pallet::weight(T::WeightInfo::set_token_burn_authority())]
		#[transactional]
		pub fn set_token_burn_authority(
			origin: OriginFor<T>,
			token_id: TokenId,
			burn_authority: TokenBurnAuthority,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let collection_info =
				<SftCollectionInfo<T>>::get(token_id.0).ok_or(Error::<T>::NoCollectionFound)?;
			ensure!(collection_info.collection_owner == who, Error::<T>::NotCollectionOwner);

			let token_info = <TokenInfo<T>>::get(token_id).ok_or(Error::<T>::NoToken)?;

			ensure!(token_info.token_issuance == 0, Error::<T>::TokenAlreadyIssued);

			TokenUtilityFlags::<T>::try_mutate(token_id, |flags| -> DispatchResult {
				ensure!(flags.burn_authority.is_none(), Error::<T>::BurnAuthorityAlreadySet);
				flags.burn_authority = Some(burn_authority);
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::TokenBurnAuthoritySet { token_id, burn_authority });
			Ok(())
		}

		/// Burn a token as the collection owner.
		///
		/// The burn authority must have already been set and set to either
		/// the collection owner or both.
		#[pallet::call_index(16)]
		#[pallet::weight(T::WeightInfo::burn_as_collection_owner())]
		#[transactional]
		pub fn burn_as_collection_owner(
			origin: OriginFor<T>,
			token_owner: T::AccountId,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_burn(&who, &token_owner, collection_id, serial_numbers)
		}

		/// Issue a soulbound token. The issuance will be pending until the
		/// token owner accepts the issuance.
		#[pallet::call_index(17)]
		#[pallet::weight(T::WeightInfo::issue_soulbound(serial_numbers.len() as u32))]
		#[transactional]
		pub fn issue_soulbound(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
			token_owner: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let collection_info =
				<SftCollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			// Only the owner can make this call
			ensure!(collection_info.collection_owner == who, Error::<T>::NotCollectionOwner);

			Self::pre_mint(
				collection_info.collection_owner.clone(),
				collection_id,
				collection_info.clone(),
				serial_numbers.clone(),
			)?;

			<PendingIssuances<T>>::try_mutate(
				collection_id,
				|pending_issuances| -> DispatchResult {
					for (serial_number, _) in serial_numbers.iter() {
						// ensure burn authority has been pre declared
						ensure!(
							<TokenUtilityFlags<T>>::get((collection_id, serial_number))
								.burn_authority
								.is_some(),
							Error::<T>::NoBurnAuthority
						);
					}

					let issuance_id = pending_issuances
						.insert_pending_issuance(&token_owner, serial_numbers.clone())
						.map_err(Error::<T>::from)?;

					let (serial_numbers, balances) = Self::unzip_serial_numbers(serial_numbers);

					Self::deposit_event(Event::<T>::PendingIssuanceCreated {
						collection_id,
						issuance_id,
						serial_numbers,
						balances,
						token_owner: token_owner.clone(),
					});

					Ok(())
				},
			)?;

			Ok(())
		}

		/// Accept the issuance of a soulbound token.
		#[pallet::call_index(18)]
		#[pallet::weight(T::WeightInfo::accept_soulbound_issuance())]
		#[transactional]
		pub fn accept_soulbound_issuance(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			issuance_id: IssuanceId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let pending_issuance = <PendingIssuances<T>>::get(collection_id)
				.get_pending_issuance(&who, issuance_id)
				.ok_or(Error::<T>::InvalidPendingIssuance)?;

			let sft_collection_info =
				SftCollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;

			Self::pre_mint(
				sft_collection_info.collection_owner.clone(),
				collection_id,
				sft_collection_info.clone(),
				pending_issuance.serial_numbers.clone(),
			)?;

			Self::do_mint(
				sft_collection_info.collection_owner.clone(),
				collection_id,
				sft_collection_info,
				pending_issuance.serial_numbers.clone(),
				Some(who.clone()),
			)?;

			let (serial_numbers, balances) =
				Self::unzip_serial_numbers(pending_issuance.serial_numbers);

			Self::deposit_event(Event::<T>::Issued {
				token_owner: who.clone(),
				serial_numbers,
				balances,
			});

			// remove the pending issuance
			<PendingIssuances<T>>::try_mutate(
				collection_id,
				|pending_issuances| -> DispatchResult {
					pending_issuances.remove_pending_issuance(&who, issuance_id);

					Ok(())
				},
			)?;

			Ok(())
		}

		/// Sets additional data for a token group.
		/// Caller must be the collection owner.
		/// Data must not be empty
		/// Can be overwritten, call with None to remove.
		#[pallet::call_index(19)]
		#[pallet::weight(T::WeightInfo::set_additional_data())]
		pub fn set_additional_data(
			origin: OriginFor<T>,
			token_id: TokenId,
			additional_data: Option<BoundedVec<u8, T::MaxDataLength>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let collection_info =
				SftCollectionInfo::<T>::get(token_id.0).ok_or(Error::<T>::NoCollectionFound)?;
			ensure!(collection_info.collection_owner == who, Error::<T>::NotCollectionOwner);
			// Check if the token exists
			ensure!(<TokenInfo<T>>::contains_key(token_id), Error::<T>::NoToken);
			Self::do_set_additional_data(token_id, additional_data)?;
			Ok(())
		}

		/// Create a token alongside some additional data
		#[pallet::call_index(20)]
		#[pallet::weight(T::WeightInfo::create_token_with_additional_data())]
		pub fn create_token_with_additional_data(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			token_name: BoundedVec<u8, T::StringLimit>,
			initial_issuance: Balance,
			max_issuance: Option<Balance>,
			token_owner: Option<T::AccountId>,
			additional_data: BoundedVec<u8, T::MaxDataLength>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let serial_number = Self::do_create_token(
				who,
				collection_id,
				token_name,
				initial_issuance,
				max_issuance,
				token_owner,
			)?;

			// Set the additional data and emit event
			Self::do_set_additional_data((collection_id, serial_number), Some(additional_data))?;
			Ok(())
		}
	}
}

impl<T: Config> From<SftPendingIssuanceError> for Error<T> {
	fn from(val: SftPendingIssuanceError) -> Error<T> {
		match val {
			SftPendingIssuanceError::PendingIssuanceLimitExceeded => {
				Error::<T>::PendingIssuanceLimitExceeded
			},
		}
	}
}
