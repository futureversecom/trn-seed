#[allow(dead_code)]
pub mod v1_storage {
	use crate::{CollectionNameType, Config, MetadataScheme, RoyaltiesSchedule, TokenCount};
	use codec::{Decode, Encode};
	use scale_info::TypeInfo;
	use seed_primitives::CollectionUuid;
	use sp_std::prelude::*;

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
	}

	pub struct Module<T>(sp_std::marker::PhantomData<T>);
	frame_support::decl_storage! {
		trait Store for Module<T: Config> as Nft {
			pub CollectionInfo get(fn collection_info): map hasher(twox_64_concat) CollectionUuid => Option<CollectionInformation<T::AccountId>>;
		}
	}
}
