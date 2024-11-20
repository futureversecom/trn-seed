// Copyright 2023-2024 Futureverse Corporation Limited
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
	pallet_prelude::*, CheckEra, CheckNonZeroSender, CheckNonce, CheckSpecVersion, CheckTxVersion,
	CheckWeight, RawOrigin,
};
use pallet_transaction_payment::{ChargeTransactionPayment, OnChargeTransaction};
use seed_pallet_common::{log, ExtrinsicChecker};
use sp_core::{hexdisplay::AsBytesRef, H160};
use sp_runtime::{
	generic::Era,
	traits::{
		DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SignedExtension, StaticLookup, Zero,
	},
	transaction_validity::ValidTransactionBuilder,
	FixedPointOperand,
};

use crate::types::{ExtrinsicMemoData, XRPLTransaction, XrplPublicKey};

/// The logging target for this pallet
#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "xrpl";

/// Checks performed on a XRPL against origin
pub type XRPLValidations<T> = (
	CheckNonZeroSender<T>,
	CheckSpecVersion<T>,
	CheckTxVersion<T>,
	CheckEra<T>,
	CheckWeight<T>,
	ChargeTransactionPayment<T>,
);

/// Checks performed on a XRPL against EOA (tx.origin)
/// The origin changes for Futurepass based transactions; this is required as a separate
/// set of validations which must be performed on the EOA (futurepass holder).
pub type EOANonceValidation<T> = (CheckNonce<T>,);

