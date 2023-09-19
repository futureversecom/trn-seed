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

use crate::*;
use frame_support::{ensure, traits::Get, transactional};
use pallet_nft::traits::NFTExt;
use seed_pallet_common::{log, Hold, TransferExt};
use seed_primitives::{AssetId, Balance, CollectionUuid, RoyaltiesSchedule, SerialNumber, TokenId};
use sp_runtime::{
	traits::{One, Saturating, Zero},
	BoundedVec, DispatchError, DispatchResult, PerThing, Permill,
};
use sp_std::{vec, vec::Vec};
use types::*;

impl<T: Config> Pallet<T> {
	pub fn do_register_marketplace(
		who: T::AccountId,
		marketplace_account: Option<T::AccountId>,
		entitlement: Permill,
	) -> Result<MarketplaceId, DispatchError> {
		ensure!(
			entitlement.deconstruct() as u32 <= Permill::ACCURACY,
			Error::<T>::RoyaltiesInvalid
		);
		let marketplace_account = marketplace_account.unwrap_or(who);
		let marketplace_id = Self::next_marketplace_id();
		let marketplace = Marketplace { account: marketplace_account.clone(), entitlement };
		let next_marketplace_id = <NextMarketplaceId<T>>::get();
		ensure!(next_marketplace_id.checked_add(One::one()).is_some(), Error::<T>::NoAvailableIds);

		<RegisteredMarketplaces<T>>::insert(&marketplace_id, marketplace);
		<NextMarketplaceId<T>>::mutate(|i| *i += 1);

		Self::deposit_event(Event::<T>::MarketplaceRegister {
			account: marketplace_account,
			entitlement,
			marketplace_id,
		});
		Ok(marketplace_id)
	}

	pub fn do_sell_nft(
		who: T::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerListing>,
		buyer: Option<T::AccountId>,
		payment_asset: AssetId,
		fixed_price: Balance,
		duration: Option<T::BlockNumber>,
		marketplace_id: Option<MarketplaceId>,
	) -> Result<ListingId, DispatchError> {
		ensure!(!serial_numbers.is_empty(), Error::<T>::NoToken);
		let royalties_schedule = Self::calculate_bundle_royalties(collection_id, marketplace_id)?;
		let listing_id = Self::next_listing_id();

		// use the first token's collection as representative of the bundle
		Self::lock_tokens_for_listing(collection_id, &serial_numbers, &who, listing_id)?;

		let listing_end_block = <frame_system::Pallet<T>>::block_number()
			.saturating_add(duration.unwrap_or_else(T::DefaultListingDuration::get));
		let listing = Listing::<T>::FixedPrice(FixedPriceListing::<T> {
			payment_asset,
			fixed_price,
			close: listing_end_block,
			collection_id,
			serial_numbers: serial_numbers.clone(),
			buyer: buyer.clone(),
			seller: who.clone(),
			royalties_schedule,
			marketplace_id,
		});

		<ListingEndSchedule<T>>::insert(listing_end_block, listing_id, true);
		<OpenCollectionListings<T>>::insert(collection_id, listing_id, true);
		<Listings<T>>::insert(listing_id, listing);
		<NextListingId<T>>::mutate(|i| *i += 1);

		Self::deposit_event(Event::<T>::FixedPriceSaleList {
			collection_id,
			serial_numbers: serial_numbers.into_inner(),
			listing_id,
			marketplace_id,
			price: fixed_price,
			payment_asset,
			seller: who,
		});
		Ok(listing_id)
	}

	pub fn do_update_fixed_price(
		who: T::AccountId,
		listing_id: ListingId,
		new_price: Balance,
	) -> DispatchResult {
		let Some(Listing::FixedPrice(mut listing)) = Listings::<T>::get(listing_id) else {
			return Err(Error::<T>::NotForFixedPriceSale.into());
		};
		ensure!(listing.seller == who, Error::<T>::NotSeller);

		listing.fixed_price = new_price;

		<Listings<T>>::insert(listing_id, Listing::<T>::FixedPrice(listing.clone()));
		Self::deposit_event(Event::<T>::FixedPriceSalePriceUpdate {
			collection_id: listing.collection_id,
			serial_numbers: listing.serial_numbers.into_inner(),
			listing_id,
			new_price,
		});
		Ok(())
	}

