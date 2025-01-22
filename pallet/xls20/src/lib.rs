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

//! # Pallet XLS-20
//!
//! An extension pallet to pallet_nft that allows adds XLS-20 compatibility to collections.
//! This pallet throws an event when an XLS-20 compatible NFT is minted which is picked up by
//! external relayers to mint the corresponding NFT on XRPL. The relayer then stores the minted
//! XLS-20 Token String back in this pallet by calling the `fulfill_xls20_mint` extrinsic.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{fungibles::Mutate, tokens::Preservation, Get},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use pallet_nft::traits::NFTCollectionInfo;
use seed_pallet_common::{Migrator, NFTExt, NFTMinter, Xls20Ext, Xls20MintRequest};
use seed_primitives::{
	xrpl::Xls20TokenId, AssetId, Balance, CollectionUuid, CrossChainCompatibility, MetadataScheme,
	OriginChain, SerialNumber, TokenCount, TokenId, WeightedDispatchResult,
};
use sp_runtime::{
	traits::{AccountIdConversion, Zero},
	DispatchResult, SaturatedConversion,
};
use sp_std::prelude::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod weights;
pub use weights::WeightInfo;

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod types;
use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// This pallet's ID, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Max amount of tokens that can be minted in a single XLS-20 mint request
		type MaxTokensPerXls20Mint: Get<u32>;
		/// Handles a multi-currency fungible asset system
		type MultiCurrency: Mutate<Self::AccountId, Balance = Balance, AssetId = AssetId>;
		/// Interface to access weight values
		type WeightInfo: WeightInfo;
		/// NFT ownership interface
		type NFTExt: NFTExt<AccountId = Self::AccountId>;
		/// NFT CollectionInfo trait
		type NFTCollectionInfo: NFTCollectionInfo<AccountId = Self::AccountId>;
		/// AssetId used to pay Xls20 Mint Fees
		type Xls20PaymentAsset: Get<AssetId>;
		/// The NFT token minter
		type NFTMinter: NFTMinter<AccountId = Self::AccountId>;
		/// Current Migrator handling the migration of storage values
		type Migrator: Migrator;
	}

	/// The permissioned relayer
	#[pallet::storage]
	pub type Relayer<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// The extra cost of minting an XLS-20 compatible NFT
	#[pallet::storage]
	pub type Xls20MintFee<T> = StorageValue<_, Balance, ValueQuery>;

	/// Maps from TRN native token_id to XLS-20 TokenId
	#[pallet::storage]
	pub type Xls20TokenMap<T> =
		StorageDoubleMap<_, Twox64Concat, CollectionUuid, Twox64Concat, SerialNumber, Xls20TokenId>;

	/// Map from XLs-20 Collection to CollectionUuid
	#[pallet::storage]
	pub type CollectionMapping<T> = StorageMap<_, Twox64Concat, Xls20Collection, CollectionUuid>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Request sent to XLS20 Relayer
		Xls20MintRequest {
			collection_id: CollectionUuid,
			serial_numbers: Vec<SerialNumber>,
			token_uris: Vec<Vec<u8>>,
		},
		/// A new relayer has been set
		RelayerSet { account: T::AccountId },
		/// A new Xls20 Mint Fee has been set
		Xls20MintFeeSet { new_fee: Balance },
		/// A new XLS20 mapping has been set
		Xls20MappingSet {
			collection_id: CollectionUuid,
			mappings: Vec<(SerialNumber, Xls20TokenId)>,
		},
		/// Xls20 collection mappings have been set
		Xls20CollectionMappingsSet { mappings: Vec<(CollectionUuid, Xls20Collection)> },
		/// A collection has had XLS-20 compatibility enabled
		Xls20CompatibilityEnabled { collection_id: CollectionUuid },
		/// Additional mint fee for XLS-20 mint has been paid to relayer
		Xls20MintFeePaid { collection_owner: T::AccountId, total_fee: Balance },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The caller is not the relayer and does not have permission to perform this action
		NotRelayer,
		/// There is already a Root native -> XLS-20 mapping for this token
		MappingAlreadyExists,
		/// The supplied fee for minting XLS-20 tokens is too low
		Xls20MintFeeTooLow,
		/// The collection is not compatible with XLS-20
		NotXLS20Compatible,
		/// The NFT does not exist
		NoToken,
		/// No the owner of the collection
		NotCollectionOwner,
		/// The XLS20 token ID failed to decode
		CouldNotDecodeXls20Token,
		/// The token is burnable and cannot be bridged
		CannotBridgeBurnableToken,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the relayer address
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_relayer())]
		pub fn set_relayer(origin: OriginFor<T>, relayer: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;
			<Relayer<T>>::put(&relayer);
			Self::deposit_event(Event::<T>::RelayerSet { account: relayer });
			Ok(())
		}

		/// Set the xls20 mint fee which is paid per token by the collection owner
		/// This covers the additional costs incurred by the relayer for the following:
		///  - Minting the token on XRPL
		///  - Calling fulfill_xls20_mint on The Root Network
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_xls20_fee())]
		pub fn set_xls20_fee(origin: OriginFor<T>, new_fee: Balance) -> DispatchResult {
			ensure_root(origin)?;
			<Xls20MintFee<T>>::put(new_fee);
			Self::deposit_event(Event::<T>::Xls20MintFeeSet { new_fee });
			Ok(())
		}

		/// Enables XLS-20 compatibility on a collection
		///  - Collection must not have any tokens minted
		///  - Caller must be collection owner
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::enable_xls20_compatibility())]
		pub fn enable_xls20_compatibility(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			T::NFTExt::enable_xls20_compatibility(who, collection_id)?;
			Self::deposit_event(Event::<T>::Xls20CompatibilityEnabled { collection_id });
			Ok(())
		}

		// Collection owners can re-request XLS-20 mints on tokens that have failed
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::re_request_xls20_mint())]
		#[transactional]
		pub fn re_request_xls20_mint(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerXls20Mint>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			T::Migrator::ensure_migrated()?;

			// serial_numbers can't be empty
			ensure!(!serial_numbers.len().is_zero(), Error::<T>::NoToken);

			let collection_info = T::NFTCollectionInfo::get_collection_info(collection_id)?;

			// Caller must be collection owner
			ensure!(collection_info.owner == who, Error::<T>::NotCollectionOwner);

			// Must be an XLS-20 compatible collection
			ensure!(collection_info.cross_chain_compatibility.xrpl, Error::<T>::NotXLS20Compatible);

			// Check whether token exists but mapping does not exist
			for serial_number in serial_numbers.iter() {
				ensure!(
					T::NFTExt::token_exists(&(collection_id, *serial_number)),
					Error::<T>::NoToken
				);
				ensure!(
					!Xls20TokenMap::<T>::contains_key(collection_id, serial_number),
					Error::<T>::MappingAlreadyExists
				);
			}

			Self::pay_xls20_fee(&who, serial_numbers.len() as TokenCount)?;
			Self::send_xls20_requests(
				collection_id,
				serial_numbers.into_inner(),
				collection_info.metadata_scheme,
			);

			Ok(())
		}

		/// Submit XLS-20 token ids to The Root Network
		/// Only callable by the trusted relayer account
		/// Can apply multiple mappings from the same collection in one transaction
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::fulfill_xls20_mint())]
		#[transactional]
		pub fn fulfill_xls20_mint(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			token_mappings: BoundedVec<(SerialNumber, Xls20TokenId), T::MaxTokensPerXls20Mint>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			T::Migrator::ensure_migrated()?;

			// Mappings can't be empty
			ensure!(!token_mappings.is_empty(), Error::<T>::NoToken);
			// Ensure only relayer can call extrinsic
			ensure!(Some(who) == Relayer::<T>::get(), Error::<T>::NotRelayer);

			let _ = T::NFTCollectionInfo::get_collection_info(collection_id)?;

			for (serial_number, xls20_token_id) in token_mappings.iter() {
				// Ensure token exists on TRN
				ensure!(
					T::NFTExt::token_exists(&(collection_id, *serial_number)),
					Error::<T>::NoToken
				);
				// Ensure mapping doesn't already exist
				ensure!(
					!Xls20TokenMap::<T>::contains_key(collection_id, serial_number),
					Error::<T>::MappingAlreadyExists
				);
				// Insert mapping into storage
				Xls20TokenMap::<T>::insert(collection_id, serial_number, xls20_token_id);
			}

			Self::deposit_event(Event::<T>::Xls20MappingSet {
				collection_id,
				mappings: token_mappings.into_inner(),
			});
			Ok(())
		}

		/// Manually set the collection mapping for a TRN collection to XRPL issuer and taxon
		/// Used to set up initial mappings for bridged tokens
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::set_collection_mappings(mappings.len() as u32))]
		#[transactional]
		pub fn set_collection_mappings(
			origin: OriginFor<T>,
			mappings: Vec<(CollectionUuid, Xls20Collection)>,
		) -> DispatchResult {
			ensure_root(origin)?;
			mappings.iter().for_each(|(collection_id, xls20_collection)| {
				CollectionMapping::<T>::insert(xls20_collection, collection_id);
			});
			Self::deposit_event(Event::<T>::Xls20CollectionMappingsSet { mappings });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Pay additional fee to cover relayer costs for minting XLS-20 tokens
	pub fn pay_xls20_fee(who: &T::AccountId, token_count: TokenCount) -> DispatchResult {
		let xls20_mint_fee = Xls20MintFee::<T>::get();
		if xls20_mint_fee.is_zero() {
			return Ok(());
		}
		if let Some(relayer) = Relayer::<T>::get() {
			// Fee is per token minted
			let nft_count: u32 = token_count.saturated_into();
			let mint_fee: Balance = nft_count.saturating_mul(xls20_mint_fee as u32).into();
			// Make the payment
			T::MultiCurrency::transfer(
				T::Xls20PaymentAsset::get(),
				who,
				&relayer,
				mint_fee,
				Preservation::Expendable,
			)?;
			Self::deposit_event(Event::<T>::Xls20MintFeePaid {
				collection_owner: who.clone(),
				total_fee: mint_fee,
			});
		}

		Ok(())
	}

	/// For XLS-20 compatible mints, we need to throw an event that gets picked up by the relayer
	/// The relayer can then mint these tokens on XRPL and notify The Root Network by calling
	/// The fulfill_xls20_mint callback extrinsic
	pub fn send_xls20_requests(
		collection_id: CollectionUuid,
		serial_numbers: Vec<SerialNumber>,
		metadata_scheme: MetadataScheme,
	) {
		// Gather token uris for each token being requested
		let mut token_uris: Vec<Vec<u8>> = vec![];
		for serial_number in serial_numbers.iter() {
			let token_uri = metadata_scheme.construct_token_uri(*serial_number);
			token_uris.push(token_uri);
		}

		// Deposit event containing all serial numbers and token_uris
		Self::deposit_event(Event::<T>::Xls20MintRequest {
			collection_id,
			serial_numbers,
			token_uris,
		});
	}
}

