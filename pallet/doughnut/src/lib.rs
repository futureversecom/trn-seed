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

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use sp_core::H160;
use sp_runtime::FixedPointOperand;

use alloc::boxed::Box;
use frame_support::{
	dispatch::{DispatchInfo, GetDispatchInfo, PostDispatchInfo},
	traits::IsSubType,
};
use frame_system::{CheckNonZeroSender, CheckNonce, CheckWeight};
use hex_literal::hex;
use pallet_transaction_payment::{ChargeTransactionPayment, OnChargeTransaction};
use seed_pallet_common::logger::info;
use sp_runtime::{
	traits::{DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SignedExtension},
	transaction_validity::ValidTransactionBuilder,
};
use alloc::vec::Vec;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;
//
// mod weights;
//
// pub use weights::WeightInfo;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

impl<T> Call<T>
	where
		T: Send + Sync + Config,
		<T as frame_system::Config>::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
		<T as frame_system::Config>::Index : Into<u32>,
		T::AccountId: From<H160>,
		T: pallet_transaction_payment::Config,
		<<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance: Send + Sync + FixedPointOperand + From<u64>,
		<T as frame_system::Config>::RuntimeCall: From<<T as Config>::RuntimeCall>,
		PostDispatchInfo: From<<<T as Config>::RuntimeCall as Dispatchable>::PostInfo>,
		<T as frame_system::Config>::Index: From<u32>,
{
	pub fn is_self_contained(&self) -> bool {
		matches!(self, Call::transact { call, doughnut, nonce })
	}

	pub fn check_self_contained(&self) -> Option<Result<H160, TransactionValidityError>> {
		if let Call::transact { call, doughnut, nonce } = self {
			let check = || {
				// TODO: implement the following for doughnuts
				// 1. recover the outer signer/sender, verify the signer
				// 2. decode the doughnut, check success against the sender above?
				// 3. return the sender address if all good

				// for now resolve to alith
				let origin: H160 = H160::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac"));
				Ok(origin)
			};

			Some(check())
		} else {
			None
		}
	}

	pub fn pre_dispatch_self_contained(
		&self,
		_origin: &H160,
		_dispatch_info: &DispatchInfoOf<<T as frame_system::Config>::RuntimeCall>,
		_len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		if let Call::transact { .. } = self {
			// pre dispatch will be done within the "apply_self_contained" below.
			Ok(()).into()
		} else {
			None
		}
	}

	pub fn validate_self_contained(
		&self,
		origin: &H160,
		dispatch_info: &DispatchInfoOf<<T as frame_system::Config>::RuntimeCall>,
		len: usize,
	) -> Option<TransactionValidity> {
		if let Call::transact { call, doughnut, nonce } = self {
			// construct the validation instances
			let validations: DoughnutValidations<T> = (
				CheckNonZeroSender::new(),
				CheckNonce::from(nonce.clone().into()),
				CheckWeight::new(),
				ChargeTransactionPayment::<T>::from(0.into()),
			);
			SignedExtension::validate(&validations, &T::AccountId::from(*origin), &(**call).clone().into(), dispatch_info, len).ok()?;

			// TODO: do we need any validation on inner call?
			let priority = 0;
			let who: T::AccountId = (*origin).into();
			let account = frame_system::Account::<T>::get(who.clone());
			let transaction_nonce = *nonce as u32;
			let mut builder = ValidTransactionBuilder::default()
				.and_provides((origin, transaction_nonce))
				.priority(priority);

			// In the context of the pool, a transaction with
			// too high a nonce is still considered valid
			if transaction_nonce > account.nonce.clone().into() {
				if let Some(prev_nonce) = transaction_nonce.checked_sub(1) {
					builder = builder.and_requires((origin, prev_nonce))
				}
			}

			Some(builder.build())
		} else {
			None
		}
	}

	pub fn apply_self_contained(
		call: <T as Config>::RuntimeCall,
		info: &H160,
		dispatch_info: &DispatchInfoOf<<T as frame_system::Config>::RuntimeCall>,
		len: usize,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<<T as Config>::RuntimeCall>>> {

		if let Some(Call::transact { call: inner_call, doughnut, nonce }) = call.is_sub_type() {
			// Pre dispatch
			// Create the validation instances for this extrinsic
			let validations: DoughnutValidations<T> = (
				CheckNonZeroSender::new(),
				CheckNonce::from(nonce.clone().into()),
				CheckWeight::new(),
				ChargeTransactionPayment::<T>::from(0.into()),
			);
			let pre = SignedExtension::pre_dispatch(validations, &T::AccountId::from(*info), &call.clone().into(), dispatch_info, len).ok()?;

			// Dispatch
			let origin: T::RuntimeOrigin = frame_system::RawOrigin::Signed(T::AccountId::from(*info)).into();
			let res = call.dispatch(origin);
			let post_info = match res {
				Ok(info) => info,
				Err(err) => err.post_info,
			};

			// post dispatch
			<DoughnutValidations<T> as SignedExtension>::post_dispatch(
				Some(pre),
				dispatch_info,
				&post_info.into(),
				len,
				&res.map(|_| ()).map_err(|e| e.error),
			).ok()?;

			return Some(res)
		}
		None
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::traits::IsSubType;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		// /// Weight Info
		// type WeightInfo: WeightInfo;
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>
			+ IsType<<Self as frame_system::Config>::RuntimeCall>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T> {
		DoughnutCallExecuted { result: DispatchResult },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0 as u64)]
		pub fn transact(
			origin: OriginFor<T>,
			call: Box<<T as Config>::RuntimeCall>,
			doughnut: Vec<u8>,
			nonce: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;
			info!("doughnut transact");
			let e = call.dispatch(origin);
			Self::deposit_event(Event::<T>::DoughnutCallExecuted {
				result: e.map(|_| ()).map_err(|e| e.error),
			});
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {}

/// Checks performed on a Doughnut transaction
pub type DoughnutValidations<T> = (
	frame_system::CheckNonZeroSender<T>,
	// frame_system::CheckSpecVersion<Runtime>,
	// frame_system::CheckTxVersion<Runtime>,
	// frame_system::CheckGenesis<Runtime>,
	// frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<T>,
	frame_system::CheckWeight<T>,
	pallet_transaction_payment::ChargeTransactionPayment<T>,
);