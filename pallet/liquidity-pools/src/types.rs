use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::RuntimeDebug;
use scale_info::TypeInfo;

/// Stores information about a reward pool.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct PoolInfo<PoolId, AssetId, Balance, BlockNumber> {
	pub id: PoolId,
	pub creator: crate::AccountId,
	pub reward_asset_id: AssetId,
	pub staked_asset_id: AssetId,
	pub interest_rate: u32,
	pub max_tokens: Balance,
	pub last_updated: BlockNumber,
	pub lock_start_block: BlockNumber,
	pub lock_end_block: BlockNumber,
	pub locked_amount: Balance,
	pub pool_status: PoolStatus,
}

#[derive(Clone, Copy, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum PoolStatus {
	// The pool has been closed by the creator and is waiting for users to withdraw their funds.
	Closed,
	// The pool is open for users to stake funds.
	Open,
	// The pool has started and will either roll over or mature at the end of the lock period
	Started,
	// The pool is rolling over to a new period as a successor was created.
	Renewing,
	// The pool has finished it's vesting period and is not rolling over. Users can withdraw their funds.
	Matured,
}

impl Default for PoolStatus {
	fn default() -> Self {
		Self::Closed
	}
}

/// Stores relationship between pools.
#[derive(Default, Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct PoolRelationship<PoolId> {
	pub successor_id: Option<PoolId>,
}

/// Stores user information for a pool.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct UserInfo<Balance> {
	pub amount: Balance,
	pub reward_debt: Balance,
	pub should_rollover: bool,
	pub rolled_over: bool,
}

impl<Balance: Default> UserInfo<Balance> {
	pub fn should_migrate(&self) -> bool {
		self.should_rollover && !self.rolled_over
	}
}

impl<Balance: Default> Default for UserInfo<Balance> {
	fn default() -> Self {
		Self {
			amount: Balance::default(),
			reward_debt: Balance::default(),
			should_rollover: true,
			rolled_over: false,
		}
	}
}
