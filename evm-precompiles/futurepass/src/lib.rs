#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use fp_evm::{Context, PrecompileHandle, PrecompileOutput, PrecompileResult, Transfer};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{AddressMapping, ExitReason, GasWeightMapping, Precompile, PrecompileFailure};
use precompile_utils::prelude::*;
use seed_primitives::CollectionUuid;
use sp_core::{H160, U256};
use sp_runtime::{
	codec::Decode,
	traits::{ConstU32, Zero},
};
use sp_std::marker::PhantomData;

/// Solidity selector of the Futurepass logs, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_FUTUREPASS_CREATED: [u8; 32] =
	keccak256!("FuturepassCreated(address,address)"); // futurepass, owner
pub const SELECTOR_LOG_FUTUREPASS_DELEGATE_REGISTERED: [u8; 32] =
	keccak256!("FuturepassDelegateRegistered(address,address)"); // futurepass, delegate
pub const SELECTOR_LOG_FUTUREPASS_DELEGATE_UNREGISTERED: [u8; 32] =
	keccak256!("FuturepassDelegateUnregistered(address,address)"); // futurepass, delegate

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	Create = "create(address)",
	RegisterDelegate = "registerDelegate(address,address)",
	UnRegisterDelegate = "unregisterDelegate(address,address)",
	ProxyCall = "proxyCall(address,address,bytes)",
	IsDelegate = "isDelegate(address,address)",
}

pub const CALL_DATA_LIMIT: u32 = 2u32.pow(16);

type GetCallDataLimit = ConstU32<CALL_DATA_LIMIT>;

pub struct EvmSubCall {
	pub to: Address,
	pub value: U256,
	pub call_data: BoundedBytes<ConstU32<CALL_DATA_LIMIT>>,
}

