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
use sp_core::{ecdsa, H160};
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
use doughnut_rs::traits::DoughnutVerify;
use doughnut_rs::Doughnut;
use seed_primitives::AccountId20;

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

				// //
				//
				// // Doughnut work
				// // run doughnut common validations
				// let Ok(Doughnut::V0(doughnut_v0)) = crate::Pallet::<T>::run_doughnut_common_validations(doughnut.clone())?;
				// // No need to do the doughnut verification again since already did in check_self_contained()
				// let Ok(issuer_address) = crate::Pallet::<T>::get_address(doughnut_v0.issuer)?;



				// for now resolve to alith
				let origin: H160 = H160::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"));
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

		if let Some(Call::transact { call: _inner_call, doughnut, nonce }) = call.is_sub_type() {
			// Doughnut work
			// run doughnut common validations
			let Ok(Doughnut::V0(doughnut_v0)) = crate::Pallet::<T>::run_doughnut_common_validations(doughnut.clone()) else {
				return None
			};
			// No need to do the doughnut verification again since already did in check_self_contained()
			let Ok(issuer_address) = crate::Pallet::<T>::get_address(doughnut_v0.issuer) else {
				return None
			};

			// Pre dispatch
			// Create the validation instances for this extrinsic
			let validations: DoughnutValidations<T> = (
				CheckNonZeroSender::new(),
				CheckNonce::from(nonce.clone().into()),
				CheckWeight::new(),
				ChargeTransactionPayment::<T>::from(0.into()),
			);
			let pre = SignedExtension::pre_dispatch(validations, &issuer_address, &call.clone().into(), dispatch_info, len).ok()?;

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
	use doughnut_rs::traits::DoughnutApi;
	use sp_core::ecdsa;
	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config
	where <Self as frame_system::Config>::AccountId: From<H160>,
	{
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
	pub enum Event<T: Config>
	where <T as frame_system::Config>::AccountId: From<H160>,
	{
		DoughnutCallExecuted { result: DispatchResult },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Doguhnut decode failed.
		DoughnutDecodeFailed,
		/// Unsupported doughnut version
		UnsupportedDoughnutVersion,
		/// Doughnut verify failed
		DoughnutVerifyFailed,
		/// Sender is not authorized to use the doughnut
		UnauthorizedSender
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> where
		<T as frame_system::Config>::AccountId: From<H160>
	{
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where <T as frame_system::Config>::AccountId: From<H160>,
	{
		#[pallet::weight(0 as u64)]
		pub fn transact(
			origin: OriginFor<T>,
			call: Box<<T as Config>::RuntimeCall>,
			doughnut: Vec<u8>,
			nonce: u32,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;

			// run doughnut common validations
			let Doughnut::V0(doughnut_v0) = Self::run_doughnut_common_validations(doughnut)?;

			// verify the doughnut
			doughnut_v0.verify()
				.map_err(|_| {
					Error::<T>::DoughnutVerifyFailed
				})?;

			// TODO: Validate the doughnut, for now we just check sender == bearer
			// doughnut_v0.validate(sender., <frame_system::Pallet<T>>::block_number())?;
			let holder_address = Self::get_address(doughnut_v0.holder)?;
			ensure!(holder_address == sender, Error::<T>::UnauthorizedSender);

			// dispatch the inner call
			let issuer_address = Self::get_address(doughnut_v0.issuer)?;
			let issuer_origin = frame_system::RawOrigin::Signed(issuer_address).into();
			let e = call.dispatch(issuer_origin);
			Self::deposit_event(Event::<T>::DoughnutCallExecuted {
				result: e.map(|_| ()).map_err(|e| e.error),
			});

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T>
where <T as frame_system::Config>::AccountId: From<H160>,
{
	fn get_address(raw_pub_key: [u8;32]) -> Result<T::AccountId, Error<T>>
	{
		let mut public_key = [0x04; 33];
		public_key[1..].clone_from_slice(&raw_pub_key[..]);
		let account_id_20 = AccountId20::try_from(ecdsa::Public::from_raw(public_key)).map_err(|_| Error::<T>::UnauthorizedSender)?;
		Ok(T::AccountId::from(H160::from_slice(&account_id_20.0)))
	}

	fn run_doughnut_common_validations(doughnut_payload: Vec<u8>,) -> Result<Doughnut, Error<T>> {
		// decode the doughnut
		let doughnut_decoded =  Doughnut::decode(&mut &doughnut_payload[..])
			.map_err(|_| {
				Error::<T>::DoughnutDecodeFailed
			})?;

		// only supports v0 for now
		let Doughnut::V0(doughnut_v0) = doughnut_decoded.clone() else {
			return Err(Error::<T>::UnsupportedDoughnutVersion)?;
		};

		Ok(doughnut_decoded)
	}

}

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