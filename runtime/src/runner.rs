use crate::Dex;
use ethabi::{ParamType, Token};
use frame_support::{ensure, traits::fungibles::InspectMetadata};
use pallet_evm::{
	runner::stack::Runner, AddressMapping, CallInfo, CreateInfo, EvmConfig, FeeCalculator,
	Runner as RunnerT, RunnerError,
};
use precompile_utils::{
	constants::ERC20_PRECOMPILE_ADDRESS_PREFIX, Address as EthAddress, ErcIdConversion,
};
// use primitive_types::{H160, H256, U256}; // TODO: use this instead of seed_pallet_common imports
use primitive_types::{H160, H256, U256};
use seed_pallet_common::log;
use seed_primitives::{AccountId, AssetId, Balance};
use sp_runtime::traits::SaturatedConversion;
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
	UnknownError,
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

/// TODO - migrate these to precompile-utils/constants?
/// Function selector for call_with_fee_preferences
/// bytes4(keccak256(bytes("callWithFeePreferences(address,uint128,address,bytes)")));
/// TODO - use #[precompile_utils::generate_function_selector]
const FEE_FUNCTION_SELECTOR: [u8; 4] = [0x25, 0x5a, 0x34, 0x32];
/// Precompile address for fee preferences
const FEE_PROXY_ADDRESS: u64 = 1211; // 0x04BB = 00000100 10111011

