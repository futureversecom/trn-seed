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

use crate::{traits::NFTExt, *};
use frame_support::{ensure, traits::Get, transactional, weights::Weight};
use frame_system::RawOrigin;
use precompile_utils::constants::ERC721_PRECOMPILE_ADDRESS_PREFIX;
use seed_pallet_common::{
	log, utils::next_asset_uuid, Hold, OnNewAssetSubscriber, OnTransferSubscriber,
};
use seed_primitives::{
	AssetId, Balance, CollectionUuid, MetadataScheme, OriginChain, RoyaltiesSchedule, SerialNumber,
	TokenCount, TokenId,
};
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

	/// Returns number of tokens owned by an account in a collection
	/// Used by the ERC721 precompile for balance_of
	pub fn token_balance_of(who: &T::AccountId, collection_id: CollectionUuid) -> TokenCount {
		match Self::collection_info(collection_id) {
			Some(collection_info) => {
				let serial_numbers: Vec<SerialNumber> = collection_info
					.owned_tokens
					.into_iter()
					.find(|token_ownership| &token_ownership.owner == who)
					.map(|token_ownership| token_ownership.owned_serials.clone().into_inner())
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
		collection_info.metadata_scheme.construct_token_uri(token_id.1)
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
		})?;
		Ok(())
	}

	/// Check royalties will be respected on all tokens if placed into a bundle sale.
	/// We're ok iff, all tokens in the bundle are from the:
	/// 1) same collection
	/// Although possible, we do not support:
	/// 3) different collection, no royalties allowed
	pub(crate) fn calculate_bundle_royalties(
		collection_id: CollectionUuid,
		marketplace_id: Option<MarketplaceId>,
	) -> Result<RoyaltiesSchedule<T::AccountId>, Error<T>> {
		let mut royalties: RoyaltiesSchedule<T::AccountId> = Self::collection_info(collection_id)
			.ok_or(Error::<T>::NoCollectionFound)?
			.royalties_schedule
			.unwrap_or_default();

		let Some(marketplace_id) = marketplace_id else {
			return Ok(royalties)
		};

		ensure!(
			<RegisteredMarketplaces<T>>::contains_key(marketplace_id),
			Error::<T>::MarketplaceNotRegistered
		);
		if let Some(marketplace) = Self::registered_marketplaces(marketplace_id) {
			royalties.entitlements.push((marketplace.account, marketplace.entitlement));
		}
		ensure!(royalties.validate(), Error::<T>::RoyaltiesInvalid);
		Ok(royalties)
	}

	/// Transfer the given token from `current_owner` to `new_owner`
	/// Does no verification
	pub fn do_transfer(
		collection_id: CollectionUuid,
		serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
		current_owner: &T::AccountId,
		new_owner: &T::AccountId,
	) -> DispatchResult {
		CollectionInfo::<T>::try_mutate(collection_id, |maybe_collection_info| -> DispatchResult {
			let collection_info =
				maybe_collection_info.as_mut().ok_or(Error::<T>::NoCollectionFound)?;

			// Check ownership and locks
			for serial_number in serial_numbers.iter() {
				ensure!(
					collection_info.is_token_owner(current_owner, *serial_number),
					Error::<T>::NotTokenOwner
				);
				ensure!(
					!<TokenLocks<T>>::contains_key((collection_id, serial_number)),
					Error::<T>::TokenLocked
				);
			}

			collection_info
				.add_user_tokens(new_owner, serial_numbers.clone())
				.map_err(|e| Error::<T>::from(e))?;
			collection_info.remove_user_tokens(current_owner, serial_numbers.clone());

			for serial_number in serial_numbers.clone().iter() {
				T::OnTransferSubscription::on_nft_transfer(&(collection_id, *serial_number));
			}
			Self::deposit_event(Event::<T>::Transfer {
				previous_owner: current_owner.clone(),
				collection_id,
				serial_numbers: serial_numbers.into_inner(),
				new_owner: new_owner.clone(),
			});
			Ok(())
		})
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
		if serial_numbers.is_empty() {
			return 0 as Weight
		};

		let collection_info = match Self::collection_info(collection_id) {
			Some(info) => info,
			None => return T::DbWeight::get().reads(1),
		};

		// remove duplicates from serial_numbers
		let mut serial_numbers_trimmed = serial_numbers;
		serial_numbers_trimmed.sort_unstable();
		serial_numbers_trimmed.dedup();

		// Trim the new serial_numbers and remove any that have already been minted
		serial_numbers_trimmed = serial_numbers_trimmed
			.into_iter()
			.filter(|serial_number| {
				if collection_info.token_exists(*serial_number) {
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

		let serial_numbers: Result<BoundedVec<SerialNumber, T::MaxTokensPerCollection>, ()> =
			BoundedVec::try_from(serial_numbers_trimmed);
		match serial_numbers {
			Ok(serial_numbers) => {
				let _ = Self::do_mint(collection_id, collection_info, &owner, &serial_numbers);

				// throw event, listing all serial numbers minted from bridging
				// SerialNumbers will never exceed the limit denoted by nft_peg::MaxTokensPerMint
				// Which is set to 50 in the runtime, so this event is safe to list all bridged
				// serial_numbers
				Self::deposit_event(Event::<T>::BridgedMint {
					collection_id,
					serial_numbers: serial_numbers.clone(),
					owner: owner.clone(),
				});

				T::DbWeight::get().reads_writes(1, 1)
			},
			_ => T::DbWeight::get().reads(1),
		}
	}

	/// Perform validity checks on collection_info
	/// Return bounded vec of serial numbers to mint
	pub fn pre_mint(
		who: &T::AccountId,
		quantity: TokenCount,
		collection_info: &CollectionInformation<T::AccountId, T::MaxTokensPerCollection>,
	) -> Result<BoundedVec<SerialNumber, T::MaxTokensPerCollection>, DispatchError> {
		// Quantity must be some
		ensure!(quantity > Zero::zero(), Error::<T>::NoToken);
		// Caller must be collection_owner
		ensure!(collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);
		// Check we don't exceed the token limit
		ensure!(
			collection_info.collection_issuance.saturating_add(quantity) <
				T::MaxTokensPerCollection::get(),
			Error::<T>::TokenLimitExceeded
		);
		// Cannot mint for a token that was bridged from Ethereum
		ensure!(
			collection_info.origin_chain == OriginChain::Root,
			Error::<T>::AttemptedMintOnBridgedToken
		);

		let previous_serial_number = collection_info.next_serial_number;
		let next_serial_number =
			previous_serial_number.checked_add(quantity).ok_or(Error::<T>::NoAvailableIds)?;

		// Check early that we won't exceed the BoundedVec limit
		ensure!(
			next_serial_number <= T::MaxTokensPerCollection::get(),
			Error::<T>::TokenLimitExceeded
		);

		// Can't mint more than specified max_issuance
		if let Some(max_issuance) = collection_info.max_issuance {
			ensure!(max_issuance >= next_serial_number, Error::<T>::MaxIssuanceReached);
		}

		let serial_numbers_unbounded: Vec<SerialNumber> =
			(previous_serial_number..next_serial_number).collect();
		let serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection> =
			BoundedVec::try_from(serial_numbers_unbounded)
				.map_err(|_| Error::<T>::TokenLimitExceeded)?;
		Ok(serial_numbers)
	}

	/// Perform the mint operation and update storage accordingly.
	pub(crate) fn do_mint(
		collection_id: CollectionUuid,
		collection_info: CollectionInformation<T::AccountId, T::MaxTokensPerCollection>,
		token_owner: &T::AccountId,
		serial_numbers: &BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
	) -> DispatchResult {
		let mut new_collection_info = collection_info;
		// Update collection issuance
		new_collection_info.collection_issuance = new_collection_info
			.collection_issuance
			.checked_add(serial_numbers.len().saturated_into())
			.ok_or(Error::<T>::TokenLimitExceeded)?;

		new_collection_info
			.add_user_tokens(&token_owner, serial_numbers.clone())
			.map_err(|e| Error::<T>::from(e))?;

		// Update CollectionInfo storage
		<CollectionInfo<T>>::insert(collection_id, new_collection_info);
		Ok(())
	}

	/// Find the tokens owned by an `address` in the given collection
	/// limit return tokens that are larger than the cursor
	/// Returns list of tokens, the sum of all tokens owned by the user
	/// and the new cursor for the next owned SerialNumber not included in the returned list
	pub fn owned_tokens(
		collection_id: CollectionUuid,
		who: &T::AccountId,
		cursor: SerialNumber,
		limit: u16,
	) -> (SerialNumber, TokenCount, Vec<SerialNumber>) {
		let collection_info = match Self::collection_info(collection_id) {
			Some(info) => info,
			None => return (Default::default(), Default::default(), Default::default()),
		};

		// Collect all tokens owned by address
		let mut owned_tokens: Vec<SerialNumber> = match collection_info
			.owned_tokens
			.into_inner()
			.iter()
			.find(|token_ownership| &token_ownership.owner == who)
		{
			Some(token_ownership) => token_ownership.owned_serials.clone().into_inner(),
			None => vec![],
		};

		// Sort the vec to ensure no tokens are missed
		owned_tokens.sort();
		// Store the last owned token by this account
		let last_id: SerialNumber = owned_tokens.last().copied().unwrap_or_default();
		// Get the sum of all tokens owned by this account
		let total_owned: TokenCount = owned_tokens.len().saturated_into();

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

		(new_cursor, total_owned, response)
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
			TokenLocks::<T>::remove((collection_id, *serial_number));
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
		serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
		amount: Balance,
		royalties_schedule: RoyaltiesSchedule<T::AccountId>,
	) -> DispatchResult {
		let payouts = Self::calculate_royalty_payouts(seller.clone(), royalties_schedule, amount);
		// spend hold and split to royalty accounts
		T::MultiCurrency::spend_hold(T::PalletId::get(), &buyer, asset_id, &payouts)?;

		// Transfer each token
		Self::do_transfer(collection_id, serial_numbers, seller, buyer)
	}

	/// Locks a group of tokens before listing for sale
	/// Throws an error if owner does not own all tokens
	#[transactional]
	pub(crate) fn lock_tokens_for_listing(
		collection_id: CollectionUuid,
		serial_numbers: &BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
		owner: &T::AccountId,
		listing_id: ListingId,
	) -> DispatchResult {
		let collection_info =
			Self::collection_info(collection_id).ok_or(Error::<T>::NoCollectionFound)?;

		// Check whether token is locked and that owner owns each token
		for serial_number in serial_numbers.iter() {
			ensure!(
				!<TokenLocks<T>>::contains_key((collection_id, serial_number)),
				Error::<T>::TokenLocked
			);
			ensure!(
				collection_info.is_token_owner(owner, *serial_number),
				Error::<T>::NotTokenOwner
			);
		}

		// Insert locks for tokens
		for serial_number in serial_numbers.iter() {
			<TokenLocks<T>>::insert(
				(collection_id, serial_number),
				TokenLockReason::Listed(listing_id),
			);
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

	/// Create the collection
	pub fn do_create_collection(
		owner: T::AccountId,
		name: BoundedVec<u8, T::StringLimit>,
		initial_issuance: TokenCount,
		max_issuance: Option<TokenCount>,
		token_owner: Option<T::AccountId>,
		metadata_scheme: MetadataScheme,
		royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
		origin_chain: OriginChain,
		cross_chain_compatibility: CrossChainCompatibility,
	) -> Result<u32, DispatchError> {
		// Check we can issue the new tokens
		let collection_uuid = Self::next_collection_uuid()?;

		// Check max issuance is valid
		if let Some(max_issuance) = max_issuance {
			ensure!(max_issuance > Zero::zero(), Error::<T>::InvalidMaxIssuance);
			ensure!(initial_issuance <= max_issuance, Error::<T>::InvalidMaxIssuance);
			ensure!(
				max_issuance <= T::MaxTokensPerCollection::get(),
				Error::<T>::InvalidMaxIssuance
			);
		}

		// Validate collection attributes
		ensure!(!name.is_empty(), Error::<T>::CollectionNameInvalid);
		ensure!(core::str::from_utf8(&name).is_ok(), Error::<T>::CollectionNameInvalid);
		if let Some(royalties_schedule) = royalties_schedule.clone() {
			ensure!(royalties_schedule.validate(), Error::<T>::RoyaltiesInvalid);
		}

		// Now mint the collection tokens
		let mut owned_tokens = BoundedVec::default();
		if initial_issuance > Zero::zero() {
			ensure!(initial_issuance <= T::MintLimit::get(), Error::<T>::MintLimitExceeded);
			// XLS-20 compatible collections cannot have an initial issuance
			// This is to prevent the fees from being bypassed in the mint function.
			// Instead the user should specify 0 initial_issuance and use the mint function to
			// mint tokens
			ensure!(!cross_chain_compatibility.xrpl, Error::<T>::InitialIssuanceNotZero);
			// mint initial tokens to token_owner or owner
			let token_owner = token_owner.unwrap_or(owner.clone());
			let serial_numbers_unbounded: Vec<SerialNumber> = (0..initial_issuance).collect();
			let serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection> =
				BoundedVec::try_from(serial_numbers_unbounded)
					.map_err(|_| Error::<T>::TokenLimitExceeded)?;
			// Create token_ownership object with token_owner and initial serial_numbers
			let token_ownership = TokenOwnership::new(token_owner, serial_numbers);
			owned_tokens = BoundedVec::try_from(vec![token_ownership])
				.map_err(|_| Error::<T>::TokenLimitExceeded)?;
		}

		let collection_info = CollectionInformation {
			owner: owner.clone(),
			name: name.clone(),
			metadata_scheme: metadata_scheme.clone(),
			royalties_schedule: royalties_schedule.clone(),
			max_issuance,
			origin_chain: origin_chain.clone(),
			next_serial_number: initial_issuance,
			collection_issuance: initial_issuance,
			cross_chain_compatibility,
			owned_tokens,
		};
		<CollectionInfo<T>>::insert(collection_uuid, collection_info);

		// will not overflow, asserted prior qed.
		<NextCollectionId<T>>::mutate(|i| *i += u32::one());

		// Add some code to the EVM
		T::OnNewAssetSubscription::on_asset_create(
			collection_uuid,
			ERC721_PRECOMPILE_ADDRESS_PREFIX,
		);

		Self::deposit_event(Event::<T>::CollectionCreate {
			collection_uuid,
			initial_issuance,
			max_issuance,
			collection_owner: owner,
			metadata_scheme,
			name: name.into_inner(),
			royalties_schedule,
			origin_chain,
			compatibility: cross_chain_compatibility,
		});
		Ok(collection_uuid)
	}

	/// Check token locks and burn tokens
	pub fn do_burn(
		who: &T::AccountId,
		collection_id: CollectionUuid,
		serial_number: SerialNumber,
	) -> DispatchResult {
		ensure!(
			!<TokenLocks<T>>::contains_key((collection_id, serial_number)),
			Error::<T>::TokenLocked
		);

		CollectionInfo::<T>::try_mutate(collection_id, |maybe_collection_info| -> DispatchResult {
			let collection_info =
				maybe_collection_info.as_mut().ok_or(Error::<T>::NoCollectionFound)?;

			ensure!(collection_info.is_token_owner(who, serial_number), Error::<T>::NotTokenOwner);
			collection_info.collection_issuance =
				collection_info.collection_issuance.saturating_sub(1);
			collection_info.owned_tokens.iter_mut().for_each(|token_ownership| {
				if token_ownership.owner == *who {
					token_ownership.owned_serials.retain(|&serial| serial != serial_number)
				}
			});
			// Remove approvals for this token
			T::OnTransferSubscription::on_nft_transfer(&(collection_id, serial_number));
			Ok(())
		})
	}

	/// Enables XLS-20 compatibility for a collection with 0 issuance
	pub fn enable_xls20_compatibility(
		who: T::AccountId,
		collection_id: CollectionUuid,
	) -> DispatchResult {
		let mut collection_info =
			CollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;

		// Caller must be collection owner
		ensure!(collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);
		// Collection issuance must be 0 (i.e. no tokens minted)
		ensure!(
			collection_info.collection_issuance.is_zero(),
			Error::<T>::CollectionIssuanceNotZero
		);

		collection_info.cross_chain_compatibility.xrpl = true;
		CollectionInfo::<T>::insert(collection_id, collection_info);
		Ok(())
	}

	/// The account ID of the auctions pot.
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}
}

impl<T: Config> NFTExt for Pallet<T> {
	type AccountId = T::AccountId;
	type MaxTokensPerCollection = T::MaxTokensPerCollection;

	fn do_mint(
		origin: Self::AccountId,
		collection_id: CollectionUuid,
		quantity: TokenCount,
		token_owner: Option<Self::AccountId>,
	) -> DispatchResult {
		Self::mint(RawOrigin::Signed(origin).into(), collection_id, quantity, token_owner)
	}

	fn do_create_collection(
		owner: Self::AccountId,
		name: BoundedVec<u8, <Self::T as Config>::StringLimit>,
		initial_issuance: TokenCount,
		max_issuance: Option<TokenCount>,
		token_owner: Option<Self::AccountId>,
		metadata_scheme: MetadataScheme,
		royalties_schedule: Option<RoyaltiesSchedule<Self::AccountId>>,
		origin_chain: OriginChain,
	) -> Result<CollectionUuid, DispatchError> {
		Self::do_create_collection(
			owner,
			name,
			initial_issuance,
			max_issuance,
			token_owner,
			metadata_scheme,
			royalties_schedule,
			origin_chain,
			CrossChainCompatibility::default(),
		)
	}

	fn get_token_owner(token_id: &TokenId) -> Option<Self::AccountId> {
		let Some(collection) = CollectionInfo::<T>::get(token_id.0) else {
			return None
		};
		collection.get_token_owner(token_id.1)
	}

	fn get_collection_info(
		collection_id: CollectionUuid,
	) -> Result<CollectionInformation<Self::AccountId, Self::MaxTokensPerCollection>, DispatchError>
	{
		CollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound.into())
	}

	fn enable_xls20_compatibility(
		who: Self::AccountId,
		collection_id: CollectionUuid,
	) -> DispatchResult {
		Self::enable_xls20_compatibility(who, collection_id)
	}

	fn next_collection_uuid() -> Result<CollectionUuid, DispatchError> {
		Self::next_collection_uuid()
	}

	fn increment_collection_id() -> DispatchResult {
		ensure!(<NextCollectionId<T>>::get().checked_add(1).is_some(), Error::<T>::NoAvailableIds);
		<NextCollectionId<T>>::mutate(|i| *i += u32::one());
		Ok(())
	}
}
