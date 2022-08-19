#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode, MaxEncodedLen};
use root_primitives::AssetId;
use scale_info::TypeInfo;
use sp_core::U256;
use sp_runtime::{ArithmeticError, DispatchError, RuntimeDebug};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(
	Encode,
	Decode,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	Ord,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TradingPair(pub AssetId, pub AssetId);

impl From<(AssetId, AssetId)> for TradingPair {
	fn from(asset_ids: (AssetId, AssetId)) -> Self {
		if asset_ids.0 > asset_ids.1 {
			TradingPair(asset_ids.1, asset_ids.0)
		} else {
			TradingPair(asset_ids.0, asset_ids.1)
		}
	}
}

impl TradingPair {
	pub fn new(asset_id_a: AssetId, asset_id_b: AssetId) -> Self {
		TradingPair::from((asset_id_a, asset_id_b))
	}
}

/// Defines a set of safe math operations that return a `DispatchError` which is expected in an
/// anchor instruction execution.
/// adapted from: https://docs.rs/solana-safe-math/latest/src/solana_safe_math/lib.rs.html#1-107
pub trait SafeMath {
	type Output;

	fn add(&self, rhs: Self::Output) -> Result<Self::Output, DispatchError>;
	fn sub(&self, rhs: Self::Output) -> Result<Self::Output, DispatchError>;
	fn mul(&self, rhs: Self::Output) -> Result<Self::Output, DispatchError>;
	fn div(&self, rhs: Self::Output) -> Result<Self::Output, DispatchError>;
	// fn pow(&self, exp: u32) -> Result<Self::Output, DispatchError>;
}

macro_rules! safe_math {
	($type: ident) => {
		/// $type implementation of the SafeMath trait
		impl SafeMath for $type {
			type Output = $type;

			fn add(&self, rhs: Self::Output) -> Result<Self::Output, DispatchError> {
				self.checked_add(rhs).ok_or(ArithmeticError::Overflow.into())
			}

			fn sub(&self, rhs: Self::Output) -> Result<Self::Output, DispatchError> {
				self.checked_sub(rhs).ok_or(ArithmeticError::Underflow.into())
			}

			fn mul(&self, rhs: Self::Output) -> Result<Self::Output, DispatchError> {
				self.checked_mul(rhs).ok_or(ArithmeticError::Underflow.into())
			}

			fn div(&self, rhs: Self::Output) -> Result<Self::Output, DispatchError> {
				self.checked_div(rhs).ok_or(ArithmeticError::DivisionByZero.into())
			}

			// fn pow(&self, exp: u32) -> Result<Self::Output, DispatchError> {
			// 	self.checked_pow(exp).ok_or(ArithmeticError::Overflow.into())
			// }
		}
	};
}

// implement SafeMath via macro for U256 and primitive numeric types
safe_math!(U256);
safe_math!(u128);
safe_math!(u32);
