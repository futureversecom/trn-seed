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
use frame_support::{
	traits::{GetStorageVersion, PalletInfoAccess, StorageVersion},
	weights::{constants::RocksDbWeight as DbWeight, Weight},
	IterableStorageDoubleMap, IterableStorageMap, StorageMap,
};
use seed_pallet_common::log;
use sp_runtime::BoundedVec;

/// migrate NFT storage to V1
/// Changes the following storage maps:
///  - CollectionInfo
///  - TokenOffers
///  - Listings
/// Removes the following, adding them to CollectionInformation struct:
///  - CollectionIssuance
///  - NextSerialNumber
///  - TokenBalance
///  - TokenOwner
///  - StorageVersion (Old version)
///
/// Also removes custom StorageVersion and replaces it with the FrameV2 way of tracking version
pub fn try_migrate<T: Config>() -> Weight {
	let current = Pallet::<T>::current_storage_version();
	let onchain = Pallet::<T>::on_chain_storage_version();
	log::info!("Running migration with current storage version {current:?} / onchain {onchain:?}");

	if onchain == 0 {
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
				let serial_numbers: Vec<SerialNumber> = vec![serial_number];
				let serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection> =
					BoundedVec::try_from(serial_numbers).expect("Should not fail");
				if collection_info_migrated.add_user_tokens(&token_owner, serial_numbers).is_err() {
					// There was an error migrating tokens, caused by token limit being
					// reached
					log!(warn, "🃏 Error migrating tokens, collection_id: {:?}, serial_number: {:?}, token_owner: {:?}", collection_id, serial_number, token_owner);
				}
			}
			<crate::CollectionInfo<T>>::insert(collection_id, collection_info_migrated);
		}
		log!(warn, "🃏 NFT collection info migrated");
		weight += DbWeight::get().reads_writes(
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
					let serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection> =
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
					<crate::Listings<T>>::insert(listing_id, crate::Listing::Auction(new_auction));
				},
				v1_storage::Listing::FixedPrice(sale) => {
					if sale.tokens.is_empty() {
						// This shouldn't happen but we need to be sure
						log!(warn, "🃏 Error migrating fixed price sale due to empty tokens. listing_id: {:?}", listing_id);
						continue
					}
					let collection_id = sale.tokens[0].0;
					let old_serial_numbers: Vec<SerialNumber> =
						sale.tokens.into_iter().map(|(_, serial_number)| serial_number).collect();
					let serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection> =
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
					<crate::Listings<T>>::insert(listing_id, crate::Listing::FixedPrice(new_sale));
				},
			}
		}

		clear_storage_prefixes::<T>();
		weight += DbWeight::get()
			.reads_writes(old_listings.len() as Weight + 1, old_listings.len() as Weight + 1);

		weight
	} else {
		Zero::zero()
	}
}

fn clear_storage_prefixes<T: Config>() {
	let res = frame_support::migration::clear_storage_prefix(
		<Pallet<T>>::name().as_bytes(),
		b"NextSerialNumber",
		b"",
		None,
		None,
	);

	if res.maybe_cursor.is_some() {
		log::error!("NextSerialNumber storage item removal was not completed");
	} else {
		log::info!("NextSerialNumber storage item successfully removed")
	};

	let res = frame_support::migration::clear_storage_prefix(
		<Pallet<T>>::name().as_bytes(),
		b"CollectionIssuance",
		b"",
		None,
		None,
	);

	if res.maybe_cursor.is_some() {
		log::error!("CollectionIssuance storage item removal was not completed");
	} else {
		log::info!("CollectionIssuance storage item successfully removed")
	};

	let res = frame_support::migration::clear_storage_prefix(
		<Pallet<T>>::name().as_bytes(),
		b"TokenBalance",
		b"",
		None,
		None,
	);

	if res.maybe_cursor.is_some() {
		log::error!("TokenBalance storage item removal was not completed");
	} else {
		log::info!("TokenBalance storage item successfully removed")
	};

	let res = frame_support::migration::clear_storage_prefix(
		<Pallet<T>>::name().as_bytes(),
		b"TokenOwner",
		b"",
		None,
		None,
	);

	if res.maybe_cursor.is_some() {
		log::error!("TokenOwner storage item removal was not completed");
	} else {
		log::info!("TokenOwner storage item successfully removed")
	};

	let res = frame_support::migration::clear_storage_prefix(
		<Pallet<T>>::name().as_bytes(),
		b"StorageVersion",
		b"",
		None,
		None,
	);

	if res.maybe_cursor.is_some() {
		log::error!("StorageVersion storage item removal was not completed");
	} else {
		log::info!("StorageVersion storage item successfully removed")
	};
}

