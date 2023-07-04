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

use fp_evm::{PrecompileHandle, PrecompileOutput};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::{
		fungibles::{Inspect, InspectMetadata, Transfer},
		OriginTrait,
	},
};
use pallet_evm::PrecompileSet;
use precompile_utils::{constants::ERC20_PRECOMPILE_ADDRESS_PREFIX, prelude::*};
use seed_primitives::{AssetId, Balance};
use sp_core::{H160, U256};
use sp_runtime::traits::{SaturatedConversion, Zero};
use sp_std::marker::PhantomData;

/// Solidity selector of the Transfer log, which is the Keccak of the Log signature
pub const SELECTOR_LOG_TRANSFER: [u8; 32] = keccak256!("Transfer(address,address,uint256)");

/// Solidity selector of the Approval log, which is the Keccak of the Log signature
pub const SELECTOR_LOG_APPROVAL: [u8; 32] = keccak256!("Approval(address,address,uint256)");

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	TotalSupply = "totalSupply()",
	BalanceOf = "balanceOf(address)",
	Allowance = "allowance(address,address)",
	Transfer = "transfer(address,uint256)",
	Approve = "approve(address,uint256)",
	TransferFrom = "transferFrom(address,address,uint256)",
	Name = "name()",
	Symbol = "symbol()",
	Decimals = "decimals()",
}

/// The following distribution has been decided for the precompiles
/// The precompile for AssetId X, where X is a u128 (i.e.16 bytes), if 0XCCCCCCCC + Bytes(AssetId)
/// In order to route the address to Erc20Precompile<R>, we first check whether the AssetId
/// exists in pallet-assets-ext
/// This means that every address that starts with 0xCCCCCCCC will go through an additional db read,
/// but the probability for this to happen is 2^-32 for random addresses
pub struct Erc20PrecompileSet<Runtime>(PhantomData<Runtime>);

impl<T> Default for Erc20PrecompileSet<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> PrecompileSet for Erc20PrecompileSet<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: pallet_assets_ext::Config<AssetId = AssetId>
		+ pallet_evm::Config
		+ frame_system::Config
		+ pallet_assets::Config<AssetId = AssetId, Balance = Balance>
		+ pallet_token_approvals::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::RuntimeCall: From<pallet_token_approvals::Call<Runtime>>,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	Runtime: ErcIdConversion<AssetId, EvmId = Address>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: OriginTrait,
{
	fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<EvmResult<PrecompileOutput>> {
		let context = handle.context();

		if let Some(asset_id) =
			Runtime::evm_id_to_runtime_id(context.address.into(), ERC20_PRECOMPILE_ADDRESS_PREFIX)
		{
			if !<pallet_assets_ext::Pallet<Runtime> as Inspect<Runtime::AccountId>>::total_issuance(
				asset_id,
			)
			.is_zero()
			{
				let result = {
					let selector = match handle.read_selector() {
						Ok(selector) => selector,
						Err(e) => return Some(Err(e.into())),
					};

					if let Err(err) = handle.check_function_modifier(match selector {
						Action::Approve | Action::Transfer | Action::TransferFrom =>
							FunctionModifier::NonPayable,
						_ => FunctionModifier::View,
					}) {
						return Some(Err(err.into()))
					}

					match selector {
						Action::TotalSupply => Self::total_supply(asset_id, handle),
						Action::BalanceOf => Self::balance_of(asset_id, handle),
						Action::Transfer => Self::transfer(asset_id, handle),
						Action::Name => Self::name(asset_id, handle),
						Action::Symbol => Self::symbol(asset_id, handle),
						Action::Decimals => Self::decimals(asset_id, handle),
						Action::Allowance => Self::allowance(asset_id, handle),
						Action::Approve => Self::approve(asset_id, handle),
						Action::TransferFrom => Self::transfer_from(asset_id, handle),
					}
				};

				return Some(result)
			}
		}
		None
	}

	fn is_precompile(&self, address: H160) -> bool {
		if let Some(asset_id) =
			Runtime::evm_id_to_runtime_id(Address(address), ERC20_PRECOMPILE_ADDRESS_PREFIX)
		{
			// totaly supply `0` is a good enough check for asset existence
			!<pallet_assets_ext::Pallet<Runtime> as Inspect<Runtime::AccountId>>::total_issuance(
				asset_id,
			)
			.is_zero()
		} else {
			false
		}
	}
}

