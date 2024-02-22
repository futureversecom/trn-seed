use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::sp_runtime::RuntimeDebug;
use scale_info::TypeInfo;
use seed_primitives::{AccountId, AssetId, Balance, BlockNumber, CollectionUuid};
use sp_core::U256;

#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct SaleInformation<AccountId, BlockNumber> {
	pub status: SaleStatus,
	pub admin: AccountId,
	pub payment_asset: AssetId,
	pub collection_id: CollectionUuid,
  pub tokens_per_voucher: Balance,
	pub vouchers_per_nft: Balance,
	pub funds_raised: Balance,
  pub voucher: AssetId,
	pub start_block: BlockNumber,
	pub end_block: BlockNumber,
}

#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub enum SaleStatus {
	Disabled,
	Enabled,
	Paying,
	Refunding,
	Closed,
}

impl Default for SaleStatus {
	fn default() -> Self {
		SaleStatus::Disabled
	}
}