#[cfg(test)]
mod migration_tests {
	use super::*;
	use crate::{
		mock::{AccountId, Test, TestExt},
		tests::create_owned_tokens,
	};
	use frame_support::{
		migration::{have_storage_value, put_storage_value},
		traits::{OnRuntimeUpgrade, StorageVersion},
		StorageDoubleMap, StorageMap,
	};
	use migration::v1_storage;
	use sp_std::collections::btree_map::BTreeMap;

	#[test]
	fn migration_collection_info_v0_to_v1() {
		TestExt::default().build().execute_with(|| {
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), 0);

			// Mock some collections with fake user data
			let user_1 = 5_u64;
			let user_2 = 6_u64;
			let user_3 = 7_u64;
			let mut user_1_balance = BTreeMap::<CollectionUuid, TokenCount>::new();
			let mut user_2_balance = BTreeMap::<CollectionUuid, TokenCount>::new();
			let mut user_3_balance = BTreeMap::<CollectionUuid, TokenCount>::new();

			// Collection 1
			let collection_id_1 = 123;
			v1_storage::CollectionInfo::<Test>::insert(
				collection_id_1,
				v1_storage::CollectionInformation::<AccountId> {
					owner: 123_u64,
					name: b"test-collection-1".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_2, Permill::one())],
					}),
					metadata_scheme: MetadataScheme::Ipfs(b"Test1".to_vec()),
					max_issuance: None,
					origin_chain: OriginChain::Root,
				},
			);
			v1_storage::CollectionIssuance::insert(collection_id_1, 5);
			v1_storage::NextSerialNumber::insert(collection_id_1, 5);
			// Setup collection 1 balances
			v1_storage::TokenOwner::<Test>::insert(collection_id_1, 0, user_1);
			v1_storage::TokenOwner::<Test>::insert(collection_id_1, 1, user_1);
			v1_storage::TokenOwner::<Test>::insert(collection_id_1, 2, user_1);
			user_1_balance.insert(collection_id_1, 3);
			v1_storage::TokenOwner::<Test>::insert(collection_id_1, 3, user_2);
			v1_storage::TokenOwner::<Test>::insert(collection_id_1, 4, user_2);
			user_2_balance.insert(collection_id_1, 2);

			// Collection 2
			let collection_id_2 = 124;
			v1_storage::CollectionInfo::<Test>::insert(
				collection_id_2,
				v1_storage::CollectionInformation::<AccountId> {
					owner: 124_u64,
					name: b"test-collection-2".to_vec(),
					royalties_schedule: None,
					metadata_scheme: MetadataScheme::Ipfs(b"Test2".to_vec()),
					max_issuance: Some(1000),
					origin_chain: OriginChain::Ethereum,
				},
			);
			v1_storage::CollectionIssuance::insert(collection_id_2, 4);
			v1_storage::NextSerialNumber::insert(collection_id_2, 4);
			// Setup collection 2 balances
			v1_storage::TokenOwner::<Test>::insert(collection_id_2, 69, user_1);
			v1_storage::TokenOwner::<Test>::insert(collection_id_2, 123, user_1);
			user_1_balance.insert(collection_id_2, 2);
			v1_storage::TokenOwner::<Test>::insert(collection_id_2, 420, user_2);
			user_2_balance.insert(collection_id_2, 1);
			v1_storage::TokenOwner::<Test>::insert(collection_id_2, 1337, user_3);
			user_3_balance.insert(collection_id_2, 1);

			// Update token balances for both users
			v1_storage::TokenBalance::<Test>::insert(user_1, user_1_balance);
			v1_storage::TokenBalance::<Test>::insert(user_2, user_2_balance);
			v1_storage::TokenBalance::<Test>::insert(user_3, user_3_balance);

			// Run upgrade
			try_migrate::<Test>();

			// Version should be updated
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), 1);

			// Collection 1 should be correctly migrated
			let owned_tokens =
				create_owned_tokens(vec![(user_1, vec![0, 1, 2]), (user_2, vec![3, 4])]);
			assert_eq!(
				CollectionInfo::<Test>::get(collection_id_1).unwrap(),
				CollectionInformation::<Test> {
					owner: 123_u64,
					name: b"test-collection-1".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_2, Permill::one())],
					}),
					metadata_scheme: MetadataScheme::Ipfs(b"Test1".to_vec()),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 5,
					collection_issuance: 5,
					owned_tokens
				}
			);

			// Collection 2 should be correctly migrated
			let owned_tokens = create_owned_tokens(vec![
				(user_1, vec![69, 123]),
				(user_2, vec![420]),
				(user_3, vec![1337]),
			]);
			assert_eq!(
				CollectionInfo::<Test>::get(collection_id_2).unwrap(),
				CollectionInformation::<Test> {
					owner: 124_u64,
					name: b"test-collection-2".to_vec(),
					royalties_schedule: None,
					metadata_scheme: MetadataScheme::Ipfs(b"Test2".to_vec()),
					max_issuance: Some(1000),
					origin_chain: OriginChain::Ethereum,
					next_serial_number: 4,
					collection_issuance: 4,
					owned_tokens
				}
			);
		});
	}

	#[test]
	fn migration_token_offers_v0_to_v1() {
		TestExt::default().build().execute_with(|| {
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), 0);

			// Mock some fake token_id -> offer_id mappings
			let original_mappings = vec![
				((0, 0), vec![1]),
				((0, 1), vec![6, 7]),
				((1, 1), vec![80, 90, 0]),
				((1, 2), vec![16, 17, 18, 19]),
				((2, 1), vec![21, 22, 23, 24, 25]),
				((2, 2), vec![26, 27, 28, 29, 30, 31]),
				((100, 0), vec![100, 0, 123, 4, 111111, 123456, 123456789]),
				// Len above T::MaxOffers should be ignored
				((2, 3), vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]),
			];
			for (token_id, offer_ids) in original_mappings.iter() {
				v1_storage::TokenOffers::insert(token_id, offer_ids);
			}

			// Run upgrade
			try_migrate::<Test>();

			// Version should be updated
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), 1);

			// Check storage is now bounded but unchanged
			for (token_id, offer_ids) in original_mappings.iter() {
				if offer_ids.len() > mock::MaxOffers::get() as usize {
					// Too high for bounds, offers removed
					assert_eq!(crate::TokenOffers::<Test>::get(token_id), None);
					continue
				}
				// Normal offers migrated
				let expected_offer_ids: BoundedVec<OfferId, <Test as Config>::MaxOffers> =
					BoundedVec::try_from(offer_ids.clone()).unwrap();
				assert_eq!(crate::TokenOffers::<Test>::get(token_id), Some(expected_offer_ids));
			}
		});
	}

	#[test]
	fn migration_listings_v0_to_v1() {
		TestExt::default().build().execute_with(|| {
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), 0);

			// Mock some fake listings
			let listing_id_1 = 1;
			let listing_1 =
				v1_storage::Listing::<Test>::FixedPrice(v1_storage::FixedPriceListing::<Test> {
					payment_asset: 12,
					fixed_price: 1_001,
					close: 12345,
					buyer: Some(5),
					seller: 666,
					tokens: vec![(0, 1), (0, 2), (0, 3)],
					royalties_schedule: RoyaltiesSchedule {
						entitlements: vec![
							(1, Permill::from_percent(20)),
							(2, Permill::from_percent(30)),
						],
					},
					marketplace_id: Some(5),
				});
			v1_storage::Listings::insert(listing_id_1, listing_1);

			let listing_id_2 = 10;
			let listing_2 =
				v1_storage::Listing::<Test>::Auction(v1_storage::AuctionListing::<Test> {
					payment_asset: 555,
					close: 0,
					seller: 1,
					tokens: vec![(1, 1), (1, 10), (1, 100), (1, 1000)],
					royalties_schedule: RoyaltiesSchedule {
						entitlements: vec![(10, Permill::from_percent(80))],
					},
					marketplace_id: None,
					reserve_price: 12345,
				});
			v1_storage::Listings::insert(listing_id_2, listing_2);

			// Run upgrade
			try_migrate::<Test>();

			// Version should be updated
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), 1);

			// Check storage is now migrated
			let expected_serials_1: BoundedVec<
				SerialNumber,
				<Test as Config>::MaxTokensPerCollection,
			> = BoundedVec::try_from(vec![1, 2, 3]).unwrap();
			let listing_1_expected =
				crate::Listing::<Test>::FixedPrice(crate::FixedPriceListing::<Test> {
					payment_asset: 12,
					fixed_price: 1_001,
					close: 12345,
					buyer: Some(5),
					seller: 666,
					collection_id: 0,
					serial_numbers: expected_serials_1,
					royalties_schedule: RoyaltiesSchedule {
						entitlements: vec![
							(1, Permill::from_percent(20)),
							(2, Permill::from_percent(30)),
						],
					},
					marketplace_id: Some(5),
				});
			assert_eq!(crate::Listings::<Test>::get(listing_id_1), Some(listing_1_expected));

			let expected_serials_2: BoundedVec<
				SerialNumber,
				<Test as Config>::MaxTokensPerCollection,
			> = BoundedVec::try_from(vec![1, 10, 100, 1000]).unwrap();
			let listing_2_expected =
				crate::Listing::<Test>::Auction(crate::AuctionListing::<Test> {
					payment_asset: 555,
					close: 0,
					seller: 1,
					collection_id: 1,
					serial_numbers: expected_serials_2,
					royalties_schedule: RoyaltiesSchedule {
						entitlements: vec![(10, Permill::from_percent(80))],
					},
					marketplace_id: None,
					reserve_price: 12345,
				});
			assert_eq!(crate::Listings::<Test>::get(listing_id_2), Some(listing_2_expected));
		});
	}

	#[test]
	fn migration_clears_storage_prefix() {
		TestExt::default().build().execute_with(|| {
			let test_storage_key = b"";

			// Check initial state is empty
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"NextSerialNumber",
					test_storage_key
				),
				false
			);
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"TokenBalance",
					test_storage_key
				),
				false
			);
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"TokenOwner",
					test_storage_key
				),
				false
			);
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"CollectionIssuance",
					test_storage_key
				),
				false
			);
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"StorageVersion",
					test_storage_key
				),
				false
			);

			// Put some storage values
			put_storage_value(
				<Pallet<Test>>::name().as_bytes(),
				b"NextSerialNumber",
				test_storage_key,
				123,
			);
			put_storage_value(
				<Pallet<Test>>::name().as_bytes(),
				b"TokenBalance",
				test_storage_key,
				123,
			);
			put_storage_value(
				<Pallet<Test>>::name().as_bytes(),
				b"TokenOwner",
				test_storage_key,
				123,
			);
			put_storage_value(
				<Pallet<Test>>::name().as_bytes(),
				b"CollectionIssuance",
				test_storage_key,
				123,
			);
			put_storage_value(
				<Pallet<Test>>::name().as_bytes(),
				b"StorageVersion",
				test_storage_key,
				123,
			);

			// Check state is now some
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"NextSerialNumber",
					test_storage_key
				),
				true
			);
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"TokenBalance",
					test_storage_key
				),
				true
			);
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"TokenOwner",
					test_storage_key
				),
				true
			);
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"CollectionIssuance",
					test_storage_key
				),
				true
			);
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"StorageVersion",
					test_storage_key
				),
				true
			);

			// Run runtime upgrade
			try_migrate::<Test>();

			// Check state is now empty
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"NextSerialNumber",
					test_storage_key
				),
				false
			);
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"TokenBalance",
					test_storage_key
				),
				false
			);
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"TokenOwner",
					test_storage_key
				),
				false
			);
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"CollectionIssuance",
					test_storage_key
				),
				false
			);
			assert_eq!(
				have_storage_value(
					<Pallet<Test>>::name().as_bytes(),
					b"StorageVersion",
					test_storage_key
				),
				false
			);
		});
	}
}

