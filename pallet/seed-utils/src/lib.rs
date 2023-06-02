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

use frame_support::{
	pallet_prelude::DispatchResult,
	traits::{Currency, ExistenceRequirement, Get},
	PalletId,
};
use sp_std::vec::Vec;

use seed_primitives::RootUpgrader;

use frame_system::{ensure_signed, pallet_prelude::OriginFor};
pub use pallet::*;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use core::fmt::Debug;

	use codec::{Codec, MaxEncodedLen};
	use frame_support::{traits::WithdrawReasons, Parameter};
	use frame_system::ensure_root;
	use scale_info::TypeInfo;
	use seed_primitives::RootOrGovernanceKeyGetter;
	use sp_runtime::traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize, Member};

	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RootUpgrader: RootUpgrader;
		type Currency: Currency<Self::AccountId>;
		type CallerKey: RootOrGovernanceKeyGetter<Self::AccountId>;
		// type Balance: Parameter
		// 	+ Member
		// 	+ AtLeast32BitUnsigned
		// 	+ Codec
		// 	+ Default
		// 	+ Copy
		// 	+ MaybeSerializeDeserialize
		// 	+ Debug
		// 	+ MaxEncodedLen
		// 	+ TypeInfo;
		type WithdrawAmount: Get<BalanceOf<Self>>;
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(25000000)]
		pub fn cheap_upgrade(origin: OriginFor<T>, code: Vec<u8>) -> DispatchResult {
			ensure_root(origin)?;

			T::RootUpgrader::set_code_cheap(code)?;

			let privileged_caller = T::CallerKey::get();

			T::Currency::withdraw(
				// privileged_caller.into(),
				&privileged_caller,
				T::WithdrawAmount::get(),
				WithdrawReasons::FEE,
				ExistenceRequirement::KeepAlive,
			)?;

			Ok(())
		}
	}
}
