// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

//! Imbalances are an elaborate method of automatically managing total issuance of a currency
//! when they are dropped a hook is triggered to update the currency total issuance accordingly.
//! The may be added and subtracted from each other for efficiencies sake.
//!
//! These should only be created through an instance of `Currency` which will provide the correct
//! asset ID

// wrapping these imbalances in a private module is necessary to ensure absolute
// privacy of the inner member.

use frame_support::traits::{
	fungibles::{Inspect, Unbalanced},
	Get, Imbalance, SameOrOther, TryDrop,
};
use sp_runtime::traits::Zero;
use sp_std::{mem, result};

use seed_primitives::{AssetId, Balance};

use crate::Config;

/// Opaque, move-only struct with private fields that serves as a token
/// denoting that funds have been created without any equal and opposite
/// accounting.
#[must_use]
#[derive(Debug, PartialEq)]
pub struct PositiveImbalance<T: Config> {
	amount: Balance,
	asset_id: AssetId,
	_phantom: sp_std::marker::PhantomData<T>,
}

impl<T: Config> Default for PositiveImbalance<T> {
	fn default() -> Self {
		PositiveImbalance {
			_phantom: sp_std::marker::PhantomData,
			amount: Default::default(),
			asset_id: Default::default(),
		}
	}
}

impl<T: Config> PositiveImbalance<T> {
	/// Create a new positive imbalance from a `balance` and with the given `asset_id`.
	pub fn new(amount: Balance, asset_id: AssetId) -> Self {
		PositiveImbalance { amount, asset_id, _phantom: Default::default() }
	}
	pub fn asset_id(&self) -> AssetId {
		self.asset_id
	}
}

/// Opaque, move-only struct with private fields that serves as a token
/// denoting that funds have been destroyed without any equal and opposite
/// accounting.
#[must_use]
#[derive(Debug, PartialEq)]
pub struct NegativeImbalance<T: Config> {
	amount: Balance,
	asset_id: AssetId,
	_phantom: sp_std::marker::PhantomData<T>,
}

impl<T: Config> Default for NegativeImbalance<T> {
	fn default() -> Self {
		NegativeImbalance {
			_phantom: sp_std::marker::PhantomData,
			amount: Default::default(),
			asset_id: Default::default(),
		}
	}
}

impl<T: Config> NegativeImbalance<T> {
	/// Create a new negative imbalance from a `balance` and with the given `asset_id`.
	pub fn new(amount: Balance, asset_id: AssetId) -> Self {
		NegativeImbalance { amount, asset_id, _phantom: Default::default() }
	}
	pub fn asset_id(&self) -> AssetId {
		self.asset_id
	}
}

impl<T: Config> TryDrop for PositiveImbalance<T> {
	fn try_drop(self) -> result::Result<(), Self> {
		self.drop_zero()
	}
}

impl<T: Config> Imbalance<Balance> for PositiveImbalance<T> {
	type Opposite = NegativeImbalance<T>;

	fn zero() -> Self {
		Self::new(Zero::zero(), Zero::zero())
	}
	fn drop_zero(self) -> result::Result<(), Self> {
		if self.amount.is_zero() || self.asset_id.is_zero() {
			Ok(())
		} else {
			Err(self)
		}
	}
	fn split(self, amount: Balance) -> (Self, Self) {
		let first = self.amount.min(amount);
		let second = self.amount - first;
		let asset_id = self.asset_id;

		mem::forget(self);
		(Self::new(first, asset_id), Self::new(second, asset_id))
	}
	fn merge(mut self, other: Self) -> Self {
		self.amount = self.amount.saturating_add(other.amount);
		mem::forget(other);

		self
	}
	fn subsume(&mut self, other: Self) {
		self.amount = self.amount.saturating_add(other.amount);
		mem::forget(other);
	}
	fn offset(self, other: Self::Opposite) -> SameOrOther<Self, Self::Opposite> {
		let (a, b) = (self.amount, other.amount);
		let asset_id = self.asset_id;
		mem::forget((self, other));

		if a >= b {
			SameOrOther::Same(Self::new(a - b, asset_id))
		} else {
			SameOrOther::Other(NegativeImbalance::new(b - a, asset_id))
		}
	}
	fn peek(&self) -> Balance {
		self.amount
	}
}

impl<T: Config> TryDrop for NegativeImbalance<T> {
	fn try_drop(self) -> result::Result<(), Self> {
		self.drop_zero()
	}
}

impl<T: Config> Imbalance<Balance> for NegativeImbalance<T> {
	type Opposite = PositiveImbalance<T>;

	fn zero() -> Self {
		Self::new(Zero::zero(), Zero::zero())
	}
	fn drop_zero(self) -> result::Result<(), Self> {
		if self.amount.is_zero() || self.asset_id.is_zero() {
			Ok(())
		} else {
			Err(self)
		}
	}
	fn split(self, amount: Balance) -> (Self, Self) {
		let first = self.amount.min(amount);
		let second = self.amount - first;
		let asset_id = self.asset_id;

		mem::forget(self);
		(Self::new(first, asset_id), Self::new(second, asset_id))
	}
	fn merge(mut self, other: Self) -> Self {
		self.amount = self.amount.saturating_add(other.amount);
		mem::forget(other);

		self
	}
	fn subsume(&mut self, other: Self) {
		self.amount = self.amount.saturating_add(other.amount);
		mem::forget(other);
	}
	fn offset(self, other: Self::Opposite) -> SameOrOther<Self, Self::Opposite> {
		let (a, b) = (self.amount, other.amount);
		let asset_id = self.asset_id;
		mem::forget((self, other));

		if a >= b {
			SameOrOther::Same(Self::new(a - b, asset_id))
		} else {
			SameOrOther::Other(PositiveImbalance::new(b - a, asset_id))
		}
	}
	fn peek(&self) -> Balance {
		self.amount
	}
}

impl<T: Config> Drop for PositiveImbalance<T> {
	/// Basic drop handler will just square up the total issuance.
	fn drop(&mut self) {
		if self.asset_id == T::NativeAssetId::get() {
			<pallet_balances::TotalIssuance<T>>::mutate(|v: &mut Balance| {
				*v = v.saturating_add(self.amount)
			})
		} else {
			let v = <pallet_assets::Pallet<T>>::total_issuance(self.asset_id);
			<pallet_assets::Pallet<T>>::set_total_issuance(
				self.asset_id,
				v.saturating_add(self.amount),
			);
		}
	}
}

impl<T: Config> Drop for NegativeImbalance<T> {
	/// Basic drop handler will just square up the total issuance.
	fn drop(&mut self) {
		if self.asset_id == T::NativeAssetId::get() {
			<pallet_balances::TotalIssuance<T>>::mutate(|v: &mut Balance| {
				*v = v.saturating_sub(self.amount)
			})
		} else {
			let v = <pallet_assets::Pallet<T>>::total_issuance(self.asset_id);
			<pallet_assets::Pallet<T>>::set_total_issuance(
				self.asset_id,
				v.saturating_sub(self.amount),
			);
		}
	}
}
