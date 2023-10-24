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

use frame_support::{
	traits::tokens::fungibles::{Mutate, Transfer},
	transactional, PalletId,
};
use pallet_nft::traits::NFTExt;
use seed_pallet_common::{
	CreateExt, Hold, OnNewAssetSubscriber, OnTransferSubscriber, TransferExt,
};
use seed_primitives::{
	AssetId, Balance, CollectionUuid, MetadataScheme, OriginChain, ParachainId, RoyaltiesSchedule,
	SerialNumber, TokenCount, TokenId, TokenLockReason,
};
use sp_runtime::{BoundedVec, DispatchResult};
use sp_std::prelude::*;

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod impls;
mod types;
mod weights;

pub use weights::WeightInfo;

pub use impls::*;
pub use pallet::*;
pub use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::{DispatchResult, *};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use pallet_nft::PublicMintInformation;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Handles a multi-currency fungible asset system
		type MultiCurrency: TransferExt<AccountId = Self::AccountId>
			+ Hold<AccountId = Self::AccountId>
			+ Mutate<Self::AccountId, AssetId = AssetId>
			+ CreateExt<AccountId = Self::AccountId>
			+ Transfer<Self::AccountId, Balance = Balance>;
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
		StorageMap<_, Twox64Concat, CollectionUuid, PublicMintInformation>;

	/// Map from token to its token information, including ownership information
	#[pallet::storage]
	pub type TokenInfo<T: Config> = StorageMap<
		_,
		Twox64Concat,
		TokenId,
		SftTokenInformation<T::AccountId, T::StringLimit, T::MaxOwnersPerSftToken>,
	>;

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
		PublicMintToggle { collection_id: CollectionUuid, enabled: bool },
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
			collection_id: CollectionUuid,
			payment_asset: AssetId,
			payment_amount: Balance,
			token_count: Balance,
		},
		/// A mint price was set for a collection
		MintPriceSet {
			collection_id: CollectionUuid,
			payment_asset: Option<AssetId>,
			mint_price: Option<Balance>,
		},
		/// A new owner was set
		OwnerSet { collection_id: CollectionUuid, new_owner: T::AccountId },
		/// Max issuance was set
		MaxIssuanceSet { token_id: TokenId, max_issuance: Balance },
		/// Base URI was set
		BaseUriSet { collection_id: CollectionUuid, metadata_scheme: MetadataScheme },
		/// Name was set
		NameSet { collection_id: CollectionUuid, collection_name: BoundedVec<u8, T::StringLimit> },
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
		#[pallet::weight(T::WeightInfo::mint())]
		#[transactional]
		pub fn mint(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
			token_owner: Option<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_mint(who, collection_id, serial_numbers, token_owner)
		}

		/// Transfer ownership of an SFT
		/// Caller must be the token owner
		#[pallet::weight(T::WeightInfo::transfer())]
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
		#[pallet::weight(T::WeightInfo::burn())]
		#[transactional]
		pub fn burn(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_burn(who, collection_id, serial_numbers)
		}

		/// Set the owner of a collection
		/// Caller must be the current collection owner
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
		#[pallet::weight(T::WeightInfo::set_royalties_schedule())]
		pub fn set_royalties_schedule(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			royalties_schedule: RoyaltiesSchedule<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_set_royalties_schedule(who, collection_id, royalties_schedule)
		}

		#[pallet::weight(T::WeightInfo::toggle_public_mint())]
		pub fn toggle_public_mint(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			enabled: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let collection_info =
				<SftCollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			// Only the owner can make this call
			ensure!(collection_info.collection_owner == who, Error::<T>::NotCollectionOwner);

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

		#[pallet::weight(T::WeightInfo::set_mint_fee())]
		pub fn set_mint_fee(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			pricing_details: Option<(AssetId, Balance)>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let collection_info =
				<SftCollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			// Only the owner can make this call
			ensure!(collection_info.collection_owner == who, Error::<T>::NotCollectionOwner);

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
	}
}
