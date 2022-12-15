#[allow(dead_code)]
pub mod v1_storage {
	use crate::{
		CollectionNameType, Config, MetadataScheme, OriginChain, RoyaltiesSchedule, SerialNumber,
		TokenCount,
	};
	use codec::{Decode, Encode};
	use scale_info::TypeInfo;
	use seed_primitives::CollectionUuid;
	use sp_std::prelude::*;
	use std::collections::BTreeMap;

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

	pub struct Module<T>(sp_std::marker::PhantomData<T>);
	frame_support::decl_storage! {
		trait Store for Module<T: Config> as Nft {
			pub CollectionInfo get(fn collection_info): map hasher(twox_64_concat) CollectionUuid => Option<CollectionInformation<T::AccountId>>;
			pub CollectionIssuance get(fn collection_issuance): map hasher(twox_64_concat) CollectionUuid => TokenCount;
			pub NextSerialNumber get(fn next_serial_number): map hasher(twox_64_concat) CollectionUuid => SerialNumber;
			pub TokenBalance get(fn token_balance): map hasher(blake2_128_concat) T::AccountId => BTreeMap<CollectionUuid, TokenCount>;
			pub TokenOwner get(fn token_owner): double_map hasher(twox_64_concat) CollectionUuid, hasher(twox_64_concat) SerialNumber => Option<T::AccountId>;
		}
	}
}