	/// Returns the listing detail of a specified listing_id
	pub fn get_listing_detail(listing_id: ListingId) -> Result<Listing<T>, DispatchError> {
		let Some(listing) = Listings::<T>::get(listing_id) else {
			return Err(Error::<T>::NotForFixedPriceSale.into());
		};
		Ok(listing)
		// token_info.token_issuance
	}

	// /// Returns the offer detail of a specified offer_id
	pub fn get_offer_detail(offer_id: OfferId) -> Result<SimpleOffer<T::AccountId>, DispatchError> {
		let Some(OfferType::Simple(offer)) = Self::offers(offer_id) else {
			return Err(Error::<T>::InvalidOffer.into());
		};
		Ok(offer)
	}

	pub fn do_buy(who: T::AccountId, listing_id: ListingId) -> DispatchResult {
		let Some(Listing::FixedPrice(listing)) = Listings::<T>::get(listing_id) else {
			return Err(Error::<T>::NotForFixedPriceSale.into());
		};

		// if buyer is specified in the listing, then `who` must be buyer
		if let Some(buyer) = &listing.buyer {
			ensure!(&who == buyer, Error::<T>::NotBuyer);
		}

		Self::remove_listing(Listing::FixedPrice(listing.clone()), listing_id);

		let payouts = Self::calculate_royalty_payouts(
			listing.seller.clone(),
			listing.royalties_schedule,
			listing.fixed_price,
		);
		// Make split transfer
		T::MultiCurrency::split_transfer(&who, listing.payment_asset, payouts.as_slice())?;

		// Transfer the tokens
		let _ = T::NFTExt::do_transfer(
			listing.seller.clone(),
			listing.collection_id,
			listing.serial_numbers.clone().into_inner(),
			who.clone(),
		)?;

		Self::deposit_event(Event::<T>::FixedPriceSaleComplete {
			collection_id: listing.collection_id,
			serial_numbers: listing.serial_numbers.into_inner(),
			listing_id,
			price: listing.fixed_price,
			payment_asset: listing.payment_asset,
			buyer: who,
			seller: listing.seller,
		});
		Ok(())
	}

	pub fn do_auction_nft(
		who: T::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerListing>,
		payment_asset: AssetId,
		reserve_price: Balance,
		duration: Option<T::BlockNumber>,
		marketplace_id: Option<MarketplaceId>,
	) -> Result<ListingId, DispatchError> {
		ensure!(!serial_numbers.is_empty(), Error::<T>::NoToken);

		let royalties_schedule = Self::calculate_bundle_royalties(collection_id, marketplace_id)?;

		let listing_id = Self::next_listing_id();
		ensure!(listing_id.checked_add(One::one()).is_some(), Error::<T>::NoAvailableIds);

		Self::lock_tokens_for_listing(collection_id, &serial_numbers, &who, listing_id)?;

		let listing_end_block = <frame_system::Pallet<T>>::block_number()
			.saturating_add(duration.unwrap_or_else(T::DefaultListingDuration::get));
		let listing = Listing::<T>::Auction(AuctionListing::<T> {
			payment_asset,
			reserve_price,
			close: listing_end_block,
			collection_id,
			serial_numbers: serial_numbers.clone(),
			seller: who.clone(),
			royalties_schedule,
			marketplace_id,
		});

		<ListingEndSchedule<T>>::insert(listing_end_block, listing_id, true);
		<OpenCollectionListings<T>>::insert(collection_id, listing_id, true);
		<Listings<T>>::insert(listing_id, listing);
		<NextListingId<T>>::mutate(|i| *i += 1);

		Self::deposit_event(Event::<T>::AuctionOpen {
			collection_id,
			serial_numbers: serial_numbers.into_inner(),
			payment_asset,
			reserve_price,
			listing_id,
			marketplace_id,
			seller: who,
		});
		Ok(listing_id)
	}

