use seed_primitives::{CollectionUuid, TokenId};
use sp_runtime::DispatchError;

use crate::{CollectionNameType, MetadataScheme, OriginChain, RoyaltiesSchedule, TokenCount};

pub trait NFTExt {
	type AccountId;

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
