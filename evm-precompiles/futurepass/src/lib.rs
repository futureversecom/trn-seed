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

use fp_evm::{
	Context, IsPrecompileResult, PrecompileHandle, PrecompileOutput, PrecompileResult, Transfer,
};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	ensure,
};
use pallet_evm::{ExitReason, GasWeightMapping, PrecompileFailure, PrecompileSet, Runner};
use pallet_futurepass::ProxyProvider;
use precompile_utils::{
	constants::FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX, data::Bytes32PostPad, get_selector, prelude::*,
};
use sp_core::{H160, H256, U256};
use sp_runtime::{
	codec::Decode,
	traits::{ConstU32, Zero},
};
use sp_std::{marker::PhantomData, vec};

/// Solidity selector of the Futurepass logs, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_FUTUREPASS_DELEGATE_REGISTERED: [u8; 32] =
	keccak256!("FuturepassDelegateRegistered(address,address,uint8)"); // futurepass, delegate, proxyType
pub const SELECTOR_LOG_FUTUREPASS_DELEGATE_UNREGISTERED: [u8; 32] =
	keccak256!("FuturepassDelegateUnregistered(address,address)"); // futurepass, delegate
pub const SELECTOR_LOG_FUTUREPASS_EXECUTED: [u8; 32] =
	keccak256!("Executed(uint8,address,uint256,bytes4)"); // operation, contractAddress, value, functionSelector
pub const SELECTOR_LOG_FUTUREPASS_CONTRACT_CREATED: [u8; 32] =
	keccak256!("ContractCreated(uint8,address,uint256,bytes32)"); // operation, contractAddress, value, salt

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

impl From<CallType> for u8 {
	fn from(value: CallType) -> Self {
		match value {
			CallType::StaticCall => 0,
			CallType::Call => 1,
			CallType::DelegateCall => 2,
			CallType::Create => 3,
			CallType::Create2 => 4,
		}
	}
}

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	Default = "",
	DelegateType = "delegateType(address)",
	RegisterDelegateWithSignature = "registerDelegateWithSignature(address,uint8,uint32,bytes)",
	UnRegisterDelegate = "unregisterDelegate(address)",
	ProxyCall = "proxyCall(uint8,address,uint256,bytes)",
	// Ownable - https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/access/Ownable.sol
	Owner = "owner()",
	RenounceOwnership = "renounceOwnership()",
	TransferOwnership = "transferOwnership(address)",
}

pub const CALL_DATA_LIMIT: u32 = 2u32.pow(16);
type GetCallDataLimit = ConstU32<CALL_DATA_LIMIT>;