	pub fn do_bid(who: T::AccountId, listing_id: ListingId, amount: Balance) -> DispatchResult {
		let mut listing = match Listings::<T>::get(listing_id) {
			Some(Listing::Auction(listing)) => listing,
			_ => return Err(Error::<T>::NotForAuction.into()),
		};

		if let Some(current_bid) = Self::listing_winning_bid(listing_id) {
			ensure!(amount > current_bid.1, Error::<T>::BidTooLow);
		} else {
			// first bid
			ensure!(amount >= listing.reserve_price, Error::<T>::BidTooLow);
		}

		// try lock funds
		T::MultiCurrency::place_hold(T::PalletId::get(), &who, listing.payment_asset, amount)?;

		<ListingWinningBid<T>>::try_mutate(listing_id, |maybe_current_bid| -> DispatchResult {
			if let Some(current_bid) = maybe_current_bid {
				// replace old bid
				let _ = T::MultiCurrency::release_hold(
					T::PalletId::get(),
					&current_bid.0,
					listing.payment_asset,
					current_bid.1,
				)?;
			}
			*maybe_current_bid = Some((who.clone(), amount));
			Ok(())
		})?;

		// Auto extend auction if bid is made within certain amount of time of auction
		// duration
		let listing_end_block = listing.close;
		let current_block = <frame_system::Pallet<T>>::block_number();
		let blocks_till_close = listing_end_block - current_block;
		let new_closing_block = current_block + T::BlockNumber::from(AUCTION_EXTENSION_PERIOD);
		if blocks_till_close <= T::BlockNumber::from(AUCTION_EXTENSION_PERIOD) {
			ListingEndSchedule::<T>::remove(listing_end_block, listing_id);
			ListingEndSchedule::<T>::insert(new_closing_block, listing_id, true);
			listing.close = new_closing_block;
			Listings::<T>::insert(listing_id, Listing::Auction(listing.clone()));
		}

		Self::deposit_event(Event::<T>::Bid {
			collection_id: listing.collection_id,
			serial_numbers: listing.serial_numbers.into_inner(),
			listing_id,
			amount,
			bidder: who,
		});
		Ok(())
	}

	pub fn do_cancel_sale(who: T::AccountId, listing_id: ListingId) -> DispatchResult {
		let listing = Listings::<T>::get(listing_id).ok_or(Error::<T>::TokenNotListed)?;

		match listing {
			Listing::<T>::FixedPrice(sale) => {
				ensure!(sale.seller == who, Error::<T>::NotSeller);
				Listings::<T>::remove(listing_id);
				ListingEndSchedule::<T>::remove(sale.close, listing_id);
				for serial_number in sale.serial_numbers.iter() {
					T::NFTExt::set_token_lock((sale.collection_id, *serial_number), None)?;
				}
				<OpenCollectionListings<T>>::remove(sale.collection_id, listing_id);

				Self::deposit_event(Event::<T>::FixedPriceSaleClose {
					collection_id: sale.collection_id,
					serial_numbers: sale.serial_numbers.into_inner(),
					listing_id,
					reason: FixedPriceClosureReason::VendorCancelled,
				});
			},
			Listing::<T>::Auction(auction) => {
				ensure!(auction.seller == who, Error::<T>::NotSeller);
				ensure!(Self::listing_winning_bid(listing_id).is_none(), Error::<T>::TokenLocked);
				Listings::<T>::remove(listing_id);
				ListingEndSchedule::<T>::remove(auction.close, listing_id);
				for serial_number in auction.serial_numbers.iter() {
					T::NFTExt::set_token_lock((auction.collection_id, *serial_number), None)?;
				}
				<OpenCollectionListings<T>>::remove(auction.collection_id, listing_id);

				Self::deposit_event(Event::<T>::AuctionClose {
					collection_id: auction.collection_id,
					listing_id,
					reason: AuctionClosureReason::VendorCancelled,
				});
			},
		}
		Ok(())
	}

