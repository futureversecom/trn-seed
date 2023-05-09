#![cfg_attr(not(feature = "std"), no_std)]
#![allow(dead_code)]
extern crate alloc;

use fp_evm::{Context, PrecompileHandle, PrecompileOutput, PrecompileResult, Transfer};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	ensure,
};
use pallet_evm::{AddressMapping, ExitReason, PrecompileFailure, PrecompileSet};
use precompile_utils::{constants::FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX, prelude::*};
use seed_primitives::CollectionUuid;
use sp_core::{H160, U256};
use sp_runtime::{
	codec::Decode,
	traits::{ConstU32, Zero},
};
use sp_std::marker::PhantomData;

/// Solidity selector of the Futurepass logs, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_FUTUREPASS_DELEGATE_REGISTERED: [u8; 32] =
	keccak256!("FuturepassDelegateRegistered(address,address,uint8)"); // futurepass, delegate, proxyType
pub const SELECTOR_LOG_FUTUREPASS_DELEGATE_UNREGISTERED: [u8; 32] =
	keccak256!("FuturepassDelegateUnregistered(address,address)"); // futurepass, delegate

/// Solidity selector of the OwnershipTransferred log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_OWNERSHIP_TRANSFERRED: [u8; 32] =
	keccak256!("OwnershipTransferred(address,address)");

// evm proxy call type
#[derive(Debug, PartialEq)]
enum CallType {
	StaticCall,
	Call,
	DelegateCall, // Does not support in V1
	Create,       // Does not support in V1
	Create2,      // Does not support in V1
}

impl TryFrom<u8> for CallType {
	type Error = &'static str;
	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(CallType::StaticCall),
			1 => Ok(CallType::Call),
			2 => Ok(CallType::DelegateCall),
			3 => Ok(CallType::Create),
			4 => Ok(CallType::Create2),
			_ => Err("Invalid value for CallType"),
		}
	}
}

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	DelegateType = "delegateType(address)",
	RegisterDelegate = "registerDelegate(address,uint8)",
	UnRegisterDelegate = "unregisterDelegate(address)",
	ProxyCall = "proxyCall(uint8,address,uint256,bytes)",
	// Ownable - https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/access/Ownable.sol
	Owner = "owner()",
	RenounceOwnership = "renounceOwnership()",
	TransferOwnership = "transferOwnership(address)",
}

pub const CALL_DATA_LIMIT: u32 = 2u32.pow(16);

type GetCallDataLimit = ConstU32<CALL_DATA_LIMIT>;

pub struct EvmSubCall {
	pub to: Address,
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

/// Provides access to the Futurepass pallet
pub struct FuturePassPrecompileSet<Runtime>(PhantomData<Runtime>);

impl<T> Default for FuturePassPrecompileSet<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> PrecompileSet for FuturePassPrecompileSet<Runtime>
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
	<Runtime as pallet_futurepass::Config>::ProxyType: TryFrom<u8>,
	<Runtime as pallet_proxy::Config>::ProxyType: TryInto<u8>,
{
	fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
		let futurepass = Address(handle.code_address());

		let result = {
			let selector = match handle.read_selector() {
				Ok(selector) => selector,
				Err(e) => return Some(Err(e.into())),
			};

			match selector {
				Action::DelegateType => Self::delegate_type(futurepass, handle),
				Action::RegisterDelegate => Self::register_delegate(futurepass, handle),
				Action::UnRegisterDelegate => Self::unregister_delegate(futurepass, handle),
				Action::ProxyCall => Self::proxy_call(futurepass, handle),
				// Ownable
				Action::Owner => Self::owner(futurepass, handle),
				Action::RenounceOwnership => Self::renounce_ownership(handle),
				Action::TransferOwnership => Self::transfer_ownership(handle),
			}
		};
		return Some(result)
	}

	fn is_precompile(&self, address: H160) -> bool {
		// TODO - check if we need to verify whether the address is a futurepass
		address.as_bytes().starts_with(FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX)
	}
}

impl<Runtime> FuturePassPrecompileSet<Runtime> {
	pub fn new() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> FuturePassPrecompileSet<Runtime>
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
	<Runtime as pallet_futurepass::Config>::ProxyType: TryFrom<u8>,
	<Runtime as pallet_proxy::Config>::ProxyType: TryInto<u8>,
{
	fn delegate_type(
		futurepass: Address,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		read_args!(handle, { delegate: Address });
		let futurepass = Runtime::AddressMapping::into_account_id(futurepass.into());
		let delegate = Runtime::AddressMapping::into_account_id(delegate.into());

		// Manually record gas
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		let mut proxy_type: u8 = 0; // ProxyType.NoPermission
		if let Some(proxy_def) = pallet_proxy::Pallet::<Runtime>::proxies(futurepass)
			.0
			.iter()
			.find(|pd| pd.delegate == delegate)
		{
			proxy_type =
				proxy_def.proxy_type.clone().try_into().map_err(|_e| {
					RevertReason::custom("Futurepass: ProxyType conversion failure")
				})?;
		}

		Ok(succeed(EvmDataWriter::new().write::<u8>(proxy_type).build()))
	}

	fn register_delegate(
		futurepass: Address,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;
		read_args!( handle, { delegate: Address, proxy_type: u8});
		let futurepass: H160 = futurepass.into();
		let delegate: H160 = delegate.into();
		let proxy_type_enum: <Runtime as pallet_futurepass::Config>::ProxyType = proxy_type
			.try_into()
			.map_err(|_e| RevertReason::custom("Futurepass: ProxyType conversion failure"))?;

		let caller = handle.context().caller;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller.into()).into(),
			pallet_futurepass::Call::<Runtime>::register_delegate {
				futurepass: futurepass.into(),
				delegate: delegate.into(),
				proxy_type: proxy_type_enum,
			},
		)?;

