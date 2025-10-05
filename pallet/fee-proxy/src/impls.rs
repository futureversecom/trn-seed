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
use frame_support::traits::{fungibles::Inspect, GetCallMetadata, IsSubType};
use pallet_futurepass::ProxyProvider;
use pallet_transaction_payment::OnChargeTransaction;
use precompile_utils::{Address, ErcIdConversion};
use seed_primitives::{AccountId, AssetId, Balance};
use sp_core::U256;
use sp_runtime::traits::{DispatchInfoOf, PostDispatchInfoOf};

impl<T> OnChargeTransaction<T> for Pallet<T>
where
	T: Config
		+ frame_system::Config<AccountId = AccountId>
		+ pallet_transaction_payment::Config
		+ pallet_dex::Config
		+ pallet_evm::Config
		+ pallet_assets_ext::Config
		+ pallet_futurepass::Config
		+ pallet_sylo_data_verification::Config
		+ pallet_sylo_data_permissions::Config
		+ pallet_sylo_action_permissions::Config
		+ pallet_partner_attribution::Config
		+ pallet_proxy::Config
		+ pallet_utility::Config
		+ pallet_xrpl::Config,
	<T as frame_system::Config>::RuntimeCall: IsSubType<crate::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_futurepass::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_sylo_data_verification::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_sylo_data_permissions::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_sylo_action_permissions::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_proxy::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_utility::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_xrpl::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_evm::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_futurepass::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_sylo_data_verification::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_sylo_data_permissions::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_sylo_action_permissions::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_proxy::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_utility::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_xrpl::Call<T>>,
	<T as pallet_futurepass::Config>::RuntimeCall: IsSubType<pallet_evm::Call<T>>,
	<T as pallet_futurepass::Config>::RuntimeCall:
		IsSubType<pallet_sylo_data_verification::Call<T>>,
	<T as pallet_futurepass::Config>::RuntimeCall: IsSubType<pallet_sylo_data_permissions::Call<T>>,
	<T as pallet_futurepass::Config>::RuntimeCall: IsSubType<pallet_utility::Call<T>>,
	<T as pallet_utility::Config>::RuntimeCall: IsSubType<pallet_evm::Call<T>>,
	<T as Config>::OnChargeTransaction: OnChargeTransaction<T>,
	<T as Config>::ErcIdConversion: ErcIdConversion<AssetId, EvmId = Address>,
	<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
	Balance: From<<<T as Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance>,
{
	type Balance = <<T as Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance;
	type LiquidityInfo =
		<<T as Config>::OnChargeTransaction as OnChargeTransaction<T>>::LiquidityInfo;

	/// Intercept the withdraw fee, and swap any tokens to gas tokens if the call is
	/// pallet_fee_proxy.call_with_fee_preferences().
	///
	/// This also additionally will force the Sylo token as the gas token if the call
	/// is detected as an extrinsic for the sylo pallet.
	fn withdraw_fee(
		who: &T::AccountId,
		call: &<T as frame_system::Config>::RuntimeCall,
		info: &DispatchInfoOf<<T as frame_system::Config>::RuntimeCall>,
		fee: Self::Balance,
		tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		let mut who = who;

		// if the call is pallet_futurepass::Call::proxy_extrinsic(), and the caller is a delegate
		// of the FP(futurepass), we switch the gas payer to the FP
		if let Some(pallet_futurepass::Call::proxy_extrinsic { futurepass, .. }) =
			call.is_sub_type()
		{
			if <T as pallet_futurepass::Config>::Proxy::exists(futurepass, who, None) {
				who = futurepass;
			}
		}

		// Attribute the fee to the partner if the caller is attributed to a partner
		attribute_fee_to_partner::<T>(who, fee.into())?;

		// Validate spending balance if the call is an action permissions transact call
		if let Some(grantor) = is_action_permission_execute_call::<T>(call) {
			// This will also update the spender of the transaction based on the
			// transact permission record.
			who = pallet_sylo_action_permissions::Pallet::<T>::validate_spending_balance(
				grantor,
				who,
				fee.into(),
			)
			.map_err(|_| InvalidTransaction::Payment)?;
		}

		let do_fee_swap = |who: &T::AccountId,
		                   payment_asset: &AssetId,
		                   mut total_fee: Balance,
		                   max_payment: Balance|
		 -> Result<(), TransactionValidityError> {
			let native_asset = <T as Config>::FeeAssetId::get();

			// If the account has less balance than the minimum_deposit, we need to add
			// the minimum deposit onto the total_fee.
			// This is due to the preservation rules of the withdraw call made within
			// <<T as Config>::OnChargeTransaction as OnChargeTransaction<T>>::withdraw_fee
			let account_balance = pallet_assets_ext::Pallet::<T>::balance(native_asset, who);
			// Minium balance is hardcoded to 1
			let minimum_balance = pallet_assets_ext::Pallet::<T>::minimum_balance(native_asset);
			if account_balance < minimum_balance {
				total_fee = total_fee.saturating_add(minimum_balance);
			}
			let path: &[AssetId] = &[*payment_asset, native_asset];
			pallet_dex::Pallet::<T>::do_swap_with_exact_target(
				who,
				total_fee,
				max_payment,
				path,
				*who,
				None,
			)
			.map_err(|_| InvalidTransaction::Payment)?;

			Ok(())
		};

		let is_sylo_and_valid_call = is_sylo_and_valid_call::<T>(call)?;

		// if the call is a sylo pallet call, then we always force a fee swap with the
		// sylo token
		if is_sylo_and_valid_call {
			let payment_asset = pallet_sylo_data_verification::SyloAssetId::<T>::get()
				.ok_or(InvalidTransaction::Payment)?;

			do_fee_swap(who, &payment_asset, Balance::from(fee), u128::MAX)?;
		}

		// Check whether this call has specified fee preferences
		if let Some(call_with_fee_preferences { payment_asset, max_payment, call }) =
			call.is_sub_type()
		{
			let mut total_fee: Balance = Balance::from(fee);

			let mut add_evm_gas_cost =
				|gas_limit: &u64,
				 max_fee_per_gas: &U256,
				 max_priority_fee_per_gas: &Option<U256>| {
					if let Ok(FeePreferencesData { max_fee_scaled, .. }) = get_fee_preferences_data::<
						T,
						<T as Config>::ErcIdConversion,
						pallet_futurepass::Pallet<T>,
					>(
						*gas_limit,
						<T as Config>::EVMBaseFeeProvider::evm_base_fee_per_gas(),
						Some(*max_fee_per_gas),
						*max_priority_fee_per_gas,
						*payment_asset,
					) {
						total_fee = total_fee.saturating_add(max_fee_scaled);
					}
				};

			// if the inner call is pallet_futurepass::Call::proxy_extrinsic(), and the caller is a
			// delegate of the FP(futurepass), we switch the gas payer to the FP
			if let Some(pallet_futurepass::Call::proxy_extrinsic { futurepass, call }) =
				call.is_sub_type()
			{
				if <T as pallet_futurepass::Config>::Proxy::exists(futurepass, who, None) {
					who = futurepass;
				}

				// if the inner call of the proxy_extrinsic is an evm call, we need to add extra gas
				// cost for that evm call
				if let Some(pallet_evm::Call::call {
					gas_limit,
					max_fee_per_gas,
					max_priority_fee_per_gas,
					..
				}) = call.is_sub_type()
				{
					add_evm_gas_cost(gas_limit, max_fee_per_gas, max_priority_fee_per_gas);
				}

				// Check if the inner call of the proxy_extrinsic is a batch call containing EVM transactions
				match call.is_sub_type() {
					Some(pallet_utility::Call::batch_all { calls, .. })
					| Some(pallet_utility::Call::batch { calls, .. }) => {
						for batch_call in calls {
							if let Some(pallet_evm::Call::call {
								gas_limit,
								max_fee_per_gas,
								max_priority_fee_per_gas,
								..
							}) = batch_call.is_sub_type()
							{
								add_evm_gas_cost(
									gas_limit,
									max_fee_per_gas,
									max_priority_fee_per_gas,
								);
							}
						}
					},
					_ => {},
				}
			}

			// Check if the inner call of the call_with_fee_preferences is an evm call. This will
			// increase total gas to swap This is required as the fee value here does not take into
			// account the max fee from an evm call. For all other extrinsics, the fee parameter
			// should cover all required fees.
			if let Some(pallet_evm::Call::call {
				gas_limit,
				max_fee_per_gas,
				max_priority_fee_per_gas,
				..
			}) = call.is_sub_type()
			{
				add_evm_gas_cost(gas_limit, max_fee_per_gas, max_priority_fee_per_gas);
			}

			// Check if the inner call is a batch call containing EVM transactions
			match call.is_sub_type() {
				Some(pallet_utility::Call::batch_all { calls, .. })
				| Some(pallet_utility::Call::batch { calls, .. }) => {
					for batch_call in calls {
						if let Some(pallet_evm::Call::call {
							gas_limit,
							max_fee_per_gas,
							max_priority_fee_per_gas,
							..
						}) = batch_call.is_sub_type()
						{
							add_evm_gas_cost(gas_limit, max_fee_per_gas, max_priority_fee_per_gas);
						}
					}
				},
				_ => {},
			}

			do_fee_swap(who, payment_asset, total_fee, *max_payment)?;
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
		// NOTE - ideally we should check and switch the account to FP here also, But we don't have
		// the call information within this function. What this means, if any extra fee was charged,
		// that fee wont return to FP but the caller. Ideally we could pass the required info via
		// pre, But this requires a new signed extension and some research.
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

/// Helper function to attribute a fee to a partner account if caller
/// is attributed to a partner and the partner has a fee percentage.
fn attribute_fee_to_partner<T>(
	who: &T::AccountId,
	fee: Balance,
) -> Result<(), TransactionValidityError>
where
	T: Config + pallet_partner_attribution::Config,
{
	if let Some(partner_id) = pallet_partner_attribution::Attributions::<T>::get(who) {
		pallet_partner_attribution::Partners::<T>::try_mutate(partner_id, |maybe_partner| {
			if let Some(partner) = maybe_partner {
				if partner.fee_percentage.is_some() {
					partner.accumulated_fees = partner.accumulated_fees.saturating_add(fee);
				}
			}
			Ok::<(), TransactionValidityError>(())
		})?;
	}
	Ok(())
}

/// Helper function to determine if a call is a sylo pallet call that
/// should be paid using sylo tokens. This function will also attempt to destructure
/// any proxy calls and check the inner call. This includes:
///   - pallet_futurepass.proxy_extrinsic
///   - pallet_xrpl.transact
///   - pallet_proxy.proxy
///   - pallet_proxy.proxy_announce
///   - pallet_utility.batch
///   - pallet_utility.batch_all
///   - pallet_utility.force_batch
///
/// Not all proxy calls are supported, such as some sudo calls, or scheduled
/// calls. In these edge cases, the fee for the call will be paid in the native
/// fee token.
///
/// This will also return an error if the call is an invalid sylo call. A sylo call
/// can be invalid in the following cases:
///   - The sylo call has been wrapped in a call_with_fee_preferences call. Sylo
///     calls should be paid in Sylos only.
///   - The sylo call is in a batch/batch_all call. In batch calls, if any call is
///     a sylo call, then all inner calls must be a sylo call. This simplifies the
///     implementation, preventing a need to process the fee for each individual call.
fn is_sylo_and_valid_call<T>(
	call: &<T as frame_system::Config>::RuntimeCall,
) -> Result<bool, TransactionValidityError>
where
	T: Config
		+ frame_system::Config<AccountId = AccountId>
		+ pallet_futurepass::Config
		+ pallet_xrpl::Config
		+ pallet_proxy::Config
		+ pallet_utility::Config
		+ pallet_sylo_data_verification::Config
		+ pallet_sylo_data_permissions::Config
		+ pallet_sylo_action_permissions::Config,
	<T as frame_system::Config>::RuntimeCall: IsSubType<crate::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_futurepass::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_sylo_data_verification::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_sylo_data_permissions::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_sylo_action_permissions::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_proxy::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_utility::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_xrpl::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_futurepass::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_xrpl::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_sylo_data_verification::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_sylo_data_permissions::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_sylo_action_permissions::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_utility::Call<T>>,
	<T as pallet_futurepass::Config>::RuntimeCall:
		IsSubType<pallet_sylo_data_verification::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
{
	if match call.is_sub_type() {
		Some(pallet_sylo_data_verification::Call::register_resolver { .. }) => true,
		Some(pallet_sylo_data_verification::Call::update_resolver { .. }) => true,
		Some(pallet_sylo_data_verification::Call::deregister_resolver { .. }) => true,
		Some(pallet_sylo_data_verification::Call::create_validation_record { .. }) => true,
		Some(pallet_sylo_data_verification::Call::add_validation_record_entry { .. }) => true,
		Some(pallet_sylo_data_verification::Call::update_validation_record { .. }) => true,
		Some(pallet_sylo_data_verification::Call::delete_validation_record { .. }) => true,
		_ => false,
	} {
		return Ok(true);
	}

	if match call.is_sub_type() {
		Some(pallet_sylo_data_permissions::Call::grant_data_permissions { .. }) => true,
		Some(pallet_sylo_data_permissions::Call::revoke_data_permission { .. }) => true,
		Some(pallet_sylo_data_permissions::Call::grant_tagged_permissions { .. }) => true,
		Some(pallet_sylo_data_permissions::Call::revoke_tagged_permission { .. }) => true,
		Some(pallet_sylo_data_permissions::Call::grant_permission_reference { .. }) => true,
		Some(pallet_sylo_data_permissions::Call::revoke_permission_reference { .. }) => true,
		_ => false,
	} {
		return Ok(true);
	}

	if match call.is_sub_type() {
		Some(pallet_sylo_action_permissions::Call::grant_transact_permission { .. }) => true,
		Some(pallet_sylo_action_permissions::Call::revoke_transact_permission { .. }) => true,
		Some(pallet_sylo_action_permissions::Call::update_transact_permission { .. }) => true,
		Some(pallet_sylo_action_permissions::Call::accept_transact_permission { .. }) => true,
		_ => false,
	} {
		return Ok(true);
	}

	// check if the inner call of a futurepass call is a sylo call
	if let Some(pallet_futurepass::Call::proxy_extrinsic { call, .. }) = call.is_sub_type() {
		return is_sylo_and_valid_call::<T>(call.as_ref().into_ref());
	}

	// check if the inner call of a proxy pallet call is a sylo call
	match call.is_sub_type() {
		Some(pallet_proxy::Call::proxy { call, .. }) => {
			return is_sylo_and_valid_call::<T>(call.as_ref().into_ref())
		},
		Some(pallet_proxy::Call::proxy_announced { call, .. }) => {
			return is_sylo_and_valid_call::<T>(call.as_ref().into_ref())
		},
		_ => Ok::<bool, TransactionValidityError>(false),
	}?;

	// check if the inner call of a xrpl call is a sylo call
	if let Some(pallet_xrpl::Call::transact { call, .. }) = call.is_sub_type() {
		return is_sylo_and_valid_call::<T>(call.as_ref().into_ref());
	}

	match call.is_sub_type() {
		// for batch calls, if there is any call which is a sylo call, then
		// all calls must be a sylo call
		Some(pallet_utility::Call::batch { calls, .. })
		| Some(pallet_utility::Call::force_batch { calls, .. })
		| Some(pallet_utility::Call::batch_all { calls, .. }) => {
			let sylo_calls = calls
				.into_iter()
				.map(|call| is_sylo_and_valid_call::<T>(call.into_ref()))
				.collect::<Vec<Result<_, _>>>()
				.into_iter()
				.collect::<Result<Vec<_>, _>>()?;

			if sylo_calls.iter().any(|x| *x) {
				if !sylo_calls.iter().all(|x| *x) {
					Err(InvalidTransaction::Payment)?;
				} else {
					return Ok(true);
				}
			}

			Ok::<bool, TransactionValidityError>(false)
		},
		Some(pallet_utility::Call::as_derivative { call, .. }) => {
			return is_sylo_and_valid_call::<T>(call.as_ref().into_ref())
		},
		_ => Ok(false),
	}?;

	// prevent using the fee proxy if the inner call is a sylo call
	if let Some(call_with_fee_preferences { call, .. }) = call.is_sub_type() {
		let is_sylo_call = is_sylo_and_valid_call::<T>(call.as_ref().into_ref())?;
		if is_sylo_call {
			Err(InvalidTransaction::Payment)?;
		}
	}

	Ok(false)
}

/// Helper function to determine if a call is an action permissions execute
/// call. This is needed as the transaction will be paid by either the
/// grantor or the grantee, depending on the permission record. This function
/// will attempt to unwrap any proxy-like calls and check the inner call.
///
/// Returns the grantor if the call is an action permission execute call,
fn is_action_permission_execute_call<T>(
	call: &<T as frame_system::Config>::RuntimeCall,
) -> Option<&AccountId>
where
	T: Config
		+ frame_system::Config<AccountId = AccountId>
		+ pallet_futurepass::Config
		+ pallet_xrpl::Config
		+ pallet_proxy::Config
		+ pallet_utility::Config
		+ pallet_sylo_data_verification::Config
		+ pallet_sylo_data_permissions::Config
		+ pallet_sylo_action_permissions::Config,
	<T as frame_system::Config>::RuntimeCall: IsSubType<crate::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_futurepass::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_sylo_data_verification::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_sylo_data_permissions::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_sylo_action_permissions::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_proxy::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_utility::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_xrpl::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_futurepass::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_xrpl::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_sylo_data_verification::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_sylo_data_permissions::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_sylo_action_permissions::Call<T>>,
	<T as Config>::RuntimeCall: IsSubType<pallet_utility::Call<T>>,
	<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
	<T as pallet_futurepass::Config>::RuntimeCall:
		IsSubType<pallet_sylo_data_verification::Call<T>>,
{
	if let Some(pallet_sylo_action_permissions::Call::transact { grantor, .. }) = call.is_sub_type()
	{
		return Some(grantor);
	}

	// check if the inner call of a futurepass call is a sylo call
	if let Some(pallet_futurepass::Call::proxy_extrinsic { call, .. }) = call.is_sub_type() {
		return is_action_permission_execute_call::<T>(call.as_ref().into_ref());
	}

	// check if the inner call of a xrpl call is a sylo call
	if let Some(pallet_xrpl::Call::transact { call, .. }) = call.is_sub_type() {
		return is_action_permission_execute_call::<T>(call.as_ref().into_ref());
	}

	// check if the inner call of a proxy pallet call is a sylo call
	match call.is_sub_type() {
		Some(pallet_proxy::Call::proxy { call, .. }) => {
			return is_action_permission_execute_call::<T>(call.as_ref().into_ref());
		},
		Some(pallet_proxy::Call::proxy_announced { call, .. }) => {
			return is_action_permission_execute_call::<T>(call.as_ref().into_ref());
		},
		_ => {},
	};

	// check if the inner call is a fee proxy call
	if let Some(call_with_fee_preferences { call, .. }) = call.is_sub_type() {
		return is_action_permission_execute_call::<T>(call.as_ref().into_ref());
	}

	return None;
}
