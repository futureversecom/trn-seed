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
use pallet_erc20_peg::{types::WithdrawCallOrigin, WeightInfo as Erc20PegWeightInfo};
use pallet_evm::{GasWeightMapping, Precompile};
use pallet_nft_peg::WeightInfo as NftPegWeightInfo;
use precompile_utils::{
	constants::{ERC20_PRECOMPILE_ADDRESS_PREFIX, ERC721_PRECOMPILE_ADDRESS_PREFIX},
	prelude::*,
};
use seed_primitives::{AssetId, Balance, CollectionUuid, SerialNumber};
use sp_core::{H160, H256, U256};
use sp_runtime::{traits::SaturatedConversion, BoundedVec};
use sp_std::{marker::PhantomData, vec::Vec};

/// Solidity selector of the Erc20Withdrawal log, which is the Keccak of the Log signature.
/// event_proof_id, beneficiary, asset_id, amount
pub const SELECTOR_LOG_ERC20_WITHDRAWAL: [u8; 32] =
	keccak256!("Erc20Withdrawal(uint64,address,address,uint128)");

/// Solidity selector of the Erc721Withdrawal log, which is the Keccak of the Log signature.
/// event_proof_id, beneficiary, collection_address, serial_numbers
pub const SELECTOR_LOG_ERC721_WITHDRAWAL: [u8; 32] =
	keccak256!("Erc721Withdrawal(uint64,address,address,uint32[])");

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	/// Withdraw an ERC20 token
	/// (beneficiary, asset, amount)
	Erc20Withdraw = "erc20Withdraw(address,address,uint128)",
	/// Withdraw an ERC721 token
	/// (beneficiary, token_addresses+, serial_numbers)
	Erc721Withdraw = "erc721Withdraw(address,address[],uint32[][])",
}

/// Provides access to the peg pallets
pub struct PegPrecompile<Runtime>(PhantomData<Runtime>);

impl<T> Default for PegPrecompile<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> Precompile for PegPrecompile<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: frame_system::Config
		+ pallet_nft_peg::Config
		+ pallet_erc20_peg::Config
		+ pallet_evm::Config,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>
		+ ErcIdConversion<AssetId, EvmId = Address>,
{
	fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
		let selector = match handle.read_selector() {
			Ok(selector) => selector,
			Err(e) => return Err(e.into()),
		};

		if let Err(err) = handle.check_function_modifier(FunctionModifier::NonPayable) {
			return Err(err.into());
		}

		match selector {
			Action::Erc20Withdraw => Self::erc20_withdraw(handle),
			Action::Erc721Withdraw => Self::erc721_withdraw(handle),
		}
	}
}

impl<Runtime> PegPrecompile<Runtime> {
	pub fn new() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> PegPrecompile<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: frame_system::Config
		+ pallet_nft_peg::Config
		+ pallet_erc20_peg::Config
		+ pallet_evm::Config,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>
		+ ErcIdConversion<AssetId, EvmId = Address>,
{
	fn erc20_withdraw(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				beneficiary: Address,
				asset_address: Address,
				amount: U256
			}
		);

