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

use crate::Config;
use ethabi::{ParamType, Token};
use frame_support::{
	dispatch::Weight, ensure, traits::fungibles::metadata::Inspect as InspectMetadata,
};
use pallet_evm::{
	runner::stack::Runner, AddressMapping, CallInfo, CreateInfo, EvmConfig, FeeCalculator,
	Runner as RunnerT, RunnerError,
};
#[allow(deprecated)]
use precompile_utils::{
	constants::{
		ERC20_PRECOMPILE_ADDRESS_PREFIX, FEE_FUNCTION_SELECTOR, FEE_FUNCTION_SELECTOR_DEPRECATED,
		FEE_PROXY_ADDRESS,
	},
	Address as EthAddress, ErcIdConversion,
};
use seed_pallet_common::{
	log, utils::scale_wei_to_correct_decimals, AccountProxy, FeeConfig, MaintenanceCheckEVM,
};
use seed_primitives::{AccountId, AssetId};
use sp_core::{H160, H256, U256};
use sp_runtime::{
	traits::Get,
	transaction_validity::{InvalidTransaction, TransactionValidityError},
};
use sp_std::{marker::PhantomData, prelude::*};

#[derive(Debug, Eq, PartialEq)]
pub enum FeePreferencesError {
	InvalidFunctionSelector,
	WithdrawFailed,
	GasPriceTooLow,
	FeeOverflow,
	InvalidInputArguments,
	FailedToDecodeInput,
	InvalidPaymentAsset,
	FeeExceedsMaxPayment,
}

impl<T> From<FeePreferencesError> for pallet_evm::Error<T> {
	fn from(value: FeePreferencesError) -> pallet_evm::Error<T> {
		match value {
			FeePreferencesError::WithdrawFailed => pallet_evm::Error::WithdrawFailed,
			FeePreferencesError::GasPriceTooLow => pallet_evm::Error::GasPriceTooLow,
			FeePreferencesError::FeeOverflow => pallet_evm::Error::FeeOverflow,
			_ => pallet_evm::Error::WithdrawFailed,
		}
	}
}

impl From<FeePreferencesError> for TransactionValidityError {
	fn from(error: FeePreferencesError) -> Self {
		match error {
			// Errors related to improperly designating a call or something "call-like" should all
			// return an invalid call error
			FeePreferencesError::InvalidFunctionSelector
			| FeePreferencesError::InvalidInputArguments
			| FeePreferencesError::FailedToDecodeInput
			| FeePreferencesError::InvalidPaymentAsset => {
				TransactionValidityError::Invalid(InvalidTransaction::Call)
			},
			FeePreferencesError::WithdrawFailed
			| FeePreferencesError::GasPriceTooLow
			| FeePreferencesError::FeeOverflow
			| FeePreferencesError::FeeExceedsMaxPayment => {
				TransactionValidityError::Invalid(InvalidTransaction::Payment)
			},
		}
	}
}

// Any data needed for computing fee preferences
#[derive(Debug, PartialEq)]
pub struct FeePreferencesData {
	pub path: Vec<u32>,
	pub total_fee_scaled: u128,
	pub max_fee_scaled: u128,
}

pub fn get_fee_preferences_data<T, U, P>(
	gas_limit: u64,
	base_fee_per_gas: U256,
	max_fee_per_gas: Option<U256>,
	max_priority_fee_per_gas: Option<U256>,
	payment_asset_id: u32,
) -> Result<FeePreferencesData, FeePreferencesError>
where
	T: pallet_evm::Config<AccountId = AccountId> + pallet_assets_ext::Config + Config,
	U: ErcIdConversion<AssetId, EvmId = EthAddress>,
	P: AccountProxy<AccountId>,
{
	let (total_fee, max_fee) = FeePreferencesRunner::<T, U, P>::calculate_total_gas(
		gas_limit,
		base_fee_per_gas,
		max_fee_per_gas,
		max_priority_fee_per_gas,
	)?;

	let gas_token_asset_id = <T as Config>::FeeAssetId::get();
	let path = vec![payment_asset_id, gas_token_asset_id];
	// Convert EVM wei fees to runtime Balance units using the native gas token decimals (e.g. 6 for XRP)
	let decimals =
		<pallet_assets_ext::Pallet<T> as InspectMetadata<AccountId>>::decimals(gas_token_asset_id);
	let total_fee_scaled = scale_wei_to_correct_decimals(total_fee, decimals);
	let max_fee_scaled = scale_wei_to_correct_decimals(max_fee, decimals);
	Ok(FeePreferencesData { total_fee_scaled, max_fee_scaled, path })
}

