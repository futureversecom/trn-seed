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

use crate::{AssetWhitelist, Config, Pallet};
use ethabi::{ParamType, Token};
use frame_support::{ensure, fail, traits::fungibles::InspectMetadata};
use pallet_evm::{
	runner::stack::Runner, AddressMapping, CallInfo, CreateInfo, EvmConfig, FeeCalculator,
	Runner as RunnerT, RunnerError,
};
use precompile_utils::{
	constants::{ERC20_PRECOMPILE_ADDRESS_PREFIX, FEE_FUNCTION_SELECTOR, FEE_PROXY_ADDRESS},
	Address as EthAddress, ErcIdConversion,
};
use seed_pallet_common::log;
use seed_primitives::{AccountId, AssetId, Balance};
use sp_core::{H160, H256, U256};
use sp_runtime::{
	traits::{Get, SaturatedConversion},
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

impl<T> Into<pallet_evm::Error<T>> for FeePreferencesError {
	fn into(self: Self) -> pallet_evm::Error<T> {
		match self {
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
			FeePreferencesError::InvalidFunctionSelector |
			FeePreferencesError::InvalidInputArguments |
			FeePreferencesError::FailedToDecodeInput |
			FeePreferencesError::InvalidPaymentAsset =>
				TransactionValidityError::Invalid(InvalidTransaction::Call),
			FeePreferencesError::WithdrawFailed |
			FeePreferencesError::GasPriceTooLow |
			FeePreferencesError::FeeOverflow |
			FeePreferencesError::FeeExceedsMaxPayment =>
				TransactionValidityError::Invalid(InvalidTransaction::Payment),
		}
	}
}

/// Convert 18dp wei values to correct dp equivalents
/// fractional amounts < `CPAY_UNIT_VALUE` are rounded up by adding 1 / 0.000001 cpay
pub(crate) fn scale_wei_to_correct_decimals(value: U256, decimals: u8) -> u128 {
	let unit_value = U256::from(10).pow(U256::from(18) - U256::from(decimals));
	let (quotient, remainder) = (value / unit_value, value % unit_value);
	if remainder == U256::from(0) {
		quotient.as_u128()
	} else {
		// if value has a fractional part < CPAY unit value
		// it is lost in this divide operation
		(quotient + 1).as_u128()
	}
}

// Any data needed for computing fee preferences
#[derive(Debug, PartialEq)]
pub struct FeePreferencesData {
	pub path: Vec<u32>,
	pub total_fee_scaled: u128,
}

pub fn get_fee_preferences_data<T, U>(
	gas_limit: u64,
	max_fee_per_gas: Option<U256>,
	payment_asset_id: u32,
) -> Result<FeePreferencesData, FeePreferencesError>
where
	T: pallet_evm::Config<AccountId = AccountId> + pallet_assets_ext::Config + Config,
	U: ErcIdConversion<AssetId, EvmId = EthAddress>,
{
	let total_fee =
		FeePreferencesRunner::<T, U>::calculate_total_gas(gas_limit, max_fee_per_gas, false)?;

	let gas_token_asset_id = <T as Config>::FeeAssetId::get();
	let decimals =
		<pallet_assets_ext::Pallet<T> as InspectMetadata<AccountId>>::decimals(&gas_token_asset_id);
	let total_fee_scaled = scale_wei_to_correct_decimals(total_fee, decimals);

	let path = vec![payment_asset_id, gas_token_asset_id];
	Ok(FeePreferencesData { total_fee_scaled, path })
}

/// seed implementation of the evm runner which handles the case where users are attempting
/// to set their payment asset. In this case, we will exchange their desired asset into gas
/// token (XRP) to complete the transaction
pub struct FeePreferencesRunner<T, U>(PhantomData<(T, U)>);

impl<T, U> FeePreferencesRunner<T, U>
where
	T: pallet_evm::Config<AccountId = AccountId> + Config,
	U: ErcIdConversion<AssetId, EvmId = EthAddress>,
{
	/// Decodes the input for call_with_fee_preferences
	pub fn decode_input(
		input: Vec<u8>,
	) -> Result<(AssetId, Balance, H160, Vec<u8>), FeePreferencesError> {
		ensure!(input.len() >= 4, FeePreferencesError::InvalidInputArguments);
		ensure!(input[..4] == FEE_FUNCTION_SELECTOR, FeePreferencesError::InvalidFunctionSelector);

		let types =
			[ParamType::Address, ParamType::Uint(128), ParamType::Address, ParamType::Bytes];
		let tokens = ethabi::decode(&types, &input[4..])
			.map_err(|_| FeePreferencesError::FailedToDecodeInput)?;

		if let [Token::Address(payment_asset_address), Token::Uint(max_payment), Token::Address(new_target), Token::Bytes(new_input)] =
			tokens.as_slice()
		{
			let payment_asset = U::evm_id_to_runtime_id(
				(*payment_asset_address).into(),
				ERC20_PRECOMPILE_ADDRESS_PREFIX,
			);

			if let Some(payment_asset) = payment_asset {
				ensure!(
					AssetWhitelist::<T>::get(payment_asset),
					FeePreferencesError::InvalidPaymentAsset
				);
			} else {
				fail!(FeePreferencesError::InvalidPaymentAsset);
			}

			Ok((
				payment_asset.unwrap(),
				(*max_payment).saturated_into::<Balance>(),
				(*new_target).into(),
				new_input.clone(),
			))
		} else {
			Err(FeePreferencesError::InvalidInputArguments)
		}
	}

	// Calculate gas price for transaction to use for exchanging asset into gas-token currency
	pub fn calculate_total_gas(
		gas_limit: u64,
		max_fee_per_gas: Option<U256>,
		is_transactional: bool,
	) -> Result<U256, FeePreferencesError> {
		let max_fee_per_gas = match (max_fee_per_gas, is_transactional) {
			(Some(max_fee_per_gas), _) => max_fee_per_gas,
			// Gas price check is skipped for non-transactional calls that don't
			// define a `max_fee_per_gas` input.
			(None, false) => Default::default(),
			// Unreachable, previously validated. Handle gracefully.
			_ => return Err(FeePreferencesError::FeeOverflow),
		};

		// After eip-1559 we make sure the account can pay both the evm execution and priority
		// fees.
		let total_fee = max_fee_per_gas
			.checked_mul(U256::from(gas_limit))
			.ok_or(FeePreferencesError::FeeOverflow)?;

		Ok(total_fee.into())
	}
}

impl<T, U> RunnerT<T> for FeePreferencesRunner<T, U>
where
	T: pallet_evm::Config<AccountId = AccountId>
		+ pallet_assets_ext::Config
		+ pallet_dex::Config
		+ Config,
	U: ErcIdConversion<AssetId, EvmId = EthAddress>,
	pallet_evm::BalanceOf<T>: TryFrom<U256> + Into<U256>,
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
		config: &EvmConfig,
	) -> Result<CallInfo, RunnerError<Self::Error>> {
		// These values may change if we are using the fee_preferences precompile
		let mut input = input;
		let mut target = target;

		// Check if we are calling with fee preferences
		if target == H160::from_low_u64_be(FEE_PROXY_ADDRESS) {
			let (_, weight) = T::FeeCalculator::min_gas_price();

			let (payment_asset_id, max_payment, new_target, new_input) = Self::decode_input(input)
				.map_err(|err| RunnerError { error: err.into(), weight })?;

			// set input and target to new input and actual target for passthrough
			input = new_input;
			target = new_target;

			let FeePreferencesData { path, total_fee_scaled } =
				get_fee_preferences_data::<T, U>(gas_limit, max_fee_per_gas, payment_asset_id)
					.map_err(|_| RunnerError { error: Self::Error::FeeOverflow, weight })?;

			let account =
				<T as pallet_evm::Config>::AddressMapping::into_account_id(source.clone());
			if total_fee_scaled > 0 {
				// total_fee_scaled is 0 when user doesnt have gas asset currency
				pallet_dex::Pallet::<T>::do_swap_with_exact_target(
					&account,
					total_fee_scaled,
					max_payment,
					&path,
				)
				.map_err(|err| {
					log!(
							error,
							"⛽️ swapping {:?} (max {:?} units) for fee {:?} units failed: {:?} path: {:?}",
							payment_asset_id,
							max_payment,
							total_fee_scaled,
							err,
							path
						);
					RunnerError { error: Self::Error::WithdrawFailed, weight }
				})?;
			}
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
		config: &EvmConfig,
	) -> Result<CreateInfo, RunnerError<Self::Error>> {
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
		config: &EvmConfig,
	) -> Result<CreateInfo, RunnerError<Self::Error>> {
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
			config,
		)
	}
}
