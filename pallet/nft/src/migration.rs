#[allow(dead_code)]
pub mod v1_storage {
	use crate::{
		CollectionNameType, Config, ListingId, MarketplaceId, MetadataScheme, OfferId, OriginChain,
		RoyaltiesSchedule, SerialNumber, TokenCount,
	};
	use codec::{Decode, Encode};
	use scale_info::TypeInfo;
	use seed_primitives::{AssetId, Balance, CollectionUuid, TokenId};
	use sp_std::{collections::btree_map::BTreeMap, prelude::*};

	/// information about a collection v1
	#[derive(Debug, Clone, Encode, Decode, PartialEq, TypeInfo)]
	pub struct CollectionInformation<AccountId> {
		// The owner of the collection
		pub owner: AccountId,
		// A human friendly name
		pub name: CollectionNameType,
		// Collection metadata reference scheme
		pub metadata_scheme: MetadataScheme,
		// configured royalties schedule
		pub royalties_schedule: Option<RoyaltiesSchedule<AccountId>>,
		// Maximum number of tokens allowed in a collection
		pub max_issuance: Option<TokenCount>,
		// The chain in which the collection was created initially
		pub origin_chain: OriginChain,
	}

	/// A type of NFT sale listing v1
	#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub enum Listing<T: Config> {
		FixedPrice(FixedPriceListing<T>),
		Auction(AuctionListing<T>),
	}

	/// Information about an auction listing v1
	#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct AuctionListing<T: Config> {
		pub payment_asset: AssetId,
		pub reserve_price: Balance,
		pub close: T::BlockNumber,
		pub seller: T::AccountId,
		pub tokens: Vec<TokenId>,
		pub royalties_schedule: RoyaltiesSchedule<T::AccountId>,
		pub marketplace_id: Option<MarketplaceId>,
	}

	/// Information about a fixed price listing v1
	#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct FixedPriceListing<T: Config> {
		pub payment_asset: AssetId,
		pub fixed_price: Balance,
		pub close: T::BlockNumber,
		pub buyer: Option<T::AccountId>,
		pub seller: T::AccountId,
		pub tokens: Vec<TokenId>,
		pub royalties_schedule: RoyaltiesSchedule<T::AccountId>,
		pub marketplace_id: Option<MarketplaceId>,
	}

	pub struct Module<T>(sp_std::marker::PhantomData<T>);
	frame_support::decl_storage! {
		trait Store for Module<T: Config> as Nft {
			pub CollectionInfo get(fn collection_info): map hasher(twox_64_concat) CollectionUuid => Option<CollectionInformation<T::AccountId>>;
			pub CollectionIssuance get(fn collection_issuance): map hasher(twox_64_concat) CollectionUuid => TokenCount;
			pub NextSerialNumber get(fn next_serial_number): map hasher(twox_64_concat) CollectionUuid => SerialNumber;
			pub TokenBalance get(fn token_balance): map hasher(blake2_128_concat) T::AccountId => BTreeMap<CollectionUuid, TokenCount>;
			pub TokenOffers get(fn token_offers): map hasher(twox_64_concat) TokenId => Vec<OfferId>;
			pub TokenOwner get(fn token_owner): double_map hasher(twox_64_concat) CollectionUuid, hasher(twox_64_concat) SerialNumber => Option<T::AccountId>;
			pub Listings get(fn listings): map hasher(twox_64_concat) ListingId => Option<Listing<T>>;
		}
	}
}

use super::*;
use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};
use seed_pallet_common::log;
use sp_runtime::BoundedVec;

