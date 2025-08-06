use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{BoundedVec, RuntimeDebug};
use scale_info::TypeInfo;

/// Stores information about a reward pool.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct PoolInfo<PoolId, AssetId, Balance, BlockNumber, AccountId> {
	pub id: PoolId,
	pub creator: AccountId,
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
	Closed,
	Open,
	Started,
	Renewing,
	Matured,
	Closing, // FRN-68: New status for pools undergoing closure
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

/// FRN-68: Tracks pools that are being closed with bounded processing
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ClosureState<PoolId> {
	pub pool_id: PoolId,
	pub closure_type: ClosureType,
	pub users_processed: u32,
	pub last_processed_user: Option<BoundedVec<u8, frame_support::traits::ConstU32<128>>>,
}

/// FRN-68: Types of pool closure
#[derive(Clone, Copy, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ClosureType {
	/// Normal closure - creator initiated
	Normal,
	/// Emergency closure - forced cleanup
	Emergency,
}

/// FRN-69: Tracks idle processing state for weight accounting
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct IdleProcessingState<PoolId> {
	pub last_processed_pool: Option<PoolId>,
	pub pools_processed_this_block: u32,
	pub total_weight_consumed: u64,
}

impl<PoolId> Default for IdleProcessingState<PoolId> {
	fn default() -> Self {
		Self { last_processed_pool: None, pools_processed_this_block: 0, total_weight_consumed: 0 }
	}
}

/// FRN-71: Tracks processing state for fair pool processing
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ProcessingState<PoolId> {
	pub last_processed_pool: Option<PoolId>,
	pub round_robin_position: u32,
}

impl<PoolId> Default for ProcessingState<PoolId> {
	fn default() -> Self {
		Self { last_processed_pool: None, round_robin_position: 0 }
	}
}
