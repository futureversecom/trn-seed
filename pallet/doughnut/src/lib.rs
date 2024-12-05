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

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use sp_core::{ecdsa, H160};
use sp_io::hashing::keccak_256;
use sp_runtime::FixedPointOperand;

use alloc::{boxed::Box, string::String, vec::Vec};
use doughnut_rs::{
	doughnut::Doughnut,
	signature::{crypto::verify_signature, SignatureVersion},
	traits::{DoughnutApi, DoughnutVerify},
	Topping,
};
use frame_support::{
	dispatch::{DispatchInfo, GetDispatchInfo, PostDispatchInfo},
	traits::{GetCallMetadata, IsSubType},
};
use frame_system::{CheckNonZeroSender, CheckNonce, CheckWeight};
use pallet_transaction_payment::{ChargeTransactionPayment, OnChargeTransaction};
use seed_pallet_common::{log, ExtrinsicChecker};
use seed_primitives::AccountId20;
use sp_runtime::{
	traits::{
		DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SignedExtension, StaticLookup, Zero,
	},
	transaction_validity::ValidTransactionBuilder,
};

pub mod weights;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

const TRN_PERMISSION_DOMAIN: &str = "trn";
/// The logging target for this pallet
#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "doughnut";

/// Checks performed on a fee payer of a Doughnut transaction
pub type DoughnutFeePayerValidations<T> =
	(pallet_transaction_payment::ChargeTransactionPayment<T>,);

/// Checks performed on a sender of a Doughnut transaction
pub type DoughnutSenderValidations<T> = (
	frame_system::CheckNonZeroSender<T>,
	frame_system::CheckNonce<T>,
	frame_system::CheckWeight<T>,
);

