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

use crate::{Call::call_with_fee_preferences, *};
use frame_support::traits::IsSubType;
use pallet_futurepass::ProxyProvider;
use pallet_transaction_payment::OnChargeTransaction;
use precompile_utils::{Address, ErcIdConversion};
use seed_primitives::{AccountId, AssetId, Balance};
use sp_runtime::traits::{DispatchInfoOf, PostDispatchInfoOf};

impl<T> OnChargeTransaction<T> for Pallet<T>
where
	T: Config
		+ frame_system::Config<AccountId = AccountId>
		+ pallet_transaction_payment::Config
		+ pallet_dex::Config
		+ pallet_evm::Config
		+ pallet_assets_ext::Config
		+ pallet_futurepass::Config,
	<T as frame_system::Config>::RuntimeCall: IsSubType<crate::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_evm::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_futurepass::Call<T>>,
	<T as Config>::OnChargeTransaction: OnChargeTransaction<T>,
	<T as Config>::ErcIdConversion: ErcIdConversion<AssetId, EvmId = Address>,
	Balance: From<<<T as Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance>,
{
	type Balance = <<T as Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance;
	type LiquidityInfo =
		<<T as Config>::OnChargeTransaction as OnChargeTransaction<T>>::LiquidityInfo;

	/// Intercept the withdraw fee, and swap any tokens to gas tokens if the call is
	/// pallet_fee_proxy.call_with_fee_preferences()
	fn withdraw_fee(
		who: &T::AccountId,
		call: &<T as frame_system::Config>::RuntimeCall,
		info: &DispatchInfoOf<<T as frame_system::Config>::RuntimeCall>,
		fee: Self::Balance,
		tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		let mut who = who;

		// Check whether this call has specified fee preferences
		if let Some(call_with_fee_preferences { payment_asset, max_payment, call }) =
			call.is_sub_type()
		{
			let mut total_fee: Balance = Balance::from(fee);
			let native_asset = <T as Config>::FeeAssetId::get();

			// if the inner call is pallet_futurepass::Call::proxy_extrinsic(), and the caller is a
			// delegate of the FP(futurepass), we switch the gas payer to the FP
			if let Some(pallet_futurepass::Call::proxy_extrinsic { futurepass, .. }) =
				call.is_sub_type()
			{
				if <T as pallet_futurepass::Config>::Proxy::exists(futurepass, who, None) {
					who = futurepass;
				}
			}

			// Check if the inner call is an evm call. This will increase total gas to swap
			// This is required as the fee value here does not take into account the max
			// fee from an evm call. For all other extrinsics, the fee parameter
			// should cover all required fees.
			if let Some(pallet_evm::Call::call {
				gas_limit,
				max_fee_per_gas,
				max_priority_fee_per_gas,
				..
			}) = call.is_sub_type()
			{
				if let Some(FeePreferencesData { max_fee_scaled, .. }) = get_fee_preferences_data::<
					T,
					<T as Config>::ErcIdConversion,
					pallet_futurepass::Pallet<T>,
				>(
					*gas_limit,
					<T as Config>::EVMBaseFeeProvider::evm_base_fee_per_gas(),
					Some(*max_fee_per_gas),
					*max_priority_fee_per_gas,
					*payment_asset,
				)
				.ok()
				{
					total_fee = total_fee.saturating_add(max_fee_scaled);
				}
			}

			let path: &[AssetId] = &[*payment_asset, native_asset];
			pallet_dex::Pallet::<T>::do_swap_with_exact_target(
				who,
				total_fee,
				*max_payment,
				path,
				who.clone(),
				None,
			)
			.map_err(|_| InvalidTransaction::Payment)?;
		};

		<<T as Config>::OnChargeTransaction as OnChargeTransaction<T>>::withdraw_fee(
			who, call, info, fee, tip,
		)
	}

	/// Hand the fee and the tip over to the `[OnUnbalanced]` implementation.
	/// Since the predicted fee might have been too high, parts of the fee may
	/// be refunded.
	///
	/// Note: The `corrected_fee` already includes the `tip`.
	fn correct_and_deposit_fee(
		who: &T::AccountId,
		dispatch_info: &DispatchInfoOf<<T as frame_system::Config>::RuntimeCall>,
		post_info: &PostDispatchInfoOf<<T as frame_system::Config>::RuntimeCall>,
		corrected_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), TransactionValidityError> {
		// <T as OnChargeTransaction<T>>::correct_and_deposit_fee(
		<<T as Config>::OnChargeTransaction as OnChargeTransaction<T>>::correct_and_deposit_fee(
			who,
			dispatch_info,
			post_info,
			corrected_fee,
			tip,
			already_withdrawn,
		)
	}
}
