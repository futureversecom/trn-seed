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

use fp_evm::{IsPrecompileResult, PrecompileHandle, PrecompileOutput};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::{
		fungibles::{metadata::Inspect as InspectMetadata, Inspect},
		tokens::{Fortitude, Preservation},
		OriginTrait,
	},
};
use pallet_evm::PrecompileSet;
use precompile_utils::{constants::ERC20_PRECOMPILE_ADDRESS_PREFIX, prelude::*};
use seed_primitives::{AssetId, Balance};
use sp_core::{Encode, H160, U256};
use sp_runtime::traits::SaturatedConversion;
use sp_std::marker::PhantomData;

/// Solidity selector of the Transfer log, which is the Keccak of the Log signature
pub const SELECTOR_LOG_TRANSFER: [u8; 32] = keccak256!("Transfer(address,address,uint256)");

/// Solidity selector of the Approval log, which is the Keccak of the Log signature
pub const SELECTOR_LOG_APPROVAL: [u8; 32] = keccak256!("Approval(address,address,uint256)");

/// Interface IDs for the ERC20, ERC20Metadata, and ERC165 interfaces
pub const ERC165_INTERFACE_IDS: &[u32] = &[
	0x01ffc9a7, // ERC165
	0x36372b07, // ERC20
	0xa219a025, // ERC20Metadata
];

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	TotalSupply = "totalSupply()",
	BalanceOf = "balanceOf(address)",
	Allowance = "allowance(address,address)",
	Transfer = "transfer(address,uint256)",
	Approve = "approve(address,uint256)",
	TransferFrom = "transferFrom(address,address,uint256)",
	// ERC20Metadata - https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/token/ERC20/extensions/IERC20Metadata.sol
	Name = "name()",
	Symbol = "symbol()",
	Decimals = "decimals()",
	// ERC165 - https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/utils/introspection/ERC165.sol
	SupportsInterface = "supportsInterface(bytes4)",
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
	Runtime::RuntimeCall: From<pallet_assets_ext::Call<Runtime>>,
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
			if !<pallet_assets_ext::Pallet<Runtime>>::asset_exists(asset_id) {
				return None;
			}

			let result = {
				let selector = match handle.read_selector() {
					Ok(selector) => selector,
					Err(e) => return Some(Err(e.into())),
				};

				if let Err(err) = handle.check_function_modifier(match selector {
					Action::Approve | Action::Transfer | Action::TransferFrom => {
						FunctionModifier::NonPayable
					},
					_ => FunctionModifier::View,
				}) {
					return Some(Err(err.into()));
				}

				match selector {
					Action::TotalSupply => Self::total_supply(asset_id, handle),
					Action::BalanceOf => Self::balance_of(asset_id, handle),
					Action::Transfer => Self::transfer(asset_id, handle),
					Action::Allowance => Self::allowance(asset_id, handle),
					Action::Approve => Self::approve(asset_id, handle),
					Action::TransferFrom => Self::transfer_from(asset_id, handle),
					// ERC20Metadata
					Action::Name => Self::name(asset_id, handle),
					Action::Symbol => Self::symbol(asset_id, handle),
					Action::Decimals => Self::decimals(asset_id, handle),
					// ERC165
					Action::SupportsInterface => Self::supports_interface(handle),
				}
			};

			return Some(result);
		}
		None
	}

	fn is_precompile(&self, address: H160, _remaining_gas: u64) -> IsPrecompileResult {
		if let Some(asset_id) =
			Runtime::evm_id_to_runtime_id(Address(address), ERC20_PRECOMPILE_ADDRESS_PREFIX)
		{
			let extra_cost = RuntimeHelper::<Runtime>::db_read_gas_cost();
			// Check if the asset exists
			IsPrecompileResult::Answer {
				is_precompile: <pallet_assets_ext::Pallet<Runtime>>::asset_exists(asset_id),
				extra_cost,
			}
		} else {
			IsPrecompileResult::Answer { is_precompile: false, extra_cost: 0 }
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
	Runtime::RuntimeCall:
		From<pallet_token_approvals::Call<Runtime>> + From<pallet_assets_ext::Call<Runtime>>,
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
				Preservation::Expendable,
				Fortitude::Polite,
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
		let amount: U256 =
			pallet_token_approvals::ERC20Approvals::<Runtime>::get((&owner, &asset_id), &spender)
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
		let caller = handle.context().caller;
		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(Runtime::AccountId::from(caller)).into(),
			pallet_token_approvals::Call::<Runtime>::erc20_approval {
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

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin).into(),
			pallet_assets_ext::Call::<Runtime>::transfer {
				asset_id,
				destination: to.into(),
				amount,
				keep_alive: false,
			},
		)?;
		let caller = handle.context().caller;

		log3(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER,
			caller,
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
				RuntimeHelper::<Runtime>::db_read_gas_cost()
					+ RuntimeHelper::<Runtime>::db_write_gas_cost(),
			)?;

			// Update approval balance,
			// will error if no approval exists or approval is of insufficient amount
			RuntimeHelper::<Runtime>::try_dispatch(
				handle,
				Some(from.clone()).into(),
				pallet_token_approvals::Call::<Runtime>::erc20_update_approval {
					spender: caller,
					asset_id,
					amount,
				},
			)?;

			RuntimeHelper::<Runtime>::try_dispatch(
				handle,
				Some(from).into(),
				pallet_assets_ext::Call::<Runtime>::transfer {
					asset_id,
					destination: to.clone(),
					amount,
					keep_alive: false,
				},
			)?;
		}
		log3(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER,
			from,
			to,
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
						>>::name(asset_id)
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
						>>::symbol(asset_id)
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
				>>::decimals(asset_id))
				.build(),
		))
	}

	fn supports_interface(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;
		read_args!(handle, { interface_id: U256 });

		// Convert to bytes4 by getting the last 4 bytes of the BE representation
		let interface_id_bytes = interface_id.encode();
		let interface_id_u32 = u32::from_le_bytes(
			interface_id_bytes[28..32]
				.try_into()
				.map_err(|_| revert("ERC165: Invalid interface ID"))?,
		);

		// ERC165 requires returning false for 0xffffffff
		// https://eips.ethereum.org/EIPS/eip-165#how-a-contract-will-publish-the-interfaces-it-implements
		if interface_id_u32 == 0xffffffff {
			return Ok(succeed(EvmDataWriter::new().write(false).build()));
		}

		let supported = ERC165_INTERFACE_IDS.contains(&interface_id_u32);
		Ok(succeed(EvmDataWriter::new().write(supported).build()))
	}
}
