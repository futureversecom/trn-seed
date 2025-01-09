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

use fp_evm::{PrecompileFailure, PrecompileHandle, PrecompileOutput, PrecompileResult};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_dex::WeightInfo;
use pallet_evm::{GasWeightMapping, Precompile};
use precompile_utils::{constants::ERC20_PRECOMPILE_ADDRESS_PREFIX, prelude::*};
use seed_primitives::{AccountId, AssetId, Balance, BlockNumber, CollectionUuid};
use sp_core::{H160, H256, U256};
use sp_runtime::SaturatedConversion;
use sp_std::{marker::PhantomData, vec::Vec};

/// The ID of the gas token on TRN, equivalent to ETH on ethereum
const GAS_TOKEN_ID: AssetId = 2_u32;

/// Solidity selector of the Mint log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_MINT: [u8; 32] = keccak256!("Mint(address,uint256,uint256)");

/// Solidity selector of the Burn log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_BURN: [u8; 32] = keccak256!("Burn(address,uint256,uint256,address)");

/// Solidity selector of the Swap log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_SWAP: [u8; 32] =
	keccak256!("Swap(address,uint256,uint256,uint256,uint256,address)");

/// Saturated conversion from EVM uint256 to Balance
fn saturated_convert_balance(input: U256) -> Result<Balance, PrecompileFailure> {
	if input > Balance::MAX.into() {
		return Err(revert("DEX: Input number exceeds the Balance type boundary (2^128)"));
	}
	Ok(input.saturated_into())
}

/// Saturated conversion from EVM uint256 to Blocknumber
fn saturated_convert_blocknumber(input: U256) -> Result<BlockNumber, PrecompileFailure> {
	if input > BlockNumber::MAX.into() {
		return Err(revert("DEX: Input number exceeds the BlockNumber type boundary (2^32)"));
	}
	Ok(input.saturated_into())
}

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	AddLiquidity = "addLiquidity(address,address,uint256,uint256,uint256,uint256,address,uint256)",
	AddLiquidityETH = "addLiquidityETH(address,uint256,uint256,uint256,address,uint256)",
	RemoveLiquidity = "removeLiquidity(address,address,uint256,uint256,uint256,address,uint256)",
	RemoveLiquidityETH = "removeLiquidityETH(address,uint256,uint256,uint256,address,uint256)",
	SwapExactTokensForTokens =
		"swapExactTokensForTokens(uint256,uint256,address[],address,uint256)",
	SwapTokensForExactTokens =
		"swapTokensForExactTokens(uint256,uint256,address[],address,uint256)",
	SwapExactETHForTokens = "swapExactETHForTokens(uint256,address[],address,uint256)",
	SwapTokensForExactETH = "swapTokensForExactETH(uint256,uint256,address[],address,uint256)",
	SwapExactTokensForETH = "swapExactTokensForETH(uint256,uint256,address[],address,uint256)",
	SwapETHForExactTokens = "swapETHForExactTokens(uint256,address[],address,uint256)",
	Quote = "quote(uint256,uint256,uint256)",
	GetAmountOut = "getAmountOut(uint256,uint256,uint256)",
	GetAmountIn = "getAmountIn(uint256,uint256,uint256)",
	GetAmountsOut = "getAmountsOut(uint256,address[])",
	GetAmountsIn = "getAmountsIn(uint256,address[])",
}

/// Provides access to the Dex pallet
pub struct DexPrecompile<Runtime>(PhantomData<Runtime>);

