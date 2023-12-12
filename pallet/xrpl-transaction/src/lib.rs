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

//! # XRPL transaction pallet
//!
//! TODO: Add description of the pallet.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;

pub mod types;
// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
// mod weights;

// pub use weights::WeightInfo;

use codec::Decode;
use frame_support::{
	dispatch::{DispatchInfo, PostDispatchInfo},
	pallet_prelude::*,
	traits::{IsSubType, IsType},
	transactional,
};
use frame_system::{
	pallet_prelude::*, CheckGenesis, CheckNonZeroSender, CheckNonce, CheckSpecVersion,
	CheckTxVersion, CheckWeight, RawOrigin,
};
use pallet_transaction_payment::{ChargeTransactionPayment, OnChargeTransaction};
use seed_pallet_common::ExtrinsicChecker;
use sp_core::{hexdisplay::AsBytesRef, H160};
use sp_runtime::{
	traits::{DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SignedExtension, StaticLookup},
	transaction_validity::ValidTransactionBuilder,
	FixedPointOperand,
};

use crate::types::{ExtrinsicMemoData, XUMMTransaction};

/// The logging target for this pallet
#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "xrpl-transaction";

/// Checks performed on a XUMM transaction
pub type XUMMValidations<T> = (
	frame_system::CheckNonZeroSender<T>,
	frame_system::CheckSpecVersion<T>,
	frame_system::CheckTxVersion<T>,
	frame_system::CheckGenesis<T>,
	// frame_system::CheckEra<T>,
	frame_system::CheckNonce<T>,
	frame_system::CheckWeight<T>,
	ChargeTransactionPayment<T>,
);

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
		matches!(self, Call::submit_encoded_xumm_transaction { .. })
	}

	/// Checks if the extrinsic is self-contained.
	/// An error returned here will not be reported to the caller,
	/// implying that the caller will be waiting indefinitely for a transaction.
	pub fn check_self_contained(&self) -> Option<Result<H160, TransactionValidityError>> {
		if let Call::submit_encoded_xumm_transaction { encoded_msg, .. } = self {
			let check = || {
				let tx: XUMMTransaction = XUMMTransaction::try_from(encoded_msg.as_bytes_ref())
					.map_err(|e| {
						log::error!("⛔️ failed to convert encoded_msg to XUMMTransaction: {:?}", e);
						InvalidTransaction::Call
					})?;
				let origin = tx.get_account().map_err(|e| {
					log::error!("⛔️ failed to extract account from memo data: {:?}, err: {:?}", tx.account, e);
					InvalidTransaction::Call
				})?;

				// check if the origin is a futurepass holder, to switch the caller to the futurepass
				let call_data = tx.get_extrinsic_data()
					.map_err(|e| {
						log::error!("⛔️ failed to extract extrinsic data from memo data: {:?}, err: {:?}", tx.memos, e);
						InvalidTransaction::Call
					})?
					.call;
				let call = Pallet::<T>::get_runtime_call_from_xumm_extrinsic(&call_data)
					.map_err(|e| {
						log::error!("⛔️ failed to get runtime call from xumm extrinsic: {:?}", e);
						InvalidTransaction::Call
					})?;
				if <T as pallet::Config>::FuturepassLookup::check_extrinsic(&call) {
					if let Ok(futurepass) = <T as pallet::Config>::FuturepassLookup::lookup(origin) {
						return Ok(futurepass);
					}
					log::error!("⛔️ caller is not a futurepass holder");
					return Err(InvalidTransaction::Call.into());
				}

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
		if let Call::submit_encoded_xumm_transaction { .. } = self {
			// pre dispatch will be done within the `apply_self_contained`` below.
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
		if let Call::submit_encoded_xumm_transaction { encoded_msg, signature } = self {
			let tx = XUMMTransaction::try_from(encoded_msg.as_bytes_ref())
				.map_err(|e| {
					log::error!("⛔️ failed to convert encoded_msg to XUMMTransaction: {:?}", e);
					e
				})
				.ok()?;
			let ExtrinsicMemoData { nonce, call, max_block_number } = tx.get_extrinsic_data()
				.map_err(|e| {
					log::error!("⛔️ failed to extract extrinsic data from memo data: {:?}, err: {:?}", tx.memos, e);
					e
				})
				.ok()?;

			// ensure inner nested call is not the same call
			let call = Pallet::<T>::get_runtime_call_from_xumm_extrinsic(&call)
				.map_err(|e| {
					log::error!("⛔️ failed to get runtime call from xumm extrinsic: {:?}", e);
					e
				})
				.ok()?;

			if let Some(Call::submit_encoded_xumm_transaction { .. }) = call.is_sub_type() {
        log::error!("⛔️ cannot nest submit_encoded_xumm_transaction call");
        return None;
    	}

			if <frame_system::Pallet<T>>::block_number() > max_block_number.into() {
				log::error!("⛔️ max block number too low");
				return None;
			}

			let success = tx.verify_transaction(&signature).map_err(|e| {
					log::error!("⛔️ failed to verify transaction: {:?}", e);
					e
				})
				.ok()?;
			if !success {
				log::error!("⛔️ transaction verification unsuccessful");
				return None;
			}

			let validations: XUMMValidations<T> = (
				CheckNonZeroSender::new(),
				CheckSpecVersion::<T>::new(),
				CheckTxVersion::<T>::new(),
				CheckGenesis::<T>::new(),
				CheckNonce::from(nonce.into()),
				CheckWeight::new(),
				ChargeTransactionPayment::<T>::from(0.into()),
			);

			SignedExtension::validate(&validations, &T::AccountId::from(*origin), &call.into(), dispatch_info, len).ok()?;

			let priority = 0; // TODO: determine priority by debugging signed extrinsics
			let who: T::AccountId = (*origin).into();
			let account = frame_system::Account::<T>::get(who.clone());
			let mut builder = ValidTransactionBuilder::default()
				.and_provides((origin, nonce))
				.priority(priority);

			// in the context of the pool, a transaction with too high a nonce is still considered valid
			if nonce > account.nonce.into() {
				if let Some(prev_nonce) = nonce.checked_sub(1) {
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
		if let Some(Call::submit_encoded_xumm_transaction { encoded_msg, .. }) = call.is_sub_type() {
			// Pre Dispatch
			let tx = XUMMTransaction::try_from(encoded_msg.as_bytes_ref())
				.map_err(|e| {
					log::error!("⛔️ failed to convert encoded_msg to XUMMTransaction: {:?}", e);
					InvalidTransaction::Call
				})
				.ok()?;
			let ExtrinsicMemoData { nonce, .. } = tx.get_extrinsic_data()
				.map_err(|e| {
					log::error!("⛔️ failed to extract extrinsic data from memo data: {:?}, err: {:?}", tx.memos, e);
					InvalidTransaction::Call
				})
				.ok()?;
			// validation instances for this extrinsic; these are responsible for potential state changes
			let validations: XUMMValidations<T> = (
				CheckNonZeroSender::new(),
				CheckSpecVersion::<T>::new(),
				CheckTxVersion::<T>::new(),
				CheckGenesis::<T>::new(),
				CheckNonce::from(nonce.into()),
				CheckWeight::new(),
				ChargeTransactionPayment::<T>::from(0.into()),
			);
			let pre = SignedExtension::pre_dispatch(validations, &T::AccountId::from(*info), &call.clone().into(), dispatch_info, len).ok()?;

			// Dispatch
			let res = call.dispatch(frame_system::RawOrigin::None.into());
			let post_info = res.map_or_else(|err| err.post_info, |info| info);

			// Post Dispatch
			<XUMMValidations<T> as SignedExtension>::post_dispatch(
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

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::origin]
	pub type Origin = seed_primitives::AccountId20;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_transaction_payment::Config
	where
		<Self as frame_system::Config>::AccountId: From<H160>,
	{
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// A lookup to get futurepass account id for a futurepass holder.
		/// Additionally validates if a call is a futurepass extrinsic.
		type FuturepassLookup: StaticLookup<Source = H160, Target = H160>
			+ ExtrinsicChecker<Call = <Self as pallet::Config>::RuntimeCall>;

		/// The aggregated and decodable `RuntimeCall` type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			// + GetDispatchInfo
			+ From<frame_system::Call<Self>>
			// + IsType<<Self as frame_system::Config>::RuntimeCall>
			+ IsSubType<Call<Self>>;

		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: Parameter
			+ Into<<Self as frame_system::Config>::RuntimeOrigin>
			+ IsType<<<Self as frame_system::Config>::RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin>;

		#[pallet::constant]
		type MaxMessageLength: Get<u32>;

		#[pallet::constant]
		type MaxSignatureLength: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Failed to decode XUMM transaction
		DecodeXUMMTransaction,
		/// Failed to get account from XUMM transaction
		DecodeXUMMTransactionAccount,
		/// Failed to decode XUMM transaction extrinsic data
		DecodeXUMMTransactionExtrinsicData,
		/// Failed to decode XUMM transaction memo data
		DecodeXUMMTransactionMemoData,
		/// XUMM transaction extrinsic not found
		XUMMTransactionExtrinsicNotFound,
		/// XUMM tranaction extrinsic length is invalid
		XUMMTransactionExtrinsicLengthInvalid,
		/// Cannot decode XUMM extrinsic call
		CannotDecodeXUMMExtrinsicCall,
		/// Account nonce mismatch
		NonceMismatch,
		/// Max block number exceeded
		MaxBlockNumberExceeded,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config>
	where
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		/// XUMM transaction with encoded extrinsic executed
		XUMMExtrinsicExecuted { caller: T::AccountId, call: <T as pallet::Config>::RuntimeCall },
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> where
		<T as frame_system::Config>::AccountId: From<H160>
	{
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		/// Dispatch the given call through an XRPL account (signer). Transaction fees will be paid
		/// by the signer.
		///
		/// Parameters:
		/// - `origin`: The origin of the call; must be `None` - as this is an unsigned extrinsic.
		/// - `encoded_msg`: The encoded, verified XUMM transaction.
		#[pallet::weight(0)]
		// TODO
		// #[pallet::weight({
		// 	let without_base_extrinsic_weight = true;
		// 	<T as pallet_evm::Config>::GasWeightMapping::gas_to_weight({
		// 		let transaction_data: TransactionData = transaction.into();
		// 		transaction_data.gas_limit.unique_saturated_into()
		// 	}, without_base_extrinsic_weight)
		// })]
		#[transactional]
		pub fn submit_encoded_xumm_transaction(
			origin: OriginFor<T>,
			encoded_msg: BoundedVec<u8, T::MaxMessageLength>,
			_signature: BoundedVec<u8, T::MaxSignatureLength>,
		) -> DispatchResult {
			ensure_none(origin)?;

			let tx: XUMMTransaction = XUMMTransaction::try_from(encoded_msg.as_bytes_ref())
				.map_err(|e| {
					log::error!("⛔️ failed to convert encoded_msg to XUMMTransaction: {:?}", e);
					Error::<T>::DecodeXUMMTransaction
				})?;

			let who: T::AccountId = tx
				.get_account()
				.map_err(|e| {
					log::error!(
						"⛔️ failed to extract account from memo data: {:?}, err: {:?}",
						tx.account,
						e
					);
					Error::<T>::DecodeXUMMTransactionAccount
				})?
				.into();

			let ExtrinsicMemoData { call, .. } = tx.get_extrinsic_data().map_err(|e| {
				log::error!(
					"⛔️ failed to extract extrinsic data from memo data: {:?}, err: {:?}",
					tx.memos,
					e
				);
				Error::<T>::DecodeXUMMTransactionExtrinsicData
			})?;

			let dispatch_origin = T::RuntimeOrigin::from(RawOrigin::Signed(who.clone()));
			let call = Self::get_runtime_call_from_xumm_extrinsic(&call)?;
			call.clone().dispatch(dispatch_origin).map_err(|e| e.error)?;

			Self::deposit_event(Event::XUMMExtrinsicExecuted { caller: who, call });
			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		/// Given a full SCALE encoded extrinsic, strips the first 4 byte prefix,
		/// decodes call data to a Runtime call and returns it.
		/// This can also be a call that nests other calls and can target any pallet in the
		/// runtime.
		///
		/// # Returns
		/// The `RuntimeCall` that is encoded in the memo data.
		pub fn get_runtime_call_from_xumm_extrinsic(
			scale_encoded_extrinsic: &[u8],
		) -> Result<<T as pallet::Config>::RuntimeCall, DispatchError> {
			ensure!(
				scale_encoded_extrinsic.len() >= 4,
				Error::<T>::XUMMTransactionExtrinsicLengthInvalid
			);

			let call =
				<T as pallet::Config>::RuntimeCall::decode(&mut &scale_encoded_extrinsic[2..])
					.map_err(|e| {
						log::warn!("⛔️ Failed to decode the call: {:?}", e);
						Error::<T>::CannotDecodeXUMMExtrinsicCall
					})?;
			Ok(call)
		}
	}
}