#[allow(dead_code)]
pub mod v2 {
	use super::*;
	use crate::{Config, Pallet};
	use codec::{Decode, Encode};
	use frame_support::{storage_alias, weights::Weight, Twox64Concat};
	use scale_info::TypeInfo;
	use sp_core::H160;

	/// Denotes the metadata URI referencing scheme used by a collection
	/// Enable token metadata URI construction by clients
	#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
	pub enum OldMetadataScheme {
		/// Collection metadata is hosted by an HTTPS server
		/// Inner value is the URI without protocol prefix 'https://' or trailing '/'
		/// full metadata URI construction: `https://<domain>/<path+>/<serial_number>.json`
		/// Https(b"example.com/metadata")
		Https(Vec<u8>),
		/// Collection metadata is hosted by an unsecured HTTP server
		/// Inner value is the URI without protocol prefix 'http://' or trailing '/'
		/// full metadata URI construction: `https://<domain>/<path+>/<serial_number>.json`
		/// Https(b"example.com/metadata")
		Http(Vec<u8>),
		/// Collection metadata is hosted by an IPFS directory
		/// Inner value is the directory's IPFS CID
		/// full metadata URI construction: `ipfs://<directory_CID>/<serial_number>.json`
		/// IpfsDir(b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi")
		IpfsDir(Vec<u8>),
		/// Collection metadata is hosted by an IPFS directory
		/// Inner value is the shared IPFS CID, each token in the collection shares the same CID
		/// full metadata URI construction: `ipfs://<shared_file_CID>.json`
		IpfsShared(Vec<u8>),
		// Collection metadata is located on Ethereum in the relevant field on the source token
		// ethereum://<contractaddress>/<originalid>
		Ethereum(H160),
	}

