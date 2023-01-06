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
