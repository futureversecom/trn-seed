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

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod types;
mod weights;

pub use weights::WeightInfo;

use alloc::{boxed::Box, string::String};
use frame_support::{
	dispatch::{DispatchInfo, GetDispatchInfo, PostDispatchInfo},
	pallet_prelude::*,
	traits::{IsSubType, IsType},
	transactional,
};
use frame_system::{
	pallet_prelude::*, CheckEra, CheckGenesis, CheckNonZeroSender, CheckNonce, CheckSpecVersion,
	CheckTxVersion, CheckWeight, RawOrigin,
};
use pallet_transaction_payment::{ChargeTransactionPayment, OnChargeTransaction};
use seed_pallet_common::ExtrinsicChecker;
use sp_core::{hexdisplay::AsBytesRef, H160};
use sp_runtime::{
	generic::Era,
	traits::{DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SignedExtension, StaticLookup},
	transaction_validity::ValidTransactionBuilder,
	FixedPointOperand,
};

use crate::types::{ExtrinsicMemoData, XRPLTransaction};

/// The logging target for this pallet
#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "xrpl";

/// Checks performed on a XRPL transaction
pub type XRPLValidations<T> = (
	CheckNonZeroSender<T>,
	CheckSpecVersion<T>,
	CheckTxVersion<T>,
	CheckGenesis<T>,
	CheckEra<T>,
	CheckNonce<T>,
	CheckWeight<T>,
	ChargeTransactionPayment<T>,
);

