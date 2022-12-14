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

use crate::*;
use core::fmt::Write;
use frame_support::{ensure, traits::Get, transactional, weights::Weight};
use precompile_utils::constants::ERC721_PRECOMPILE_ADDRESS_PREFIX;
use seed_pallet_common::{
	log, utils::next_asset_uuid, GetTokenOwner, Hold, OnNewAssetSubscriber, OnTransferSubscriber,
};
use seed_primitives::{AssetId, Balance, CollectionUuid, SerialNumber, TokenId};
use sp_runtime::{traits::Zero, BoundedVec, DispatchError, DispatchResult, SaturatedConversion};

impl<T: Config> Pallet<T> {
	/// Returns the CollectionUuid unique across parachains
	pub fn next_collection_uuid() -> Result<CollectionUuid, DispatchError> {
		let collection_id = <NextCollectionId<T>>::get();
		match next_asset_uuid(collection_id, T::ParachainId::get().into()) {
			Some(next_collection_uuid) => Ok(next_collection_uuid),
			None => Err(Error::<T>::NoAvailableIds.into()),
		}
	}

	/// Return whether the collection exists or not
	pub fn collection_exists(collection_id: CollectionUuid) -> bool {
		<CollectionInfo<T>>::contains_key(collection_id)
	}

	/// Check whether a token has been minted in a collection
	pub fn token_exists(
		serial_number: SerialNumber,
		collection_info: &CollectionInformation<T>,
	) -> bool {
		collection_info
			.owned_tokens
			.iter()
			.any(|(_, tokens)| tokens.clone().into_inner().contains(&serial_number))
	}

	/// Check whether who owns the serial number in collection_info
	pub fn is_token_owner(
		who: &T::AccountId,
		collection_info: &CollectionInformation<T>,
		serial_number: SerialNumber,
	) -> bool {
		collection_info.owned_tokens.iter().any(|(account, tokens)| {
			if account == who {
				tokens.clone().into_inner().contains(&serial_number)
			} else {
				false
			}
		})
	}

	/// Returns number of tokens owned by an account in a collection
	pub fn token_balance_of(who: &T::AccountId, collection_id: CollectionUuid) -> TokenCount {
		match Self::collection_info(collection_id) {
			Some(collection_info) => {
				let serial_numbers: Vec<SerialNumber> = collection_info
					.owned_tokens
					.into_iter()
					.find(|(account, _)| account == who)
					.map(|(_, serial_numbers)| serial_numbers.clone().into_inner())
					.unwrap_or_default();
				serial_numbers.len() as TokenCount
			},
			None => TokenCount::zero(),
		}
	}

	/// Construct & return the full metadata URI for a given `token_id` (analogous to ERC721
	/// metadata token_uri)
	pub fn token_uri(token_id: TokenId) -> Vec<u8> {
		let collection_info = Self::collection_info(token_id.0);
		if collection_info.is_none() {
			// should not happen
			log!(warn, "🃏 Unexpected empty metadata scheme: {:?}", token_id);
			return Default::default()
		}

		let collection_info = collection_info.unwrap();
		let scheme = collection_info.metadata_scheme;
		let mut token_uri = sp_std::Writer::default();
		match scheme {
			MetadataScheme::Http(path) => {
				let path = core::str::from_utf8(&path).unwrap_or("");
				write!(&mut token_uri, "http://{}/{}.json", path, token_id.1).expect("Not written");
			},
			MetadataScheme::Https(path) => {
				let path = core::str::from_utf8(&path).unwrap_or("");
				write!(&mut token_uri, "https://{}/{}.json", path, token_id.1)
					.expect("Not written");
			},
			MetadataScheme::IpfsDir(dir_cid) => {
				write!(
					&mut token_uri,
					"ipfs://{}/{}.json",
					core::str::from_utf8(&dir_cid).unwrap_or(""),
					token_id.1
				)
				.expect("Not written");
			},
			MetadataScheme::IpfsShared(shared_cid) => {
				write!(
					&mut token_uri,
					"ipfs://{}.json",
					core::str::from_utf8(&shared_cid).unwrap_or("")
				)
				.expect("Not written");
			},
			MetadataScheme::Ethereum(contract_address) => {
				write!(&mut token_uri, "ethereum://{:?}/{}", contract_address, token_id.1)
					.expect("Not written");
			},
		}
		token_uri.inner().clone()
	}

