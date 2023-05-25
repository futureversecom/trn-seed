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
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	ensure,
};
use pallet_evm::{GasWeightMapping, Precompile};
use precompile_utils::{constants::ERC721_PRECOMPILE_ADDRESS_PREFIX, prelude::*};
use seed_primitives::{AssetId, Balance, BlockNumber, CollectionUuid, MetadataScheme, TokenCount};
use sp_core::{H160, U128, U256};
use sp_runtime::{traits::SaturatedConversion, Permill};
use sp_std::{marker::PhantomData, vec::Vec};

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
	RemoveLiquidityWithPermit = "removeLiquidityWithPermit(address,address,uint,uint,uint,address,uint,bool,uint8,bytes32,bytes32)",
	RemoveLiquidityETHWithPermit = "removeLiquidityETHWithPermit(address,uint,uint,uint,address,uint,bool,uint8,bytes32,bytes32)",
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
				_ => return Err(revert("DEX: Function not implemented").into()),
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
		handle.record_log_costs_manual(2, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				asset_id_a: AssetId,
				asset_id_b: AssetId,
				amount_a_desired: Balance,
				amount_b_desired: Balance,
				amount_a_min: Balance,
				amount_b_min: Balance,
				to: AssetId,
				deadline: U256
			}
		);

		let caller = handle.context().caller;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller.into()).into(),
			pallet_dex::Call::<Runtime>::add_liquidity {
				asset_id_a,
				asset_id_b,
				amount_a_desired,
				amount_b_desired,
				amount_a_min,
				amount_b_min,
				min_share_increment: 0,
			},
		)?;

		log3(
			handle.code_address(),
			SELECTOR_LOG_MINT,
			caller,
			amount_a_desired,
			EvmDataWriter::new().write(amount_b_desired).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn add_liquidity_eth(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(7, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				asset_id_a: AssetId,
				asset_id_b: AssetId,
				amount_a_desired: Balance,
				amount_b_desired: Balance,
				amount_a_min: Balance,
				amount_b_min: Balance,
				to: AssetId,
				deadline: U256
			}
		);

		let caller = handle.context().caller;
		let current_block_number = frame_system::Pallet::<Runtime>::block_number();

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller.into()).into(),
			pallet_dex::Call::<Runtime>::add_liquidity {
				asset_id_a,
				asset_id_b,
				amount_a_desired,
				amount_b_desired,
				amount_a_min,
				amount_b_min,
				min_share_increment: 0,
			},
		)?;

		log3(
			handle.code_address(),
			SELECTOR_LOG_MINT,
			caller,
			amount_a_desired,
			EvmDataWriter::new().write(amount_b_desired).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn quote(
		amount_a: U128,
		reserve_a: Balance,
		reserve_b: Balance,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		match pallet_dex::Pallet::<Runtime>::quote(amount_a, reserve_a, reserve_b) {
			Ok(amount_b) => Ok(succeed(EvmDataWriter::new().write::<U128>(amount_b).build())),
			Err(e) => Err(revert(
				alloc::format!("DEX: Dispatched call failed with error: {:?}", e)
					.as_bytes()
					.to_vec(),
			)),
		}
	}
}