impl<Runtime> Erc20PrecompileSet<Runtime> {
	pub fn new() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> Erc20PrecompileSet<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: pallet_assets_ext::Config<AssetId = AssetId>
		+ pallet_evm::Config
		+ frame_system::Config
		+ pallet_assets::Config<AssetId = AssetId, Balance = Balance>
		+ pallet_token_approvals::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::RuntimeCall: From<pallet_token_approvals::Call<Runtime>>,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	Runtime: ErcIdConversion<AssetId, EvmId = Address>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: OriginTrait,
{
	fn total_supply(
		asset_id: AssetId,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Fetch info.
		let amount: U256 =
			<pallet_assets_ext::Pallet<Runtime> as Inspect<Runtime::AccountId>>::total_issuance(
				asset_id,
			)
			.into();

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(amount).build()))
	}

	fn balance_of(
		asset_id: AssetId,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		// Read input.
		read_args!(handle, { owner: Address });
		let owner: H160 = owner.into();

		// Fetch info.
		// TODO Check staking balances
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost() * 2)?;
		let amount: U256 =
			<pallet_assets_ext::Pallet<Runtime> as Inspect<Runtime::AccountId>>::reducible_balance(
				asset_id,
				&owner.into(),
				false,
			)
			.into();

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(amount).build()))
	}

	fn allowance(
		asset_id: AssetId,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;

		// Read input.
		read_args!(handle, { owner: Address, spender: Address });

		let owner: Runtime::AccountId = H160::from(owner).into();
		let spender: Runtime::AccountId = H160::from(spender).into();

		// Fetch info.
		let amount: U256 = pallet_token_approvals::Pallet::<Runtime>::erc20_approvals(
			(&owner, &asset_id),
			&spender,
		)
		.unwrap_or_default()
		.into();

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(amount).build()))
	}

	fn approve(
		asset_id: AssetId,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;

		// Parse input.
		read_args!(handle, { spender: Address, amount: U256 });
		let spender: H160 = spender.into();
		// Amount saturate if too high.
		let amount: Balance = amount.saturated_into();

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_token_approvals::Call::<Runtime>::erc20_approval {
				caller: handle.context().caller.into(),
				spender: spender.into(),
				asset_id,
				amount,
			},
		)?;

		log3(
			handle.code_address(),
			SELECTOR_LOG_APPROVAL,
			handle.context().caller,
			spender,
			EvmDataWriter::new().write(amount).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn transfer(
		asset_id: AssetId,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;

		// Parse input.
		read_args!(handle, { to: Address, amount: U256 });
		let to: H160 = to.into();
		let amount: Balance = amount.saturated_into();

		let origin: Runtime::AccountId = handle.context().caller.into();
		let _ = <pallet_assets_ext::Pallet<Runtime> as Transfer<Runtime::AccountId>>::transfer(
			asset_id,
			&origin,
			&to.clone().into(),
			amount,
			false,
		)
		.map_err(|e| revert(alloc::format!("ERC20: Dispatched call failed with error: {:?}", e)))?;

		log3(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER,
			handle.context().caller,
			to,
			EvmDataWriter::new().write(amount).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn transfer_from(
		asset_id: AssetId,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(handle, { from: Address, to: Address, amount: U256 });
		let from: H160 = from.into();
		let to: H160 = to.into();
		let amount: Balance = amount.saturated_into();

		{
			// Convert address types into Runtime::AccountId
			let from: Runtime::AccountId = from.into();
			let to: Runtime::AccountId = to.into();
			let caller: Runtime::AccountId = handle.context().caller.into();

			handle.record_cost(
				RuntimeHelper::<Runtime>::db_read_gas_cost() +
					RuntimeHelper::<Runtime>::db_write_gas_cost(),
			)?;

			// Update approval balance,
			// will error if no approval exists or approval is of insufficient amount
			RuntimeHelper::<Runtime>::try_dispatch(
				handle,
				None.into(),
				pallet_token_approvals::Call::<Runtime>::erc20_update_approval {
					caller: from.clone(),
					spender: caller,
					asset_id,
					amount,
				},
			)?;

			// Transfer
			let _ = <pallet_assets_ext::Pallet<Runtime> as Transfer<Runtime::AccountId>>::transfer(
				asset_id,
				&from,
				&to.clone(),
				amount,
				false,
			)
			.map_err(|e| {
				revert(alloc::format!("ERC20: Dispatched call failed with error: {:?}", e))
			})?;
		}
		log3(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER,
			H160::from(from),
			H160::from(to),
			EvmDataWriter::new().write(amount).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn name(asset_id: AssetId, handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Build output.
		Ok(
			succeed(
				EvmDataWriter::new()
					.write::<Bytes>(
						<pallet_assets_ext::Pallet<Runtime> as InspectMetadata<
							Runtime::AccountId,
						>>::name(&asset_id)
						.as_slice()
						.into(),
					)
					.build(),
			),
		)
	}

	fn symbol(
		asset_id: AssetId,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Build output.
		Ok(
			succeed(
				EvmDataWriter::new()
					.write::<Bytes>(
						<pallet_assets_ext::Pallet<Runtime> as InspectMetadata<
							Runtime::AccountId,
						>>::symbol(&asset_id)
						.as_slice()
						.into(),
					)
					.build(),
			),
		)
	}

	fn decimals(
		asset_id: AssetId,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Build output.
		Ok(succeed(
			EvmDataWriter::new()
				.write::<u8>(<pallet_assets_ext::Pallet<Runtime> as InspectMetadata<
					Runtime::AccountId,
				>>::decimals(&asset_id))
				.build(),
		))
	}
}