impl<T> Default for DexPrecompile<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> Precompile for DexPrecompile<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: frame_system::Config
		+ pallet_dex::Config
		+ pallet_evm::Config
		+ pallet_assets::Config<AssetId = AssetId, Balance = Balance>,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::RuntimeCall: From<pallet_dex::Call<Runtime>>,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
{
	fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
		let selector = match handle.read_selector() {
			Ok(selector) => selector,
			Err(e) => return Err(e.into()),
		};

		if let Err(err) = handle.check_function_modifier(match selector {
			Action::AddLiquidity
			| Action::RemoveLiquidity
			| Action::RemoveLiquidityETH
			| Action::SwapExactTokensForTokens
			| Action::SwapTokensForExactTokens
			| Action::SwapTokensForExactETH
			| Action::SwapExactTokensForETH => FunctionModifier::NonPayable,
			Action::AddLiquidityETH
			| Action::SwapExactETHForTokens
			| Action::SwapETHForExactTokens => FunctionModifier::Payable,
			_ => FunctionModifier::View,
		}) {
			return Err(err.into());
		}

		match selector {
			Action::AddLiquidity => Self::add_liquidity(handle),
			Action::AddLiquidityETH => Self::add_liquidity_eth(handle),
			Action::RemoveLiquidity => Self::remove_liquidity(handle),
			Action::RemoveLiquidityETH => Self::remove_liquidity_eth(handle),
			Action::SwapExactTokensForTokens => Self::swap_exact_tokens_for_tokens(handle),
			Action::SwapTokensForExactTokens => Self::swap_tokens_for_exact_tokens(handle),
			Action::SwapExactETHForTokens => Self::swap_exact_eth_for_tokens(handle),
			Action::SwapTokensForExactETH => Self::swap_tokens_for_exact_eth(handle),
			Action::SwapExactTokensForETH => Self::swap_exact_tokens_for_eth(handle),
			Action::SwapETHForExactTokens => Self::swap_eth_for_exact_tokens(handle),
			Action::Quote => Self::quote(handle),
			Action::GetAmountIn => Self::get_amount_in(handle),
			Action::GetAmountOut => Self::get_amount_out(handle),
			Action::GetAmountsIn => Self::get_amounts_in(handle),
			Action::GetAmountsOut => Self::get_amounts_out(handle),
		}
	}
}

impl<Runtime> DexPrecompile<Runtime> {
	pub fn new() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> DexPrecompile<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: frame_system::Config
		+ pallet_dex::Config
		+ pallet_evm::Config
		+ pallet_assets::Config<AssetId = AssetId, Balance = Balance>,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::RuntimeCall: From<pallet_dex::Call<Runtime>>,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
{
	fn add_liquidity(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				token_a: Address,
				token_b: Address,
				amount_a_desired: U256,
				amount_b_desired: U256,
				amount_a_min: U256,
				amount_b_min: U256,
				to: Address,
				deadline: U256
			}
		);

