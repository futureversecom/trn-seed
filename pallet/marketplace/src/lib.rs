// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

#![cfg_attr(not(feature = "std"), no_std)]
//! # Marketplace Module
//!
//! Provides marketplace functionality for NFT and SFT pallets
//!
//! Allows users to buy or sell tokens, register as a marketplace and distribute royalties
//! per sale.
//! Also allows for offers on these tokens, which can be accepted by the owner of the token.

use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_nft::{weights::WeightInfo as NftWeightInfo, ListingId, MarketplaceId, OfferId};
use seed_primitives::{AssetId, Balance, CollectionUuid, SerialNumber, TokenId};
use sp_runtime::{DispatchResult, Permill};

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::{DispatchResult, *};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_nft::Config {
		/// The overarching call type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo
			+ From<pallet_nft::Call<Self>>;
		/// Provides the public call to weight mapping
		type WeightInfo: NftWeightInfo;
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Flag an account as a marketplace
		///
		/// `marketplace_account` - if specified, this account will be registered
		/// `entitlement` - Permill, percentage of sales to go to the marketplace
		/// If no marketplace is specified the caller will be registered
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::register_marketplace())]
		pub fn register_marketplace(
			origin: OriginFor<T>,
			marketplace_account: Option<T::AccountId>,
			entitlement: Permill,
		) -> DispatchResult {
			let call =
				pallet_nft::Call::<T>::register_marketplace { marketplace_account, entitlement };
			let call = <T as Config>::RuntimeCall::from(call);
			call.dispatch(origin).map_err(|err| err.error)?;
			Ok(())
		}

		/// Sell a bundle of tokens at a fixed price
		/// - Tokens must be from the same collection
		/// - Tokens with individual royalties schedules cannot be sold with this method
		///
		/// `buyer` optionally, the account to receive the NFT. If unspecified, then any account may
		/// purchase `asset_id` fungible asset Id to receive as payment for the NFT
		/// `fixed_price` ask price
		/// `duration` listing duration time in blocks from now
		/// Caller must be the token owner
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::sell())]
		pub fn sell_nft(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
			buyer: Option<T::AccountId>,
			payment_asset: AssetId,
			fixed_price: Balance,
			duration: Option<T::BlockNumber>,
			marketplace_id: Option<MarketplaceId>,
		) -> DispatchResult {
			let call = pallet_nft::Call::<T>::sell {
				collection_id,
				serial_numbers,
				buyer,
				payment_asset,
				fixed_price,
				duration,
				marketplace_id,
			};
			let call = <T as Config>::RuntimeCall::from(call);
			call.dispatch(origin).map_err(|err| err.error)?;
			Ok(())
		}

		/// Update fixed price for a single token sale
		///
		/// `listing_id` id of the fixed price listing
		/// `new_price` new fixed price
		/// Caller must be the token owner
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::update_fixed_price())]
		pub fn update_fixed_price(
			origin: OriginFor<T>,
			listing_id: ListingId,
			new_price: Balance,
		) -> DispatchResult {
			let call = pallet_nft::Call::<T>::update_fixed_price { listing_id, new_price };
			let call = <T as Config>::RuntimeCall::from(call);
			call.dispatch(origin).map_err(|err| err.error)?;
			Ok(())
		}

		/// Buy a token listing for its specified price
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::buy())]
		pub fn buy(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
			let call = pallet_nft::Call::<T>::buy { listing_id };
			let call = <T as Config>::RuntimeCall::from(call);
			call.dispatch(origin).map_err(|err| err.error)?;
			Ok(())
		}

		/// Auction a bundle of tokens on the open market to the highest bidder
		/// - Tokens must be from the same collection
		/// - Tokens with individual royalties schedules cannot be sold in bundles
		///
		/// Caller must be the token owner
		/// - `payment_asset` fungible asset Id to receive payment with
		/// - `reserve_price` winning bid must be over this threshold
		/// - `duration` length of the auction (in blocks), uses default duration if unspecified
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::auction())]
		pub fn auction_nft(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
			payment_asset: AssetId,
			reserve_price: Balance,
			duration: Option<T::BlockNumber>,
			marketplace_id: Option<MarketplaceId>,
		) -> DispatchResult {
			let call = pallet_nft::Call::<T>::auction {
				collection_id,
				serial_numbers,
				payment_asset,
				reserve_price,
				duration,
				marketplace_id,
			};
			let call = <T as Config>::RuntimeCall::from(call);
			call.dispatch(origin).map_err(|err| err.error)?;
			Ok(())
		}

		/// Place a bid on an open auction
		/// - `amount` to bid (in the seller's requested payment asset)
		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::bid())]
		pub fn bid(origin: OriginFor<T>, listing_id: ListingId, amount: Balance) -> DispatchResult {
			let call = pallet_nft::Call::<T>::bid { listing_id, amount };
			let call = <T as Config>::RuntimeCall::from(call);
			call.dispatch(origin).map_err(|err| err.error)?;
			Ok(())
		}

		/// Close a sale or auction returning tokens
		/// Requires no successful bids have been made for an auction.
		/// Caller must be the listed seller
		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::cancel_sale())]
		pub fn cancel_sale(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
			let call = pallet_nft::Call::<T>::cancel_sale { listing_id };
			let call = <T as Config>::RuntimeCall::from(call);
			call.dispatch(origin).map_err(|err| err.error)?;
			Ok(())
		}

		/// Create an offer on a token
		/// Locks funds until offer is accepted, rejected or cancelled
		/// An offer can't be made on a token currently in an auction
		/// (This follows the behaviour of Opensea and forces the buyer to bid rather than create an
		/// offer)
		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::make_simple_offer())]
		pub fn make_simple_offer(
			origin: OriginFor<T>,
			token_id: TokenId,
			amount: Balance,
			asset_id: AssetId,
			marketplace_id: Option<MarketplaceId>,
		) -> DispatchResult {
			let call = pallet_nft::Call::<T>::make_simple_offer {
				token_id,
				amount,
				asset_id,
				marketplace_id,
			};
			let call = <T as Config>::RuntimeCall::from(call);
			call.dispatch(origin).map_err(|err| err.error)?;
			Ok(())
		}

		/// Cancels an offer on a token
		/// Caller must be the offer buyer
		#[pallet::call_index(8)]
		#[pallet::weight(<T as Config>::WeightInfo::cancel_offer())]
		pub fn cancel_offer(origin: OriginFor<T>, offer_id: OfferId) -> DispatchResult {
			let call = pallet_nft::Call::<T>::cancel_offer { offer_id };
			let call = <T as Config>::RuntimeCall::from(call);
			call.dispatch(origin).map_err(|err| err.error)?;
			Ok(())
		}

		/// Accepts an offer on a token
		/// Caller must be token owner
		#[pallet::call_index(9)]
		#[pallet::weight(<T as Config>::WeightInfo::accept_offer())]
		pub fn accept_offer(origin: OriginFor<T>, offer_id: OfferId) -> DispatchResult {
			let call = pallet_nft::Call::<T>::accept_offer { offer_id };
			let call = <T as Config>::RuntimeCall::from(call);
			call.dispatch(origin).map_err(|err| err.error)?;
			Ok(())
		}
	}
}
