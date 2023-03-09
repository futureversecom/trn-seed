use seed_primitives::{CollectionUuid, MetadataScheme, TokenId};
use sp_runtime::{DispatchError, DispatchResult};

use crate::{
	CollectionInformation, CollectionNameType, Config, OriginChain, RoyaltiesSchedule, TokenCount,
};

pub trait NFTExt {
	type AccountId;
	type T: Config;

	fn do_mint(
		origin: Self::AccountId,
		collection_id: CollectionUuid,
		quantity: TokenCount,
		token_owner: Option<Self::AccountId>,
	) -> DispatchResult;

	fn do_create_collection(
		owner: Self::AccountId,
		name: CollectionNameType,
		initial_issuance: TokenCount,
		max_issuance: Option<TokenCount>,
		token_owner: Option<Self::AccountId>,
		metadata_scheme: MetadataScheme,
		royalties_schedule: Option<RoyaltiesSchedule<Self::AccountId>>,
		origin_chain: OriginChain,
	) -> Result<CollectionUuid, DispatchError>;

	fn get_token_owner(token_id: &TokenId) -> Option<Self::AccountId>;

	fn get_collection_info(
		collection_id: CollectionUuid,
	) -> Result<CollectionInformation<Self::T>, DispatchError>;

	fn enable_xls20_compatibility(
		who: Self::AccountId,
		collection_id: CollectionUuid,
	) -> DispatchResult;
}