pub const SIGNATURE_LENGTH: u32 = 65;
type GetSignatureLimit = ConstU32<SIGNATURE_LENGTH>;

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
	<Runtime as pallet_proxy::Config>::ProxyType: Decode + EvmProxyCallFilter,
	<Runtime as frame_system::Config>::RuntimeCall:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_futurepass::Call<Runtime>>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<Runtime::AccountId>>,
	<Runtime as pallet_futurepass::Config>::ProxyType: TryFrom<u8>,
	<Runtime as pallet_proxy::Config>::ProxyType: Into<u8>,
{
	fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
		let result = {
			let selector = match handle.read_selector() {
				Ok(selector) => selector,
				Err(e) => {
					if handle.input().is_empty() && !handle.context().apparent_value.is_zero() {
						// This is the default receive function for the futurepass
						Action::Default
					} else {
						return Some(Err(e.into()));
					}
				},
			};

			match selector {
				Action::Default => Self::receive(handle),
				Action::DelegateType => Self::delegate_type(handle),
				Action::RegisterDelegateWithSignature => {
					Self::register_delegate_with_signature(handle)
				},
				Action::UnRegisterDelegate => Self::unregister_delegate(handle),
				Action::ProxyCall => Self::proxy_call(handle),
				// Ownable
				Action::Owner => Self::owner(handle),
				Action::RenounceOwnership => Self::renounce_ownership(handle),
				Action::TransferOwnership => Self::transfer_ownership(handle),
			}
		};
		Some(result)
	}

	fn is_precompile(&self, address: H160, _remaining_gas: u64) -> IsPrecompileResult {
		// check if the address starts with futurepass precompile prefix
		IsPrecompileResult::Answer {
			is_precompile: address.as_bytes().starts_with(FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX),
			extra_cost: 0,
		}
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
	<Runtime as pallet_proxy::Config>::ProxyType: Decode + EvmProxyCallFilter,
	<Runtime as frame_system::Config>::RuntimeCall:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_futurepass::Call<Runtime>>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<Runtime::AccountId>>,
	<Runtime as pallet_futurepass::Config>::ProxyType: TryFrom<u8>,
	<Runtime as pallet_proxy::Config>::ProxyType: Into<u8>,
{
	fn delegate_type(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		read_args!(handle, { delegate: Address });
		let futurepass: Runtime::AccountId = handle.code_address().into();
		let delegate: H160 = delegate.into();

		// Manually record gas
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		let mut proxy_type: u8 = 0; // ProxyType.NoPermission
		if let Some(proxy_def) = pallet_proxy::Pallet::<Runtime>::proxies(futurepass)
			.0
			.iter()
			.find(|pd| pd.delegate == delegate.into())
		{
			proxy_type = proxy_def.clone().proxy_type.into();
		}

		Ok(succeed(EvmDataWriter::new().write::<u8>(proxy_type).build()))
	}

	fn register_delegate_with_signature(
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;
		read_args!( handle, { delegate: Address, proxy_type: u8, deadline: u32, signature: BoundedBytes<GetSignatureLimit> });
		let futurepass: H160 = handle.code_address();
		let delegate: H160 = delegate.into();
		let proxy_type_enum: <Runtime as pallet_futurepass::Config>::ProxyType = proxy_type
			.try_into()
			.map_err(|_e| RevertReason::custom("Futurepass: ProxyType conversion failure"))?;
		let signature: [u8; 65] = signature
			.inner
			.try_into()
			.map_err(|_e| RevertReason::custom("Futurepass: Signature length mismatch"))?;

		let caller = handle.context().caller;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller.into()).into(),
			pallet_futurepass::Call::<Runtime>::register_delegate_with_signature {
				futurepass: futurepass.into(),
				delegate: delegate.into(),
				proxy_type: proxy_type_enum,
				deadline,
				signature,
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

	fn unregister_delegate(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;
		read_args!(handle, { delegate: Address });
		let futurepass: H160 = handle.code_address();
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

	fn proxy_call(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		read_args!(handle, {
			call_type: u8,
			call_to: Address,
			value: U256,
			call_data: BoundedBytes<GetCallDataLimit>
		});
		let call_type: CallType = call_type
			.try_into()
			.map_err(|err| RevertReason::custom(alloc::format!("Futurepass: {}", err)))?;

		// restrict delegate access to whitelist
		if call_to.0.as_bytes().starts_with(FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX) {
			let sub_call_selector = &call_data.inner[..4];
			if sub_call_selector
				== &keccak256!("registerDelegateWithSignature(address,uint8,uint32,bytes)")[..4]
				|| sub_call_selector == &keccak256!("unregisterDelegate(address)")[..4]
				|| sub_call_selector == &keccak256!("transferOwnership(address)")[..4]
			{
				let futurepass: H160 = handle.code_address();
				let caller: H160 = handle.context().caller;
				ensure!(
					pallet_futurepass::Holders::<Runtime>::get(Runtime::AccountId::from(caller))
						== Some(futurepass.into()),
					revert("Futurepass: NotFuturepassOwner")
				);
			}
		}

		let evm_subcall = EvmSubCall { to: call_to, call_data };
		Self::do_proxy(handle, handle.code_address(), call_type, evm_subcall, value)
	}

	fn receive(_handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		// do nothing
		Ok(succeed([]))
	}

	fn do_proxy(
		handle: &mut impl PrecompileHandle,
		futurepass: H160,
		call_type: CallType,
		evm_subcall: EvmSubCall,
		value: U256,
	) -> EvmResult<PrecompileOutput> {
		// Read proxy
		let futurepass_account_id = futurepass.into();
		let who = handle.context().caller.into();
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
			Context { caller: futurepass, address, apparent_value: value };

		let transfer = if value.is_zero() {
			None
		} else {
			// Transfer should happen from the futurepass and the value should be equal to the
			// "value" input parameter.
			Some(Transfer { source: futurepass, target: address, value })
		};

		let (reason, output) = match call_type {
			CallType::StaticCall => {
				handle.record_log_costs_manual(4, 32)?;
				let call_data = call_data.into_vec();
				let (reason, output) = handle.call(
					address,
					transfer,
					call_data.clone(),
					Some(handle.remaining_gas()),
					true,
					&sub_context,
				);

				// emit Executed(CALL, target, value, bytes4(data));
				log4(
					handle.code_address(),
					SELECTOR_LOG_FUTUREPASS_EXECUTED,
					H256::from_low_u64_be(<CallType as Into<u8>>::into(call_type).into()),
					address,
					H256::from_slice(&Into::<[u8; 32]>::into(value)),
					EvmDataWriter::new()
						.write(Bytes32PostPad::from(get_selector(&call_data).as_slice()))
						.build(),
				)
				.record(handle)?;

				(reason, output)
			},
			CallType::Call => {
				handle.record_log_costs_manual(4, 32)?;
				let call_data = call_data.into_vec();
				let (reason, output) = handle.call(
					address,
					transfer,
					call_data.clone(),
					Some(handle.remaining_gas()),
					false,
					&sub_context,
				);

				// emit Executed(CALL, target, value, bytes4(data));
				log4(
					handle.code_address(),
					SELECTOR_LOG_FUTUREPASS_EXECUTED,
					H256::from_low_u64_be(<CallType as Into<u8>>::into(call_type).into()),
					address,
					H256::from_slice(&Into::<[u8; 32]>::into(value)),
					EvmDataWriter::new()
						.write(Bytes32PostPad::from(get_selector(&call_data).as_slice()))
						.build(),
				)
				.record(handle)?;

				(reason, output)
			},
			CallType::Create => {
				handle.record_log_costs_manual(4, 32)?;

				let is_transactional = false;
				let validate = true;
				let evm_config = <Runtime as pallet_evm::Config>::config();
				// TODO: refactor the code once we are on polkadot-v1.1.0, ref - https://github.com/polkadot-evm/frontier/pull/1121
				let estimated_transaction_len = call_data.inner.len() +
					20 + // from
					32 + // value
					32 + // gas_limit
					32 + // nonce
					1 + // TransactionAction
					8 + // chain id
					65; // signature

				let gas_limit = handle.remaining_gas().min(u64::MAX);
				let without_base_extrinsic_weight = true;

				let (weight_limit, proof_size_base_cost) =
					match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
						gas_limit,
						without_base_extrinsic_weight,
					) {
						weight_limit if weight_limit.proof_size() > 0 => {
							(Some(weight_limit), Some(estimated_transaction_len as u64))
						},
						_ => (None, None),
					};
				let execution_info = <Runtime as pallet_evm::Config>::Runner::create(
					futurepass,
					call_data.into_vec(),
					value,
					gas_limit,
					None,
					None,
					None, // handled by EVM
					alloc::vec![],
					is_transactional,
					validate,
					weight_limit,
					proof_size_base_cost,
					evm_config,
				)
				.map_err(|_| RevertReason::custom("Futurepass: create failed"))?;

				// emit ContractCreated(CREATE, contractAddress, value, bytes32(0));
				log4(
					handle.code_address(),
					SELECTOR_LOG_FUTUREPASS_CONTRACT_CREATED,
					H256::from_low_u64_be(<CallType as Into<u8>>::into(call_type).into()),
					execution_info.value,
					H256::from_slice(&Into::<[u8; 32]>::into(value)),
					EvmDataWriter::new().write(U256::zero()).build(),
				)
				.record(handle)?;

				(execution_info.exit_reason, execution_info.value.to_fixed_bytes().to_vec())
			},
			CallType::Create2 => {
				handle.record_log_costs_manual(4, 32)?;

				let creation_code_len = call_data.inner.len();
				let call_data_vec = call_data.into_vec();

				// salt is the last 32 bytes of the creation code
				// source: https://github.com/ERC725Alliance/ERC725/blob/c7f009261ff72b488f160028b835c311987638af/implementations/contracts/ERC725XCore.sol#L261
				let salt = call_data_vec
					.clone() // clone here is on Vec<u8>, which is clonable
					.into_iter()
					.skip(creation_code_len - 32)
					.collect::<alloc::vec::Vec<u8>>();
				let salt = H256::from_slice(&salt);

				let is_transactional = false;
				let validate = true;
				let evm_config = <Runtime as pallet_evm::Config>::config();
				// TODO: refactor the code once we are on polkadot-v1.1.0, ref - https://github.com/polkadot-evm/frontier/pull/1121
				let estimated_transaction_len = call_data_vec.len() +
					20 + // from
					32 + // salt
					32 + // value
					32 + // gas_limit
					32 + // nonce
					1 + // TransactionAction
					8 + // chain id
					65; // signature

				let gas_limit = handle.remaining_gas().min(u64::MAX);
				let without_base_extrinsic_weight = true;

				let (weight_limit, proof_size_base_cost) =
					match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
						gas_limit,
						without_base_extrinsic_weight,
					) {
						weight_limit if weight_limit.proof_size() > 0 => {
							(Some(weight_limit), Some(estimated_transaction_len as u64))
						},
						_ => (None, None),
					};

				let execution_info = <Runtime as pallet_evm::Config>::Runner::create2(
					futurepass,
					call_data_vec, // reuse the vector here
					salt,
					value,
					handle.remaining_gas(),
					None,
					None,
					None, // handled by EVM
					alloc::vec![],
					is_transactional,
					validate,
					weight_limit,
					proof_size_base_cost,
					evm_config,
				)
				.map_err(|_| RevertReason::custom("Futurepass: create2 failed"))?;

				// emit ContractCreated(CREATE2, contractAddress, value, salt);
				log4(
					handle.code_address(),
					SELECTOR_LOG_FUTUREPASS_CONTRACT_CREATED,
					H256::from_low_u64_be(<CallType as Into<u8>>::into(call_type).into()),
					execution_info.value,
					H256::from_slice(&Into::<[u8; 32]>::into(value)),
					EvmDataWriter::new().write(salt).build(),
				)
				.record(handle)?;

				(execution_info.exit_reason, execution_info.value.to_fixed_bytes().to_vec())
			},
			CallType::DelegateCall => {
				Err(RevertReason::custom("Futurepass: call type not supported"))?
			},
		};

		// Return subcall result
		match reason {
			ExitReason::Fatal(exit_status) => Err(PrecompileFailure::Fatal { exit_status }),
			ExitReason::Revert(exit_status) => {
				Err(PrecompileFailure::Revert { exit_status, output })
			},
			ExitReason::Error(exit_status) => Err(PrecompileFailure::Error { exit_status }),
			ExitReason::Succeed(_) => Ok(succeed(output)),
		}
	}

	fn owner(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		let futurepass = handle.code_address();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let owner: H160 = <Runtime as pallet_futurepass::Config>::Proxy::owner(&futurepass.into())
			.map(|owner: Runtime::AccountId| owner.into())
			.unwrap_or(H160::default());

		Ok(succeed(EvmDataWriter::new().write(Address::from(owner)).build()))
	}

	fn renounce_ownership(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		let caller = handle.context().caller;
		let burn_account: H160 = H160::default();

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller.into()).into(),
			pallet_futurepass::Call::<Runtime>::transfer_futurepass {
				current_owner: caller.into(),
				new_owner: None,
			},
		)?;

		// emit OwnershipTransferred(address,address) event
		log3(
			handle.code_address(),
			SELECTOR_LOG_OWNERSHIP_TRANSFERRED,
			caller,
			burn_account,
			vec![],
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn transfer_ownership(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(handle, { new_owner: Address });
		let new_owner: H160 = new_owner.into();
		let caller = handle.context().caller;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller.into()).into(),
			pallet_futurepass::Call::<Runtime>::transfer_futurepass {
				current_owner: caller.into(),
				new_owner: Some(new_owner.into()),
			},
		)?;

		// emit OwnershipTransferred(address,address) event
		log3(handle.code_address(), SELECTOR_LOG_OWNERSHIP_TRANSFERRED, caller, new_owner, vec![])
			.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}
}