impl<T> Call<T>
where
	T: Send + Sync + Config,
	<T as frame_system::Config>::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	<T as frame_system::Config>::Nonce: Into<u32>,
	T::AccountId: From<H160>,
	T: pallet_transaction_payment::Config,
	<<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance:
		Send + Sync + FixedPointOperand + From<u64>,
	<T as frame_system::Config>::RuntimeCall: From<<T as Config>::RuntimeCall>,
	PostDispatchInfo: From<<<T as Config>::RuntimeCall as Dispatchable>::PostInfo>,
	<T as frame_system::Config>::Nonce: From<u32>,
{
	pub fn is_self_contained(&self) -> bool {
		matches!(self, Call::transact { .. })
	}

	/// Checks if the extrinsic is self-contained.
	/// An error returned here will not be reported to the caller,
	/// implying that the caller will be waiting indefinitely for a transaction.
	pub fn check_self_contained(&self) -> Option<Result<H160, TransactionValidityError>> {
		if let Call::transact { encoded_msg, call, .. } = self {
			let check = || {
				let tx: XRPLTransaction = XRPLTransaction::try_from(encoded_msg.as_bytes_ref()).map_err(|e| {
					log!(info, "⛔️ failed to convert encoded_msg to XRPLTransaction: {:?}", e);
					InvalidTransaction::Call
				})?;
				let origin = tx.get_account().map_err(|e| {
					log!(info, "⛔️ failed to extract account from memo data: {:?}, err: {:?}", tx.account, e);
					InvalidTransaction::Call
				})?;

				// check if the origin is a futurepass holder, to switch the caller to the futurepass
				if <T as pallet::Config>::FuturepassLookup::check_extrinsic(call, &()) {
					if let Ok(futurepass) = <T as pallet::Config>::FuturepassLookup::lookup(origin) {
						return Ok(futurepass);
					}
					log!(info, "⛔️ caller is not a futurepass holder");
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
		if let Call::transact { .. } = self {
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
		if let Call::transact { encoded_msg, signature, call } = self {
			let validate = || -> TransactionValidity {
				let (nonce, tip) = validate_params::<T>(encoded_msg.as_bytes_ref(), signature.as_bytes_ref(), call)
					.map_err(|e| {
						log!(info, "⛔️ validate_self_contained: failed to validate params: {:?}", e);
						InvalidTransaction::Call
					})?;

				let validations: XRPLValidations<T> = (
					CheckNonZeroSender::new(),
					CheckSpecVersion::<T>::new(),
					CheckTxVersion::<T>::new(),
					CheckEra::<T>::from(Era::immortal()),
					CheckWeight::new(),
					ChargeTransactionPayment::<T>::from(tip.into()),
				);

				let mut tx_origin = T::AccountId::from(*origin);

				// validate signed extensions using origin
				SignedExtension::validate(&validations, &tx_origin, &(*call.clone()).into(), dispatch_info, len)?;

				// validate signed extensions using EOA - for futurepass based transactions
				if <T as pallet::Config>::FuturepassLookup::check_extrinsic(call, &()) {
					// this implies that the origin is futurepass address; we need to get the EOA associated with it
					let eoa = <T as pallet::Config>::FuturepassLookup::unlookup(*origin);
					if eoa == H160::zero() {
						log!(info, "⛔️ failed to get EOA from futurepass address");
						return Err(TransactionValidityError::Invalid(InvalidTransaction::BadProof));
					}
					tx_origin = T::AccountId::from(eoa);
				}

				// validate nonce signed extension using EOA
				let validations: EOANonceValidation<T> = (CheckNonce::from(nonce.into()),);
				SignedExtension::validate(&validations, &tx_origin, &(*call.clone()).into(), dispatch_info, len)?;

				// priority is based on the provided tip in the xrpl transaction data
				let priority = ChargeTransactionPayment::<T>::get_priority(&dispatch_info, len, tip.into(), 0.into());
				let who: T::AccountId = (tx_origin).clone();
				let account = frame_system::Account::<T>::get(who);
				let mut builder =
					ValidTransactionBuilder::default().and_provides((tx_origin.clone(), nonce)).priority(priority);

				// in the context of the pool, a transaction with too high a nonce is still considered valid
				if nonce > account.nonce.into() {
					if let Some(prev_nonce) = nonce.checked_sub(1) {
						builder = builder.and_requires((tx_origin, prev_nonce))
					}
				}

				builder.build()
			};

			Some(validate())
		} else {
			None
		}
	}

	pub fn apply_self_contained(
		outer_call: <T as Config>::RuntimeCall,
		info: &H160,
		dispatch_info: &DispatchInfoOf<<T as frame_system::Config>::RuntimeCall>,
		len: usize,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<<T as Config>::RuntimeCall>>> {
		if let Some(Call::transact { encoded_msg, call, signature }) = outer_call.is_sub_type() {
			// Pre Dispatch
			let (nonce, tip) = validate_params::<T>(encoded_msg.as_bytes_ref(), signature.as_bytes_ref(), call)
				.map_err(|e| {
					log!(info, "⛔️ apply_self_contained: failed to validate params: {:?}", e);
					InvalidTransaction::Call
				})
				.ok()?;

			// validation instances for this extrinsic; these are responsible for potential state changes
			let validations: XRPLValidations<T> = (
				CheckNonZeroSender::new(),
				CheckSpecVersion::<T>::new(),
				CheckTxVersion::<T>::new(),
				CheckEra::<T>::from(Era::immortal()),
				CheckWeight::new(),
				ChargeTransactionPayment::<T>::from(tip.into()),
			);

			let mut tx_origin = T::AccountId::from(*info);

			// Pre Dispatch - execute signed extensions with inner call
			let pre =
				SignedExtension::pre_dispatch(validations, &tx_origin, &(*call.clone()).into(), dispatch_info, len)
					.ok()?;

			// Pre Dispatch - execute signed extensions with EOA - for futurepass based transactions
			if <T as pallet::Config>::FuturepassLookup::check_extrinsic(call, &()) {
				// this implies that the origin is futurepass address; we need to get the EOA associated with it
				let eoa = <T as pallet::Config>::FuturepassLookup::unlookup(*info);
				if eoa == H160::zero() {
					log!(info, "⛔️ failed to get EOA from futurepass address");
					return None;
				}
				tx_origin = T::AccountId::from(eoa);
			}

			// validate nonce signed extension using EOA
			let validations: EOANonceValidation<T> = (CheckNonce::from(nonce.into()),);
			SignedExtension::pre_dispatch(validations, &tx_origin, &(*call.clone()).into(), dispatch_info, len).ok()?;

			// Dispatch - execute outer call (transact)
			let res = outer_call.dispatch(frame_system::RawOrigin::None.into());
			let post_info = res.unwrap_or_else(|err| err.post_info);

			// Post Dispatch
			<XRPLValidations<T> as SignedExtension>::post_dispatch(
				Some(pre),
				dispatch_info,
				&post_info.into(),
				len,
				&res.map(|_| ()).map_err(|e| e.error),
			)
			.ok()?;

			return Some(res);
		}
		None
	}
}

fn validate_params<T: Config>(
	encoded_msg: &[u8],
	signature: &[u8],
	call: &<T as Config>::RuntimeCall,
) -> Result<(u32, u64), String>
where
	<T as frame_system::Config>::AccountId: From<H160>,
{
	let tx = XRPLTransaction::try_from(encoded_msg.as_bytes_ref()).map_err(|e| {
		alloc::format!("⛔️ failed to convert encoded_msg to XRPLTransaction: {:?}", e)
	})?;

	let ExtrinsicMemoData { genesis_hash, nonce, max_block_number, tip, hashed_call } =
		tx.get_extrinsic_data().map_err(|e| {
			alloc::format!(
				"⛔️ failed to extract extrinsic data from memo data: {:?}, err: {:?}",
				tx.memos,
				e
			)
		})?;

	// check if genesis hash matches chain genesis hash
	if <frame_system::Pallet<T>>::block_hash(BlockNumberFor::<T>::zero()).as_ref()
		!= genesis_hash.as_ref()
	{
		return Err("⛔️ genesis hash mismatch".into());
	}

	// check the call against hex encoded hashed (blake256) call
	if sp_io::hashing::blake2_256(&call.encode()) != hashed_call {
		return Err("⛔️ hashed call mismatch".into());
	}

	// ensure inner nested call is not the same call
	if let Some(Call::transact { .. }) = call.is_sub_type() {
		return Err("⛔️ cannot nest transact call".into());
	}

	if <frame_system::Pallet<T>>::block_number() > max_block_number.into() {
		return Err("⛔️ max block number too low".into());
	}

	let success = tx
		.verify_transaction(signature)
		.map_err(|e| alloc::format!("⛔️ failed to verify transaction: {:?}", e))?;
	if !success {
		return Err("⛔️ transaction verification unsuccessful".into());
	}
	Ok((nonce, tip))
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
	pub trait Config: frame_system::Config
	where
		<Self as frame_system::Config>::AccountId: From<H160>,
	{
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// A lookup to get futurepass account id for a futurepass holder.
		/// Additionally validates if a call is a futurepass extrinsic.
		type FuturepassLookup: StaticLookup<Source = H160, Target = H160>
			+ ExtrinsicChecker<
				Call = <Self as pallet::Config>::RuntimeCall,
				Extra = (),
				Result = bool,
			>;

		/// The aggregated and decodable `RuntimeCall` type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>
			// + IsType<<Self as frame_system::Config>::RuntimeCall>
			+ IsSubType<Call<Self>>;

		/// Inner call validator
		type CallValidator: ExtrinsicChecker<
			Call = <Self as Config>::RuntimeCall,
			Extra = (),
			Result = bool,
		>;

		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: Parameter
			+ Into<<Self as frame_system::Config>::RuntimeOrigin>
			+ IsType<<<Self as frame_system::Config>::RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin>;

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
		/// Call filtered
		CallFiltered,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config>
	where
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		/// XRPL transaction with encoded extrinsic executed
		XRPLExtrinsicExecuted {
			public_key: XrplPublicKey,
			caller: T::AccountId,
			r_address: String,
			call: <T as pallet::Config>::RuntimeCall,
		},
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> where
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
		#[pallet::call_index(0)]
		#[pallet::weight({
			let dispatch_info = call.get_dispatch_info();
			T::WeightInfo::transact().saturating_add(dispatch_info.weight)
		})]
		#[transactional]
		pub fn transact(
			origin: OriginFor<T>,
			encoded_msg: BoundedVec<u8, T::MaxMessageLength>,
			_signature: BoundedVec<u8, T::MaxSignatureLength>,
			call: Box<<T as Config>::RuntimeCall>,
		) -> DispatchResult {
			ensure_none(origin)?;

			// validate the inner call
			if !T::CallValidator::check_extrinsic(&call, &()) {
				return Err(Error::<T>::CallFiltered.into());
			}

			let tx: XRPLTransaction = XRPLTransaction::try_from(encoded_msg.as_bytes_ref())
				.map_err(|e| {
					log!(info, "⛔️ failed to convert encoded_msg to XRPLTransaction: {:?}", e);
					Error::<T>::XRPLTransaction
				})?;

			let public_key = tx.get_public_key().map_err(|e| {
				log!(
					info,
					"⛔️ failed to extract public key from tx data: {:?}, err: {:?}",
					tx.signing_pub_key,
					e
				);
				Error::<T>::XRPLTransactionAccount
			})?;
			let who: T::AccountId = tx
				.get_account()
				.map_err(|e| {
					log!(
						info,
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
			Ok(())
		}
	}
}