	/// Removes an offer, cleaning storage if it's the last offer for the token
	pub(crate) fn remove_offer(offer_id: OfferId, token_id: TokenId) {
		Offers::<T>::remove(offer_id);
		if let Some(offers) = Self::token_offers(token_id) {
			if offers.len() == 1 {
				// this is the last of the token offers
				<TokenOffers<T>>::remove(token_id);
			} else {
				<TokenOffers<T>>::mutate(token_id, |mut offers| {
					if let Some(offers) = &mut offers {
						offers.binary_search(&offer_id).map(|idx| offers.remove(idx)).unwrap();
					}
				});
			}
		};
	}

	/// Check royalties will be respected on all tokens if placed into a bundle sale.
	/// We're ok iff, all tokens in the bundle are from the:
	/// 1) same collection
	/// Although possible, we do not support:
	/// 3) different collection, no royalties allowed
	pub(crate) fn check_bundle_royalties(
		tokens: &[TokenId],
		marketplace_id: Option<MarketplaceId>,
	) -> Result<RoyaltiesSchedule<T::AccountId>, Error<T>> {
		// use the first token's collection as representative of the bundle
		let (bundle_collection_id, _serial_number) = tokens[0];

		for (collection_id, _serial_number) in tokens.iter() {
			ensure!(*collection_id == bundle_collection_id, Error::<T>::MixedBundleSale);
		}

		let collection_info = Self::collection_info(bundle_collection_id);
		ensure!(collection_info.is_some(), Error::<T>::NoCollection);
		let collection_royalties = collection_info.unwrap().royalties_schedule;

		let mut royalties: RoyaltiesSchedule<T::AccountId> =
			collection_royalties.unwrap_or_else(|| RoyaltiesSchedule { entitlements: vec![] });

		let royalties = match marketplace_id {
			Some(marketplace_id) => {
				ensure!(
					<RegisteredMarketplaces<T>>::contains_key(marketplace_id),
					Error::<T>::MarketplaceNotRegistered
				);
				if let Some(marketplace) = Self::registered_marketplaces(marketplace_id) {
					royalties.entitlements.push((marketplace.account, marketplace.entitlement));
				}
				ensure!(royalties.validate(), Error::<T>::RoyaltiesInvalid);
				royalties
			},
			None => royalties,
		};
		Ok(royalties)
	}

	/// Transfer the given token from `current_owner` to `new_owner`
	/// Does no verification
	pub(crate) fn do_transfer_unchecked(
		token_id: TokenId,
		collection_info: CollectionInformation<T>,
		current_owner: &T::AccountId,
		new_owner: &T::AccountId,
	) -> DispatchResult {
		let (collection_id, serial_number) = token_id;

		let mut new_collection_info: CollectionInformation<T> = collection_info;

		Self::remove_user_tokens(current_owner, &mut new_collection_info, vec![serial_number])?;
		Self::add_user_tokens(new_owner, &mut new_collection_info, vec![serial_number])?;

		// Update CollectionInfo storage
		<CollectionInfo<T>>::insert(collection_id, new_collection_info);

		T::OnTransferSubscription::on_nft_transfer(&token_id);
		Ok(())
	}

