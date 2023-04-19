// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code
use crate::Config;
use codec::{Decode, Encode, MaxEncodedLen};
use hex;
use scale_info::TypeInfo;
use seed_primitives::AssetId;
use sp_arithmetic::traits::SaturatedConversion;
use sp_core::{H160, U256};
use sp_runtime::{ArithmeticError, DispatchError, RuntimeDebug};
use sp_std::{marker::PhantomData, prelude::*};
use serde::{Deserialize, Deserializer, Serialize};

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

/// A DEX exchange address generator implementation
pub struct ExchangeAddressGenerator<T>(PhantomData<T>);

impl From<(AssetId, AssetId)> for TradingPair {
	fn from(asset_ids: (AssetId, AssetId)) -> Self {
		if asset_ids.0 > asset_ids.1 {
			TradingPair(asset_ids.1, asset_ids.0)
		} else {
			TradingPair(asset_ids.0, asset_ids.1)
		}
	}
}

/// A function that generates an `AccountId` for a DEX exchange / (asset_0, asset_1) pair
pub trait ExchangeAddressFor {
	/// The Account Id type
	type AccountId;
	/// The Asset Id type
	type AssetId;
	/// Create and exchange address given `asset_id`
	fn exchange_address_for(asset_id: AssetId, asset_id: AssetId) -> Self::AccountId;
}


impl<T: Config> ExchangeAddressFor for ExchangeAddressGenerator<T>
where
	T::AccountId: From<H160>,
	T::AssetId: Into<u64>,
{
	type AccountId = T::AccountId;
	type AssetId = T::AssetId;

	/// Generates a unique, deterministic exchange address for the given `asset_id_0`, `asset_id_1`
	/// pair
	fn exchange_address_for(asset_id_0: AssetId, asset_id_1: AssetId) -> T::AccountId {
		let mut buf = Vec::<u8>::with_capacity(160);
		buf.extend_from_slice(b"dex::address");
		buf.extend_from_slice(&asset_id_0.to_le_bytes());
		buf.extend_from_slice(&asset_id_1.to_le_bytes());
		let data: H160 = H160::from_slice(buf.as_slice());
		T::AccountId::from(data)
	}
}

impl TradingPair {
	pub fn new(asset_id_a: AssetId, asset_id_b: AssetId) -> Self {
		TradingPair::from((asset_id_a, asset_id_b))
	}
}

#[derive(Debug, PartialEq)]
// A balance type for receiving over RPC
pub struct WrappedBalance(pub u128);
#[derive(Debug, Default, Serialize, Deserialize)]
/// Private, used to help serde handle `WrappedBalance`
/// https://github.com/serde-rs/serde/issues/751#issuecomment-277580700
struct WrappedBalanceHelper {
	value: u128,
}
impl Serialize for WrappedBalance {
	fn serialize<S>(&self, serializer: S) -> sp_std::result::Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		WrappedBalanceHelper { value: self.0 }.serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for WrappedBalance {
	fn deserialize<D>(deserializer: D) -> sp_std::result::Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer
			.deserialize_any(WrappedBalanceVisitor)
			.map_err(|_| serde::de::Error::custom("deserialize failed"))
	}
}

/// Implements custom serde visitor for decoding balance inputs as integer or hex
struct WrappedBalanceVisitor;

impl<'de> serde::de::Visitor<'de> for WrappedBalanceVisitor {
	type Value = WrappedBalance;
	fn expecting(&self, formatter: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(formatter, "an integer or hex-string")
	}

	fn visit_u64<E>(self, v: u64) -> sp_std::result::Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		Ok(WrappedBalance(v.saturated_into()))
	}

	fn visit_str<E>(self, s: &str) -> sp_std::result::Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		//remove the first two chars as we are expecting a string prefixed with '0x'
		let decoded_string = hex::decode(&s[2..])
			.map_err(|_| serde::de::Error::custom("expected hex encoded string"))?;
		let fixed_16_bytes: [u8; 16] = decoded_string
			.try_into()
			.map_err(|_| serde::de::Error::custom("parse big int as u128 failed"))?;
		Ok(WrappedBalance(u128::from_be_bytes(fixed_16_bytes)))
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
