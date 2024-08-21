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
};
use frame_system::pallet_prelude::*;
use seed_pallet_common::{NFIRequest, NFTExt, SFTExt};
use seed_primitives::{AssetId, Balance, CollectionUuid, SerialNumber, TokenCount, TokenId};
use sp_runtime::{traits::Zero, DispatchResult, Permill};
use sp_std::prelude::*;

pub use pallet::*;

mod types;
use types::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;
pub use weights::WeightInfo;

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
		/// NFT Extension
		type NFTExt: NFTExt<AccountId = Self::AccountId>;
		/// SFT Extension
		type SFTExt: SFTExt<AccountId = Self::AccountId>;
		/// Percentage of sale price to charge for network fee
		#[pallet::constant]
		type NetworkFeePercentage: Get<Permill>;
		/// Max length of data stored per token
		#[pallet::constant]
		type MaxDataLength: Get<u32>;
		/// Provides the public call to weight mapping
		type WeightInfo: WeightInfo;
	}

	/// The permission enabled relayer
	#[pallet::storage]
	pub type Relayer<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// The extra cost to cover
	#[pallet::storage]
	pub type MintFee<T: Config> = StorageMap<_, Twox64Concat, NFISubType, FeeDetails<T::AccountId>>;

	#[pallet::storage]
	pub type NfiData<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		TokenId,
		Twox64Concat,
		NFISubType,
		NFIDataType<T::MaxDataLength>,
	>;

	#[pallet::storage]
	pub type NfiEnabled<T> = StorageDoubleMap<
		_,
		Twox64Concat,
		CollectionUuid,
		Twox64Concat,
		NFISubType,
		bool,
		ValueQuery,
	>;

	/// The pallet id for the tx fee pot
	#[pallet::storage]
	pub type FeeTo<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Request for new NFI data creation
		DataRequest {
			sub_type: NFISubType,
			caller: T::AccountId,
			collection_id: CollectionUuid,
			serial_numbers: Vec<SerialNumber>,
		},
		/// A new NFI storage item has been set
		DataSet {
			sub_type: NFISubType,
			token_id: TokenId,
			data_item: NFIDataType<T::MaxDataLength>,
		},
		/// New Fee details have been set
		FeeDetailsSet { sub_type: NFISubType, fee_details: Option<FeeDetails<T::AccountId>> },
		/// The network fee receiver address has been updated
		FeeToSet { account: Option<T::AccountId> },
		/// Additional mint fee has been paid to the receiver address
		MintFeePaid {
			sub_type: NFISubType,
			who: T::AccountId,
			asset_id: AssetId,
			total_fee: Balance,
		},
		/// NFI compatibility enabled for a collection
		NfiEnabled { sub_type: NFISubType, collection_id: CollectionUuid },
		/// A new relayer has been set
		RelayerSet { account: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The mint fee must be a valid integer above 0
		InvalidMintFee,
		/// NFI storage is not enabled for this collection
		NotEnabled,
		/// The caller is not the relayer and does not have permission to perform this action
		NotRelayer,
		/// No the owner of the collection
		NotCollectionOwner,
		/// The caller is not the owner of the token
		NotTokenOwner,
		/// The token does not exist
		NoToken,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the relayer address
		/// This address is able to submit the NFI data back to the chain
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_relayer())]
		pub fn set_relayer(origin: OriginFor<T>, relayer: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;
			<Relayer<T>>::put(&relayer);
			Self::deposit_event(Event::<T>::RelayerSet { account: relayer });
			Ok(())
		}

		/// Set the `FeeTo` account
		/// This operation requires root access
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_fee_to())]
		pub fn set_fee_to(origin: OriginFor<T>, fee_to: Option<T::AccountId>) -> DispatchResult {
			ensure_root(origin)?;
			match fee_to.clone() {
				None => FeeTo::<T>::kill(),
				Some(account) => FeeTo::<T>::put(account),
			}
			Self::deposit_event(Event::FeeToSet { account: fee_to });
			Ok(())
		}

		/// Set the NFI mint fee which is paid per token by the minter
		/// Setting fee_details to None removes the mint fee
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_fee_details())]
		pub fn set_fee_details(
			origin: OriginFor<T>,
			sub_type: NFISubType,
			fee_details: Option<FeeDetails<T::AccountId>>,
		) -> DispatchResult {
			ensure_root(origin)?;
			match fee_details.clone() {
				Some(details) => {
					ensure!(!details.amount.is_zero(), Error::<T>::InvalidMintFee);
					<MintFee<T>>::insert(sub_type, details);
				},
				None => <MintFee<T>>::remove(sub_type),
			}
			Self::deposit_event(Event::<T>::FeeDetailsSet { sub_type, fee_details });
			Ok(())
		}

		/// Enables NFI compatibility on a collection
		///  - Caller must be collection owner
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::enable_nfi())]
		pub fn enable_nfi(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			sub_type: NFISubType,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_collection_owner(collection_id, &who), Error::<T>::NotCollectionOwner);
			<NfiEnabled<T>>::insert(collection_id, sub_type, true);
			Self::deposit_event(Event::<T>::NfiEnabled { sub_type, collection_id });
			Ok(())
		}

		/// Users can manually request NFI data if it does not already exist on a token.
		/// This can be used to manually request data for pre-existing tokens in a collection
		/// that has had nfi enabled
		/// Caller must be either the token owner or the collection owner
		/// Note. the mint fee will need to be paid for any manual request
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::manual_data_request())]
		pub fn manual_data_request(
			origin: OriginFor<T>,
			token_id: TokenId,
			sub_type: NFISubType,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(NfiEnabled::<T>::get(token_id.0, sub_type), Error::<T>::NotEnabled);
			// Check that the caller is the token or collection owner
			ensure!(Self::check_permissions(token_id.clone(), &who), Error::<T>::NotTokenOwner);
			Self::pay_mint_fee(&who, 1, sub_type)?;
			Self::send_data_request(who, sub_type, token_id.0, vec![token_id.1]);
			Ok(())
		}

		/// submit NFI data to the chain
		/// Caller must be the relayer
		/// NFI must be enabled for the collection
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::submit_nfi_data())]
		pub fn submit_nfi_data(
			origin: OriginFor<T>,
			token_id: TokenId,
			data_item: NFIDataType<T::MaxDataLength>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Some(who) == Relayer::<T>::get(), Error::<T>::NotRelayer);
			let sub_type = NFISubType::from(data_item.clone());
			ensure!(NfiEnabled::<T>::get(token_id.0, sub_type), Error::<T>::NotEnabled);
			ensure!(Self::token_exists(token_id.clone()), Error::<T>::NoToken);
			NfiData::<T>::insert(token_id.clone(), sub_type.clone(), data_item.clone());
			Self::deposit_event(Event::<T>::DataSet { sub_type, token_id, data_item });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Pay additional fee to cover relayer costs for creating extra NFI storage
	pub fn pay_mint_fee(
		who: &T::AccountId,
		token_count: TokenCount,
		sub_type: NFISubType,
	) -> DispatchResult {
		let Some(fee_details) = MintFee::<T>::get(sub_type) else { return Ok(()) };
		// Fee is per token minted
		let total_fee: Balance = (token_count as u128).saturating_mul(fee_details.amount);
		let mut total_fee_adjusted = total_fee;

		// Get network fee portion and pay out network fees if applicable
		if let Some(tx_fee_pot_id) = FeeTo::<T>::get() {
			let network_fee = T::NetworkFeePercentage::get();
			let network_amount = network_fee * total_fee;
			total_fee_adjusted = total_fee.saturating_sub(network_amount);
			T::MultiCurrency::transfer(
				fee_details.asset_id,
				who,
				&tx_fee_pot_id,
				network_amount,
				Preservation::Expendable,
			)?;
		}

		// Make the payment to the receiver address
		T::MultiCurrency::transfer(
			fee_details.asset_id,
			who,
			&fee_details.receiver,
			total_fee_adjusted,
			Preservation::Expendable,
		)?;

		// Deposit event with total fee paid
		Self::deposit_event(Event::<T>::MintFeePaid {
			sub_type,
			who: who.clone(),
			asset_id: fee_details.asset_id,
			total_fee,
		});

		Ok(())
	}

	/// Emits an event to display which tokens need NFI data to be created off-chain
	pub fn send_data_request(
		caller: T::AccountId,
		sub_type: NFISubType,
		collection_id: CollectionUuid,
		serial_numbers: Vec<SerialNumber>,
	) {
		// Deposit event containing collection_id and all serial numbers
		Self::deposit_event(Event::<T>::DataRequest {
			caller,
			sub_type,
			collection_id,
			serial_numbers,
		});
	}

	/// Returns true if who is the owner of the collection.
	/// Checks both NFT and SFT pallet
	fn is_collection_owner(collection_id: CollectionUuid, who: &T::AccountId) -> bool {
		if let Ok(nft_owner) = T::NFTExt::get_collection_owner(collection_id) {
			return who == &nft_owner;
		}
		if let Ok(sft_owner) = T::SFTExt::get_collection_owner(collection_id) {
			return who == &sft_owner;
		}
		false
	}

	// Returns true if who is the owner of the token for an NFT, or the collection owner.
	// For SFT it only checks whether who is the owner of the collection
	// This is due to SFT tokens being owned by the collection owner, where users can have some
	// balance of the token
	fn check_permissions(token_id: TokenId, who: &T::AccountId) -> bool {
		if let Some(nft_token_owner) = T::NFTExt::get_token_owner(&token_id) {
			if who == &nft_token_owner {
				return true;
			}
			// Not token owner, check if who is the collection owner
			if let Ok(nft_collection_owner) = T::NFTExt::get_collection_owner(token_id.0) {
				return who == &nft_collection_owner;
			}
		}
		if let Ok(sft_collection_owner) = T::SFTExt::get_collection_owner(token_id.0) {
			return who == &sft_collection_owner;
		}
		false
	}

	// Check whether a token exists or not
	fn token_exists(token_id: TokenId) -> bool {
		if T::NFTExt::get_token_owner(&token_id).is_some() {
			return true;
		}
		T::SFTExt::token_exists(token_id)
	}
}

impl<T: Config> NFIRequest for Pallet<T> {
	type AccountId = T::AccountId;

	/// Request from the NFT pallet to create an NFI for a token
	/// Hardcoded to NFI for now. In future, there may be use cases for extending this pallet to
	/// handle multiple NFISubTypes
	fn request(
		who: &Self::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: Vec<SerialNumber>,
	) -> DispatchResult {
		let sub_type = NFISubType::NFI;
		// Check if NFI is enabled for this collection. If not, we don't need to do anything
		if !NfiEnabled::<T>::get(collection_id, sub_type) {
			return Ok(());
		}
		// Pay the mint fee for the NFI storage, return an error if this is not possible
		Self::pay_mint_fee(who, serial_numbers.len() as TokenCount, sub_type)?;
		Self::send_data_request(who.clone(), sub_type, collection_id, serial_numbers);
		Ok(())
	}

	// A token was burned so we can remove the data assosciated with it to save space in the pallet
	fn on_burn(token_id: TokenId) {
		// Limit of tokens to be removed with the clear_prefix call. This should be larger than the
		// number of enum variants in NFISubType
		let limit: u32 = 10;
		// Remove all NFI data for this token
		let _ = NfiData::<T>::clear_prefix(token_id, limit, None);
	}
}