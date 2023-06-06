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

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

use frame_support::traits::{Currency, ExistenceRequirement, Get};
use sp_std::vec::Vec;

use seed_primitives::RootUpgrader;

use frame_system::pallet_prelude::OriginFor;
pub use pallet::*;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::DispatchResultWithPostInfo, traits::WithdrawReasons,
		weights::PostDispatchInfo,
	};
	use frame_system::ensure_root;
	use seed_primitives::RootOrGovernanceKeyGetter;

	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RootUpgrader: RootUpgrader;
		type Currency: Currency<Self::AccountId>;
		type CallerKey: RootOrGovernanceKeyGetter<Self::AccountId>;
		type WithdrawAmount: Get<BalanceOf<Self>>;
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(25000000)]
		pub fn set_code_cheap(origin: OriginFor<T>, code: Vec<u8>) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			T::RootUpgrader::set_code_cheap(code)?;

			let privileged_caller = T::CallerKey::get();

			T::Currency::withdraw(
				&privileged_caller,
				T::WithdrawAmount::get(),
				WithdrawReasons::FEE,
				ExistenceRequirement::KeepAlive,
			)?;

			Ok(PostDispatchInfo {
				actual_weight: Some(2500000),
				pays_fee: frame_support::weights::Pays::No,
			})
		}
	}
}