	pub fn do_make_simple_offer(
		who: T::AccountId,
		token_id: TokenId,
		amount: Balance,
		asset_id: AssetId,
		marketplace_id: Option<MarketplaceId>,
	) -> sp_std::result::Result<OfferId, DispatchError> {
		ensure!(!amount.is_zero(), Error::<T>::ZeroOffer);
		let collection_info = T::NFTExt::get_collection_info(token_id.0)?;
		ensure!(!collection_info.is_token_owner(&who, token_id.1), Error::<T>::IsTokenOwner);
		let offer_id = Self::next_offer_id();
		ensure!(offer_id.checked_add(One::one()).is_some(), Error::<T>::NoAvailableIds);

		// ensure the token_id is not currently in an auction
		if let Some(TokenLockReason::Listed(listing_id)) = T::NFTExt::get_token_lock(token_id) {
			match Listings::<T>::get(listing_id) {
				Some(Listing::<T>::Auction(_)) => return Err(Error::<T>::TokenOnAuction.into()),
				None | Some(Listing::<T>::FixedPrice(_)) => (),
			}
		}

		// try lock funds
		T::MultiCurrency::place_hold(T::PalletId::get(), &who, asset_id, amount)?;
		<TokenOffers<T>>::try_append(token_id, offer_id)
			.map_err(|_| Error::<T>::MaxOffersReached)?;
		let new_offer = OfferType::<T::AccountId>::Simple(SimpleOffer {
			token_id,
			asset_id,
			amount,
			buyer: who.clone(),
			marketplace_id,
		});
		<Offers<T>>::insert(offer_id, new_offer);
		<NextOfferId<T>>::mutate(|i| *i += 1);

		Self::deposit_event(Event::<T>::Offer {
			offer_id,
			amount,
			asset_id,
			marketplace_id,
			buyer: who,
		});
		Ok(offer_id)
	}

	pub fn do_cancel_offer(who: T::AccountId, offer_id: OfferId) -> DispatchResult {
		let Some(OfferType::Simple(offer)) = Self::offers(offer_id) else {
			return Err(Error::<T>::InvalidOffer.into());
		};
		ensure!(offer.buyer == who, Error::<T>::NotBuyer);
		T::MultiCurrency::release_hold(T::PalletId::get(), &who, offer.asset_id, offer.amount)?;
		let _ = Self::remove_offer(offer_id, offer.token_id)?;
		Self::deposit_event(Event::<T>::OfferCancel { offer_id, token_id: offer.token_id });
		Ok(())
	}

	pub fn do_accept_offer(who: T::AccountId, offer_id: OfferId) -> DispatchResult {
		let Some(OfferType::Simple(offer)) = Self::offers(offer_id) else {
			return Err(Error::<T>::InvalidOffer.into());
		};

		let (collection_id, serial_number) = offer.token_id;
		ensure!(
			T::NFTExt::get_token_owner(&(offer.token_id)) == Some(who),
			Error::<T>::NotTokenOwner
		);

		// Check whether token is listed for fixed price sale
		if let Some(TokenLockReason::Listed(listing_id)) = T::NFTExt::get_token_lock(offer.token_id)
		{
			if let Some(listing) = <Listings<T>>::get(listing_id) {
				Self::remove_listing(listing, listing_id);
			}
		}

		let royalties_schedule =
			Self::calculate_bundle_royalties(collection_id, offer.marketplace_id)?;
		let serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerListing> =
			BoundedVec::truncate_from(vec![serial_number]);

		Self::process_payment_and_transfer(
			&offer.buyer,
			&who,
			offer.asset_id,
			collection_id,
			serial_numbers,
			offer.amount,
			royalties_schedule,
		)?;

		let _ = Self::remove_offer(offer_id, offer.token_id)?;
		Self::deposit_event(Event::<T>::OfferAccept {
			offer_id,
			token_id: offer.token_id,
			amount: offer.amount,
			asset_id: offer.asset_id,
		});
		Ok(())
	}

	/// Removes an offer, cleaning storage if it's the last offer for the token
	pub(crate) fn remove_offer(offer_id: OfferId, token_id: TokenId) -> DispatchResult {
		Offers::<T>::remove(offer_id);
		TokenOffers::<T>::try_mutate(token_id, |maybe_offers| -> DispatchResult {
			if let Some(offers) = maybe_offers {
				let pos =
					offers.binary_search(&offer_id).map_err(|_| Error::<T>::NoAvailableIds)?;
				offers.remove(pos);

				if offers.is_empty() {
					*maybe_offers = None;
				}
			}
			Ok(())
		})
	}

