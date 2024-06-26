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

use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{fungibles::Mutate, tokens::Preservation, Get},
	transactional,
};
use frame_system::pallet_prelude::*;
use pallet_nft::traits::NFTCollectionInfo;
use seed_pallet_common::MetaStorageRequest;
use seed_primitives::{AssetId, Balance, CollectionUuid, MetadataScheme, SerialNumber, TokenCount};
use sp_runtime::{traits::Zero, DispatchResult, SaturatedConversion};
use sp_std::prelude::*;

pub use pallet::*;

mod types;
use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

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
		type MultiCurrency: Mutate<Self::AccountId, Balance = Balance, AssetId = AssetId>;
		/// AssetId used to pay Xls20 Mint Fees
		type MetaStorageFeeAsset: Get<AssetId>;
	}

	/// The permissioned relayer
	#[pallet::storage]
	pub type Relayer<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// The extra cost of minting an XLS-20 compatible NFT
	#[pallet::storage]
	pub type MintFee<T> = StorageValue<_, Balance, ValueQuery>;

	#[pallet::storage]
	pub type MetaStorage<T> = StorageDoubleMap<
		_,
		Twox64Concat,
		MetaStorageItem,
		Twox64Concat,
		MetaSubType,
		MetaStorageType,
	>;

	#[pallet::storage]
	pub type MetaStorageEnabled<T> =
		StorageDoubleMap<_, Twox64Concat, MetaStorageItem, Twox64Concat, MetaSubType, bool>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Request sent to XLS20 Relayer
		MetaStorageMintRequest { collection_id: CollectionUuid, serial_numbers: Vec<SerialNumber> },
		/// A new relayer has been set
		RelayerSet { account: T::AccountId },
		/// A new Xls20 Mint Fee has been set
		MintFeeSet { new_fee: Balance },
		/// A new Meta Storage item has been set
		MetaStorageSet {
			token: MetaStorageItem,
			sub_type: MetaSubType,
			meta_storage_item: MetaStorageType,
		},
		/// Meta Storage compatibility enabled
		MetaStorageEnabled { token: MetaStorageItem, sub_type: MetaSubType },
		/// Additional mint fee for Meta Storage creation has been paid to relayer
		MintFeePaid { collection_owner: T::AccountId, total_fee: Balance },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The caller is not the relayer and does not have permission to perform this action
		NotRelayer,
		/// There is already a Root native -> XLS-20 mapping for this token
		MappingAlreadyExists,
		/// The supplied fee for minting XLS-20 tokens is too low
		MintFeeTooLow,
		/// The collection is not compatible with XLS-20
		NotXLS20Compatible,
		/// The NFT does not exist
		NoToken,
		/// No the owner of the collection
		NotCollectionOwner,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the relayer address
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
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
		#[pallet::weight(0)]
		pub fn set_mint_fee(origin: OriginFor<T>, new_fee: Balance) -> DispatchResult {
			ensure_root(origin)?;
			<MintFee<T>>::put(new_fee);
			Self::deposit_event(Event::<T>::MintFeeSet { new_fee });
			Ok(())
		}

		/// Enables XLS-20 compatibility on a collection
		///  - Collection must not have any tokens minted
		///  - Caller must be collection owner
		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn enable_meta_storage(
			origin: OriginFor<T>,
			token: MetaStorageItem,
			sub_type: MetaSubType,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// TODO

			Self::deposit_event(Event::<T>::MetaStorageEnabled { token, sub_type });
			Ok(())
		}

		/// Submit XLS-20 token ids to The Root Network
		/// Only callable by the trusted relayer account
		/// Can apply multiple mappings from the same collection in one transaction
		#[pallet::call_index(4)]
		#[pallet::weight(0)]
		#[transactional]
		pub fn submit_meta_storage(
			origin: OriginFor<T>,
			token: MetaStorageItem,
			sub_type: MetaSubType,
			meta_storage_item: MetaStorageType,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Ensure only relayer can call extrinsic
			ensure!(Some(who) == Relayer::<T>::get(), Error::<T>::NotRelayer);

			MetaStorage::<T>::insert(token.clone(), sub_type.clone(), meta_storage_item);

			Self::deposit_event(Event::<T>::MetaStorageSet { token, sub_type, meta_storage_item });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Pay additional fee to cover relayer costs for creating extra meta storage
	pub fn pay_mint_fee(who: &T::AccountId, token_count: TokenCount) -> DispatchResult {
		// let mint_fee = MintFee::<T>::get();
		// if mint_fee.is_zero() {
		// 	return Ok(())
		// }
		// if let Some(relayer) = Relayer::<T>::get() {
		// 	// Fee is per token minted
		// 	let nft_count: u32 = token_count.saturated_into();
		// 	let mint_fee: Balance = nft_count.saturating_mul(mint_fee as u32).into();
		// 	// Make the payment
		// 	T::MultiCurrency::transfer(
		// 		T::Xls20PaymentAsset::get().into(),
		// 		who,
		// 		&relayer,
		// 		mint_fee,
		// 		Preservation::Expendable,
		// 	)?;
		// 	Self::deposit_event(Event::<T>::MintFeePaid {
		// 		collection_owner: who.clone(),
		// 		total_fee: mint_fee,
		// 	});
		// }

		Ok(())
	}

	pub fn send_meta_request(collection_id: CollectionUuid, serial_numbers: Vec<SerialNumber>) {
		// Gather token uris for each token being requested
		let mut token_uris: Vec<Vec<u8>> = vec![];

		// Deposit event containing all serial numbers and token_uris
		Self::deposit_event(Event::<T>::MetaStorageMintRequest { collection_id, serial_numbers });
	}
}

impl<T: Config> MetaStorageRequest for Pallet<T> {
	type AccountId = T::AccountId;

	fn request_meta_storage(
		who: &Self::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: Vec<SerialNumber>,
	) -> DispatchResult {
		// TODO if enabled only, otherwise return error

		Self::pay_mint_fee(who, serial_numbers.len() as TokenCount)?;
		Self::send_meta_request(collection_id, serial_numbers);
		Ok(())
	}
}