impl<T> Call<T>
where
	T: Send + Sync + Config,
	<T as frame_system::Config>::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	<T as frame_system::Config>::Nonce: Into<u32>,
	T::AccountId: From<H160> + Into<H160>,
	T: pallet_transaction_payment::Config,
	<<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance:
		Send + Sync + FixedPointOperand + From<u64>,
	<T as frame_system::Config>::RuntimeCall: From<<T as Config>::RuntimeCall>,
	PostDispatchInfo: From<<<T as Config>::RuntimeCall as Dispatchable>::PostInfo>,
	<T as frame_system::Config>::Nonce: From<u32>,
	<T as Config>::RuntimeCall: GetCallMetadata,
{
	pub fn is_self_contained(&self) -> bool {
		matches!(self, Call::transact { .. })
	}

	pub fn check_self_contained(&self) -> Option<Result<H160, TransactionValidityError>> {
		if let Call::transact { call, doughnut, nonce, genesis_hash, tip, signature } = self {
			let check = || {
				// run doughnut common validations
				let Ok(Doughnut::V1(doughnut_v1)) = Pallet::<T>::run_doughnut_common_validations(doughnut.clone())
				else {
					return Err(TransactionValidityError::Invalid(InvalidTransaction::BadProof));
				};

				// Validate doughnut - expiry
				doughnut_v1.validate(doughnut_v1.holder, frame_system::Pallet::<T>::block_number()).map_err(|e| {
					log!(info, "🍩 failed to validate doughnut expiry: {:?}", e);
					TransactionValidityError::Invalid(InvalidTransaction::BadProof)
				})?;

				// Verify doughnut signature
				doughnut_v1.verify().map_err(|e| {
					log!(info, "🍩 failed to verify doughnut signature: {:?}", e);
					TransactionValidityError::Invalid(InvalidTransaction::BadProof)
				})?;

				// Retrieve holder address
				let Ok(holder_address) = crate::Pallet::<T>::get_address(doughnut_v1.holder) else {
					log!(info, "🍩 failed to get holder address: {:?}", doughnut_v1.holder);
					return Err(TransactionValidityError::Invalid(InvalidTransaction::BadSigner));
				};

				// Verify outer signature against holder address
				let outer_call: Call<T> = Call::transact {
					call: call.clone(),
					doughnut: doughnut.clone(),
					nonce: *nonce,
					genesis_hash: *genesis_hash,
					tip: *tip,
					signature: Vec::<u8>::new(),
				};

				verify_signature(
					SignatureVersion::EIP191 as u8,
					signature,
					&doughnut_v1.holder(),
					outer_call.encode().as_slice(),
				)
				.map_err(|e| {
					log!(info, "🍩 failed to verify outer signature: {:?}", e);
					TransactionValidityError::Invalid(InvalidTransaction::BadProof)
				})?;

				// Resolve to holder address
				Ok(holder_address.into())
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
		if let Call::transact { call: inner_call, doughnut, genesis_hash, nonce, tip, .. } = self {
			let validate = || -> TransactionValidity {
				let fee_payer_address = Self::validate_params(doughnut, genesis_hash, inner_call).map_err(|_| {
					InvalidTransaction::Call
				})?;
				let sender_address = T::AccountId::from(*origin);

			// construct the validation instances
			let validations_fee_payer: DoughnutFeePayerValidations<T> =
				(ChargeTransactionPayment::<T>::from((*tip).into()),);
			let validations_sender: DoughnutSenderValidations<T> =
				(CheckNonZeroSender::new(), CheckNonce::from((*nonce).into()), CheckWeight::new());

				SignedExtension::validate(
					&validations_sender,
					&sender_address,
					&(**inner_call).clone().into(),
					dispatch_info,
					len,
				)?;
				SignedExtension::validate(
					&validations_fee_payer,
					&fee_payer_address,
					&(**inner_call).clone().into(),
					dispatch_info,
					len,
				)?;

			// priority is based on the provided tip in the doughnut transaction data
			let priority = ChargeTransactionPayment::<T>::get_priority(dispatch_info, len, (*tip).into(), 0.into());
			let who: T::AccountId = (*origin).into();
			let account = frame_system::Account::<T>::get(who.clone());
			let transaction_nonce = { *nonce };
			let mut builder =
				ValidTransactionBuilder::default().and_provides((origin, transaction_nonce)).priority(priority);

			// In the context of the pool, a transaction with
			// too high a nonce is still considered valid
			if transaction_nonce > account.nonce.into() {
				if let Some(prev_nonce) = transaction_nonce.checked_sub(1) {
					builder = builder.and_requires((origin, prev_nonce))
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
		call: <T as Config>::RuntimeCall,
		info: &H160,
		dispatch_info: &DispatchInfoOf<<T as frame_system::Config>::RuntimeCall>,
		len: usize,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<<T as Config>::RuntimeCall>>> {
		if let Some(Call::transact { call: inner_call, doughnut, genesis_hash, nonce, tip, .. }) = call.is_sub_type() {
			let fee_payer_address = Self::validate_params(doughnut, genesis_hash, inner_call).ok()?;
			let sender_address = T::AccountId::from(*info);

			// Pre dispatch
			// Create the validation instances for this extrinsic
			let validations_fee_payer: DoughnutFeePayerValidations<T> =
				(ChargeTransactionPayment::<T>::from((*tip).into()),);
			let validations_sender: DoughnutSenderValidations<T> =
				(CheckNonZeroSender::new(), CheckNonce::from((*nonce).into()), CheckWeight::new());

			let pre_sender = SignedExtension::pre_dispatch(
				validations_sender,
				&sender_address,
				&(**inner_call).clone().into(),
				dispatch_info,
				len,
			)
			.ok()?;
			let pre_issuer = SignedExtension::pre_dispatch(
				validations_fee_payer,
				&fee_payer_address,
				&(**inner_call).clone().into(),
				dispatch_info,
				len,
			)
			.ok()?;

			// Dispatch the outer call. i.e Doughnut::transact() with None as the origin
			let res = call.dispatch(frame_system::RawOrigin::None.into());
			let post_info = res.unwrap_or_else(|err| err.post_info);

			// post dispatch
			<DoughnutFeePayerValidations<T> as SignedExtension>::post_dispatch(
				Some(pre_issuer),
				dispatch_info,
				&post_info.into(),
				len,
				&res.map(|_| ()).map_err(|e| e.error),
			)
			.ok()?;
			<DoughnutSenderValidations<T> as SignedExtension>::post_dispatch(
				Some(pre_sender),
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

	fn validate_params(
		doughnut: &[u8],
		genesis_hash: &T::Hash,
		call: &<T as Config>::RuntimeCall,
	) -> Result<T::AccountId, String> {
		// Genesis hash check
		let genesis_hash_onchain: T::Hash = frame_system::Pallet::<T>::block_hash(BlockNumberFor::<T>::zero());
		if *genesis_hash != genesis_hash_onchain {
			log!(info, "🍩 genesis hash mismatch: {:?}", genesis_hash);
			return Err("🍩 genesis hash mismatch".into());
		}

		// Doughnut work
		// run doughnut common validations
		let Ok(Doughnut::V1(doughnut_v1)) = crate::Pallet::<T>::run_doughnut_common_validations(doughnut.to_vec())
		else {
			return Err("🍩 Doughnut validation failed.".into());
		};
		// No need to do the doughnut verification again since already did in check_self_contained()
		let Ok(fee_payer_doughnut) = crate::Pallet::<T>::get_address(doughnut_v1.fee_payer()) else {
			log!(info, "🍩 failed to get fee payer address: {:?}", doughnut_v1.fee_payer());
			return Err("🍩 failed to get fee payer address".into());
		};
		let mut fee_payer_address = fee_payer_doughnut;
		// Futurepass check
		if <T as Config>::FuturepassLookup::check_extrinsic(call, &()).is_ok() {
			let Ok(futurepass) = <T as Config>::FuturepassLookup::lookup(fee_payer_address.clone().into()) else {
				log!(info, "🍩 failed to retrieve futurepass address for the address: {:?}", fee_payer_address);
				return Err("🍩 failed to retrieve futurepass address for the address".into());
			};
			fee_payer_address = futurepass.into();
		}

		Ok(fee_payer_address)
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config
	where
		<Self as frame_system::Config>::AccountId: From<H160>,
	{
		/// The overarching event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The overarching call type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>
			+ GetCallMetadata;
		/// Inner call validator
		type CallValidator: ExtrinsicChecker<
			Call = <Self as Config>::RuntimeCall,
			Extra = Topping,
			Result = DispatchResult,
		>;
		/// A lookup to get futurepass account id for a futurepass holder.
		type FuturepassLookup: StaticLookup<Source = H160, Target = H160>
			+ ExtrinsicChecker<
				Call = <Self as Config>::RuntimeCall,
				Extra = (),
				Result = DispatchResult,
			>;
		/// Weight information for the extrinsic call in this module.
		type WeightInfo: WeightInfo;
	}

	/// Storage map for revoked doughnut information
	#[pallet::storage]
	pub type BlockedDoughnuts<T: Config> = StorageMap<_, Twox64Concat, [u8; 32], bool, ValueQuery>;

	/// Double map from issuer to blocked holder
	#[pallet::storage]
	pub type BlockedHolders<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		T::AccountId,
		bool,
		ValueQuery,
	>;

	/// Storage map for whitelisted holder information
	#[pallet::storage]
	pub type WhitelistedHolders<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, bool, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config>
	where
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		/// Doughnut transaction executed
		DoughnutCallExecuted {
			doughnut: Vec<u8>,
			call: <T as Config>::RuntimeCall,
			result: DispatchResult,
		},
		/// Whitelisted holders updated
		WhitelistedHoldersUpdated { holder: T::AccountId, enabled: bool },
		/// Doughnut revoke state updated
		DoughnutRevokeStateUpdated { doughnut_hash: [u8; 32], revoked: bool },
		/// Holder revocation updated
		HolderRevokeStateUpdated { issuer: T::AccountId, holder: T::AccountId, revoked: bool },
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
		UnauthorizedSender,
		/// Cannot revoke doughnut that does was issued by another account
		CallerNotIssuer,
		/// The doughnut has been revoked by the issuer
		DoughnutRevoked,
		/// The holder address has been revoked by the issuer
		HolderRevoked,
		/// Topping decode failed.
		ToppingDecodeFailed,
		/// Unable to find TRN domain.
		TRNDomainNotfound,
		/// Topping permissions denied.
		ToppingPermissionDenied,
		/// Inner call is not whitelisted
		UnsupportedInnerCall,
		/// Holder not whitelisted
		HolderNotWhitelisted,
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
		<T as Config>::RuntimeCall: GetCallMetadata,
	{
		#[pallet::call_index(0)]
		#[pallet::weight({
			let call_weight = call.get_dispatch_info().weight;
			T::WeightInfo::transact().saturating_add(call_weight)
		})]
		pub fn transact(
			origin: OriginFor<T>,
			call: Box<<T as Config>::RuntimeCall>,
			doughnut: Vec<u8>,
			_nonce: u32,
			_genesis_hash: T::Hash,
			_tip: u64,
			_signature: Vec<u8>,
		) -> DispatchResult {
			ensure_none(origin)?;

			// run doughnut common validations
			let Doughnut::V1(doughnut_v1) =
				Self::run_doughnut_common_validations(doughnut.clone())?
			else {
				return Err(Error::<T>::UnsupportedDoughnutVersion)?;
			};

			// verify the doughnut
			doughnut_v1.verify().map_err(|_| Error::<T>::DoughnutVerifyFailed)?;

			// Check holder == sender
			let issuer_address = Self::get_address(doughnut_v1.issuer)?;
			let holder_address = Self::get_address(doughnut_v1.holder)?;

			// check whitelisted holder
			ensure!(
				WhitelistedHolders::<T>::get(holder_address.clone()),
				Error::<T>::HolderNotWhitelisted
			);

			// Ensure doughnut is not revoked
			let doughnut_hash = keccak_256(&doughnut);
			ensure!(!BlockedDoughnuts::<T>::get(doughnut_hash), Error::<T>::DoughnutRevoked);

			// Ensure holder is not revoked
			ensure!(
				!BlockedHolders::<T>::get(issuer_address.clone(), holder_address.clone()),
				Error::<T>::HolderRevoked
			);

			// permission domain - topping validations
			let Some(mut topping_payload) = doughnut_v1.get_topping(TRN_PERMISSION_DOMAIN) else {
				return Err(Error::<T>::TRNDomainNotfound)?;
			};
			let topping = Topping::decode(&mut topping_payload)
				.map_err(|_| Error::<T>::ToppingDecodeFailed)?;

			// check topping permissions
			T::CallValidator::check_extrinsic(&(*call), &topping)?;

			// dispatch the inner call
			let issuer_origin = frame_system::RawOrigin::Signed(issuer_address).into();
			let e = call.clone().dispatch(issuer_origin);
			Self::deposit_event(Event::<T>::DoughnutCallExecuted {
				doughnut,
				call: *call,
				result: e.map(|_| ()).map_err(|e| e.error),
			});

			Ok(())
		}

		/// Block a specific doughnut to be used
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::revoke_doughnut())]
		pub fn revoke_doughnut(
			origin: OriginFor<T>,
			doughnut: Vec<u8>,
			revoke: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;
			// run doughnut common validations
			let Ok(Doughnut::V1(doughnut_v1)) =
				Self::run_doughnut_common_validations(doughnut.clone())
			else {
				return Err(Error::<T>::UnsupportedDoughnutVersion)?;
			};
			// Only the issuer of the doughnut can revoke the doughnut
			ensure!(who == Self::get_address(doughnut_v1.issuer)?, Error::<T>::CallerNotIssuer);
			let doughnut_hash = keccak_256(&doughnut);
			match revoke {
				true => BlockedDoughnuts::<T>::insert(doughnut_hash, true),
				false => BlockedDoughnuts::<T>::remove(doughnut_hash),
			}
			Self::deposit_event(Event::<T>::DoughnutRevokeStateUpdated {
				doughnut_hash,
				revoked: revoke,
			});
			Ok(())
		}

		/// Block a holder from executing any doughnuts from a specific issuer
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::revoke_holder())]
		pub fn revoke_holder(
			origin: OriginFor<T>,
			holder: T::AccountId,
			revoke: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;
			match revoke {
				true => BlockedHolders::<T>::insert(who.clone(), holder.clone(), true),
				false => BlockedHolders::<T>::remove(who.clone(), holder.clone()),
			}
			Self::deposit_event(Event::<T>::HolderRevokeStateUpdated {
				issuer: who,
				holder,
				revoked: revoke,
			});
			Ok(())
		}

		/// Update whitelisted holders list
		// Note: this is for temporary purpose. Might change in the future
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::update_whitelisted_holders())]
		pub fn update_whitelisted_holders(
			origin: OriginFor<T>,
			holder: T::AccountId,
			add: bool,
		) -> DispatchResult {
			ensure_root(origin)?;
			WhitelistedHolders::<T>::set(holder.clone(), add);
			Self::deposit_event(Event::<T>::WhitelistedHoldersUpdated { holder, enabled: add });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<H160>,
{
	fn get_address(raw_pub_key: [u8; 33]) -> Result<T::AccountId, DispatchError> {
		let account_id_20 =
			AccountId20::try_from(ecdsa::Public::from_raw(raw_pub_key)).map_err(|e| {
				log!(info, "🍩 failed to convert pubkey to account id: {:?}", e);
				Error::<T>::UnauthorizedSender
			})?;
		Ok(T::AccountId::from(H160::from_slice(&account_id_20.0)))
	}

	fn run_doughnut_common_validations(
		doughnut_payload: Vec<u8>,
	) -> Result<Doughnut, DispatchError> {
		// decode the doughnut
		let doughnut_decoded = Doughnut::decode(&mut &doughnut_payload[..]).map_err(|e| {
			log!(info, "🍩 failed to decode doughnut: {:?}", e);
			Error::<T>::DoughnutDecodeFailed
		})?;

		// only supports v1 for now
		let Doughnut::V1(_) = doughnut_decoded.clone() else {
			log!(info, "🍩 unsupported doughnut version");
			return Err(Error::<T>::UnsupportedDoughnutVersion)?;
		};

		Ok(doughnut_decoded)
	}
}
