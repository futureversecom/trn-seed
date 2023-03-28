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
	ensure,
	traits::{tokens::fungibles::Mutate, Get},
	transactional, PalletId,
};
use seed_pallet_common::{
	CreateExt, Hold, OnNewAssetSubscriber, OnTransferSubscriber, TransferExt, Xls20MintRequest,
};
use seed_primitives::{
	AccountId, AssetId, Balance, CollectionUuid, MetadataScheme, ParachainId, SerialNumber, TokenId,
};
use sp_runtime::{
	traits::{AccountIdConversion, One, Saturating, Zero},
	DispatchResult, PerThing, Permill,
};
use sp_std::prelude::*;

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
pub use weights::WeightInfo;

mod impls;
mod migration;
mod types;

pub use impls::*;
pub use pallet::*;
pub use types::*;

/// The maximum length of valid collection IDs
pub const MAX_COLLECTION_NAME_LENGTH: u8 = 32;
/// The maximum amount of listings to return
pub const MAX_COLLECTION_LISTING_LIMIT: u16 = 100;
/// The logging target for this module
pub(crate) const LOG_TARGET: &str = "nft";

#[frame_support::pallet]
pub mod pallet {
	use super::{DispatchResult, *};
	use frame_support::{pallet_prelude::*, traits::fungibles::Transfer};
	use frame_system::pallet_prelude::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		_phantom: sp_std::marker::PhantomData<T>,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		/// Default auction / sale length in blocks
		#[pallet::constant]
		type DefaultListingDuration: Get<Self::BlockNumber>;
		/// The system event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Max tokens that a collection can contain
		type MaxTokensPerCollection: Get<u32>;
		/// Handles a multi-currency fungible asset system
		type MultiCurrency: TransferExt<AccountId = Self::AccountId>
			+ Hold<AccountId = Self::AccountId>
			+ Mutate<Self::AccountId, AssetId = AssetId>
			+ CreateExt<AccountId = Self::AccountId>
			+ Transfer<Self::AccountId, Balance = Balance>;
		/// Handler for when an SFT has been transferred
		type OnTransferSubscription: OnTransferSubscriber;
		/// Handler for when an SFT collection has been created
		type OnNewAssetSubscription: OnNewAssetSubscriber<CollectionUuid>;
		/// This pallet's Id, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// The parachain_id being used by this parachain
		type ParachainId: Get<ParachainId>;
		/// Provides the public call to weight mapping
		type WeightInfo: WeightInfo;
	}

	/// Map from collection to its information
	#[pallet::storage]
	#[pallet::getter(fn collection_info)]
	pub type CollectionInfo<T: Config> =
		StorageMap<_, Twox64Concat, CollectionUuid, CollectionInformation<T>>;

	/// TODO Use NFT pallet NextCollectionID
	//#[pallet::storage]
	//pub type NextCollectionId<T> = StorageValue<_, u32, ValueQuery>;

	/// Map from a token to lock status if any
	#[pallet::storage]
	#[pallet::getter(fn token_locks)]
	pub type TokenLocks<T> = StorageMap<_, Twox64Concat, TokenId, TokenLockReason>;

	// TODO Remove Events not being used
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new collection of tokens was created
		CollectionCreate {
			collection_uuid: CollectionUuid,
			initial_issuance: TokenCount,
			max_issuance: Option<TokenCount>,
			collection_owner: T::AccountId,
			metadata_scheme: MetadataScheme,
			name: CollectionNameType,
			royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
			origin_chain: OriginChain,
			compatibility: CrossChainCompatibility,
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

		/// TODO Use set_owner from NFT pallet
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
		#[pallet::weight(100000)]
		#[transactional]
		pub fn create_collection(
			origin: OriginFor<T>,
			collection_name: CollectionNameType,
			token_owner: Option<T::AccountId>,
			metadata_scheme: MetadataScheme,
			royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
			cross_chain_compatibility: CrossChainCompatibility,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

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
			token_name: CollectionNameType,
			initial_issuance: Balance,
			max_issuance: Option<Balance>,
			token_owner: Option<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// Ensure who == owner

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
	}
}