		let to: H160 = to.into();
		// Parse asset_id
		let asset_id_a: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			token_a,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("DEX: Invalid asset address"))?;
		let asset_id_b: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			token_b,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("DEX: Invalid asset address"))?;
		let caller: Runtime::AccountId = handle.context().caller.into();

		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_dex::Config>::WeightInfo::add_liquidity(),
		))?;

		let (amount_0, amount_1, liquidity) = pallet_dex::Pallet::<Runtime>::do_add_liquidity(
			&caller,
			asset_id_a,
			asset_id_b,
			saturated_convert_balance(amount_a_desired)?,
			saturated_convert_balance(amount_b_desired)?,
			saturated_convert_balance(amount_a_min)?,
			saturated_convert_balance(amount_b_min)?,
			to.into(),
			Some(saturated_convert_blocknumber(deadline)?.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		let pair: AccountId =
			pallet_dex::types::TradingPair::new(asset_id_a, asset_id_b).pool_address();
		log3(
			<H160 as Into<Address>>::into(pair.into()),
			SELECTOR_LOG_MINT,
			caller.into(),
			H256::from_slice(&EvmDataWriter::new().write(amount_0).build()),
			EvmDataWriter::new().write(amount_1).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(
			EvmDataWriter::new()
				.write::<u128>(amount_0)
				.write::<u128>(amount_1)
				.write::<u128>(liquidity)
				.build(),
		))
	}

	fn add_liquidity_eth(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(
			handle,
			{
				token_a: Address,
				amount_a_desired: U256,
				amount_a_min: U256,
				amount_b_min: U256,
				to: Address,
				deadline: U256
			}
		);

		let to: H160 = to.into();
		// Parse asset_id
		let asset_id_a: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			token_a,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("DEX: Invalid asset address"))?;
		let caller: Runtime::AccountId = handle.context().caller.into();

		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_dex::Config>::WeightInfo::add_liquidity(),
		))?;

		let (amount_0, amount_1, liquidity) = pallet_dex::Pallet::<Runtime>::do_add_liquidity(
			&caller,
			asset_id_a,
			GAS_TOKEN_ID,
			saturated_convert_balance(amount_a_desired)?,
			saturated_convert_balance(handle.context().apparent_value)?,
			saturated_convert_balance(amount_a_min)?,
			saturated_convert_balance(amount_b_min)?,
			to.into(),
			Some(saturated_convert_blocknumber(deadline)?.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		let pair: AccountId =
			pallet_dex::types::TradingPair::new(GAS_TOKEN_ID, asset_id_a).pool_address();
		log3(
			<H160 as Into<Address>>::into(pair.into()),
			SELECTOR_LOG_MINT,
			caller.into(),
			H256::from_slice(&EvmDataWriter::new().write(amount_0).build()),
			EvmDataWriter::new().write(amount_1).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(
			EvmDataWriter::new()
				.write::<u128>(amount_0)
				.write::<u128>(amount_1)
				.write::<u128>(liquidity)
				.build(),
		))
	}

	fn remove_liquidity(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(4, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				token_a: Address,
				token_b: Address,
				liquidity: U256,
				amount_a_min: U256,
				amount_b_min: U256,
				to: Address,
				deadline: U256
			}
		);

		let to: H160 = to.into();
		// Parse asset_id
		let asset_id_a: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			token_a,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("DEX: Invalid asset address"))?;
		let asset_id_b: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			token_b,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("DEX: Invalid asset address"))?;
		let caller: Runtime::AccountId = handle.context().caller.into();

		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_dex::Config>::WeightInfo::remove_liquidity(),
		))?;

		let (amount_0, amount_1) = pallet_dex::Pallet::<Runtime>::do_remove_liquidity(
			&caller,
			asset_id_a,
			asset_id_b,
			saturated_convert_balance(liquidity)?,
			saturated_convert_balance(amount_a_min)?,
			saturated_convert_balance(amount_b_min)?,
			to.into(),
			Some(saturated_convert_blocknumber(deadline)?.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		let pair: AccountId =
			pallet_dex::types::TradingPair::new(asset_id_a, asset_id_b).pool_address();
		log4(
			<H160 as Into<Address>>::into(pair.into()),
			SELECTOR_LOG_BURN,
			caller.into(),
			H256::from_slice(&EvmDataWriter::new().write(amount_0).build()),
			H256::from_slice(&EvmDataWriter::new().write(amount_1).build()),
			EvmDataWriter::new().write(Address::from(to)).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write::<u128>(amount_0).write::<u128>(amount_1).build()))
	}

	fn remove_liquidity_eth(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(4, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				token_a: Address,
				liquidity: U256,
				amount_a_min: U256,
				amount_b_min: U256,
				to: Address,
				deadline: U256
			}
		);

		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_dex::Config>::WeightInfo::remove_liquidity(),
		))?;

		let to: H160 = to.into();
		// Parse asset_id
		let asset_id_a: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			token_a,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("DEX: Invalid asset address"))?;
		let caller: Runtime::AccountId = handle.context().caller.into();

		let (amount_0, amount_1) = pallet_dex::Pallet::<Runtime>::do_remove_liquidity(
			&caller,
			asset_id_a,
			GAS_TOKEN_ID,
			saturated_convert_balance(liquidity)?,
			saturated_convert_balance(amount_a_min)?,
			saturated_convert_balance(amount_b_min)?,
			to.into(),
			Some(saturated_convert_blocknumber(deadline)?.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		let pair: AccountId =
			pallet_dex::types::TradingPair::new(GAS_TOKEN_ID, asset_id_a).pool_address();
		log4(
			<H160 as Into<Address>>::into(pair.into()),
			SELECTOR_LOG_BURN,
			caller.into(),
			H256::from_slice(&EvmDataWriter::new().write(amount_0).build()),
			H256::from_slice(&EvmDataWriter::new().write(amount_1).build()),
			EvmDataWriter::new().write(Address::from(to)).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write::<u128>(amount_0).write::<u128>(amount_1).build()))
	}

	fn swap_exact_tokens_for_tokens(
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(
			handle,
			{
				amount_in: U256,
				amount_out_min: U256,
				path: Vec<Address>,
				to: Address,
				deadline: U256
			}
		);

		let to: H160 = to.into();
		let mut path_assets = Vec::new();
		for token_address in path.into_iter() {
			let asset_id = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
				token_address,
				ERC20_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("DEX: Invalid asset address"))?;
			path_assets.push(asset_id);
		}
		let caller: Runtime::AccountId = handle.context().caller.into();

		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_dex::Config>::WeightInfo::swap_with_exact_supply(),
		))?;

		let (amounts, swap_res) = pallet_dex::Pallet::<Runtime>::do_swap_with_exact_supply(
			&caller,
			saturated_convert_balance(amount_in)?,
			saturated_convert_balance(amount_out_min)?,
			&path_assets,
			to.into(),
			Some(saturated_convert_blocknumber(deadline)?.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		handle.record_log_costs_manual(4 * swap_res.len(), 32)?;
		for (asset_index, (amount_0_in, amount_1_in, amount_0_out, amount_1_out)) in
			swap_res.into_iter().enumerate()
		{
			let pair: AccountId = pallet_dex::types::TradingPair::new(
				path_assets[asset_index],
				path_assets[asset_index + 1],
			)
			.pool_address();
			log4(
				<H160 as Into<Address>>::into(pair.into()),
				SELECTOR_LOG_SWAP,
				caller.clone().into(),
				H256::from_slice(&EvmDataWriter::new().write(amount_0_in).build()),
				H256::from_slice(&EvmDataWriter::new().write(amount_1_in).build()),
				EvmDataWriter::new()
					.write(amount_0_out)
					.write(amount_1_out)
					.write(Address::from(to))
					.build(),
			)
			.record(handle)?;
		}

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(amounts).build()))
	}

	fn swap_tokens_for_exact_tokens(
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(
			handle,
			{
				amount_out: U256,
				amount_in_max: U256,
				path: Vec<Address>,
				to: Address,
				deadline: U256
			}
		);

		let to: H160 = to.into();
		let mut path_assets = Vec::new();
		for token_address in path.into_iter() {
			let asset_id = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
				token_address,
				ERC20_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("DEX: Invalid asset address"))?;
			path_assets.push(asset_id);
		}
		let caller: Runtime::AccountId = handle.context().caller.into();

		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_dex::Config>::WeightInfo::swap_with_exact_target(),
		))?;

		let (amounts, swap_res) = pallet_dex::Pallet::<Runtime>::do_swap_with_exact_target(
			&caller,
			saturated_convert_balance(amount_out)?,
			saturated_convert_balance(amount_in_max)?,
			&path_assets,
			to.into(),
			Some(saturated_convert_blocknumber(deadline)?.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		handle.record_log_costs_manual(4 * swap_res.len(), 32)?;
		for (asset_index, (amount_0_in, amount_1_in, amount_0_out, amount_1_out)) in
			swap_res.into_iter().enumerate()
		{
			let pair: AccountId = pallet_dex::types::TradingPair::new(
				path_assets[asset_index],
				path_assets[asset_index + 1],
			)
			.pool_address();
			log4(
				<H160 as Into<Address>>::into(pair.into()),
				SELECTOR_LOG_SWAP,
				caller.clone().into(),
				H256::from_slice(&EvmDataWriter::new().write(amount_0_in).build()),
				H256::from_slice(&EvmDataWriter::new().write(amount_1_in).build()),
				EvmDataWriter::new()
					.write(amount_0_out)
					.write(amount_1_out)
					.write(Address::from(to))
					.build(),
			)
			.record(handle)?;
		}

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(amounts).build()))
	}

	fn swap_exact_eth_for_tokens(
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(
			handle,
			{
				amount_out_min: U256,
				path: Vec<Address>,
				to: Address,
				deadline: U256
			}
		);

		let to: H160 = to.into();
		let mut path_assets = Vec::new();
		for token_address in path.into_iter() {
			let asset_id = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
				token_address,
				ERC20_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("DEX: Invalid asset address"))?;
			path_assets.push(asset_id);
		}
		let caller: Runtime::AccountId = handle.context().caller.into();

		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_dex::Config>::WeightInfo::swap_with_exact_supply(),
		))?;

		let (amounts, swap_res) = pallet_dex::Pallet::<Runtime>::do_swap_with_exact_supply(
			&caller,
			saturated_convert_balance(handle.context().apparent_value)?,
			saturated_convert_balance(amount_out_min)?,
			&path_assets,
			to.into(),
			Some(saturated_convert_blocknumber(deadline)?.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		handle.record_log_costs_manual(4 * swap_res.len(), 32)?;
		for (asset_index, (amount_0_in, amount_1_in, amount_0_out, amount_1_out)) in
			swap_res.into_iter().enumerate()
		{
			let pair: AccountId = pallet_dex::types::TradingPair::new(
				path_assets[asset_index],
				path_assets[asset_index + 1],
			)
			.pool_address();
			log4(
				<H160 as Into<Address>>::into(pair.into()),
				SELECTOR_LOG_SWAP,
				caller.clone().into(),
				H256::from_slice(&EvmDataWriter::new().write(amount_0_in).build()),
				H256::from_slice(&EvmDataWriter::new().write(amount_1_in).build()),
				EvmDataWriter::new()
					.write(amount_0_out)
					.write(amount_1_out)
					.write(Address::from(to))
					.build(),
			)
			.record(handle)?;
		}

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(amounts).build()))
	}

	fn swap_tokens_for_exact_eth(
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(
			handle,
			{
				amount_out: U256,
				amount_in_max: U256,
				path: Vec<Address>,
				to: Address,
				deadline: U256
			}
		);

		let to: H160 = to.into();
		let mut path_assets = Vec::new();
		for token_address in path.into_iter() {
			let asset_id = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
				token_address,
				ERC20_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("DEX: Invalid asset address"))?;
			path_assets.push(asset_id);
		}
		let caller: Runtime::AccountId = handle.context().caller.into();

		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_dex::Config>::WeightInfo::swap_with_exact_target(),
		))?;

		let (amounts, swap_res) = pallet_dex::Pallet::<Runtime>::do_swap_with_exact_target(
			&caller,
			saturated_convert_balance(amount_out)?,
			saturated_convert_balance(amount_in_max)?,
			&path_assets,
			to.into(),
			Some(saturated_convert_blocknumber(deadline)?.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		handle.record_log_costs_manual(4 * swap_res.len(), 32)?;
		for (asset_index, (amount_0_in, amount_1_in, amount_0_out, amount_1_out)) in
			swap_res.into_iter().enumerate()
		{
			let pair: AccountId = pallet_dex::types::TradingPair::new(
				path_assets[asset_index],
				path_assets[asset_index + 1],
			)
			.pool_address();
			log4(
				<H160 as Into<Address>>::into(pair.into()),
				SELECTOR_LOG_SWAP,
				caller.clone().into(),
				H256::from_slice(&EvmDataWriter::new().write(amount_0_in).build()),
				H256::from_slice(&EvmDataWriter::new().write(amount_1_in).build()),
				EvmDataWriter::new()
					.write(amount_0_out)
					.write(amount_1_out)
					.write(Address::from(to))
					.build(),
			)
			.record(handle)?;
		}

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(amounts).build()))
	}

	fn swap_exact_tokens_for_eth(
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(
			handle,
			{
				amount_in: U256,
				amount_out_min: U256,
				path: Vec<Address>,
				to: Address,
				deadline: U256
			}
		);

		let to: H160 = to.into();
		let mut path_assets = Vec::new();
		for token_address in path.into_iter() {
			let asset_id = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
				token_address,
				ERC20_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("DEX: Invalid asset address"))?;
			path_assets.push(asset_id);
		}
		let caller: Runtime::AccountId = handle.context().caller.into();

		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_dex::Config>::WeightInfo::swap_with_exact_supply(),
		))?;

		let (amounts, swap_res) = pallet_dex::Pallet::<Runtime>::do_swap_with_exact_supply(
			&caller,
			saturated_convert_balance(amount_in)?,
			saturated_convert_balance(amount_out_min)?,
			&path_assets,
			to.into(),
			Some(saturated_convert_blocknumber(deadline)?.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		handle.record_log_costs_manual(4 * swap_res.len(), 32)?;
		for (asset_index, (amount_0_in, amount_1_in, amount_0_out, amount_1_out)) in
			swap_res.into_iter().enumerate()
		{
			let pair: AccountId = pallet_dex::types::TradingPair::new(
				path_assets[asset_index],
				path_assets[asset_index + 1],
			)
			.pool_address();
			log4(
				<H160 as Into<Address>>::into(pair.into()),
				SELECTOR_LOG_SWAP,
				caller.clone().into(),
				H256::from_slice(&EvmDataWriter::new().write(amount_0_in).build()),
				H256::from_slice(&EvmDataWriter::new().write(amount_1_in).build()),
				EvmDataWriter::new()
					.write(amount_0_out)
					.write(amount_1_out)
					.write(Address::from(to))
					.build(),
			)
			.record(handle)?;
		}

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(amounts).build()))
	}

	fn swap_eth_for_exact_tokens(
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(
			handle,
			{
				amount_out: U256,
				path: Vec<Address>,
				to: Address,
				deadline: U256
			}
		);

		let to: H160 = to.into();
		let mut path_assets = Vec::new();
		for token_address in path.into_iter() {
			let asset_id = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
				token_address,
				ERC20_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("DEX: Invalid asset address"))?;
			path_assets.push(asset_id);
		}
		let caller: Runtime::AccountId = handle.context().caller.into();

		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_dex::Config>::WeightInfo::swap_with_exact_target(),
		))?;

		let (amounts, swap_res) = pallet_dex::Pallet::<Runtime>::do_swap_with_exact_target(
			&caller,
			saturated_convert_balance(amount_out)?,
			saturated_convert_balance(handle.context().apparent_value)?,
			&path_assets,
			to.into(),
			Some(saturated_convert_blocknumber(deadline)?.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		handle.record_log_costs_manual(4 * swap_res.len(), 32)?;
		for (asset_index, (amount_0_in, amount_1_in, amount_0_out, amount_1_out)) in
			swap_res.into_iter().enumerate()
		{
			let pair: AccountId = pallet_dex::types::TradingPair::new(
				path_assets[asset_index],
				path_assets[asset_index + 1],
			)
			.pool_address();
			log4(
				<H160 as Into<Address>>::into(pair.into()),
				SELECTOR_LOG_SWAP,
				caller.clone().into(),
				H256::from_slice(&EvmDataWriter::new().write(amount_0_in).build()),
				H256::from_slice(&EvmDataWriter::new().write(amount_1_in).build()),
				EvmDataWriter::new()
					.write(amount_0_out)
					.write(amount_1_out)
					.write(Address::from(to))
					.build(),
			)
			.record(handle)?;
		}

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(amounts).build()))
	}

	fn quote(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(
			handle,
			{
				amount_a: U256,
				reserve_a: U256,
				reserve_b: U256
			}
		);

		match pallet_dex::Pallet::<Runtime>::quote(
			amount_a,
			saturated_convert_balance(reserve_a)?,
			saturated_convert_balance(reserve_b)?,
		) {
			Ok(amount_b) => Ok(succeed(EvmDataWriter::new().write::<U256>(amount_b).build())),
			Err(e) => Err(revert(
				alloc::format!("DEX: Dispatched call failed with error: {:?}", e).as_bytes(),
			)),
		}
	}

	fn get_amount_out(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input.
		read_args!(
			handle,
			{
				amount_in: U256,
				reserve_in: U256,
				reserve_out: U256
			}
		);

		match pallet_dex::Pallet::<Runtime>::get_amount_out(
			saturated_convert_balance(amount_in)?,
			saturated_convert_balance(reserve_in)?,
			saturated_convert_balance(reserve_out)?,
		) {
			Ok(amount_out) => Ok(succeed(EvmDataWriter::new().write::<u128>(amount_out).build())),
			Err(e) => Err(revert(
				alloc::format!("DEX: Dispatched call failed with error: {:?}", e).as_bytes(),
			)),
		}
	}

	fn get_amount_in(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input.
		read_args!(
			handle,
			{
				amount_out: U256,
				reserve_in: U256,
				reserve_out: U256
			}
		);

		match pallet_dex::Pallet::<Runtime>::get_amount_in(
			saturated_convert_balance(amount_out)?,
			saturated_convert_balance(reserve_in)?,
			saturated_convert_balance(reserve_out)?,
		) {
			Ok(amount_in) => Ok(succeed(EvmDataWriter::new().write::<u128>(amount_in).build())),
			Err(e) => Err(revert(
				alloc::format!("DEX: Dispatched call failed with error: {:?}", e).as_bytes(),
			)),
		}
	}

	fn get_amounts_out(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(
			handle,
			{
				amount_in: U256,
				path: Vec<Address>
			}
		);

		let path_len = path.len() as u64;
		let mut path_assets = Vec::new();
		for token_address in path.into_iter() {
			let asset_id = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
				token_address,
				ERC20_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("DEX: Invalid asset address"))?;
			path_assets.push(asset_id);
		}
		handle.record_cost(
			RuntimeHelper::<Runtime>::db_read_gas_cost()
				.saturating_mul(3 * path_len)
				.saturating_mul(4),
		)?;

		match pallet_dex::Pallet::<Runtime>::get_amounts_out(
			saturated_convert_balance(amount_in)?,
			&path_assets,
		) {
			Ok(amounts) => Ok(succeed(EvmDataWriter::new().write(amounts).build())),
			Err(e) => Err(revert(
				alloc::format!("DEX: Dispatched call failed with error: {:?}", e).as_bytes(),
			)),
		}
	}

	fn get_amounts_in(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(
			handle,
			{
				amount_out: U256,
				path: Vec<Address>
			}
		);

		let path_len = path.len() as u64;
		let mut path_assets = Vec::new();
		for token_address in path.into_iter() {
			let asset_id = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
				token_address,
				ERC20_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("DEX: Invalid asset address"))?;
			path_assets.push(asset_id);
		}
		handle.record_cost(
			RuntimeHelper::<Runtime>::db_read_gas_cost()
				.saturating_mul(3 * path_len)
				.saturating_mul(4),
		)?;

		match pallet_dex::Pallet::<Runtime>::get_amounts_in(
			saturated_convert_balance(amount_out)?,
			&path_assets,
		) {
			Ok(amounts) => Ok(succeed(EvmDataWriter::new().write(amounts).build())),
			Err(e) => Err(revert(
				alloc::format!("DEX: Dispatched call failed with error: {:?}", e).as_bytes(),
			)),
		}
	}
}