/// A trait to filter if an evm subcall is allowed to be executed by a proxy account.
/// This trait should be implemented by the `ProxyType` type configured in pallet proxy.
pub trait EvmProxyCallFilter: Sized + Send + Sync {
	/// If returns `false`, then the subcall will not be executed and the evm transaction will
	/// revert with error message "CallFiltered".
	fn is_evm_proxy_call_allowed(&self, _call: &EvmSubCall, _recipient_has_code: bool) -> bool {
		false
	}
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
	Runtime: frame_system::Config
		+ pallet_futurepass::Config
		+ pallet_evm::Config
		+ pallet_proxy::Config,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	<Runtime as pallet_proxy::Config>::ProxyType: Decode + EvmProxyCallFilter,
	<Runtime as frame_system::Config>::Call:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime as frame_system::Config>::Call: From<pallet_futurepass::Call<Runtime>>,
	<<Runtime as frame_system::Config>::Call as Dispatchable>::Origin:
		From<Option<Runtime::AccountId>>,
{
	fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
		let result = {
			let selector = match handle.read_selector() {
				Ok(selector) => selector,
				Err(e) => return Err(e.into()),
			};

			match selector {
				Action::Create => Self::create_futurepass(handle),
				Action::RegisterDelegate => Self::register_delegate(handle),
				Action::UnRegisterDelegate => Self::unregister_delegate(handle),
				Action::ProxyCall => Self::proxy_call(handle),
				Action::IsDelegate => Self::is_delegate(handle),
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
	Runtime: frame_system::Config
		+ pallet_futurepass::Config
		+ pallet_evm::Config
		+ pallet_proxy::Config,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	<Runtime as pallet_proxy::Config>::ProxyType: Decode + EvmProxyCallFilter,
	<Runtime as frame_system::Config>::Call:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime as frame_system::Config>::Call: From<pallet_futurepass::Call<Runtime>>,
	<<Runtime as frame_system::Config>::Call as Dispatchable>::Origin:
		From<Option<Runtime::AccountId>>,
{
	fn create_futurepass(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;
		read_args!(handle, { owner: Address });
		let owner: H160 = owner.into();

		//TODO(surangap):
		// Manually record gas
		// handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
		// 	<Runtime as pallet_futurepass::Config>::WeightInfo::create(),
		// ))?;
		let futurepass = pallet_futurepass::Pallet::<Runtime>::do_create_futurepass(owner.into());

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
				Ok(succeed([]))
			},
			Err(err) => Err(revert(
				alloc::format!("Futurepass: Futurepass creation failed {:?}", err)
					.as_bytes()
					.to_vec(),
			)),
		}
	}

	fn register_delegate(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;
		read_args!( handle, { futurepass: Address, delegate: Address}); // TODO(surangap): add access controlling once pallet has it
		let futurepass: H160 = futurepass.into();
		let delegate: H160 = delegate.into();
		let caller = handle.context().caller;

		//TODO(surangap):
		// Manually record gas
		// handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
		// 	<Runtime as pallet_futurepass::Config>::WeightInfo::register(),
		// ))?;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller.into()).into(),
			pallet_futurepass::Call::<Runtime>::register_delegate {
				futurepass: futurepass.into(),
				delegate: delegate.into(),
			},
		)?;

		log2(
			handle.code_address(),
			SELECTOR_LOG_FUTUREPASS_DELEGATE_REGISTERED,
			futurepass,
			EvmDataWriter::new().write(Address::from(delegate)).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn unregister_delegate(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;
		read_args!( handle, { futurepass: Address, delegate: Address}); // TODO(surangap): add access controlling once pallet has it
		let futurepass: H160 = futurepass.into();
		let delegate: H160 = delegate.into();
		let caller = handle.context().caller;

		//TODO(surangap):
		// Manually record gas
		// handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
		// 	<Runtime as pallet_futurepass::Config>::WeightInfo::unregister(),
		// ))?;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller.into()).into(),
			pallet_futurepass::Call::<Runtime>::unregister_delegate {
				futurepass: futurepass.into(),
				delegate: delegate.into(),
			},
		)?;

		log2(
			handle.code_address(),
			SELECTOR_LOG_FUTUREPASS_DELEGATE_UNREGISTERED,
			futurepass,
			EvmDataWriter::new().write(Address::from(delegate)).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn proxy_call(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		read_args!(handle, {
			real: Address,
			callTo: Address,
			callData: BoundedBytes<GetCallDataLimit>
		});
		let evm_subcall =
			EvmSubCall { to: callTo, call_data: callData, value: handle.context().apparent_value };

		Self::do_proxy(handle, real, evm_subcall)
	}

	fn is_delegate(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		read_args!(handle, {
			futurepass: Address,
			delegate: Address
		});
		let delegate = Runtime::AddressMapping::into_account_id(delegate.into());
		let futurepass = Runtime::AddressMapping::into_account_id(futurepass.into());

		// Manually record gas
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let is_proxy = pallet_proxy::Pallet::<Runtime>::proxies(futurepass)
			.0
			.iter()
			.any(|pd| pd.delegate == delegate);

		Ok(succeed(EvmDataWriter::new().write::<bool>(is_proxy).build()))
	}

	fn do_proxy(
		handle: &mut impl PrecompileHandle,
		real: Address,
		evm_subcall: EvmSubCall,
	) -> EvmResult<PrecompileOutput> {
		// Read proxy
		let real_account_id = Runtime::AddressMapping::into_account_id(real.clone().into());
		let who = Runtime::AddressMapping::into_account_id(handle.context().caller);
		// find proxy
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let def = pallet_proxy::Pallet::<Runtime>::find_proxy(&real_account_id, &who, None)
			.map_err(|_| RevertReason::custom("Not proxy"))?;
		frame_support::ensure!(def.delay.is_zero(), revert("Unannounced")); // no delay for futurepass

		// Read subcall recipient code
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let recipient_has_code =
			pallet_evm::AccountCodes::<Runtime>::decode_len(evm_subcall.to.0).unwrap_or(0) > 0;

		// Apply proxy type filter
		frame_support::ensure!(
			def.proxy_type.is_evm_proxy_call_allowed(&evm_subcall, recipient_has_code),
			revert("CallFiltered")
		);

		let EvmSubCall { to, value, call_data } = evm_subcall;
		let address = to.0;
		// build the sub context. here we switch the caller and address
		let sub_context =
			Context { caller: real.0, address: address.clone(), apparent_value: value };

		let transfer = if value.is_zero() {
			None
		} else {
			Some(Transfer { source: handle.context().caller, target: address.clone(), value })
		};

		let (reason, output) = handle.call(
			address,
			transfer,
			call_data.into_vec(),
			Some(handle.remaining_gas()),
			false,
			&sub_context,
		);

		// Return subcall result
		match reason {
			ExitReason::Fatal(exit_status) => Err(PrecompileFailure::Fatal { exit_status }),
			ExitReason::Revert(exit_status) =>
				Err(PrecompileFailure::Revert { exit_status, output }),
			ExitReason::Error(exit_status) => Err(PrecompileFailure::Error { exit_status }),
			ExitReason::Succeed(_) => Ok(succeed([])),
		}
	}
}