		// Parse beneficiary
		let beneficiary: H160 = beneficiary.into();
		// Parse asset_id
		let asset_id: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			asset_address,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("PEG: Invalid asset address"))?;
		// Parse balance
		if amount > Balance::MAX.into() {
			return Err(revert("PEG: Expected balance <= 2^128"));
		}
		let amount: Balance = amount.saturated_into();

		let caller = Runtime::AccountId::from(handle.context().caller);

		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_erc20_peg::Config>::WeightInfo::withdraw(),
		))?;

		// Dispatch call
		let maybe_event_proof_id = pallet_erc20_peg::Pallet::<Runtime>::do_withdrawal(
			caller,
			asset_id,
			amount,
			beneficiary,
			WithdrawCallOrigin::Evm,
		);

		// Build output.
		match maybe_event_proof_id {
			Ok(event_proof_id) => {
				// This should always be Some(id), but let's check for safety
				let event_proof_id = event_proof_id
					.ok_or(revert("PEG: Erc20Withdraw failed: no event proof id returned"))?;

				// Throw EVM log
				log4(
					handle.code_address(),
					SELECTOR_LOG_ERC20_WITHDRAWAL,
					H256::from_low_u64_be(event_proof_id),
					beneficiary,
					H160::from(asset_address),
					EvmDataWriter::new().write(amount).build(),
				)
				.record(handle)?;

				Ok(succeed(EvmDataWriter::new().write(U256::from(event_proof_id)).build()))
			},
			Err(err) => Err(revert(
				alloc::format!("PEG: Erc20Withdraw failed {:?}", err.stripped()).as_bytes(),
			)),
		}
	}

	fn erc721_withdraw(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				beneficiary: Address,
				collection_addresses: Vec<Address>,
				serial_numbers: Vec<Vec<U256>>
			}
		);

		// Parse beneficiary
		let beneficiary: H160 = beneficiary.into();

		// Parse collection_ids
		let collection_ids_unbounded = collection_addresses
			.clone()
			.into_iter()
			.map(|address| {
				let collection_id: CollectionUuid =
					<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
						address,
						ERC721_PRECOMPILE_ADDRESS_PREFIX,
					)
					.ok_or_else(|| revert("PEG: Invalid collection address"))?;
				Ok(collection_id)
			})
			.collect::<Result<Vec<CollectionUuid>, PrecompileFailure>>()?;

		// Bound collection_ids
		let collection_ids: BoundedVec<CollectionUuid, Runtime::MaxCollectionsPerWithdraw> =
			BoundedVec::try_from(collection_ids_unbounded)
				.map_err(|_| revert("PEG: Too many collections"))?;

		// Parse serial_numbers
		let serials_unbounded: Vec<BoundedVec<SerialNumber, Runtime::MaxSerialsPerWithdraw>> =
			serial_numbers
				.into_iter()
				.map(|serial_numbers| Self::bound_serial_numbers(serial_numbers))
				.collect::<Result<
					Vec<BoundedVec<SerialNumber, Runtime::MaxSerialsPerWithdraw>>,
					PrecompileFailure,
				>>()?;

		// Bound outer serial vec
		let serial_numbers: BoundedVec<
			BoundedVec<SerialNumber, Runtime::MaxSerialsPerWithdraw>,
			Runtime::MaxCollectionsPerWithdraw,
		> = BoundedVec::try_from(serials_unbounded)
			.map_err(|_| revert("PEG: Too many collections"))?;

		// Get caller
		let caller = Runtime::AccountId::from(handle.context().caller);

		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_nft_peg::Config>::NftPegWeightInfo::withdraw(),
		))?;

		// Dispatch call
		let event_proof_id = pallet_nft_peg::Pallet::<Runtime>::do_withdrawal(
			caller,
			collection_ids,
			serial_numbers.clone(),
			beneficiary,
			None,
		)
		.map_err(|e| {
			revert(alloc::format!("PEG: Erc721Withdraw failed {:?}", e.stripped()).as_bytes())
		})?;

		// throw individual log for every collection withdrawn
		for (collection_address, serial_numbers) in
			collection_addresses.into_iter().zip(serial_numbers)
		{
			log4(
				handle.code_address(),
				SELECTOR_LOG_ERC721_WITHDRAWAL,
				H256::from_low_u64_be(event_proof_id),
				beneficiary,
				H160::from(collection_address),
				EvmDataWriter::new().write(serial_numbers.into_inner()).build(),
			)
			.record(handle)?;
		}

		Ok(succeed(EvmDataWriter::new().write(U256::from(event_proof_id)).build()))
	}

	// Convert a vector of U256 serial numbers into a bounded vector of serial numbers
	fn bound_serial_numbers(
		serial_numbers: Vec<U256>,
	) -> Result<BoundedVec<SerialNumber, Runtime::MaxSerialsPerWithdraw>, PrecompileFailure> {
		let serials_unbounded = serial_numbers
			.into_iter()
			.map(|serial_number| {
				if serial_number > SerialNumber::MAX.into() {
					return Err(revert("PEG: Expected serial_number <= 2^128"));
				}
				let serial_number: SerialNumber = serial_number.saturated_into();
				Ok(serial_number)
			})
			.collect::<Result<Vec<SerialNumber>, PrecompileFailure>>()?;

		BoundedVec::try_from(serials_unbounded).map_err(|_| revert("PEG: Too many serial numbers"))
	}
}
