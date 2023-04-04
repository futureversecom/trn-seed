/* Copyright 2019-2021 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]
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
	traits::tokens::fungibles::{Mutate, Transfer},
	transactional, PalletId,
};
use pallet_nft::traits::NFTExt;
use seed_pallet_common::{
	CreateExt, Hold, OnNewAssetSubscriber, OnTransferSubscriber, TransferExt,
};
use seed_primitives::{
	AccountId, AssetId, Balance, CollectionUuid, MetadataScheme, OriginChain, ParachainId,
	RoyaltiesSchedule, SerialNumber, TokenCount, TokenId,
};
use sp_runtime::{BoundedVec, DispatchResult};
use sp_std::prelude::*;

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

// TODO Weights
pub use frame_system::WeightInfo;

mod impls;
mod types;

pub use impls::*;
pub use pallet::*;
pub use types::*;

/// The maximum length of valid collection IDs
pub const MAX_COLLECTION_NAME_LENGTH: u8 = 32;
/// The maximum amount of listings to return
pub const MAX_COLLECTION_LISTING_LIMIT: u16 = 100;
/// The logging target for this module
pub(crate) const LOG_TARGET: &str = "sft";

#[frame_support::pallet]
pub mod pallet {
	use super::{DispatchResult, *};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		/// The system event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
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
	pub type SftCollectionInfo<T: Config> =
		StorageMap<_, Twox64Concat, CollectionUuid, SftCollectionInformation<T>>;

	#[pallet::storage]
	pub type TokenInfo<T: Config> = StorageMap<_, Twox64Concat, TokenId, SftTokenInformation<T>>;

	/// TODO Use NFT pallet NextCollectionID
	//#[pallet::storage]
	//pub type NextCollectionId<T> = StorageValue<_, u32, ValueQuery>;

	// TODO Remove Events not being used
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new collection of tokens was created
		CollectionCreate {
			collection_uuid: CollectionUuid,
			collection_owner: T::AccountId,
			metadata_scheme: MetadataScheme,
			name: BoundedVec<u8, T::StringLimit>,
			royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
			origin_chain: OriginChain,
		},
		/// Token(s) were minted
		Mint {
			collection_id: CollectionUuid,
			start: SerialNumber,
			end: SerialNumber,
			owner: T::AccountId,
		},
		/// A new owner was set
		OwnerSet { collection_id: CollectionUuid, new_owner: T::AccountId },
		/// Max issuance was set
		MaxIssuanceSet { collection_id: CollectionUuid, max_issuance: TokenCount },
		/// Base URI was set
		BaseUriSet { collection_id: CollectionUuid, base_uri: Vec<u8> },
		/// A token was transferred
		Transfer {
			previous_owner: T::AccountId,
			collection_id: CollectionUuid,
			serial_numbers: Vec<SerialNumber>,
			new_owner: T::AccountId,
		},
		/// A token was burned
		Burn { collection_id: CollectionUuid, serial_number: SerialNumber },
		/// Collection has been claimed
		CollectionClaimed { account: T::AccountId, collection_id: CollectionUuid },
	}

	// TODO Remove Errors not being used
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
		/// The token is not listed for fixed price sale
		NotForFixedPriceSale,
		/// The token is not listed for auction sale
		NotForAuction,
		/// Origin is not the collection owner and is not permitted to perform the operation
		NotCollectionOwner,
		/// The token is not listed for sale
		TokenNotListed,
		/// The maximum number of offers on this token has been reached
		MaxOffersReached,
		/// Cannot operate on a listed NFT
		TokenLocked,
		/// Total royalties would exceed 100% of sale or an empty vec is supplied
		RoyaltiesInvalid,
		/// The collection does not exist
		NoCollectionFound,
		/// The metadata path is invalid (non-utf8 or empty)
		InvalidMetadataPath,
		/// The caller owns the token and can't make an offer
		IsTokenOwner,
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
		/// Initial issuance on XLS-20 compatible collections must be zero
		InitialIssuanceNotZero,
		/// Total issuance of collection must be zero to add xls20 compatibility
		CollectionIssuanceNotZero,
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
		#[pallet::weight(100000)]
		#[transactional]
		pub fn create_sft_collection(
			origin: OriginFor<T>,
			collection_name: BoundedVec<u8, T::StringLimit>,
			collection_owner: Option<T::AccountId>,
			metadata_scheme: MetadataScheme,
			royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_create_collection(
				who,
				collection_name,
				collection_owner,
				metadata_scheme,
				royalties_schedule,
				OriginChain::Root,
			)?;

			Ok(())
		}

		/// Create additional tokens for an existing collection
		/// These tokens act similar to tokens within an ERC1155 contract
		/// Each token has individual issuance, max_issuance,
		#[pallet::weight(100000)]
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
			// Ensure who == collection_owner
			// Ensure max issuance > initial issuance
			// Creates new serialnumber (based off next_serial_number)

			// CollectionId
			// - serialNumber1
			// - - Account1: Balance
			// - - Account2: Balance
			// - serialNumber2
			// - - Account1: Balance
			// - - Account3: Balance

			// If initial issuance > 0, mint it to the token_owner
			// If token owner is not set then we mint it to the origin

			// create SftTokenInformation object and store under TokenId

			Ok(())
		}

		/// Mint balance into serialNumber
		#[pallet::weight(100000)]
		#[transactional]
		pub fn mint(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxSerialsPerMint>,
			quantities: BoundedVec<Balance, T::MaxSerialsPerMint>,
			token_owner: Option<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// ensure!(serial_numbers.length() == quantities.length(), Error::<T>::InvalidMint);
			Ok(())
		}

		/// Transfer ownership of an NFT
		/// Caller must be the token owner
		#[pallet::weight(100000)]
		#[transactional]
		pub fn transfer(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxSerialsPerMint>,
			quantities: BoundedVec<Balance, T::MaxSerialsPerMint>,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Ok(())
		}

		/// Burn a token 🔥
		///
		/// Caller must be the token owner
		#[pallet::weight(100000)]
		#[transactional]
		pub fn burn(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxSerialsPerMint>,
			quantities: BoundedVec<Balance, T::MaxSerialsPerMint>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Ok(())
		}

		#[pallet::weight(100000)]
		/// TODO Use claim_unowned_collection from NFT pallet
		pub fn claim_unowned_collection(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let _who = ensure_root(origin)?;

			Ok(())
		}

		/// TODO Can use set_owner from NFT pallet, but may be simpler to re-write here
		#[pallet::weight(100000)]
		pub fn set_owner(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Ok(())
		}

		/// TODO Can't use NFT implementation because issuance is set per token
		#[pallet::weight(100000)]
		pub fn set_max_issuance(
			origin: OriginFor<T>,
			token_id: TokenId,
			max_issuance: TokenCount,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Ok(())
		}

		/// TODO Use base_uri from NFT pallet
		#[pallet::weight(100000)]
		pub fn set_base_uri(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			base_uri: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Ok(())
		}
	}
}
