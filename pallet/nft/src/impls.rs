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
use frame_support::{ensure, traits::Get, transactional};
use seed_pallet_common::{log, utils::next_asset_uuid, Hold, IsTokenOwner, OnTransferSubscriber};
use seed_primitives::{AssetId, Balance, CollectionUuid, SerialNumber, TokenId};
use sp_runtime::{traits::Zero, DispatchError, DispatchResult};
use sp_std::collections::btree_map::BTreeMap;

use codec::alloc::string::ToString;

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
	pub fn token_balance_of(account_id: T::AccountId, collection_id: CollectionUuid) -> TokenCount {
		match Self::token_balance(account_id) {
			Some(balance_map) => *(balance_map.get(&collection_id).unwrap_or(&0)),
			None => TokenCount::zero(),
		}
	}

	pub fn do_create_collection(
		owner: T::AccountId,
		name: CollectionNameType,
		initial_issuance: TokenCount,
		max_issuance: Option<TokenCount>,
		token_owner: Option<T::AccountId>,
		metadata_scheme: MetadataScheme,
		royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
		source_chain: OriginChain,
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

		<CollectionInfo<T>>::insert(
			collection_uuid,
			CollectionInformation {
				owner: owner.clone(),
				name,
				metadata_scheme,
				royalties_schedule: None,
				max_issuance: None,
				source_chain,
			},
		);

		// Now mint the collection tokens
		let token_owner = token_owner.unwrap_or(owner);
		if initial_issuance > Zero::zero() {
			let token_ids: Vec<SerialNumber> = (0..initial_issuance).collect();
			Self::do_mint(&token_owner, collection_uuid, token_ids, source_chain)?;
		}
		// will not overflow, asserted prior qed.
		<NextCollectionId<T>>::mutate(|i| *i += u32::one());

		Self::deposit_event(Event::<T>::CollectionCreate {
			collection_uuid,
			token_count: initial_issuance,
			owner: token_owner,
		});
		Ok(collection_uuid)
	}

	pub fn do_burn(
		who: &T::AccountId,
		collection_id: CollectionUuid,
		serial_number: &SerialNumber,
	) -> DispatchResult {
		ensure!(
			!<TokenLocks<T>>::contains_key((collection_id, serial_number)),
			Error::<T>::TokenLocked
		);
		ensure!(
			Self::token_owner(collection_id, serial_number) == Some(who.clone()),
			Error::<T>::NoPermission
		);
		<TokenOwner<T>>::remove(collection_id, serial_number);

		let _ = <TokenBalance<T>>::try_mutate::<_, (), Error<T>, _>(who, |mut balances| {
			match &mut balances {
				Some(balances) => {
					match (balances).get_mut(&collection_id) {
						Some(balance) => {
							let new_balance = balance.saturating_sub(1);
							if new_balance.is_zero() {
								balances.remove(&collection_id);
							} else {
								*balance = new_balance;
							}
							Ok(())
						},
						None => return Err(Error::NoToken.into()), // should not happen
					}
				},
				None => return Err(Error::NoToken.into()), // should not happen
			}
		})?;

		if let Some(collection_issuance) = Self::collection_issuance(collection_id) {
			if collection_issuance.saturating_sub(1).is_zero() {
				// this is the last of the tokens
				<CollectionInfo<T>>::remove(collection_id);
				<CollectionIssuance<T>>::remove(collection_id);
			} else {
				<CollectionIssuance<T>>::mutate(collection_id, |mut q| {
					if let Some(q) = &mut q {
						*q = q.saturating_sub(1)
					}
				});
			}
		}

		Ok(())
	}

	/// Construct & return the full metadata URI for a given `token_id` (analogous to ERC721
	/// metadata token_uri)
	pub fn token_uri(token_id: TokenId) -> Vec<u8> {
		use core::fmt::Write;
		if let Some(collection_info) = Self::collection_info(token_id.0) {
			let scheme = collection_info.metadata_scheme;
			let mut token_uri = sp_std::Writer::default();
			match scheme {
				MetadataScheme::Http(path) => {
					let path = core::str::from_utf8(&path).unwrap_or("");
					write!(&mut token_uri, "http://{}/{}.json", path, token_id.1)
						.expect("Not written");
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
					write!(
						&mut token_uri,
						"ethereum://{}/{}",
						contract_address.to_string(),
						token_id.1
					)
					.expect("Not written");
				},
			}
			token_uri.inner().clone()
		} else {
			// should not happen
			log!(warn, "🃏 Unexpected empty metadata scheme: {:?}", token_id);
			return Default::default()
		}
	}

	/// Check royalties will be respected on all tokens if placed into a bundle sale.
	/// We're ok iff, all tokens in the bundle are from the:
	/// 1) same collection
	/// Although possible, we do not support:
	/// 3) different collection, no royalties allowed
	pub fn check_bundle_royalties(
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
	pub fn do_transfer_unchecked(
		token_id: TokenId,
		current_owner: &T::AccountId,
		new_owner: &T::AccountId,
	) -> DispatchResult {
		let (collection_id, serial_number) = token_id;

		<TokenOwner<T>>::insert(collection_id, serial_number, new_owner);

		let quantity = 1 as TokenCount;
		let _ =
			<TokenBalance<T>>::try_mutate::<_, (), Error<T>, _>(&current_owner, |mut balances| {
				match &mut balances {
					Some(balances) => {
						match (balances).get_mut(&collection_id) {
							Some(balance) => {
								let new_balance = balance.saturating_sub(quantity);
								if new_balance.is_zero() {
									balances.remove(&collection_id);
								} else {
									*balance = new_balance;
								}
								Ok(())
							},
							None => return Err(Error::NoToken.into()), // should not happen
						}
					},
					None => return Err(Error::NoToken.into()), // should not happen
				}
			});
		<TokenBalance<T>>::mutate(&new_owner, |mut balances| {
			if let Some(balances) = &mut balances {
				*balances.entry(collection_id).or_default() += quantity
			} else {
				let mut map = BTreeMap::new();
				map.insert(collection_id, quantity);
				*balances = Some(map)
			}
		});

		T::OnTransferSubscription::on_nft_transfer(&token_id);
		Ok(())
	}

	/// Mint additional tokens in a collection
	/// Token Ids are passed in manually
	pub fn do_mint(
		owner: &T::AccountId,
		collection_id: CollectionUuid,
		token_ids: Vec<SerialNumber>,
		source_chain: OriginChain,
	) -> DispatchResult {
		// counter for tokens minted in the case a token mint fails
		let mut tokens_minted: TokenCount = 0;

		// Mint the set tokens
		for serial_number in token_ids.iter() {
			if <TokenOwner<T>>::contains_key(collection_id, serial_number) {
				// This should not happen as serial numbers are handled internally
				log!(
					warn,
					"🃏 Token Couldn't be minted as a token already exists: ({:?},{:?})",
					collection_id,
					serial_number
				);
			} else {
				<TokenOwner<T>>::insert(collection_id, serial_number, &owner);
				tokens_minted += 1;
			}
		}

		// update token balances
		<TokenBalance<T>>::mutate(&owner, |mut balances| {
			if let Some(balances) = &mut balances {
				*balances.entry(collection_id).or_default() += tokens_minted
			} else {
				let mut map = BTreeMap::new();
				map.insert(collection_id, tokens_minted);
				*balances = Some(map)
			}
		});
		// Update collection issuance
		<CollectionIssuance<T>>::mutate(collection_id, |mut q| {
			if let Some(q) = &mut q {
				*q = q.saturating_add(tokens_minted)
			} else {
				*q = Some(tokens_minted)
			}
		});

		if source_chain == OriginChain::Root {
			// Only need to keep track of next serial number if minting incrementally on Root
			<NextSerialNumber<T>>::mutate(collection_id, |mut q| {
				if let Some(q) = &mut q {
					*q = q.saturating_add(tokens_minted)
				} else {
					*q = Some(tokens_minted)
				}
			});
		}
		Ok(())
	}

	/// Find the tokens owned by an `address` in the given collection
	pub fn collected_tokens(collection_id: CollectionUuid, address: &T::AccountId) -> Vec<TokenId> {
		let mut owned_tokens = Vec::<TokenId>::default();

		let mut owned_in_collection: Vec<TokenId> =
			<TokenOwner<T>>::iter_prefix(collection_id)
				.filter_map(|(serial_number, owner)| {
					if &owner == address {
						Some((collection_id, serial_number))
					} else {
						None
					}
				})
				.collect();

		if !owned_in_collection.is_empty() {
			owned_in_collection.sort_unstable();
			owned_tokens.append(&mut owned_in_collection);
		}

		return owned_tokens
	}

	/// Remove a single fixed price listing and all it's metadata
	pub fn remove_fixed_price_listing(listing_id: ListingId) {
		let listing_type = Listings::<T>::take(listing_id);
		ListingWinningBid::<T>::remove(listing_id);
		if let Some(Listing::<T>::FixedPrice(listing)) = listing_type {
			ListingEndSchedule::<T>::remove(listing.close, listing_id);
		}
	}

	/// Close all listings scheduled to close at this block `now`, ensuring payments and ownerships
	/// changes are made for winning bids Metadata for listings will be removed from storage
	/// Returns the number of listings removed
	pub fn close_listings_at(now: T::BlockNumber) -> u32 {
		let mut removed = 0_u32;
		for (listing_id, _) in ListingEndSchedule::<T>::drain_prefix(now).into_iter() {
			match Listings::<T>::take(listing_id) {
				Some(Listing::FixedPrice(listing)) => {
					// release listed tokens
					for token_id in listing.tokens.iter() {
						TokenLocks::<T>::remove(token_id);
					}
					let listing_collection_id: CollectionUuid = listing.tokens[0].0;
					OpenCollectionListings::<T>::remove(listing_collection_id, listing_id);

					Self::deposit_event(Event::<T>::FixedPriceSaleClose {
						tokens: listing.tokens,
						listing_id,
						reason: FixedPriceClosureReason::Expired,
					});
				},
				Some(Listing::Auction(listing)) => {
					// release listed tokens
					for token_id in listing.tokens.iter() {
						TokenLocks::<T>::remove(token_id);
					}
					let listing_collection_id: CollectionUuid = listing.tokens[0].0;
					OpenCollectionListings::<T>::remove(listing_collection_id, listing_id);

					if let Some((winner, hammer_price)) = ListingWinningBid::<T>::take(listing_id) {
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
					} else {
						// normal closure, no acceptable bids
						// listing metadata is removed by now.
						Self::deposit_event(Event::<T>::AuctionClose {
							collection_id: listing_collection_id,
							listing_id,
							reason: AuctionClosureReason::ExpiredNoBids,
						});
					}
				},
				None => (),
			}
			removed += 1;
		}

		removed
	}

	/// Settle an auction listing or accepted offer
	/// (guaranteed to be atomic).
	/// - transfer funds from winning bidder to entitled royalty accounts and seller
	/// - transfer ownership to the winning bidder
	#[transactional]
	pub fn process_payment_and_transfer(
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
			let _ = Self::do_transfer_unchecked(token_id, seller, buyer)?;
		}
		Ok(())
	}

	// Calculates payout splits for an amount over seller and royalty schedule
	pub fn calculate_royalty_payouts(
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

	/// Find the royalties and owner of a token
	pub fn token_info(
		collection_id: CollectionUuid,
		serial_number: SerialNumber,
	) -> Option<TokenInfo<T::AccountId>> {
		let collection_info = Self::collection_info(collection_id);
		if let Some(collection_info) = collection_info {
			if let Some(owner) = Self::token_owner(collection_id, serial_number) {
				let royalties = match collection_info.royalties_schedule {
					Some(r) => r.entitlements,
					None => Vec::new(),
				};

				return Some(TokenInfo { owner, royalties })
			}
		}
		None
	}

	/// Get list of all NFT listings within a range
	pub fn collection_listings(
		collection_id: CollectionUuid,
		cursor: u128,
		limit: u16,
	) -> (Option<u128>, Vec<(ListingId, Listing<T>)>) {
		let mut listing_ids = OpenCollectionListings::<T>::iter_prefix(collection_id)
			.map(|(listing_id, _)| listing_id)
			.collect::<Vec<u128>>();
		listing_ids.sort();
		let last_id = listing_ids.last().copied();
		let mut highest_cursor: u128 = 0;

		let response: Vec<(ListingId, Listing<T>)> = listing_ids
			.into_iter()
			.filter(|listing_id| listing_id >= &cursor)
			.take(sp_std::cmp::min(limit, MAX_COLLECTION_LISTING_LIMIT).into())
			.map(|listing_id| {
				highest_cursor = listing_id;
				match Self::listings(listing_id) {
					Some(listing) => Some((listing_id, listing)),
					None => {
						log!(error, "🃏 Unexpected empty listing: {:?}", listing_id);
						None
					},
				}
			})
			.flatten()
			.collect();

		let new_cursor = match last_id {
			Some(id) =>
				if highest_cursor != id {
					Some(highest_cursor + 1)
				} else {
					None
				},
			None => None,
		};
		(new_cursor, response)
	}
}

// Interface for determining ownership of an NFT from some account
impl<T: Config> IsTokenOwner for Pallet<T> {
	type AccountId = T::AccountId;

	fn is_owner(account: &Self::AccountId, token_id: &TokenId) -> bool {
		if let Some(owner) = Self::token_owner(token_id.0, token_id.1) {
			&owner == account
		} else {
			false
		}
	}
}