	/// Close all listings scheduled to close at this block `now`, ensuring payments and ownerships
	/// changes are made for winning bids Metadata for listings will be removed from storage
	/// Returns the number of listings removed
	pub(crate) fn close_listings_at(now: T::BlockNumber) -> u32 {
		let mut removed = 0_u32;
		for (listing_id, _) in ListingEndSchedule::<T>::drain_prefix(now).into_iter() {
			let Some(listing_outer) = Listings::<T>::get(listing_id) else {
				continue
			};
			match listing_outer.clone() {
				Listing::FixedPrice(listing) => {
					Self::remove_listing(listing_outer, listing_id);
					Self::deposit_event(Event::<T>::FixedPriceSaleClose {
						collection_id: listing.collection_id,
						serial_numbers: listing.serial_numbers.into_inner(),
						listing_id,
						reason: FixedPriceClosureReason::Expired,
					});
					removed += 1;
				},
				Listing::Auction(listing) => {
					Self::remove_listing(listing_outer, listing_id);
					Self::process_auction_closure(listing, listing_id);
					removed += 1;
				},
			}
		}
		removed
	}

	/// Removes a listing and its metadata from storage and releases locks on tokens
	pub(crate) fn remove_listing(listing: Listing<T>, listing_id: ListingId) {
		let (serial_numbers, collection_id) = match listing {
			Listing::FixedPrice(listing) => {
				ListingEndSchedule::<T>::remove(listing.close, listing_id);
				(listing.serial_numbers, listing.collection_id)
			},
			Listing::Auction(listing) => {
				ListingEndSchedule::<T>::remove(listing.close, listing_id);
				(listing.serial_numbers, listing.collection_id)
			},
		};

		OpenCollectionListings::<T>::remove(collection_id, listing_id);
		for serial_number in serial_numbers.iter() {
			let _ = T::NFTExt::set_token_lock((collection_id, *serial_number), None);
		}
		<Listings<T>>::remove(listing_id);
	}

	/// Process an auction once complete. Releasing the hold to the winner
	fn process_auction_closure(listing: AuctionListing<T>, listing_id: ListingId) {
		// Check if there was a winning bid
		let winning_bid = ListingWinningBid::<T>::take(listing_id);
		let Some((winner, hammer_price)) = winning_bid else {
			// normal closure, no acceptable bids
			// listing metadata is removed by now.
			Self::deposit_event(Event::<T>::AuctionClose {
				collection_id: listing.collection_id,
				listing_id,
				reason: AuctionClosureReason::ExpiredNoBids,
			});
			return
		};

		// Process the winning bid
		if let Err(err) = Self::process_payment_and_transfer(
			&winner,
			&listing.seller,
			listing.payment_asset,
			listing.collection_id,
			listing.serial_numbers,
			hammer_price,
			listing.royalties_schedule,
		) {
			// auction settlement failed despite our prior validations.
			// release winning bid funds
			log!(error, "🃏 auction settlement failed: {:?}", err);
			let release_hold = T::MultiCurrency::release_hold(
				T::PalletId::get(),
				&winner,
				listing.payment_asset,
				hammer_price,
			);
			if release_hold.is_err() {
				// This shouldn't happen
				log!(error, "🃏 releasing hold failed");
			}

			// listing metadata is removed by now.
			Self::deposit_event(Event::<T>::AuctionClose {
				collection_id: listing.collection_id,
				listing_id,
				reason: AuctionClosureReason::SettlementFailed,
			});
		} else {
			// auction settlement success
			Self::deposit_event(Event::<T>::AuctionSold {
				collection_id: listing.collection_id,
				listing_id,
				payment_asset: listing.payment_asset,
				hammer_price,
				winner,
			});
		}
	}

	/// Settle an auction listing or accepted offer
	/// (guaranteed to be atomic).
	/// - transfer funds from winning bidder to entitled royalty accounts and seller
	/// - transfer ownership to the winning bidder
	#[transactional]
	pub(crate) fn process_payment_and_transfer(
		buyer: &T::AccountId,
		seller: &T::AccountId,
		asset_id: AssetId,
		collection_id: CollectionUuid,
		serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerListing>,
		amount: Balance,
		royalties_schedule: RoyaltiesSchedule<T::AccountId>,
	) -> DispatchResult {
		let payouts = Self::calculate_royalty_payouts(seller.clone(), royalties_schedule, amount);
		// spend hold and split to royalty accounts
		T::MultiCurrency::spend_hold(T::PalletId::get(), &buyer, asset_id, &payouts)?;

		// Transfer each token
		T::NFTExt::do_transfer(*seller, collection_id, serial_numbers.into_inner(), *buyer)
	}