	/// Mint additional tokens in a collection
	/// This is called by the nft-peg pallet and mints tokens based on the token ids bridged
	/// An extra check is made to ensure tokens have not already been minted, if this happens
	/// execution won't fail, however those tokens will not be minted twice.
	pub fn mint_bridged_token(
		owner: &T::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: Vec<SerialNumber>,
	) -> Weight {
		if serial_numbers.len() == usize::zero() {
			return 0 as Weight
		};

		let collection_info = match Self::collection_info(collection_id) {
			Some(info) => info,
			None => return 0,
		};

		// remove duplicates from serial_numbers
		let mut serial_numbers_trimmed = serial_numbers;
		serial_numbers_trimmed.sort_unstable();
		serial_numbers_trimmed.dedup();

		// Trim the new serial_numbers and remove any that have already been minted
		serial_numbers_trimmed = serial_numbers_trimmed
			.into_iter()
			.filter(|serial_number| {
				if Self::token_exists(*serial_number, &collection_info) {
					// Since we don't want to error, throw a warning instead.
					// If we error, then some tokens may be lost
					log!(
						warn,
						"🃏 Token Couldn't be minted as this token_id already exists: ({:?},{:?})",
						collection_id,
						serial_number
					);
					false
				} else {
					true
				}
			})
			.collect::<Vec<SerialNumber>>();

		let _ =
			Self::do_mint_unchecked(collection_id, collection_info, owner, serial_numbers_trimmed);

		T::DbWeight::get().reads_writes(1, 1)
	}

	/// Perform the mint operation and update storage accordingly.
	pub(crate) fn do_mint_unchecked(
		collection_id: CollectionUuid,
		collection_info: CollectionInformation<T>,
		token_owner: &T::AccountId,
		serial_numbers: Vec<SerialNumber>,
	) -> DispatchResult {
		let mut new_collection_info = collection_info;
		// Update collection issuance
		new_collection_info.collection_issuance = new_collection_info
			.collection_issuance
			.saturating_add(serial_numbers.len().saturated_into());

		Self::add_user_tokens(token_owner, &mut new_collection_info, serial_numbers.clone())?;

		// Update CollectionInfo storage
		<CollectionInfo<T>>::insert(collection_id, new_collection_info);

		// Throw event, listing all tokens minted
		Self::deposit_event(Event::<T>::Mint {
			collection_id,
			serial_numbers,
			owner: token_owner.clone(),
		});
		Ok(())
	}

	/// Adds a list of tokens to a users balance in collection_info
	pub(crate) fn add_user_tokens(
		token_owner: &T::AccountId,
		collection_info: &mut CollectionInformation<T>,
		serial_numbers: Vec<SerialNumber>,
	) -> DispatchResult {
		if collection_info.owned_tokens.iter().any(|(owner, _)| owner == token_owner) {
			for (owner, owned_serial_numbers) in collection_info.owned_tokens.iter_mut() {
				if owner != token_owner {
					continue
				}
				// Add new serial numbers to existing owner
				for serial_number in serial_numbers.clone() {
					owned_serial_numbers
						.try_push(serial_number)
						.map_err(|_| Error::<T>::TokenLimitExceeded)?;
				}
			}
		} else {
			// If token owner doesn't exist, create new entry
			collection_info
				.owned_tokens
				.try_push((
					token_owner.clone(),
					BoundedVec::try_from(serial_numbers.clone())
						.map_err(|_| Error::<T>::TokenLimitExceeded)?,
				))
				.map_err(|_| Error::<T>::TokenLimitExceeded)?;
		}
		Ok(())
	}

	/// Removes a list of tokens from a users balance in collection_info
	pub(crate) fn remove_user_tokens(
		token_owner: &T::AccountId,
		collection_info: &mut CollectionInformation<T>,
		serial_numbers: Vec<SerialNumber>,
	) -> DispatchResult {
		let mut removing_all_tokens: bool = false;
		for (owner, owned_serial_numbers) in collection_info.owned_tokens.iter_mut() {
			if owner != token_owner {
				continue
			}
			owned_serial_numbers.retain(|serial| !serial_numbers.contains(serial));
			removing_all_tokens = owned_serial_numbers.is_empty();
		}
		// Check whether the owner has any tokens left, if not remove them from the collection
		if removing_all_tokens {
			collection_info.owned_tokens.retain(|(owner, _)| owner != token_owner);
		}

		Ok(())
	}

