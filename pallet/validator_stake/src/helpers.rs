use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use seed_primitives::{Balance, Timestamp};

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct Depository {
	/// The total amount of the stash's balance that we are currently accounting for.
	/// It's just `active` plus all the `unlocking` balances.
	#[codec(compact)]
	pub total: Balance,

	#[codec(compact)]
	pub reward: Balance,

	/// Stake Added Unix Time
	pub timestamp: Timestamp,
}