/// Convert 18dp wei values to correct dp equivalents
/// fractional amounts < `CPAY_UNIT_VALUE` are rounded up by adding 1 / 0.000001 cpay
fn scale_wei_to_correct_decimals(value: U256, decimals: u8) -> u128 {
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

/// seed implementation of the evm runner which handles the case where users are attempting
/// to set their payment asset. In this case, we will exchange their desired asset into gas
/// token (XRP) to complete the transaction
pub struct FeePreferencesRunner<T, U>(PhantomData<(T, U)>);

impl<T, U> FeePreferencesRunner<T, U>
where
	T: pallet_evm::Config<AccountId = AccountId>,
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
			ensure!(payment_asset.is_some(), FeePreferencesError::InvalidPaymentAsset);

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
	T: pallet_evm::Config<AccountId = AccountId>,
	T: pallet_assets_ext::Config,
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
		let mut gas_limit = gas_limit;

		// Check if we are calling with fee preferences
		if target == H160::from_low_u64_be(FEE_PROXY_ADDRESS) {
			gas_limit = if gas_limit == 0 { 150_000_000_u64 } else { gas_limit };
			let (_, weight) = T::FeeCalculator::min_gas_price();

			let (payment_asset_id, max_payment, new_target, new_input) = Self::decode_input(input)
				.map_err(|err| RunnerError { error: err.into(), weight })?;

			// set input and target to new input and actual target for passthrough
			input = new_input;
			target = new_target;

			let total_fee = Self::calculate_total_gas(gas_limit, max_fee_per_gas, is_transactional)
				.map_err(|err| RunnerError { error: err.into(), weight })?;

			let gas_token_asset_id = crate::constants::XRP_ASSET_ID;
			let decimals = <pallet_assets_ext::Pallet<T> as InspectMetadata<AccountId>>::decimals(
				&gas_token_asset_id,
			);
			let total_fee_scaled = scale_wei_to_correct_decimals(total_fee, decimals);

			// Buy the gas asset fee currency paying with the user's nominated token
			let account = <T as pallet_evm::Config>::AddressMapping::into_account_id(source);
			let path = vec![payment_asset_id, gas_token_asset_id];

			if total_fee_scaled > 0 {
				// total_fee_scaled is 0 when user doesnt have gas asset currency
				Dex::do_swap_with_exact_target(&account, total_fee_scaled, max_payment, &path)
					.map_err(|err| {
						// TODO implement err into RunnerError
						log!(
							debug,
							"⛽️ swapping {:?} (max {:?} units) for fee {:?} units failed: {:?}",
							payment_asset_id,
							max_payment,
							total_fee_scaled,
							err
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{BaseFee, Runtime};
	use frame_support::{assert_noop, assert_ok};
	use hex_literal::hex;

	/// type alias for runtime configured FeePreferencesRunner
	type Runner = FeePreferencesRunner<Runtime, Runtime>;

	#[test]
	fn decode_input() {
		sp_io::TestExternalities::new_empty().execute_with(|| {
      // Abi generated from below parameters using the following function name:
      // callWithFeePreferences
      // abi can be easily generated here https://abi.hashex.org/
      let exp_payment_asset = 16000_u32;
      let exp_max_payment = 123_456_789 as Balance;
      let exp_target = H160::from_slice(&hex!("cCccccCc00003E80000000000000000000000000"));
      let exp_input: Vec<u8> =
        hex!("a9059cbb0000000000000000000000007a107fc1794f505cb351148f529accae12ffbcd8000000000000000000000000000000000000000000000000000000000000007b"
).to_vec();
      let mut input = FEE_FUNCTION_SELECTOR.to_vec();
      input.append(&mut ethabi::encode(&[
        Token::Address(Runtime::runtime_id_to_evm_id(exp_payment_asset, ERC20_PRECOMPILE_ADDRESS_PREFIX).0),
        Token::Uint(exp_max_payment.into()),
        Token::Address(exp_target),
        Token::Bytes(exp_input.clone())],
      ));

      assert_eq!(
        Runner::decode_input(input),
        Ok((exp_payment_asset, exp_max_payment, exp_target, exp_input))
      );
    });
	}

	#[test]
	fn decode_input_invalid_function_selector_should_fail() {
		sp_io::TestExternalities::new_empty().execute_with(|| {
			let bad_selector_input = vec![0x01, 0x02, 0x03, 0x04];
			assert_noop!(
				Runner::decode_input(bad_selector_input),
				FeePreferencesError::InvalidFunctionSelector
			);
		});
	}

	#[test]
	fn decode_input_empty_input_should_fail() {
		sp_io::TestExternalities::new_empty().execute_with(|| {
			assert_noop!(
				Runner::decode_input(Default::default()),
				FeePreferencesError::InvalidInputArguments
			);
		});
	}

	#[test]
	fn decode_input_invalid_input_args_should_fail() {
		sp_io::TestExternalities::new_empty().execute_with(|| {
			let mut input = FEE_FUNCTION_SELECTOR.to_vec();
			input.append(&mut ethabi::encode(&[
				Token::Bytes(vec![1_u8, 2, 3, 4, 5]),
				Token::Array(vec![
					Token::Uint(1u64.into()),
					Token::Uint(2u64.into()),
					Token::Uint(3u64.into()),
					Token::Uint(4u64.into()),
					Token::Uint(5u64.into()),
				]),
			]));

			assert_noop!(Runner::decode_input(input), FeePreferencesError::FailedToDecodeInput);
		});
	}

	#[test]
	fn decode_input_zero_payment_asset_should_fail() {
		sp_io::TestExternalities::new_empty().execute_with(|| {
			let mut input = FEE_FUNCTION_SELECTOR.to_vec();
			input.append(&mut ethabi::encode(&[
				Token::Address(H160::zero()),
				Token::Uint(5u64.into()),
				Token::Address(H160::default()),
				Token::Bytes(vec![1_u8, 2, 3, 4, 5]),
			]));

			assert_noop!(
				Runner::decode_input(input.to_vec()),
				FeePreferencesError::InvalidPaymentAsset
			);
		});
	}

	#[test]
	fn calculate_total_gas() {
		sp_io::TestExternalities::new_empty().execute_with(|| {
			let gas_limit: u64 = 100000;
			let max_fee_per_gas = U256::from(20000000000000u64);
			let max_priority_fee_per_gas = U256::from(1000000u64);
			let (base_fee, _weight) = BaseFee::min_gas_price();

			assert_ok!(Runner::calculate_total_gas(gas_limit, Some(max_fee_per_gas), false));
		});
	}

	#[test]
	fn calculate_total_gas_low_max_fee_should_fail() {
		sp_io::TestExternalities::new_empty().execute_with(|| {
			let gas_limit = 100_000_u64;
			let (base_fee, _weight) = BaseFee::min_gas_price();

			assert_noop!(
				Runner::calculate_total_gas(
					gas_limit,
					Some(base_fee.saturating_sub(1_u64.into())),
					false,
				),
				FeePreferencesError::GasPriceTooLow
			);
		});
	}

	#[test]
	fn calculate_total_gas_no_max_fee_ok() {
		sp_io::TestExternalities::new_empty().execute_with(|| {
			let gas_limit = 100_000_u64;
			let max_fee_per_gas = None;
			let max_priority_fee_per_gas = U256::from(1_000_000_u64);
			let (base_fee, _weight) = BaseFee::min_gas_price();

			assert_ok!(Runner::calculate_total_gas(gas_limit, max_fee_per_gas, false));
		});
	}

	#[test]
	fn calculate_total_gas_max_priority_fee_too_large_should_fail() {
		sp_io::TestExternalities::new_empty().execute_with(|| {
			let gas_limit: u64 = 100000;
			let max_fee_per_gas = U256::from(20000000000000u64);
			let max_priority_fee_per_gas = U256::MAX;
			let (base_fee, _weight) = BaseFee::min_gas_price();

			assert_noop!(
				Runner::calculate_total_gas(gas_limit, Some(max_fee_per_gas), false),
				FeePreferencesError::FeeOverflow
			);
		});
	}

	#[test]
	fn calculate_total_gas_max_fee_too_large_should_fail() {
		sp_io::TestExternalities::new_empty().execute_with(|| {
			let gas_limit: u64 = 100000;
			let max_fee_per_gas = U256::MAX;
			let max_priority_fee_per_gas = U256::from(1000000u64);
			let (base_fee, _weight) = BaseFee::min_gas_price();

			assert_noop!(
				Runner::calculate_total_gas(gas_limit, Some(max_fee_per_gas), false),
				FeePreferencesError::FeeOverflow
			);
		});
	}
}
