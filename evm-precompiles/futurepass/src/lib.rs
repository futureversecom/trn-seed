#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use fp_evm::{PrecompileHandle, PrecompileOutput, PrecompileResult};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{GasWeightMapping, Precompile};
use precompile_utils::{constants::ERC721_PRECOMPILE_ADDRESS_PREFIX, prelude::*};
use seed_primitives::{CollectionUuid, MetadataScheme};
use sp_core::{H160, U256};
use sp_runtime::{traits::SaturatedConversion, Permill};
use sp_std::{marker::PhantomData, vec::Vec};

/// Solidity selector of the FuturepassCreated log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_FUTUREPASS_CREATED: [u8; 32] = keccak256!("FuturepassCreated(address,address)"); // futurepass, owner

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	Create = "create(address)",
	Register = "register(address, address)",
	UnRegister = "unregister(address, address)",
	Proxy = "proxy(address, address, bytes)",
}

/// Provides access to the NFT pallet
pub struct FuturePassPrecompile<Runtime>(PhantomData<Runtime>);

impl<T> Default for FuturePassPrecompile<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> Precompile for FuturePassPrecompile<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: frame_system::Config + pallet_futurepass::Config + pallet_evm::Config,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::Call: From<pallet_futurepass::Call<Runtime>>,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
{
	fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
		let result = {
			let selector = match handle.read_selector() {
				Ok(selector) => selector,
				Err(e) => return Err(e.into()),
			};

			// TODO(surangap): enable modifier check
			// if let Err(err) = handle.check_function_modifier(FunctionModifier::NonPayable) {
			// 	return Err(err.into())
			// }

			match selector {
				Action::Create => Self::create_futurepass(handle),
				Action::Register => Self::register_delegate(handle),
				Action::UnRegister => Self::unregister_delegate(handle),
				Action::Proxy => Self::proxy(handle),
			}
		};
		return result
	}
}

impl<Runtime> FuturePassPrecompile<Runtime> {
	pub fn new() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> FuturePassPrecompile<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: frame_system::Config + pallet_futurepass::Config + pallet_evm::Config,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::Call: From<pallet_futurepass::Call<Runtime>>,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
{
	fn create_futurepass(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;
		// Parse input.
		read_args!( handle, { owner: Address});
		let owner: H160 = owner.into();

		//TODO(surangap):
		// Manually record gas
		// handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
		// 	<Runtime as pallet_futurepass::Config>::WeightInfo::create(),
		// ))?;
		let futurepass =  pallet_futurepass::Pallet::<Runtime>::do_create_futurepass(owner.into());

		match futurepass {
			Ok(futurepass_id) => {
				let futurepass_id : H160 = futurepass_id.into();

				log2(
					handle.code_address(),
					SELECTOR_LOG_FUTUREPASS_CREATED,
					futurepass_id,
					EvmDataWriter::new().write(Address::from(owner)).build(),
				)
				.record(handle)?;

				// Build output.
				Ok(succeed([]))
			},
			Err(err) => Err(revert(
				alloc::format!("Futurepass: Futurepass creation failed {:?}", err.stripped())
					.as_bytes()
					.to_vec(),
			)),
		}
	}

	fn register_delegate(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> { todo!() }
	fn unregister_delegate(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> { todo!() }
	fn proxy(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> { todo!() }
}
