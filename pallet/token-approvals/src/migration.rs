#[allow(dead_code)]
pub mod v1_storage {
	use crate::Config;
	use seed_primitives::CollectionUuid;
	use sp_std::prelude::*;

	pub struct Module<T>(sp_std::marker::PhantomData<T>);
	frame_support::decl_storage! {
		trait Store for Module<T: Config> as TokenApprovals {
			pub ERC721ApprovalsForAll get(fn erc721_approvals_for_all): double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) CollectionUuid => Option<T::AccountId>;
		}
	}
}