	/// Locks a group of tokens before listing for sale
	/// Throws an error if owner does not own all tokens
	#[transactional]
	pub(crate) fn lock_tokens_for_listing(
		collection_id: CollectionUuid,
		serial_numbers: &BoundedVec<SerialNumber, T::MaxTokensPerListing>,
		owner: &T::AccountId,
		listing_id: ListingId,
	) -> DispatchResult {
		let collection_info = T::NFTExt::get_collection_info(collection_id)?;

		// Check whether token is locked and that owner owns each token
		for serial_number in serial_numbers.iter() {
			ensure!(
				T::NFTExt::get_token_lock((collection_id, *serial_number)).is_none(),
				Error::<T>::TokenLocked
			);
			ensure!(
				collection_info.is_token_owner(owner, *serial_number),
				Error::<T>::NotTokenOwner
			);
		}

		// Insert locks for tokens
		for serial_number in serial_numbers.iter() {
			T::NFTExt::set_token_lock(
				(collection_id, *serial_number),
				Some(TokenLockReason::Listed(listing_id)),
			)?;
		}
		Ok(())
	}

	/// Calculates payout splits for an amount over seller and royalty schedule
	pub(crate) fn calculate_royalty_payouts(
		seller: T::AccountId,
		royalties_schedule: RoyaltiesSchedule<T::AccountId>,
		amount: Balance,
	) -> Vec<(T::AccountId, Balance)> {
		let mut for_seller = amount;
		let mut payouts: Vec<(T::AccountId, Balance)> = vec![];

		// Calculate royalty split
		if !royalties_schedule.calculate_total_entitlement().is_zero() {
			let entitlements = royalties_schedule.entitlements.clone();
			for (who, entitlement) in entitlements.into_iter() {
				let royalty: Balance = entitlement * amount;
				for_seller -= royalty;
				payouts.push((who, royalty));
			}
		}
		payouts.push((seller, for_seller));
		payouts
	}

	/// Check royalties will be respected on all tokens if placed into a bundle sale.
	/// We're ok iff, all tokens in the bundle are from the:
	/// 1) same collection
	/// Although possible, we do not support:
	/// 3) different collection, no royalties allowed
	pub(crate) fn calculate_bundle_royalties(
		collection_id: CollectionUuid,
		marketplace_id: Option<MarketplaceId>,
	) -> Result<RoyaltiesSchedule<T::AccountId>, DispatchError> {
		// Get collection royalties portion
		let mut royalties: RoyaltiesSchedule<T::AccountId> =
			T::NFTExt::get_collection_info(collection_id)?
				.royalties_schedule
				.unwrap_or_default();

		// Get network fee portion
		if let Some(tx_fee_pot_id) = FeeTo::<T>::get() {
			// We can handle the network fee payout to the tx fee pot as well here
			let network_fee = T::NetworkFeePercentage::get();
			royalties
				.entitlements
				.try_push((tx_fee_pot_id, network_fee))
				.map_err(|_| Error::<T>::RoyaltiesInvalid)?;
		}

		// Get marketplace fee portion
		if let Some(marketplace_id) = marketplace_id {
			if let Some(marketplace) = <RegisteredMarketplaces<T>>::get(marketplace_id) {
				royalties
					.entitlements
					.try_push((marketplace.account, marketplace.entitlement))
					.map_err(|_| Error::<T>::RoyaltiesInvalid)?;
			} else {
				return Err(Error::<T>::MarketplaceNotRegistered.into())
			}
		};

		// Validate all royalties
		if !royalties.entitlements.is_empty() {
			ensure!(royalties.validate(), Error::<T>::RoyaltiesInvalid);
		}

		Ok(royalties)
	}

	pub(crate) fn do_set_fee_to(fee_to: Option<T::AccountId>) -> DispatchResult {
		FeeTo::<T>::put(&fee_to);
		Self::deposit_event(Event::FeeToSet { account: fee_to });
		Ok(())
	}
}
