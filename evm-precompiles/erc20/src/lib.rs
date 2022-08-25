#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use fp_evm::{ExitSucceed, PrecompileHandle, PrecompileOutput, PrecompileResult};
use frame_support::{
	dispatch::Dispatchable,
	traits::{
		fungibles::{Inspect, InspectMetadata},
		OriginTrait,
	},
};
use pallet_evm::PrecompileSet;
use precompile_utils::{constants::ERC20_PRECOMPILE_ADDRESS_PREFIX, prelude::*};
use seed_pallet_common::TransferExt;
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

/// Convert EVM addresses into GA module identifiers and vice versa
pub trait Erc20IdConversion {
	/// ID type used by EVM
	type EvmId;
	/// ID type used by runtime
	type RuntimeId;
	// Get runtime Id from EVM id
	fn evm_id_to_runtime_id(evm_id: Self::EvmId) -> Option<Self::RuntimeId>;
	// Get EVM id from runtime Id
	fn runtime_id_to_evm_id(runtime_id: Self::RuntimeId) -> Self::EvmId;
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
		+ pallet_assets::Config<AssetId = AssetId, Balance = Balance>,
	Runtime: ErcIdConversion<AssetId, EvmId = Address>,
	<<Runtime as frame_system::Config>::Call as Dispatchable>::Origin: OriginTrait,
{
	fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
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
						Err(e) => return Some(Err(e)),
					};

					if let Err(err) = handle.check_function_modifier(match selector {
						Action::Approve | Action::Transfer | Action::TransferFrom =>
							FunctionModifier::NonPayable,
						_ => FunctionModifier::View,
					}) {
						return Some(Err(err))
					}

					match selector {
						Action::TotalSupply => Self::total_supply(asset_id, handle),
						Action::BalanceOf => Self::balance_of(asset_id, handle),
						Action::Transfer => Self::transfer(asset_id, handle),
						Action::Name => Self::name(asset_id, handle),
						Action::Symbol => Self::symbol(asset_id, handle),
						Action::Decimals => Self::decimals(asset_id, handle),
						Action::Allowance | Action::Approve | Action::TransferFrom =>
							return Some(Err(error("function not implemented yet").into())),
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
		+ pallet_assets::Config<AssetId = AssetId, Balance = Balance>,
	Runtime: ErcIdConversion<AssetId, EvmId = Address>,
	<<Runtime as frame_system::Config>::Call as Dispatchable>::Origin: OriginTrait,
{
	fn total_supply(
		asset_id: AssetId,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input.
		let input = handle.read_input()?;
		input.expect_arguments(0)?;

		// Fetch info.
		let amount: U256 =
			<pallet_assets_ext::Pallet<Runtime> as Inspect<Runtime::AccountId>>::total_issuance(
				asset_id,
			)
			.into();

		// Build output.
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(amount).build(),
		})
	}

	fn balance_of(
		asset_id: AssetId,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost() * 2)?;

		// Read input.
		let mut input = handle.read_input()?;
		input.expect_arguments(1)?;

		let owner: H160 = input.read::<Address>()?.into();

		// Fetch info.
		// TODO Check staking balances
		let amount: U256 =
			<pallet_assets_ext::Pallet<Runtime> as Inspect<Runtime::AccountId>>::reducible_balance(
				asset_id,
				&owner.into(),
				false,
			)
			.into();

		// Build output.
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(amount).build(),
		})
	}

	fn transfer(
		asset_id: AssetId,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		let mut input = handle.read_input()?;
		input.expect_arguments(2)?;

		let to: H160 = input.read::<Address>()?.into();
		let amount: Balance = input.read::<U256>()?.saturated_into();

		let origin: Runtime::AccountId = handle.context().caller.into();
		let _ = <pallet_assets_ext::Pallet<Runtime> as TransferExt>::split_transfer(
			&origin,
			asset_id,
			&[(to.clone().into(), amount)],
		)
		.map_err(|e| revert(alloc::format!("Dispatched call failed with error: {:?}", e)))?;

		log3(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER,
			handle.context().caller,
			to,
			EvmDataWriter::new().write(amount).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(true).build(),
		})
	}

	fn name(asset_id: AssetId, handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Build output.
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output:
				EvmDataWriter::new()
					.write::<Bytes>(
						<pallet_assets_ext::Pallet<Runtime> as InspectMetadata<
							Runtime::AccountId,
						>>::name(&asset_id)
						.as_slice()
						.into(),
					)
					.build(),
		})
	}

	fn symbol(
		asset_id: AssetId,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Build output.
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output:
				EvmDataWriter::new()
					.write::<Bytes>(
						<pallet_assets_ext::Pallet<Runtime> as InspectMetadata<
							Runtime::AccountId,
						>>::symbol(&asset_id)
						.as_slice()
						.into(),
					)
					.build(),
		})
	}

	fn decimals(
		asset_id: AssetId,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Build output.
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new()
				.write::<u8>(<pallet_assets_ext::Pallet<Runtime> as InspectMetadata<
					Runtime::AccountId,
				>>::decimals(&asset_id))
				.build(),
		})
	}
}