	/// Find the tokens owned by an `address` in the given collection
	/// limit return tokens that are larger than the cursor
	/// Returns list of tokens and the new cursor for the next owned SerialNumber
	/// not included in the returned list
	pub fn owned_tokens(
		collection_id: CollectionUuid,
		who: &T::AccountId,
		cursor: SerialNumber,
		limit: u16,
	) -> (SerialNumber, Vec<SerialNumber>) {
		let collection_info = match Self::collection_info(collection_id) {
			Some(info) => info,
			None => return (Default::default(), Default::default()),
		};

		// Collect all tokens owned by address
		let mut owned_tokens: Vec<SerialNumber> = match collection_info
			.owned_tokens
			.into_inner()
			.iter()
			.find(|(owner, _)| owner == who)
		{
			Some((_, serial_numbers)) => serial_numbers.clone().into_inner(),
			None => vec![],
		};

		// Sort the vec to ensure no tokens are missed
		owned_tokens.sort();
		// Store the last owned token by this account
		let last_id: SerialNumber = owned_tokens.last().copied().unwrap_or_default();

		// Shorten list to any tokens above the cursor and return the limit
		// Note max limit is restricted by MAX_OWNED_TOKENS_LIMIT const
		let response: Vec<SerialNumber> = owned_tokens
			.into_iter()
			.filter(|serial_number| serial_number >= &cursor)
			.take(sp_std::cmp::min(limit, MAX_OWNED_TOKENS_LIMIT).into())
			.collect();

		let new_cursor: SerialNumber = match response.last().copied() {
			Some(highest) => {
				if highest != last_id {
					// There are still tokens remaining that aren't being returned in this call,
					// return the next cursor
					highest.saturating_add(1)
				} else {
					// 0 indicates that this is the end of the owned tokens
					0
				}
			},
			None => 0,
		};

		(new_cursor, response)
	}

	/// Remove a single fixed price listing and all it's metadata
	pub(crate) fn remove_fixed_price_listing(listing_id: ListingId) {
		let listing_type = Listings::<T>::take(listing_id);
		ListingWinningBid::<T>::remove(listing_id);
		if let Some(Listing::<T>::FixedPrice(listing)) = listing_type {
			ListingEndSchedule::<T>::remove(listing.close, listing_id);
		}
	}

	/// Close all listings scheduled to close at this block `now`, ensuring payments and ownerships
	/// changes are made for winning bids Metadata for listings will be removed from storage
	/// Returns the number of listings removed
	pub(crate) fn close_listings_at(now: T::BlockNumber) -> u32 {
		let mut removed = 0_u32;
		for (listing_id, _) in ListingEndSchedule::<T>::drain_prefix(now).into_iter() {
			match Listings::<T>::take(listing_id) {
				Some(Listing::FixedPrice(listing)) => {
					Self::remove_listing(listing_id, &listing.tokens);
					Self::deposit_event(Event::<T>::FixedPriceSaleClose {
						tokens: listing.tokens,
						listing_id,
						reason: FixedPriceClosureReason::Expired,
					});
					removed += 1;
				},
				Some(Listing::Auction(listing)) => {
					Self::remove_listing(listing_id, &listing.tokens);
					Self::process_auction_closure(listing, listing_id);
					removed += 1;
				},
				None => (),
			}
		}
		removed
	}

	/// Removes a listing from storage and releases locks on tokens
	fn remove_listing(listing_id: ListingId, tokens: &Vec<TokenId>) {
		for token_id in tokens.iter() {
			TokenLocks::<T>::remove(token_id);
		}
		let listing_collection_id: CollectionUuid = tokens[0].0;
		OpenCollectionListings::<T>::remove(listing_collection_id, listing_id);
	}

