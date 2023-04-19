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

use fp_evm::{PrecompileFailure, PrecompileHandle, PrecompileOutput, PrecompileResult};
use pallet_erc20_peg::{types::WithdrawCallOrigin, WeightInfo as Erc20PegWeightInfo};
use pallet_evm::{GasWeightMapping, Precompile};
use pallet_nft_peg::WeightInfo as NftPegWeightInfo;
use precompile_utils::{
	constants::{ERC20_PRECOMPILE_ADDRESS_PREFIX, ERC721_PRECOMPILE_ADDRESS_PREFIX},
	prelude::*,
};
use seed_primitives::{AssetId, Balance, CollectionUuid, SerialNumber};
use sp_core::{H160, U256};
use sp_runtime::traits::SaturatedConversion;
use sp_std::{marker::PhantomData, vec::Vec};

/// Solidity selector of the Erc20Withdrawal log, which is the Keccak of the Log signature.
/// beneficiary, asset_id, amount
pub const SELECTOR_LOG_ERC20_WITHDRAWAL: [u8; 32] =
	keccak256!("Erc20Withdrawal(address,address,uint128)");

/// Solidity selector of the Erc721Withdrawal log, which is the Keccak of the Log signature.
/// beneficiary, collection_address, serial_numbers
pub const SELECTOR_LOG_ERC721_WITHDRAWAL: [u8; 32] =
	keccak256!("Erc721Withdrawal(address,address,uint32[])");

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	/// Withdraw an ERC20 token
	/// (beneficiary, asset, amount)
	Erc20Withdraw = "Erc20Withdraw(address,address,uint128)",
	/// Withdraw an ERC721 token
	/// (beneficiary, token_addresses+, serial_numbers)
	Erc721Withdraw = "Erc721Withdraw(address,address[],uint32[][])",
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
		let result = {
			let selector = match handle.read_selector() {
				Ok(selector) => selector,
				Err(e) => return Err(e.into()),
			};

			if let Err(err) = handle.check_function_modifier(FunctionModifier::NonPayable) {
				return Err(err.into())
			}

			match selector {
				Action::Erc20Withdraw => Self::erc20_withdraw(handle),
				Action::Erc721Withdraw => Self::erc721_withdraw(handle),
			}
		};
		return result
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
			return Err(revert("PEG: Expected balance <= 2^128").into())
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
				// Throw EVM log
				log3(
					handle.code_address(),
					SELECTOR_LOG_ERC20_WITHDRAWAL,
					beneficiary,
					H160::from(asset_address),
					EvmDataWriter::new().write(amount).build(),
				)
				.record(handle)?;

				Ok(succeed(EvmDataWriter::new().write(U256::from(event_proof_id)).build()))
			},
			Err(err) => Err(revert(
				alloc::format!("PEG: Erc20Withdraw failed {:?}", err.stripped())
					.as_bytes()
					.to_vec(),
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
		let collection_ids = collection_addresses
			.clone()
			.into_iter()
			.map(|address| {
				//let collection_address: H160 = address.into();
				let collection_id: CollectionUuid =
					<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
						address,
						ERC721_PRECOMPILE_ADDRESS_PREFIX,
					)
					.ok_or_else(|| revert("PEG: Invalid collection address"))?;
				Ok(collection_id)
			})
			.collect::<Result<Vec<CollectionUuid>, PrecompileFailure>>()?;

		// Parse serial_numbers
		let serial_numbers: Vec<Vec<SerialNumber>> = serial_numbers
			.into_iter()
			.map(|serial_number| {
				serial_number
					.into_iter()
					.map(|serial_number| {
						if serial_number > SerialNumber::MAX.into() {
							return Err(revert("PEG: Expected serial_number <= 2^128").into())
						}
						let serial_number: SerialNumber = serial_number.saturated_into();
						Ok(serial_number)
					})
					.collect::<Result<Vec<SerialNumber>, PrecompileFailure>>()
			})
			.collect::<Result<Vec<Vec<SerialNumber>>, PrecompileFailure>>()?;

		// Get caller
		let caller = Runtime::AccountId::from(handle.context().caller);

		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_nft_peg::Config>::NftPegWeightInfo::withdraw(),
		))?;

		// Dispatch call
		let maybe_event_proof_id = pallet_nft_peg::Pallet::<Runtime>::do_withdrawal(
			caller,
			collection_ids,
			serial_numbers.clone(),
			beneficiary,
		);

		// Handle error case
		if let Err(err) = maybe_event_proof_id {
			return Err(revert(
				alloc::format!("PEG: Erc721Withdraw failed {:?}", err.stripped())
					.as_bytes()
					.to_vec(),
			))
		};

		// throw individual log for every collection withdrawn
		for (collection_address, serial_numbers) in
			collection_addresses.into_iter().zip(serial_numbers)
		{
			log3(
				handle.code_address(),
				SELECTOR_LOG_ERC721_WITHDRAWAL,
				beneficiary,
				H160::from(collection_address),
				EvmDataWriter::new().write(serial_numbers).build(),
			)
			.record(handle)?;
		}

		let event_proof_id = maybe_event_proof_id.unwrap();
		Ok(succeed(EvmDataWriter::new().write(U256::from(event_proof_id)).build()))
	}
}