impl<T> Call<T>
	where
		T: Send + Sync + Config,
		<T as frame_system::Config>::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
		<T as frame_system::Config>::Index: Into<u32>,
		T::AccountId: From<H160>,
		T: pallet_transaction_payment::Config,
		<<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance: Send + Sync + FixedPointOperand + From<u64>,
		<T as frame_system::Config>::RuntimeCall: From<<T as Config>::RuntimeCall>,
		PostDispatchInfo: From<<<T as Config>::RuntimeCall as Dispatchable>::PostInfo>,
		<T as frame_system::Config>::Index: From<u32>,
{

	pub fn is_self_contained(&self) -> bool {
		matches!(self, Call::submit_encoded_xrpl_transaction { .. })
	}

	/// Checks if the extrinsic is self-contained.
	/// An error returned here will not be reported to the caller,
	/// implying that the caller will be waiting indefinitely for a transaction.
	pub fn check_self_contained(&self) -> Option<Result<H160, TransactionValidityError>> {
		if let Call::submit_encoded_xrpl_transaction { encoded_msg, call, .. } = self {
			let check = || {
				let tx: XRPLTransaction = XRPLTransaction::try_from(encoded_msg.as_bytes_ref())
					.map_err(|e| {
						log::error!("⛔️ failed to convert encoded_msg to XRPLTransaction: {:?}", e);
						InvalidTransaction::Call
					})?;
				let origin = tx.get_account().map_err(|e| {
					log::error!("⛔️ failed to extract account from memo data: {:?}, err: {:?}", tx.account, e);
					InvalidTransaction::Call
				})?;

				// check if the origin is a futurepass holder, to switch the caller to the futurepass
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
		if let Call::submit_encoded_xrpl_transaction { .. } = self {
			// pre dispatch will be done within the `apply_self_contained` below.
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
		if let Call::submit_encoded_xrpl_transaction { encoded_msg, signature, call } = self {
			let tx = XRPLTransaction::try_from(encoded_msg.as_bytes_ref())
				.map_err(|e| {
					log::error!("⛔️ failed to convert encoded_msg to XRPLTransaction: {:?}", e);
					e
				})
				.ok()?;
			let ExtrinsicMemoData { chain_id, nonce, max_block_number, tip, hashed_call } = tx.get_extrinsic_data()
				.map_err(|e| {
					log::error!("⛔️ failed to extract extrinsic data from memo data: {:?}, err: {:?}", tx.memos, e);
					e
				})
				.ok()?;
			if chain_id != T::ChainId::get() {
				log::error!("⛔️ chain id mismatch");
				return None;
			}

			// check the call against hex encoded hashed (blake256) call
			if sp_io::hashing::blake2_256(&call.encode()) != hashed_call {
				log::error!("⛔️ hashed call mismatch");
				return None;
			}

			// ensure inner nested call is not the same call
			if let Some(Call::submit_encoded_xrpl_transaction { .. }) = call.is_sub_type() {
        log::error!("⛔️ cannot nest submit_encoded_xrpl_transaction call");
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

			let validations: XRPLValidations<T> = (
				CheckNonZeroSender::new(),
				CheckSpecVersion::<T>::new(),
				CheckTxVersion::<T>::new(),
				CheckGenesis::<T>::new(),
				CheckEra::<T>::from(Era::immortal()),
				CheckNonce::from(nonce.into()),
				CheckWeight::new(),
				ChargeTransactionPayment::<T>::from(tip.into()),
			);

			SignedExtension::validate(&validations, &T::AccountId::from(*origin), &(*call.clone()).into(), dispatch_info, len).ok()?;

			// priority is based on the provided tip in the xrpl transaction data
			let priority = ChargeTransactionPayment::<T>::get_priority(&dispatch_info, len, tip.into(), 0.into());
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
		if let Some(Call::submit_encoded_xrpl_transaction { encoded_msg, .. }) = call.is_sub_type() {
			// Pre Dispatch
			let tx = XRPLTransaction::try_from(encoded_msg.as_bytes_ref())
				.map_err(|e| {
					log::error!("⛔️ failed to convert encoded_msg to XRPLTransaction: {:?}", e);
					InvalidTransaction::Call
				})
				.ok()?;
			let ExtrinsicMemoData { nonce, tip, .. } = tx.get_extrinsic_data()
				.map_err(|e| {
					log::error!("⛔️ failed to extract extrinsic data from memo data: {:?}, err: {:?}", tx.memos, e);
					InvalidTransaction::Call
				})
				.ok()?;
			// validation instances for this extrinsic; these are responsible for potential state changes
			let validations: XRPLValidations<T> = (
				CheckNonZeroSender::new(),
				CheckSpecVersion::<T>::new(),
				CheckTxVersion::<T>::new(),
				CheckGenesis::<T>::new(),
				CheckEra::<T>::from(Era::immortal()),
				CheckNonce::from(nonce.into()),
				CheckWeight::new(),
				ChargeTransactionPayment::<T>::from(tip.into()),
			);

			// Pre Dispatch
			let pre = SignedExtension::pre_dispatch(validations, &T::AccountId::from(*info), &call.clone().into(), dispatch_info, len).ok()?;

			// Dispatch
			let res = call.dispatch(frame_system::RawOrigin::None.into());
			let post_info = res.map_or_else(|err| err.post_info, |info| info);

			// Post Dispatch
			<XRPLValidations<T> as SignedExtension>::post_dispatch(
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
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>
			// + IsType<<Self as frame_system::Config>::RuntimeCall>
			+ IsSubType<Call<Self>>;

		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: Parameter
			+ Into<<Self as frame_system::Config>::RuntimeOrigin>
			+ IsType<<<Self as frame_system::Config>::RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin>;

		/// Chain ID of EVM.
		type ChainId: Get<u64>;

		/// The maximum bounded length for the XRPL signed message/transaction.
		#[pallet::constant]
		type MaxMessageLength: Get<u32>;

		/// The maximum bounded length for the XRPL signature.
		#[pallet::constant]
		type MaxSignatureLength: Get<u32>;

		/// Interface to generate weights
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Failed to decode XRPL transaction
		XRPLTransaction,
		/// Failed to get account from XRPL transaction
		XRPLTransactionAccount,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config>
	where
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		/// XRPL transaction with encoded extrinsic executed
		XRPLExtrinsicExecuted {
			public_key: [u8; 33],
			caller: T::AccountId,
			r_address: String,
			call: <T as pallet::Config>::RuntimeCall,
		},
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
		/// - `encoded_msg`: The encoded, verified XRPL transaction.
		/// - `signature`: The signature of the XRPL transaction; ignored since it's verified in
		///   self-contained call trait impl.
		/// - `call`: The call to dispatch by the XRPL transaction signer (pubkey).
		#[pallet::weight({
			let dispatch_info = call.get_dispatch_info();
			T::WeightInfo::submit_encoded_xrpl_transaction().saturating_add(dispatch_info.weight)
		})]
		#[transactional]
		pub fn submit_encoded_xrpl_transaction(
			origin: OriginFor<T>,
			encoded_msg: BoundedVec<u8, T::MaxMessageLength>,
			_signature: BoundedVec<u8, T::MaxSignatureLength>,
			call: Box<<T as Config>::RuntimeCall>,
		) -> DispatchResult {
			ensure_none(origin)?;

			let tx: XRPLTransaction = XRPLTransaction::try_from(encoded_msg.as_bytes_ref())
				.map_err(|e| {
					log::error!("⛔️ failed to convert encoded_msg to XRPLTransaction: {:?}", e);
					Error::<T>::XRPLTransaction
				})?;

			let public_key = tx.get_public_key().map_err(|e| {
				log::error!(
					"⛔️ failed to extract public key from memo data: {:?}, err: {:?}",
					tx.memos,
					e
				);
				Error::<T>::XRPLTransactionAccount
			})?;
			let who: T::AccountId = tx
				.get_account()
				.map_err(|e| {
					log::error!(
						"⛔️ failed to extract account from memo data: {:?}, err: {:?}",
						tx.account,
						e
					);
					Error::<T>::XRPLTransactionAccount
				})?
				.into();

			let dispatch_origin = T::RuntimeOrigin::from(RawOrigin::Signed(who.clone()));
			call.clone().dispatch(dispatch_origin).map_err(|e| e.error)?;

			Self::deposit_event(Event::XRPLExtrinsicExecuted {
				public_key,
				caller: who,
				r_address: tx.account,
				call: *call,
			});
			Ok(().into())
		}
	}
}