	/// Information related to a specific collection
	#[derive(Debug, Clone, Encode, Decode, PartialEq, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct OldCollectionInformation<T: Config> {
		/// The owner of the collection
		pub owner: T::AccountId,
		/// A human friendly name
		pub name: CollectionNameType,
		/// Collection metadata reference scheme
		pub metadata_scheme: OldMetadataScheme,
		/// configured royalties schedule
		pub royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
		/// Maximum number of tokens allowed in a collection
		pub max_issuance: Option<TokenCount>,
		/// The chain in which the collection was minted originally
		pub origin_chain: OriginChain,
		/// The next available serial_number
		pub next_serial_number: SerialNumber,
		/// the total count of tokens in this collection
		pub collection_issuance: TokenCount,
		/// All serial numbers owned by an account in a collection
		pub owned_tokens: BoundedVec<TokenOwnership<T>, <T as Config>::MaxTokensPerCollection>,
	}

	#[storage_alias]
	type CollectionInfo<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, CollectionUuid, OldCollectionInformation<T>>;

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade<T: Config>() -> Result<(), &'static str> {
		log::info!(target: "Nft", "Upgrade to V2 Pre Upgrade.");
		let onchain = Pallet::<T>::on_chain_storage_version();

		assert_eq!(onchain, 1);
		Ok(())
	}

	pub fn on_runtime_upgrade<T: Config>() -> Weight {
		let current = Pallet::<T>::current_storage_version();
		let onchain = Pallet::<T>::on_chain_storage_version();
		log::info!(target: "Nft", "Running migration with current storage version {current:?} / onchain {onchain:?}");

		let mut weight = DbWeight::get().reads_writes(2, 0);

		if onchain == 1 {
			log::info!(target: "Nft", "Migrating from onchain version 0 to onchain version 2.");
			weight += migrate::<T>();

			log::info!(target: "Nft", "Migration successfully finished.");
			StorageVersion::new(2).put::<Pallet<T>>();
		} else {
			log::info!(target: "Nft", "No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade<T: Config>() -> Result<(), &'static str> {
		log::info!(target: "Nft", "Upgrade to V2 Post Upgrade.");
		let onchain = Pallet::<T>::on_chain_storage_version();

		assert_eq!(onchain, 2);
		Ok(())
	}

	pub fn migrate<T: Config>() -> Weight {
		crate::CollectionInfo::<T>::translate(|_, old: OldCollectionInformation<T>| {
			let add_slash = |mut x: Vec<u8>| -> Vec<u8> {
				if x.last() != Some(&b'/') {
					x.push(b'/');
				}

				x
			};

			let metadata_scheme = match old.metadata_scheme {
				OldMetadataScheme::Https(x) => MetadataScheme::Https(x),
				OldMetadataScheme::Http(x) => MetadataScheme::Http(x),
				OldMetadataScheme::IpfsDir(x) => MetadataScheme::Ipfs(add_slash(x)),
				OldMetadataScheme::IpfsShared(x) => MetadataScheme::Ipfs(add_slash(x)),
				OldMetadataScheme::Ethereum(x) => MetadataScheme::Ethereum(x),
			};

			let new = CollectionInformation {
				owner: old.owner,
				name: old.name,
				metadata_scheme,
				royalties_schedule: old.royalties_schedule,
				max_issuance: old.max_issuance,
				origin_chain: old.origin_chain,
				next_serial_number: old.next_serial_number,
				collection_issuance: old.collection_issuance,
				owned_tokens: old.owned_tokens,
			};

			Some(new)
		});

		Weight::from(22u32)
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::mock::{Test, TestExt};

		#[test]
		fn migration_test() {
			TestExt::default().build().execute_with(|| {
				StorageVersion::new(1).put::<Pallet<Test>>();
				let user_1 = 5_u64;

				let old_info = OldCollectionInformation::<Test> {
					owner: 123_u64,
					name: b"test-collection-1".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_1, Permill::one())],
					}),
					metadata_scheme: OldMetadataScheme::IpfsDir(b"Test1/".to_vec()),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					owned_tokens: BoundedVec::default(),
				};

				// Collection 1
				let collection_id_1 = 678;
				CollectionInfo::<Test>::insert(collection_id_1, old_info.clone());

				let collection_id_2 = 679;
				let info = OldCollectionInformation::<Test> {
					metadata_scheme: OldMetadataScheme::IpfsShared(b"Test1/".to_vec()),
					..old_info.clone()
				};
				CollectionInfo::<Test>::insert(collection_id_2, info);

				let collection_id_3 = 680;
				let info = OldCollectionInformation::<Test> {
					metadata_scheme: OldMetadataScheme::IpfsDir(b"Test1".to_vec()),
					..old_info.clone()
				};
				CollectionInfo::<Test>::insert(collection_id_3, info);

				let collection_id_4 = 681;
				let info = OldCollectionInformation::<Test> {
					metadata_scheme: OldMetadataScheme::Https(b"Test1/".to_vec()),
					..old_info.clone()
				};
				CollectionInfo::<Test>::insert(collection_id_4, info);

				on_runtime_upgrade::<Test>();

				let expected_value = CollectionInformation::<Test> {
					owner: 123_u64,
					name: b"test-collection-1".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_1, Permill::one())],
					}),
					metadata_scheme: MetadataScheme::Ipfs(b"Test1/".to_vec()),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					owned_tokens: BoundedVec::default(),
				};

				// This checks IpfsDir -> Ipfs
				let actual_value = crate::CollectionInfo::<Test>::get(collection_id_1).unwrap();
				assert_eq!(expected_value, actual_value);

				// This checks IpfsShared -> Ipfs
				let actual_value = crate::CollectionInfo::<Test>::get(collection_id_2).unwrap();
				assert_eq!(expected_value, actual_value);

				// This checks that a forward slash is added at the end
				let actual_value = crate::CollectionInfo::<Test>::get(collection_id_3).unwrap();
				assert_eq!(expected_value, actual_value);

				// this checks that other enum variants are not messed with
				let metadata = MetadataScheme::Https(b"Test1/".to_vec());
				let expected_value =
					CollectionInformation::<Test> { metadata_scheme: metadata, ..expected_value };
				let actual_value = crate::CollectionInfo::<Test>::get(collection_id_4).unwrap();
				assert_eq!(expected_value, actual_value);

				assert_eq!(StorageVersion::get::<Pallet<Test>>(), 2);
			});
		}
	}
}