	/// Process an auction once complete. Releasing the hold to the winner
	fn process_auction_closure(listing: AuctionListing<T>, listing_id: ListingId) {
		let listing_collection_id: CollectionUuid = listing.tokens[0].0;

		// Check if there was a winning bid
		let winning_bid = ListingWinningBid::<T>::take(listing_id);
		if winning_bid.is_none() {
			// normal closure, no acceptable bids
			// listing metadata is removed by now.
			Self::deposit_event(Event::<T>::AuctionClose {
				collection_id: listing_collection_id,
				listing_id,
				reason: AuctionClosureReason::ExpiredNoBids,
			});
			return
		}
		let (winner, hammer_price) = winning_bid.unwrap();

		// Process the winning bid
		if let Err(err) = Self::process_payment_and_transfer(
			&winner,
			&listing.seller,
			listing.payment_asset,
			listing.tokens,
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
				collection_id: listing_collection_id,
				listing_id,
				reason: AuctionClosureReason::SettlementFailed,
			});
		} else {
			// auction settlement success
			Self::deposit_event(Event::<T>::AuctionSold {
				collection_id: listing_collection_id,
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
		token_ids: Vec<TokenId>,
		amount: Balance,
		royalties_schedule: RoyaltiesSchedule<T::AccountId>,
	) -> DispatchResult {
		let payouts = Self::calculate_royalty_payouts(seller.clone(), royalties_schedule, amount);
		// spend hold and split to royalty accounts
		T::MultiCurrency::spend_hold(T::PalletId::get(), &buyer, asset_id, &payouts)?;

		// Transfer each token
		for token_id in token_ids {
			let collection_info = match Self::collection_info(token_id.0) {
				Some(info) => info,
				None => return Err(Error::<T>::NoCollection.into()),
			};
			let _ = Self::do_transfer_unchecked(token_id, collection_info, seller, buyer)?;
		}
		Ok(())
	}

	/// Locks a group of tokens before listing for sale
	/// Throws an error if owner does not own all tokens
	pub(crate) fn lock_tokens_for_listing(
		tokens: &Vec<TokenId>,
		owner: &T::AccountId,
		listing_id: ListingId,
	) -> DispatchResult {
		for (collection_id, serial_number) in tokens.iter() {
			ensure!(
				!<TokenLocks<T>>::contains_key((collection_id, serial_number)),
				Error::<T>::TokenLocked
			);
			let collection_info = match Self::collection_info(collection_id) {
				Some(info) => info,
				None => return Err(Error::<T>::NoCollection.into()),
			};
			ensure!(
				Self::is_token_owner(owner, &collection_info, *serial_number),
				Error::<T>::NoPermission
			);
			<TokenLocks<T>>::insert(
				(collection_id, serial_number),
				TokenLockReason::Listed(listing_id),
			);
		}
		Ok(())
	}

	// Calculates payout splits for an amount over seller and royalty schedule
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

	pub fn do_create_collection(
		owner: T::AccountId,
		name: CollectionNameType,
		initial_issuance: TokenCount,
		max_issuance: Option<TokenCount>,
		token_owner: Option<T::AccountId>,
		metadata_scheme: MetadataScheme,
		royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
		origin_chain: OriginChain,
	) -> Result<u32, DispatchError> {
		// Check we can issue the new tokens
		let collection_uuid = Self::next_collection_uuid()?;

		// Check max issuance is valid
		if let Some(max_issuance) = max_issuance {
			ensure!(max_issuance > Zero::zero(), Error::<T>::InvalidMaxIssuance);
			ensure!(initial_issuance <= max_issuance, Error::<T>::InvalidMaxIssuance);
		}

		// Validate collection attributes
		ensure!(
			!name.is_empty() && name.len() <= MAX_COLLECTION_NAME_LENGTH as usize,
			Error::<T>::CollectionNameInvalid
		);
		ensure!(core::str::from_utf8(&name).is_ok(), Error::<T>::CollectionNameInvalid);
		let metadata_scheme =
			metadata_scheme.sanitize().map_err(|_| Error::<T>::InvalidMetadataPath)?;
		if let Some(royalties_schedule) = royalties_schedule.clone() {
			ensure!(royalties_schedule.validate(), Error::<T>::RoyaltiesInvalid);
		}

		let collection_info = CollectionInformation {
			owner: owner.clone(),
			name: name.clone(),
			metadata_scheme: metadata_scheme.clone(),
			royalties_schedule: royalties_schedule.clone(),
			max_issuance,
			origin_chain: origin_chain.clone(),
			next_serial_number: initial_issuance,
			collection_issuance: 0,
			owned_tokens: Default::default(),
		};

		// Now mint the collection tokens
		if initial_issuance > Zero::zero() {
			let token_owner = token_owner.unwrap_or(owner.clone());
			let serial_numbers: Vec<SerialNumber> = (0..initial_issuance).collect();
			// CollectionInfo gets inserted inside this mint function
			Self::do_mint_unchecked(
				collection_uuid,
				collection_info,
				&token_owner,
				serial_numbers,
			)?;
		} else {
			// initial_issuance is 0 so we don't need to mint. However we need to still add
			// collectionInfo to storage
			<CollectionInfo<T>>::insert(collection_uuid, collection_info);
		}

		// will not overflow, asserted prior qed.
		<NextCollectionId<T>>::mutate(|i| *i += u32::one());

		// Add some code to the EVM
		T::OnNewAssetSubscription::on_asset_create(
			collection_uuid,
			ERC721_PRECOMPILE_ADDRESS_PREFIX,
		);

		Self::deposit_event(Event::<T>::CollectionCreate {
			collection_uuid,
			max_issuance,
			collection_owner: owner,
			metadata_scheme,
			name,
			royalties_schedule,
			origin_chain,
		});
		Ok(collection_uuid)
	}

	pub fn do_burn(
		who: &T::AccountId,
		collection_id: CollectionUuid,
		serial_number: SerialNumber,
	) -> DispatchResult {
		ensure!(
			!<TokenLocks<T>>::contains_key((collection_id, serial_number)),
			Error::<T>::TokenLocked
		);

		let mut collection_info = match Self::collection_info(collection_id) {
			Some(info) => info,
			None => return Err(Error::<T>::NoCollection.into()),
		};

		ensure!(
			Self::is_token_owner(who, &collection_info, serial_number),
			Error::<T>::NoPermission
		);

		collection_info.collection_issuance = collection_info.collection_issuance.saturating_sub(1);
		collection_info.owned_tokens.iter_mut().for_each(|(owner, serial_numbers)| {
			if owner == who {
				serial_numbers.retain(|&serial| serial != serial_number)
			}
		});

		// Remove approvals for this token
		T::OnTransferSubscription::on_nft_transfer(&(collection_id, serial_number));

		// Update storage with new info
		<CollectionInfo<T>>::insert(collection_id, collection_info);

		Ok(())
	}

	/// The account ID of the auctions pot.
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}
}

// Interface for getting ownership of an NFT
impl<T: Config> GetTokenOwner for Pallet<T> {
	type AccountId = T::AccountId;

	fn get_owner(token_id: &TokenId) -> Option<Self::AccountId> {
		let collection_info = match Self::collection_info(token_id.0) {
			Some(info) => info,
			None => return None,
		};
		match collection_info
			.owned_tokens
			.into_iter()
			.find(|(_, serial_numbers)| serial_numbers.clone().into_inner().contains(&token_id.1))
		{
			Some((owner, _)) => Some(owner),
			None => None,
		}
	}
}