/// A struct that migrates all bags lists to contain a score value.
pub struct MigrateToV1<T: Config>(sp_std::marker::PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
	fn on_runtime_upgrade() -> Weight {
		use super::*;
		use frame_support::{
			traits::StorageVersion, weights::constants::RocksDbWeight as DbWeight,
			IterableStorageDoubleMap, IterableStorageMap, StorageMap,
		};

		if StorageVersion::get::<Pallet<T>>() == 0 {
			StorageVersion::new(1).put::<Pallet<T>>();

			let mut weight = 0;

			// Migrate Collection Info
			let old_collection_info: Vec<(
				CollectionUuid,
				v1_storage::CollectionInformation<T::AccountId>,
			)> = v1_storage::CollectionInfo::<T>::iter().collect();

			for (collection_id, info) in old_collection_info.clone() {
				let next_serial_number = v1_storage::NextSerialNumber::get(collection_id);
				let collection_issuance = v1_storage::CollectionIssuance::get(collection_id);
				let mut collection_info_migrated = crate::CollectionInformation {
					owner: info.owner,
					name: info.name,
					metadata_scheme: info.metadata_scheme,
					royalties_schedule: info.royalties_schedule,
					max_issuance: info.max_issuance,
					origin_chain: info.origin_chain,
					next_serial_number,
					collection_issuance,
					owned_tokens: Default::default(),
				};

				// Add tokens for each user
				for (serial_number, token_owner) in
					v1_storage::TokenOwner::<T>::iter_prefix(collection_id)
				{
					if collection_info_migrated
						.add_user_tokens(&token_owner, vec![serial_number])
						.is_err()
					{
						// There was an error migrating tokens, caused by token limit being
						// reached
						log!(warn, "🃏 Error migrating tokens, collection_id: {:?}, serial_number: {:?}, token_owner: {:?}", collection_id, serial_number, token_owner);
					}
				}
				<crate::CollectionInfo<T>>::insert(collection_id, collection_info_migrated);
			}
			log!(warn, "🃏 NFT collection info migrated");
			weight += 6_000_000 as Weight +
				DbWeight::get().reads_writes(
					old_collection_info.len() as Weight + 1,
					old_collection_info.len() as Weight + 1,
				);

			// Migrate TokenOffers
			let old_token_offers: Vec<(TokenId, Vec<OfferId>)> =
				v1_storage::TokenOffers::iter().collect();
			for (token_id, offer_ids) in old_token_offers.clone() {
				let new_offer_ids: BoundedVec<OfferId, T::MaxOffers> =
					match BoundedVec::try_from(offer_ids) {
						Ok(offer_ids) => offer_ids,
						Err(_) => {
							log!(warn, "🃏 Error migrating token offers, token_id: {:?}", token_id);
							continue
						},
					};
				<crate::TokenOffers<T>>::insert(token_id, new_offer_ids);
			}
			weight += DbWeight::get().reads_writes(
				old_token_offers.len() as Weight + 1,
				old_token_offers.len() as Weight + 1,
			);

			// Migrate Listings
			let old_listings: Vec<(ListingId, v1_storage::Listing<T>)> =
				v1_storage::Listings::iter().collect();

			for (listing_id, listing) in old_listings.clone() {
				match listing {
					v1_storage::Listing::Auction(auction) => {
						if auction.tokens.is_empty() {
							// This shouldn't happen but we need to be sure
							log!(
								warn,
								"🃏 Error migrating auction due to empty tokens. listing_id: {:?}",
								listing_id
							);
							continue
						}
						let collection_id = auction.tokens[0].0;
						let old_serial_numbers: Vec<SerialNumber> = auction
							.tokens
							.into_iter()
							.map(|(_, serial_number)| serial_number)
							.collect();
						let serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerListing> =
							match BoundedVec::try_from(old_serial_numbers) {
								Ok(serial_numbers) => serial_numbers,
								Err(_) => {
									log!(warn, "🃏 Error migrating auction due to too many offers, listing_id: {:?}", listing_id);
									continue
								},
							};
						let new_auction = crate::AuctionListing {
							payment_asset: auction.payment_asset,
							reserve_price: auction.reserve_price,
							close: auction.close,
							seller: auction.seller,
							collection_id,
							serial_numbers,
							royalties_schedule: auction.royalties_schedule,
							marketplace_id: auction.marketplace_id,
						};
						<crate::Listings<T>>::insert(
							listing_id,
							crate::Listing::Auction(new_auction),
						);
					},
					v1_storage::Listing::FixedPrice(sale) => {
						if sale.tokens.is_empty() {
							// This shouldn't happen but we need to be sure
							log!(warn, "🃏 Error migrating fixed price sale due to empty tokens. listing_id: {:?}", listing_id);
							continue
						}
						let collection_id = sale.tokens[0].0;
						let old_serial_numbers: Vec<SerialNumber> = sale
							.tokens
							.into_iter()
							.map(|(_, serial_number)| serial_number)
							.collect();
						let serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerListing> =
							match BoundedVec::try_from(old_serial_numbers) {
								Ok(serial_numbers) => serial_numbers,
								Err(_) => {
									log!(warn, "🃏 Error migrating fixed price sale due to too many offers, listing_id: {:?}", listing_id);
									continue
								},
							};
						let new_sale = crate::FixedPriceListing {
							payment_asset: sale.payment_asset,
							close: sale.close,
							buyer: sale.buyer,
							seller: sale.seller,
							collection_id,
							serial_numbers,
							royalties_schedule: sale.royalties_schedule,
							marketplace_id: sale.marketplace_id,
							fixed_price: sale.fixed_price,
						};
						<crate::Listings<T>>::insert(
							listing_id,
							crate::Listing::FixedPrice(new_sale),
						);
					},
				}
			}

			weight += DbWeight::get()
				.reads_writes(old_listings.len() as Weight + 1, old_listings.len() as Weight + 1);

			weight
		} else {
			Zero::zero()
		}
	}
}