		log3(
			handle.code_address(),
			SELECTOR_LOG_FUTUREPASS_DELEGATE_REGISTERED,
			futurepass,
			delegate,
			EvmDataWriter::new().write(proxy_type).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn unregister_delegate(
		futurepass: Address,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;
		read_args!(handle, { delegate: Address });
		let futurepass: H160 = futurepass.into();
		let delegate: H160 = delegate.into();
		let caller = handle.context().caller;

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

	fn proxy_call(
		futurepass: Address,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		read_args!(handle, {
			call_type: u8,
			call_to: Address,
			value: U256,
			call_data: BoundedBytes<GetCallDataLimit>
		});
		let call_type: CallType = call_type
			.try_into()
			.map_err(|err| RevertReason::custom(alloc::format!("Futurepass: {}", err)))?;
		let evm_subcall = EvmSubCall { to: call_to, call_data };

		Self::do_proxy(handle, futurepass, call_type, evm_subcall, value)
	}

	fn do_proxy(
		handle: &mut impl PrecompileHandle,
		futurepass: Address,
		call_type: CallType,
		evm_subcall: EvmSubCall,
		value: U256,
	) -> EvmResult<PrecompileOutput> {
		// Read proxy
		let futurepass_account_id =
			Runtime::AddressMapping::into_account_id(futurepass.clone().into());
		let who = Runtime::AddressMapping::into_account_id(handle.context().caller);
		// find proxy
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let def = pallet_proxy::Pallet::<Runtime>::find_proxy(&futurepass_account_id, &who, None)
			.map_err(|_| RevertReason::custom("Futurepass: Not proxy"))?;
		ensure!(def.delay.is_zero(), revert("Futurepass: Unannounced")); // no delay for futurepass

		// Read subcall recipient code
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let recipient_has_code =
			pallet_evm::AccountCodes::<Runtime>::decode_len(evm_subcall.to.0).unwrap_or(0) > 0;

		// Apply proxy type filter
		ensure!(
			def.proxy_type.is_evm_proxy_call_allowed(&evm_subcall, recipient_has_code),
			revert("Futurepass: CallFiltered")
		);

		let EvmSubCall { to, call_data } = evm_subcall;
		let address = to.0;

		// build the sub context. here we update the caller to the futurepass.
		// We also update the value for the sub call and for the transfer to match the "value" input
		// parameter
		let sub_context =
			Context { caller: futurepass.0, address: address.clone(), apparent_value: value };

		let transfer = if value.is_zero() {
			None
		} else {
			// Transfer should happen from the futurepass and the value should be equal to the
			// "value" input parameter.
			Some(Transfer { source: futurepass.0, target: address.clone(), value })
		};

		let (reason, output) = match call_type {
			CallType::StaticCall => handle.call(
				address,
				transfer,
				call_data.into_vec(),
				Some(handle.remaining_gas()),
				true,
				&sub_context,
			),
			CallType::Call => handle.call(
				address,
				transfer,
				call_data.into_vec(),
				Some(handle.remaining_gas()),
				false,
				&sub_context,
			),
			CallType::DelegateCall | CallType::Create | CallType::Create2 =>
				Err(RevertReason::custom("Futurepass: call type not supported"))?,
		};

		// Return subcall result
		match reason {
			ExitReason::Fatal(exit_status) => Err(PrecompileFailure::Fatal { exit_status }),
			ExitReason::Revert(exit_status) =>
				Err(PrecompileFailure::Revert { exit_status, output }),
			ExitReason::Error(exit_status) => Err(PrecompileFailure::Error { exit_status }),
			ExitReason::Succeed(_) => Ok(succeed([])),
		}
	}

	fn owner(
		futurepass: Address,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		let futurepass: H160 = futurepass.into();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let owner = pallet_futurepass::Holders::<Runtime>::iter()
			.find_map(|(holder, fp)| {
				let holder: H160 = holder.into();
				if futurepass == fp.into() {
					return Some(holder)
				}
				None
			})
			.unwrap_or(H160::default());

		Ok(succeed(EvmDataWriter::new().write(Address::from(owner)).build()))
	}

	fn renounce_ownership(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		let caller = handle.context().caller;
		let burn_account: H160 = H160::default();

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller.into()).into(),
			pallet_futurepass::Call::<Runtime>::transfer_futurepass { new_owner: None },
		)?;

		// emit OwnershipTransferred(address,address) event
		log2(
			handle.code_address(),
			SELECTOR_LOG_OWNERSHIP_TRANSFERRED,
			caller,
			EvmDataWriter::new().write(Address::from(burn_account)).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn transfer_ownership(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		// Parse input.
		read_args!(handle, { new_owner: Address });
		let new_owner: H160 = new_owner.into();
		let caller = handle.context().caller;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller.into()).into(),
			pallet_futurepass::Call::<Runtime>::transfer_futurepass {
				new_owner: Some(new_owner.into()),
			},
		)?;

		// emit OwnershipTransferred(address,address) event
		log2(
			handle.code_address(),
			SELECTOR_LOG_OWNERSHIP_TRANSFERRED,
			caller,
			EvmDataWriter::new().write(Address::from(new_owner)).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}
}