impl<T: Config> Xls20MintRequest for Pallet<T> {
	type AccountId = T::AccountId;

	fn request_xls20_mint(
		who: &Self::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: Vec<SerialNumber>,
		metadata_scheme: MetadataScheme,
	) -> DispatchResult {
		Self::pay_xls20_fee(who, serial_numbers.len() as TokenCount)?;
		Self::send_xls20_requests(collection_id, serial_numbers, metadata_scheme);
		Ok(())
	}
}

impl<T: Config> Xls20Ext for Pallet<T> {
	type AccountId = T::AccountId;

	fn deposit_xls20_token(
		receiver: &Self::AccountId,
		xls20_token_id: Xls20TokenId,
	) -> WeightedDispatchResult {
		// Ensure the migration is complete
		T::Migrator::ensure_migrated().map_err(|e| (Weight::zero(), e))?;
		let xls20_token = Xls20Token::from(xls20_token_id);

		// Check flag is not burnable, if the burnable flag is set then the issuer can
		// burn the token at any time. For simplicity it is easier to disallow bridging
		// of these tokens to respect the original design of the XLS-20 standard
		// TODO, we may need some way to retrieve tokens that failed to bridge
		if xls20_token.is_burnable() {
			return Err((Weight::zero(), Error::<T>::CannotBridgeBurnableToken.into()));
		}

		let pallet_address = T::PalletId::get().into_account_truncating();
		let xls20_collection = Xls20Collection::new(xls20_token.issuer, xls20_token.taxon);
		if let Some(collection_uuid) = CollectionMapping::<T>::get(xls20_collection) {
			// Mint the token and set up the mapping for reverse bridging
			let _ = T::NFTMinter::mint_bridged_nft(
				receiver,
				collection_uuid,
				vec![xls20_token.sequence],
			)
			.map_err(|(_, e)| (T::WeightInfo::deposit_token_mint(), e))?;
			Xls20TokenMap::<T>::insert(collection_uuid, xls20_token.sequence, xls20_token_id);
			return Ok(T::WeightInfo::deposit_token_mint());
		}

		// The collection does not exist on TRN yet for a newly bridged XRPL token.
		// Create the collection and mint the token
		let collection_name = BoundedVec::truncate_from(b"xls20-bridged-collection".to_vec());
		let metadata_scheme = MetadataScheme(BoundedVec::truncate_from(b"xls20://".to_vec()));
		let collection_uuid = T::NFTExt::do_create_collection(
			pallet_address,
			collection_name,
			0,
			None,
			None,
			metadata_scheme,
			None,
			OriginChain::XRPL,
			CrossChainCompatibility { xrpl: true },
		)
		.map_err(|e| {
			// Although not 100% accurate as this benchmark is for a successful case, it is unlikely
			// that this will fail
			(T::WeightInfo::deposit_token_create_collection(), e)
		})?;
		CollectionMapping::<T>::insert(xls20_collection, collection_uuid);

		let _ =
			T::NFTMinter::mint_bridged_nft(receiver, collection_uuid, vec![xls20_token.sequence])
				.map_err(|(_, e)| (T::WeightInfo::deposit_token_create_collection(), e))?;
		Xls20TokenMap::<T>::insert(collection_uuid, xls20_token.sequence, xls20_token_id);
		Ok(T::WeightInfo::deposit_token_create_collection())
	}

	fn get_xls20_token_id(token_id: TokenId) -> Option<Xls20TokenId> {
		// Ensure the migration is complete
		T::Migrator::ensure_migrated().ok()?;
		Xls20TokenMap::<T>::get(token_id.0, token_id.1)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn set_xls20_token_id(token_id: TokenId, xls20_token_id: Xls20TokenId) {
		Xls20TokenMap::<T>::insert(token_id.0, token_id.1, xls20_token_id);
	}
}