/// seed implementation of the evm runner which handles the case where users are attempting
/// to set their payment asset. In this case, we will exchange their desired asset into gas
/// token (XRP) to complete the transaction
pub struct FeePreferencesRunner<T, U, P: AccountProxy<AccountId>> {
	_proxy: P,
	_phantom: PhantomData<(T, U)>,
}

#[allow(deprecated)]
impl<T, U, P> FeePreferencesRunner<T, U, P>
where
	T: pallet_evm::Config<AccountId = AccountId>,
	U: ErcIdConversion<AssetId, EvmId = EthAddress>,
	P: AccountProxy<AccountId>,
{
	/// Decodes the input for call_with_fee_preferences
	pub fn decode_input(input: Vec<u8>) -> Result<(AssetId, H160, Vec<u8>), FeePreferencesError> {
		ensure!(input.len() >= 4, FeePreferencesError::InvalidInputArguments);
		ensure!(
			input[..4] == FEE_FUNCTION_SELECTOR_DEPRECATED || input[..4] == FEE_FUNCTION_SELECTOR,
			FeePreferencesError::InvalidFunctionSelector,
		);

		if input[..4] == FEE_FUNCTION_SELECTOR_DEPRECATED {
			log!(
				warn,
				"⚠️ using deprecated fee function selector: call_with_fee_preferences(address,uint128,address,bytes)"
			);
			let types =
				[ParamType::Address, ParamType::Uint(128), ParamType::Address, ParamType::Bytes];
			let tokens = ethabi::decode(&types, &input[4..])
				.map_err(|_| FeePreferencesError::FailedToDecodeInput)?;
			if let [Token::Address(payment_asset_address), Token::Uint(_max_payment), Token::Address(new_target), Token::Bytes(new_input)] =
				tokens.as_slice()
			{
				let Some(payment_asset) = U::evm_id_to_runtime_id(
					(*payment_asset_address).into(),
					ERC20_PRECOMPILE_ADDRESS_PREFIX,
				) else {
					return Err(FeePreferencesError::InvalidPaymentAsset);
				};
				Ok((payment_asset, (*new_target), new_input.clone()))
			} else {
				Err(FeePreferencesError::InvalidInputArguments)?
			}
		} else {
			let types = [ParamType::Address, ParamType::Address, ParamType::Bytes];
			let tokens = ethabi::decode(&types, &input[4..])
				.map_err(|_| FeePreferencesError::FailedToDecodeInput)?;
			if let [Token::Address(payment_asset_address), Token::Address(new_target), Token::Bytes(new_input)] =
				tokens.as_slice()
			{
				let Some(payment_asset) = U::evm_id_to_runtime_id(
					(*payment_asset_address).into(),
					ERC20_PRECOMPILE_ADDRESS_PREFIX,
				) else {
					return Err(FeePreferencesError::InvalidPaymentAsset);
				};
				Ok((payment_asset, (*new_target), new_input.clone()))
			} else {
				Err(FeePreferencesError::InvalidInputArguments)?
			}
		}
	}

	// Calculate gas price for transaction to use for exchanging asset into gas-token currency using
	// eip1559
	pub fn calculate_total_gas(
		gas_limit: u64,
		base_fee_per_gas: U256,
		max_fee_per_gas: Option<U256>,
		max_priority_fee_per_gas: Option<U256>,
	) -> Result<(U256, U256), FeePreferencesError> {
		// fee = gas_limit * (base_fee_per_gas + max_priority_fee_per_gas)
		let total_fee = U256::from(gas_limit)
			.checked_mul(
				base_fee_per_gas
					.checked_add(max_priority_fee_per_gas.unwrap_or_default())
					.ok_or(FeePreferencesError::FeeOverflow)?,
			)
			.ok_or(FeePreferencesError::FeeOverflow)?;

		// max_fee = gas_limit * ((2 * base_fee_per_gas) + max_priority_fee_per_gas)
		let max_fee = max_fee_per_gas
			.unwrap_or(
				base_fee_per_gas
					.checked_mul(U256::from(2))
					.ok_or(FeePreferencesError::FeeOverflow)?
					.checked_add(max_priority_fee_per_gas.unwrap_or_default())
					.ok_or(FeePreferencesError::FeeOverflow)?,
			)
			.checked_mul(U256::from(gas_limit))
			.ok_or(FeePreferencesError::FeeOverflow)?;

		ensure!(total_fee <= max_fee, FeePreferencesError::FeeExceedsMaxPayment);

		Ok((total_fee, max_fee))
	}
}

