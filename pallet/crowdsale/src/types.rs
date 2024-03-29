use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::sp_runtime::RuntimeDebug;
use scale_info::TypeInfo;
use seed_primitives::{AssetId, Balance, CollectionUuid};

/// The unique identifier for a sale
pub type SaleId = u64;

/// Number of decimal places for each voucher asset
pub const VOUCHER_DECIMALS: u8 = 6;

#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct SaleInformation<AccountId, BlockNumber> {
	/// The current sale status
	pub status: SaleStatus<BlockNumber>,
	/// The admin account that can manage the sale
	pub admin: AccountId,
	/// The account that will receive and hold the funds raised
	pub vault: AccountId,
	/// The payment asset used for participation
	pub payment_asset_id: AssetId,
	/// The reward NFT collection id
	pub reward_collection_id: CollectionUuid,
	/// The soft cap price set per token at the sale start
	pub soft_cap_price: Balance,
	/// Total funds raised during the crowdsale
	pub funds_raised: Balance,
	/// Total number of unique contributors that have participated in the crowdsale
	pub participant_count: u64,
	/// The voucher asset id to be paid out
	pub voucher_asset_id: AssetId,
	/// How long the sale will last in blocks
	pub duration: BlockNumber,
}

#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub enum SaleStatus<BlockNumber> {
	/// The sale is awaiting activation
	Pending(BlockNumber),
	/// The sale has been started and is accepting contributions
	Enabled(BlockNumber),
	/// Distributing the rewards to participants,
	/// Balance represents total vouchers paid out
	Distributing(BlockNumber, Balance),
	/// The sale rewards have been distributed to participants
	Ended(BlockNumber),
	/// Distribution has not triggered automatically due to too much
	/// Network traffic
	DistributionFailed(BlockNumber),
}
