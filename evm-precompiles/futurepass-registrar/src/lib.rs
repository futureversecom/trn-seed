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
#![allow(dead_code)]
extern crate alloc;

use fp_evm::{PrecompileHandle, PrecompileOutput, PrecompileResult};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{AddressMapping, GasWeightMapping, Precompile};
use pallet_futurepass::WeightInfo;
use precompile_utils::prelude::*;
use seed_primitives::CollectionUuid;
use sp_core::H160;
use sp_std::marker::PhantomData;

/// Solidity selector of the Futurepass logs, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_FUTUREPASS_CREATED: [u8; 32] =
	keccak256!("FuturepassCreated(address,address)"); // futurepass, owner

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	FuturepassOf = "futurepassOf(address)",
	Create = "create(address)",
}

/// Provides access to the Futurepass pallet
pub struct FuturePassRegistrarPrecompile<Runtime>(PhantomData<Runtime>);

impl<T> Default for FuturePassRegistrarPrecompile<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> Precompile for FuturePassRegistrarPrecompile<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: frame_system::Config + pallet_futurepass::Config + pallet_evm::Config,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	<Runtime as frame_system::Config>::RuntimeCall:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_futurepass::Call<Runtime>>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<Runtime::AccountId>>,
{
	fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
		let selector = match handle.read_selector() {
			Ok(selector) => selector,
			Err(e) => return Err(e.into()),
		};
		if let Err(err) = handle.check_function_modifier(match selector {
			Action::Create => FunctionModifier::NonPayable,
			Action::FuturepassOf => FunctionModifier::View,
		}) {
			return Err(err.into());
		}

		match selector {
			Action::FuturepassOf => Self::futurepass_of(handle),
			Action::Create => Self::create_futurepass(handle),
		}
	}
}

impl<Runtime> FuturePassRegistrarPrecompile<Runtime> {
	pub fn new() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> FuturePassRegistrarPrecompile<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: frame_system::Config + pallet_futurepass::Config + pallet_evm::Config,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	<Runtime as frame_system::Config>::RuntimeCall:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_futurepass::Call<Runtime>>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<Runtime::AccountId>>,
{
	fn futurepass_of(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		read_args!(handle, { owner: Address });
		let owner = Runtime::AddressMapping::into_account_id(owner.into());

		// Manually record gas
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let futurepass: H160 = pallet_futurepass::Holders::<Runtime>::get(owner)
			.map(|fp| fp.into())
			.unwrap_or_default();

		Ok(succeed(EvmDataWriter::new().write::<Address>(futurepass.into()).build()))
	}

	fn create_futurepass(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;
		read_args!(handle, { owner: Address });
		let owner: H160 = owner.into();

		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_futurepass::Config>::WeightInfo::create(),
		))?;
		let futurepass = pallet_futurepass::Pallet::<Runtime>::do_create_futurepass(
			handle.context().caller.into(),
			owner.into(),
		);

		match futurepass {
			Ok(futurepass_id) => {
				let futurepass_id: H160 = futurepass_id.into();

				log2(
					handle.code_address(),
					SELECTOR_LOG_FUTUREPASS_CREATED,
					futurepass_id,
					EvmDataWriter::new().write(Address::from(owner)).build(),
				)
				.record(handle)?;

				// Build output.
				Ok(succeed(EvmDataWriter::new().write(Address::from(futurepass_id)).build()))
			},
			Err(err) => Err(revert(
				alloc::format!("Futurepass Registrar: Futurepass creation failed {:?}", err)
					.as_bytes(),
			)),
		}
	}
}