impl<T, U, P> RunnerT<T> for FeePreferencesRunner<T, U, P>
where
	T: pallet_evm::Config<AccountId = AccountId>
		+ pallet_assets_ext::Config
		+ pallet_dex::Config
		+ Config,
	U: ErcIdConversion<AssetId, EvmId = EthAddress>,
	pallet_evm::BalanceOf<T>: TryFrom<U256> + Into<U256>,
	P: AccountProxy<AccountId>,
{
	type Error = pallet_evm::Error<T>;

	fn validate(
		source: H160,
		target: Option<H160>,
		input: Vec<u8>,
		value: U256,
		gas_limit: u64,
		max_fee_per_gas: Option<U256>,
		max_priority_fee_per_gas: Option<U256>,
		nonce: Option<U256>,
		access_list: Vec<(H160, Vec<H256>)>,
		is_transactional: bool,
		weight_limit: Option<Weight>,
		proof_size_base_cost: Option<u64>,
		evm_config: &EvmConfig,
	) -> Result<(), RunnerError<Self::Error>> {
		<Runner<T> as RunnerT<T>>::validate(
			source,
			target,
			input,
			value,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			nonce,
			access_list,
			is_transactional,
			weight_limit,
			proof_size_base_cost,
			evm_config,
		)
	}

	fn call(
		source: H160,
		target: H160,
		input: Vec<u8>,
		value: U256,
		gas_limit: u64,
		max_fee_per_gas: Option<U256>,
		max_priority_fee_per_gas: Option<U256>,
		nonce: Option<U256>,
		access_list: Vec<(H160, Vec<H256>)>,
		is_transactional: bool,
		validate: bool,
		weight_limit: Option<Weight>,
		proof_size_base_cost: Option<u64>,
		config: &EvmConfig,
	) -> Result<CallInfo, RunnerError<Self::Error>> {
		// Futurepass v2 code, should not have any impact
		let mut source = source;
		if let Some(futurepass) = P::primary_proxy(&source.into()) {
			source = futurepass.into();
		}

		// Futurepass v2 code, should not have any impact
		let mut target = target;
		if let Some(futurepass) = P::primary_proxy(&target.into()) {
			target = futurepass.into();
		}

		// Verify that the chain is not in maintenance mode,
		// the signer account is not blocked,
		// And the target address is not blocked
		let account = <T as pallet_evm::Config>::AddressMapping::into_account_id(source);
		if !<T as Config>::MaintenanceChecker::validate_evm_call(&account, &target) {
			return Err(RunnerError {
				error: Self::Error::WithdrawFailed,
				weight: Weight::default(),
			});
		}

		// These values may change if we are using the fee_preferences precompile
		let mut input = input;

		// Check if we are calling with fee preferences
		if target == H160::from_low_u64_be(FEE_PROXY_ADDRESS) {
			let (_, weight) = T::FeeCalculator::min_gas_price();

			let (payment_asset_id, new_target, new_input) = Self::decode_input(input)
				.map_err(|err| RunnerError { error: err.into(), weight })?;

			// set input and target to new input and actual target for passthrough
			input = new_input;
			target = new_target;

			let base_fee_per_gas = <T as Config>::EVMBaseFeeProvider::evm_base_fee_per_gas();
			let FeePreferencesData { path, total_fee_scaled, max_fee_scaled } =
				get_fee_preferences_data::<T, U, P>(
					gas_limit,
					base_fee_per_gas,
					max_fee_per_gas,
					max_priority_fee_per_gas,
					payment_asset_id,
				)
				.map_err(|_| RunnerError { error: Self::Error::FeeOverflow, weight })?;

			let max_payment_tokens = {
				let amounts_in =
					pallet_dex::Pallet::<T>::get_amounts_in(max_fee_scaled, &path) // [token, xrp]
						.map_err(|_| RunnerError { error: Self::Error::Undefined, weight })?;
				amounts_in[0]
			};

			let final_fee = {
				// account for rounding up for XRP below 1 drip when scaling fees
				if (max_fee_scaled - total_fee_scaled) == 1 {
					max_fee_scaled
				} else {
					total_fee_scaled
				}
			};

			let account = <T as pallet_evm::Config>::AddressMapping::into_account_id(source);

			pallet_dex::Pallet::<T>::do_swap_with_exact_supply(
				&account,
				max_payment_tokens,
				final_fee,
				&path,
				account,
				None,
			)
			.map_err(|err| {
				log!(error, "⛽️ swap failed payment_asset_id={:?} supply={} desired_fee={} err={:?} path={:?}", payment_asset_id, max_payment_tokens, final_fee, err, path);
				RunnerError { error: Self::Error::WithdrawFailed, weight }
			})?;
		}

		// continue with the call - with fees payable in gas asset currency - via dex swap
		<Runner<T> as RunnerT<T>>::call(
			source,
			target,
			input,
			value,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			nonce,
			access_list,
			is_transactional,
			validate,
			weight_limit,
			proof_size_base_cost,
			config,
		)
	}

	fn create(
		source: H160,
		init: Vec<u8>,
		value: U256,
		gas_limit: u64,
		max_fee_per_gas: Option<U256>,
		max_priority_fee_per_gas: Option<U256>,
		nonce: Option<U256>,
		access_list: Vec<(H160, Vec<H256>)>,
		is_transactional: bool,
		validate: bool,
		weight_limit: Option<Weight>,
		proof_size_base_cost: Option<u64>,
		config: &EvmConfig,
	) -> Result<CreateInfo, RunnerError<Self::Error>> {
		// @todo check source, proxy request if needed

		// Verify that the chain is not in maintenance mode, and the signer account is not blocked
		let account = <T as pallet_evm::Config>::AddressMapping::into_account_id(source);
		if !<T as Config>::MaintenanceChecker::validate_evm_create(&account) {
			return Err(RunnerError {
				error: Self::Error::WithdrawFailed,
				weight: Weight::default(),
			});
		}

		<Runner<T> as RunnerT<T>>::create(
			source,
			init,
			value,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			nonce,
			access_list,
			is_transactional,
			validate,
			weight_limit,
			proof_size_base_cost,
			config,
		)
	}

	fn create2(
		source: H160,
		init: Vec<u8>,
		salt: H256,
		value: U256,
		gas_limit: u64,
		max_fee_per_gas: Option<U256>,
		max_priority_fee_per_gas: Option<U256>,
		nonce: Option<U256>,
		access_list: Vec<(H160, Vec<H256>)>,
		is_transactional: bool,
		validate: bool,
		weight_limit: Option<Weight>,
		proof_size_base_cost: Option<u64>,
		config: &EvmConfig,
	) -> Result<CreateInfo, RunnerError<Self::Error>> {
		// @todo check source, proxy request if needed

		// Verify that the chain is not in maintenance mode, and the signer account is not blocked
		let account = <T as pallet_evm::Config>::AddressMapping::into_account_id(source);
		if !<T as Config>::MaintenanceChecker::validate_evm_create(&account) {
			return Err(RunnerError {
				error: Self::Error::WithdrawFailed,
				weight: Weight::default(),
			});
		}

		<Runner<T> as RunnerT<T>>::create2(
			source,
			init,
			salt,
			value,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			nonce,
			access_list,
			is_transactional,
			validate,
			weight_limit,
			proof_size_base_cost,
			config,
		)
	}
}
