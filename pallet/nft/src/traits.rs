use seed_primitives::{CollectionUuid, TokenId};
use sp_runtime::{DispatchError, DispatchResult};

use crate::{
	CollectionNameType, Config, MetadataScheme, OriginChain, RoyaltiesSchedule, TokenCount,
};

pub trait NFTExt {
	type AccountId;
	type MaxTokensPerCollection;
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
}
