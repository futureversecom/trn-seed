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

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use fp_evm::{PrecompileHandle, PrecompileOutput, PrecompileResult};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::Precompile;
use precompile_utils::{constants::ERC721_PRECOMPILE_ADDRESS_PREFIX, prelude::*};
use seed_primitives::{AssetId, Balance, BlockNumber, CollectionUuid};
use sp_core::{H160, H256, U256};
use sp_std::{marker::PhantomData, vec::Vec};

/// The ID of the gas token on TRN, equivalent to ETH on ethereum
const GAS_TOKEN_ID: AssetId = 2_u32;

/// Solidity selector of the Mint log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_MINT: [u8; 32] = keccak256!("Mint(address,uint,uint)");

/// Solidity selector of the Burn log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_BURN: [u8; 32] = keccak256!("Burn(address,uint,uint,address)");

/// Solidity selector of the Swap log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_SWAP: [u8; 32] = keccak256!("Swap(address,uint,uint,uint,uint,address)");

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	AddLiquidity = "addLiquidity(address,address,uint,uint,uint,uint,address,uint)",
	AddLiquidityETH = "addLiquidityETH(address,uint,uint,uint,address,uint)",
	RemoveLiquidity = "removeLiquidity(address,address,uint,uint,uint,address,uint)",
	RemoveLiquidityETH = "removeLiquidityETH(address,uint,uint,uint,address,uint)",
	SwapExactTokensForTokens = "swapExactTokensForTokens(uint,uint,address[],address,uint)",
	SwapTokensForExactTokens = "swapTokensForExactTokens(uint,uint,address[],address,uint)",
	SwapExactETHForTokens = "swapExactETHForTokens(uint,address[],address,uint)",
	SwapTokensForExactETH = "swapTokensForExactETH(uint,uint,address[],address,uint)",
	SwapExactTokensForETH = "swapExactTokensForETH(uint,uint,address[],address,uint)",
	SwapETHForExactTokens = "swapETHForExactTokens(uint,address[],address,uint)",
	Quote = "quote(uint,uint,uint)",
	GetAmountOut = "getAmountOut(uint,uint,uint)",
	GetAmountIn = "getAmountIn(uint,uint,uint)",
	GetAmountsOut = "getAmountsOut(uint,address[])",
	GetAmountsIn = "getAmountsIn(uint,address[])",
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
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::Call: From<pallet_dex::Call<Runtime>>,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
{
	fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
		let result = {
			let selector = match handle.read_selector() {
				Ok(selector) => selector,
				Err(e) => return Err(e.into()),
			};

			if let Err(err) = handle.check_function_modifier(FunctionModifier::NonPayable) {
				return Err(err.into())
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
		};
		return result
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
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::Call: From<pallet_dex::Call<Runtime>>,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
{
	fn add_liquidity(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				token_a: AssetId,
				token_b: AssetId,
				amount_a_desired: Balance,
				amount_b_desired: Balance,
				amount_a_min: Balance,
				amount_b_min: Balance,
				to: Address,
				deadline: BlockNumber
			}
		);

		let to: H160 = to.into();
		let caller: Runtime::AccountId = handle.context().caller.into();

		let (amount_0, amount_1, liquidity) = pallet_dex::Pallet::<Runtime>::do_add_liquidity(
			&caller,
			token_a,
			token_b,
			amount_a_desired,
			amount_b_desired,
			amount_a_min,
			amount_b_min,
			to.into(),
			Some(deadline.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		log3(
			handle.code_address(),
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
				token_a: AssetId,
				amount_a_desired: Balance,
				amount_a_min: Balance,
				amount_b_min: Balance,
				to: Address,
				deadline: BlockNumber
			}
		);

		let to: H160 = to.into();
		let caller: Runtime::AccountId = handle.context().caller.into();

		let (amount_0, amount_1, liquidity) = pallet_dex::Pallet::<Runtime>::do_add_liquidity(
			&caller,
			token_a,
			GAS_TOKEN_ID,
			amount_a_desired,
			handle.context().apparent_value.as_u128(),
			amount_a_min,
			amount_b_min,
			to.into(),
			Some(deadline.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		log3(
			handle.code_address(),
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
				token_a: AssetId,
				token_b: AssetId,
				liquidity: Balance,
				amount_a_min: Balance,
				amount_b_min: Balance,
				to: Address,
				deadline: BlockNumber
			}
		);

		let to: H160 = to.into();
		let caller: Runtime::AccountId = handle.context().caller.into();

		let (amount_0, amount_1) = pallet_dex::Pallet::<Runtime>::do_remove_liquidity(
			&caller,
			token_a,
			token_b,
			liquidity,
			amount_a_min,
			amount_b_min,
			to.into(),
			Some(deadline.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		log4(
			handle.code_address(),
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
				token_a: AssetId,
				liquidity: Balance,
				amount_a_min: Balance,
				amount_b_min: Balance,
				to: Address,
				deadline: BlockNumber
			}
		);

		let to: H160 = to.into();
		let caller: Runtime::AccountId = handle.context().caller.into();

		let (amount_0, amount_1) = pallet_dex::Pallet::<Runtime>::do_remove_liquidity(
			&caller,
			token_a,
			GAS_TOKEN_ID,
			liquidity,
			amount_a_min,
			amount_b_min,
			to.into(),
			Some(deadline.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		log4(
			handle.code_address(),
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
				amount_in: Balance,
				amount_out_min: Balance,
				path: Vec<AssetId>,
				to: Address,
				deadline: BlockNumber
			}
		);

		let to: H160 = to.into();
		let caller: Runtime::AccountId = handle.context().caller.into();

		let (amounts, swap_res) = pallet_dex::Pallet::<Runtime>::do_swap_with_exact_supply(
			&caller,
			amount_in,
			amount_out_min,
			&path,
			to.into(),
			Some(deadline.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		handle.record_log_costs_manual(4 * swap_res.len(), 32)?;
		for (amount_0_in, amount_1_in, amount_0_out, amount_1_out) in swap_res {
			log4(
				handle.code_address(),
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
				amount_out: Balance,
				amount_in_max: Balance,
				path: Vec<AssetId>,
				to: Address,
				deadline: BlockNumber
			}
		);

		let to: H160 = to.into();
		let caller: Runtime::AccountId = handle.context().caller.into();

		let (amounts, swap_res) = pallet_dex::Pallet::<Runtime>::do_swap_with_exact_target(
			&caller,
			amount_out,
			amount_in_max,
			&path,
			to.into(),
			Some(deadline.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		handle.record_log_costs_manual(4 * swap_res.len(), 32)?;
		for (amount_0_in, amount_1_in, amount_0_out, amount_1_out) in swap_res {
			log4(
				handle.code_address(),
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
				amount_out_min: Balance,
				path: Vec<AssetId>,
				to: Address,
				deadline: BlockNumber
			}
		);

		let to: H160 = to.into();
		let caller: Runtime::AccountId = handle.context().caller.into();

		let (amounts, swap_res) = pallet_dex::Pallet::<Runtime>::do_swap_with_exact_supply(
			&caller,
			handle.context().apparent_value.as_u128(),
			amount_out_min,
			&path,
			to.into(),
			Some(deadline.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		handle.record_log_costs_manual(4 * swap_res.len(), 32)?;
		for (amount_0_in, amount_1_in, amount_0_out, amount_1_out) in swap_res {
			log4(
				handle.code_address(),
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
				amount_out: Balance,
				amount_in_max: Balance,
				path: Vec<AssetId>,
				to: Address,
				deadline: BlockNumber
			}
		);

		let to: H160 = to.into();
		let caller: Runtime::AccountId = handle.context().caller.into();

		let (amounts, swap_res) = pallet_dex::Pallet::<Runtime>::do_swap_with_exact_target(
			&caller,
			amount_out,
			amount_in_max,
			&path,
			to.into(),
			Some(deadline.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		handle.record_log_costs_manual(4 * swap_res.len(), 32)?;
		for (amount_0_in, amount_1_in, amount_0_out, amount_1_out) in swap_res {
			log4(
				handle.code_address(),
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
				amount_in: Balance,
				amount_out_min: Balance,
				path: Vec<AssetId>,
				to: Address,
				deadline: BlockNumber
			}
		);

		let to: H160 = to.into();
		let caller: Runtime::AccountId = handle.context().caller.into();

		let (amounts, swap_res) = pallet_dex::Pallet::<Runtime>::do_swap_with_exact_supply(
			&caller,
			amount_in,
			amount_out_min,
			&path,
			to.into(),
			Some(deadline.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		handle.record_log_costs_manual(4 * swap_res.len(), 32)?;
		for (amount_0_in, amount_1_in, amount_0_out, amount_1_out) in swap_res {
			log4(
				handle.code_address(),
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
				amount_out: Balance,
				path: Vec<AssetId>,
				to: Address,
				deadline: BlockNumber
			}
		);

		let to: H160 = to.into();
		let caller: Runtime::AccountId = handle.context().caller.into();

		let (amounts, swap_res) = pallet_dex::Pallet::<Runtime>::do_swap_with_exact_target(
			&caller,
			amount_out,
			handle.context().apparent_value.as_u128(),
			&path,
			to.into(),
			Some(deadline.into()),
		)
		.map_err(|e| revert(alloc::format!("DEX: Dispatched call failed with error: {:?}", e)))?;

		handle.record_log_costs_manual(4 * swap_res.len(), 32)?;
		for (amount_0_in, amount_1_in, amount_0_out, amount_1_out) in swap_res {
			log4(
				handle.code_address(),
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
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input.
		read_args!(
			handle,
			{
				amount_a: U256,
				reserve_a: Balance,
				reserve_b: Balance
			}
		);

		match pallet_dex::Pallet::<Runtime>::quote(amount_a, reserve_a, reserve_b) {
			Ok(amount_b) => Ok(succeed(EvmDataWriter::new().write::<U256>(amount_b).build())),
			Err(e) => Err(revert(
				alloc::format!("DEX: Dispatched call failed with error: {:?}", e)
					.as_bytes()
					.to_vec(),
			)),
		}
	}

	fn get_amount_out(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input.
		read_args!(
			handle,
			{
				amount_in: Balance,
				reserve_in: Balance,
				reserve_out: Balance
			}
		);

		match pallet_dex::Pallet::<Runtime>::get_amount_out(amount_in, reserve_in, reserve_out) {
			Ok(amount_out) => Ok(succeed(EvmDataWriter::new().write::<u128>(amount_out).build())),
			Err(e) => Err(revert(
				alloc::format!("DEX: Dispatched call failed with error: {:?}", e)
					.as_bytes()
					.to_vec(),
			)),
		}
	}

	fn get_amount_in(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input.
		read_args!(
			handle,
			{
				amount_out: Balance,
				reserve_in: Balance,
				reserve_out: Balance
			}
		);

		match pallet_dex::Pallet::<Runtime>::get_amount_in(amount_out, reserve_in, reserve_out) {
			Ok(amount_in) => Ok(succeed(EvmDataWriter::new().write::<u128>(amount_in).build())),
			Err(e) => Err(revert(
				alloc::format!("DEX: Dispatched call failed with error: {:?}", e)
					.as_bytes()
					.to_vec(),
			)),
		}
	}

	fn get_amounts_out(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input.
		read_args!(
			handle,
			{
				amount_in: Balance,
				path: Vec<AssetId>
			}
		);

		match pallet_dex::Pallet::<Runtime>::get_amounts_out(amount_in, &path) {
			Ok(amounts) => Ok(succeed(EvmDataWriter::new().write(amounts).build())),
			Err(e) => Err(revert(
				alloc::format!("DEX: Dispatched call failed with error: {:?}", e)
					.as_bytes()
					.to_vec(),
			)),
		}
	}

	fn get_amounts_in(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input.
		read_args!(
			handle,
			{
				amount_out: Balance,
				path: Vec<AssetId>
			}
		);

		match pallet_dex::Pallet::<Runtime>::get_amounts_in(amount_out, &path) {
			Ok(amounts) => Ok(succeed(EvmDataWriter::new().write(amounts).build())),
			Err(e) => Err(revert(
				alloc::format!("DEX: Dispatched call failed with error: {:?}", e)
					.as_bytes()
					.to_vec(),
			)),
		}
	}
}
