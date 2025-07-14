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

use alloc::string::String;
use core::convert::{TryFrom, TryInto};
use ethereum_types::BigEndianHash;
use fp_evm::{IsPrecompileResult, PrecompileHandle, PrecompileOutput};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	ensure,
	traits::OriginTrait,
};
use pallet_evm::{Context, ExitReason, PrecompileFailure, PrecompileSet};
use precompile_utils::{
	constants::{ERC1155_PRECOMPILE_ADDRESS_PREFIX, ERC20_PRECOMPILE_ADDRESS_PREFIX},
	prelude::*,
};
use seed_pallet_common::utils::TokenBurnAuthority;
use seed_primitives::{
	AssetId, Balance, CollectionUuid, IssuanceId, MetadataScheme, SerialNumber, TokenId,
};
use sp_core::{Encode, H160, H256, U256};
use sp_runtime::{traits::SaturatedConversion, BoundedVec};
use sp_std::{marker::PhantomData, vec, vec::Vec};

/// Solidity selector of the TransferSingle log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_TRANSFER_SINGLE: [u8; 32] =
	keccak256!("TransferSingle(address,address,address,uint256,uint256)");

/// Solidity selector of the TransferBatch log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_TRANSFER_BATCH: [u8; 32] =
	keccak256!("TransferBatch(address,address,address,uint256[],uint256[])");

/// Solidity selector of the Approval log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_APPROVAL_FOR_ALL: [u8; 32] =
	keccak256!("ApprovalForAll(address,address,bool)");

/// Solidity selector of the OwnershipTransferred log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_OWNERSHIP_TRANSFERRED: [u8; 32] =
	keccak256!("OwnershipTransferred(address,address)");

/// Solidity selector of the MaxSupplyUpdated log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_MAX_SUPPLY_UPDATED: [u8; 32] = keccak256!("MaxSupplyUpdated(uint128)");

/// Solidity selector of the TokenCreated log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_TOKEN_CREATED: [u8; 32] = keccak256!("TokenCreated(uint32)");

/// Solidity selector of the BaseURIUpdated log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_BASE_URI_UPDATED: [u8; 32] = keccak256!("BaseURIUpdated(string)");

/// Solidity selector of the onERC1155Received function
/// bytes4(keccak256("onERC1155Received(address,address,uint256,uint256,bytes)"));
pub const ON_ERC1155_RECEIVED_FUNCTION_SELECTOR: [u8; 4] = [0xf2, 0x3a, 0x6e, 0x61];

/// Solidity selector of the onERC1155BatchReceived function
/// bytes4(keccak256("onERC1155BatchReceived(address,address,uint256[],uint256[],bytes)"));
pub const ON_ERC1155_BATCH_RECEIVED_FUNCTION_SELECTOR: [u8; 4] = [0xbc, 0x19, 0x7c, 0x81];

pub const SELECTOR_LOG_PUBLIC_MINT_TOGGLED: [u8; 32] = keccak256!("PublicMintToggled(uint32,bool)");

pub const SELECTOR_LOG_MINT_FEE_UPDATED: [u8; 32] =
	keccak256!("MintFeeUpdated(uint32,address,uint128)");

pub const SELECTOR_PENDING_ISSUANCE_CREATED: [u8; 32] =
	keccak256!("PendingIssuanceCreated(address,uint256,uint256[],uint256[])");

pub const SELECTOR_ISSUED: [u8; 32] = keccak256!("Issued(address,address,uint256,uint8)");

/// Interface IDs for the ERC165, ERC1155, ERC1155Burnable, ERC1155Supply, ERC1155MetadataURI, Ownable and TRN1155 interfaces
pub const ERC165_INTERFACE_IDS: &[u32] = &[
	0x01ffc9a7, // ERC165
	0xd9b67a26, // ERC1155
	0x9e094e9e, // ERC1155Burnable
	0x0e89341c, // ERC1155MetadataURI
	0xf2d03e40, // ERC1155Supply
	0x0e083076, // Ownable
	0xf0f03f65, // TRN1155
];

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	// ERC1155 standard functions
	BalanceOf = "balanceOf(address,uint256)",
	BalanceOfBatch = "balanceOfBatch(address[],uint256[])",
	SetApprovalForAll = "setApprovalForAll(address,bool)",
	IsApprovedForAll = "isApprovedForAll(address,address)",
	SafeTransferFrom = "safeTransferFrom(address,address,uint256,uint256,bytes)",
	SafeBatchTransferFrom = "safeBatchTransferFrom(address,address,uint256[],uint256[],bytes)",
	// ERC1155 burnable extensions
	Burn = "burn(address,uint256,uint256)",
	BurnBatch = "burnBatch(address,uint256[],uint256[])",
	// ERC1155 supply extensions
	TotalSupply = "totalSupply(uint256)",
	Exists = "exists(uint256)",
	// ERC1155 metadata URI extensions
	Uri = "uri(uint256)",
	// Ownable - https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/access/Ownable.sol
	Owner = "owner()",
	RenounceOwnership = "renounceOwnership()",
	TransferOwnership = "transferOwnership(address)",
	// TRN extensions
	CreateToken = "createToken(bytes,uint128,uint128,address)",
	Mint = "mint(address,uint256,uint256)",
	MintBatch = "mintBatch(address,uint256[],uint256[])",
	SetMaxSupply = "setMaxSupply(uint256,uint32)",
	SetBaseURI = "setBaseURI(bytes)",
	TogglePublicMint = "togglePublicMint(uint256,bool)",
	SetMintFee = "setMintFee(uint256,address,uint128)",
	// Selector used by SafeTransferFrom function
	OnErc1155Received = "onERC1155Received(address,address,uint256,uint256,bytes)",
	OnErc1155BatchReceived = "onERC1155BatchReceived(address,address,uint256[],uint256[],bytes)",
	// ERC165 - https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/utils/introspection/ERC165.sol
	SupportsInterface = "supportsInterface(bytes4)",
	// ERC5484 Soulbound tokens
	SetBurnAuth = "setBurnAuth(uint256,uint8)",
	IssueSoulbound = "issueSoulbound(address,uint256[],uint256[])",
	AcceptSoulboundIssuance = "acceptSouldboundIssuance(uint32)",
	PendingIssuances = "pendingIssuances(address)",
	BurnAsCollectionOwner = "burnAsCollectionOwner(address,uint256[],uint256[])",
	BurnAuth = "burnAuth(uint256)",
}

/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
/// 1024-2047 Precompiles that are not in Ethereum Mainnet but are neither Root specific
/// 2048-4095 Seed specific precompiles
/// SFT precompile addresses can only fall between
///     0xBBBBBBBB00000000000000000000000000000000 - 0xBBBBBBBBFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
/// The precompile for SFT series X where X is a u32 (i.e.4 bytes), if 0XFFFFFFFF +
/// Bytes(CollectionUuid) In order to route the address to Erc1155Precompile<R>, we
/// check whether the CollectionUuid exists in pallet-sft

/// This means that every address that starts with 0xBBBBBBBBwill go through an additional db read,
/// but the probability for this to happen is 2^-32 for random addresses
pub struct Erc1155PrecompileSet<Runtime>(PhantomData<Runtime>);

impl<T> Default for Erc1155PrecompileSet<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> PrecompileSet for Erc1155PrecompileSet<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: pallet_sft::Config
		+ pallet_evm::Config
		+ frame_system::Config
		+ pallet_token_approvals::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::RuntimeCall:
		From<pallet_sft::Call<Runtime>> + From<pallet_token_approvals::Call<Runtime>>,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: OriginTrait,
{
	fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<EvmResult<PrecompileOutput>> {
		// Convert target `address` into it's runtime SFT Id
		if let Some(collection_id) = Runtime::evm_id_to_runtime_id(
			Address(handle.code_address()),
			ERC1155_PRECOMPILE_ADDRESS_PREFIX,
		) {
			// 'collection name' is empty when the collection doesn't exist yet
			if pallet_sft::Pallet::<Runtime>::collection_exists(collection_id) {
				let result = {
					let selector = match handle.read_selector() {
						Ok(selector) => selector,
						Err(e) => return Some(Err(e.into())),
					};

					if let Err(err) = handle.check_function_modifier(match selector {
						Action::SetApprovalForAll
						| Action::SafeTransferFrom
						| Action::SafeBatchTransferFrom
						| Action::Burn
						| Action::BurnBatch
						| Action::Mint
						| Action::TogglePublicMint
						| Action::SetMintFee
						| Action::MintBatch => FunctionModifier::NonPayable,
						_ => FunctionModifier::View,
					}) {
						return Some(Err(err.into()));
					}

					match selector {
						// Core ERC1155
						Action::BalanceOf => Self::balance_of(collection_id, handle),
						Action::BalanceOfBatch => Self::balance_of_batch(collection_id, handle),
						Action::SetApprovalForAll => {
							Self::set_approval_for_all(collection_id, handle)
						},
						Action::IsApprovedForAll => {
							Self::is_approved_for_all(collection_id, handle)
						},
						Action::SafeTransferFrom => Self::safe_transfer_from(collection_id, handle),
						Action::SafeBatchTransferFrom => {
							Self::safe_batch_transfer_from(collection_id, handle)
						},
						// Burnable
						Action::Burn => Self::burn(collection_id, handle),
						Action::BurnBatch => Self::burn_batch(collection_id, handle),
						// Supply
						Action::TotalSupply => Self::total_supply(collection_id, handle),
						Action::Exists => Self::exists(collection_id, handle),
						// Metadata
						Action::Uri => Self::uri(collection_id, handle),
						// Ownable
						Action::Owner => Self::owner(collection_id, handle),
						Action::RenounceOwnership => {
							Self::renounce_ownership(collection_id, handle)
						},
						Action::TransferOwnership => {
							Self::transfer_ownership(collection_id, handle)
						},
						// TRN
						Action::CreateToken => Self::create_token(collection_id, handle),
						Action::Mint => Self::mint(collection_id, handle),
						Action::MintBatch => Self::mint_batch(collection_id, handle),
						Action::SetMaxSupply => Self::set_max_supply(collection_id, handle),
						Action::SetBaseURI => Self::set_base_uri(collection_id, handle),
						Action::TogglePublicMint => Self::toggle_public_mint(collection_id, handle),
						Action::SetMintFee => Self::set_mint_fee(collection_id, handle),
						// ERC165
						Action::SupportsInterface => Self::supports_interface(handle),
						// ERC5484
						Action::SetBurnAuth => Self::set_burn_auth(collection_id, handle),
						Action::IssueSoulbound => Self::issue_soulbound(collection_id, handle),
						Action::AcceptSoulboundIssuance => {
							Self::accept_soulbound_issuance(collection_id, handle)
						},
						Action::PendingIssuances => Self::pending_issuances(collection_id, handle),
						Action::BurnAuth => Self::burn_auth(collection_id, handle),
						Action::BurnAsCollectionOwner => {
							Self::burn_as_collection_owner(collection_id, handle)
						},
						_ => return Some(Err(revert("ERC1155: Function not implemented"))),
					}
				};
				return Some(result);
			}
		}
		None
	}

	fn is_precompile(&self, address: H160, _remaining_gas: u64) -> IsPrecompileResult {
		if let Some(collection_id) =
			Runtime::evm_id_to_runtime_id(Address(address), ERC1155_PRECOMPILE_ADDRESS_PREFIX)
		{
			let extra_cost = RuntimeHelper::<Runtime>::db_read_gas_cost();
			// Check whether the collection exists
			IsPrecompileResult::Answer {
				is_precompile: pallet_sft::Pallet::<Runtime>::collection_exists(collection_id),
				extra_cost,
			}
		} else {
			IsPrecompileResult::Answer { is_precompile: false, extra_cost: 0 }
		}
	}
}

impl<Runtime> Erc1155PrecompileSet<Runtime> {
	pub fn new() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> Erc1155PrecompileSet<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: pallet_sft::Config
		+ pallet_evm::Config
		+ frame_system::Config
		+ pallet_token_approvals::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::RuntimeCall:
		From<pallet_sft::Call<Runtime>> + From<pallet_token_approvals::Call<Runtime>>,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: OriginTrait,
{
	fn balance_of(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		read_args!(handle, { owner: Address, id: U256 });

		// Parse args
		let owner: H160 = owner.into();
		ensure!(id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
		let serial_number: SerialNumber = id.saturated_into();

		// Get balance from SFT pallet
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let balance = pallet_sft::Pallet::<Runtime>::balance_of(
			&owner.into(),
			(collection_id, serial_number),
		);

		Ok(succeed(EvmDataWriter::new().write(U256::from(balance)).build()))
	}

	fn balance_of_batch(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		read_args!(handle, { accounts: Vec<Address>, ids: Vec<U256> });

		ensure!(accounts.len() == ids.len(), revert("ERC1155: accounts and ids length mismatch"));

		// Parse args
		let owners: Vec<H160> = accounts.into_iter().map(|a| a.into()).collect();
		let ids: Vec<SerialNumber> = ids
			.into_iter()
			.map(|id| {
				if id > u32::MAX.into() {
					return Err(revert("ERC1155: Expected token id <= 2^32"));
				}
				Ok(id.saturated_into())
			})
			.collect::<Result<Vec<SerialNumber>, PrecompileFailure>>()?;

		// Record one read cost per token
		handle.record_cost(
			RuntimeHelper::<Runtime>::db_read_gas_cost().saturating_mul(ids.len() as u64),
		)?;

		// Get balance from SFT pallet for each
		let mut balances: Vec<U256> = vec![];
		owners.iter().zip(ids.iter()).for_each(|(owner, id)| {
			let balance = pallet_sft::Pallet::<Runtime>::balance_of(
				&Runtime::AccountId::from(*owner),
				(collection_id, *id),
			);
			balances.push(U256::from(balance));
		});

		Ok(succeed(EvmDataWriter::new().write(balances).build()))
	}

	fn is_approved_for_all(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(handle, { owner: Address, operator: Address });
		let owner: Runtime::AccountId = H160::from(owner).into();
		let operator: Runtime::AccountId = H160::from(operator).into();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let is_approved = pallet_token_approvals::ERC1155ApprovalsForAll::<Runtime>::get(
			owner,
			(collection_id, operator),
		)
		.unwrap_or_default();

		Ok(succeed(EvmDataWriter::new().write(is_approved).build()))
	}

	fn set_approval_for_all(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(handle, { operator: Address, approved: bool });
		let operator = H160::from(operator);
		let caller = handle.context().caller;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(Runtime::AccountId::from(caller)).into(),
			pallet_token_approvals::Call::<Runtime>::erc1155_approval_for_all {
				operator_account: operator.into(),
				collection_uuid: collection_id,
				approved,
			},
		)?;

		log3(
			handle.code_address(),
			SELECTOR_LOG_APPROVAL_FOR_ALL,
			handle.context().caller,
			operator,
			EvmDataWriter::new().write(approved).build(),
		)
		.record(handle)?;
		Ok(succeed([]))
	}

	fn safe_transfer_from(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(4, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				from: Address,
				to: Address,
				id: U256,
				amount: U256,
				data: Bytes
			}
		);

		let to: H160 = to.into();
		Self::do_safe_transfer_acceptance_check(handle, from, to, id, amount, data)?;

		ensure!(id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
		ensure!(amount <= Balance::MAX.into(), revert("ERC1155: Expected amounts <= 2^128"));
		let serial_number: SerialNumber = id.saturated_into();
		let balance: Balance = amount.saturated_into();

		let from: H160 = from.into();
		let res = Self::do_safe_transfer(
			collection_id,
			handle,
			from,
			to,
			vec![serial_number],
			vec![balance],
		)?;

		log4(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER_SINGLE,
			handle.context().caller,
			from,
			to,
			EvmDataWriter::new().write(id).write(amount).build(),
		)
		.record(handle)?;

		Ok(res)
	}

	// Check that target implements onERC1155Received
	// Check that caller is not a smart contract s.t. no code is inserted into
	// pallet_evm::AccountCodes except if the caller is another precompile i.e. CallPermit
	fn do_safe_transfer_acceptance_check(
		handle: &mut impl PrecompileHandle,
		from: Address,
		to: H160,
		id: U256,
		amount: U256,
		data: Bytes,
	) -> Result<(), PrecompileFailure> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller_code = pallet_evm::AccountCodes::<Runtime>::get(to);
		if !(caller_code.is_empty()) {
			let operator = handle.context().caller;
			// Setup input for onErc1155Received call
			let sub_context =
				Context { address: to, caller: operator, apparent_value: Default::default() };
			let input = EvmDataWriter::new_with_selector(Action::OnErc1155Received)
				.write::<Address>(operator.into())
				.write::<Address>(from)
				.write::<U256>(id)
				.write::<U256>(amount)
				.write::<Bytes>(data)
				.build();
			let (reason, output) =
				handle.call(to, None, input.clone(), handle.gas_limit(), false, &sub_context);
			// Check response from call
			match reason {
				ExitReason::Succeed(_) => {
					if output[..4] != ON_ERC1155_RECEIVED_FUNCTION_SELECTOR.to_vec() {
						return Err(revert("ERC1155: ERC1155Receiver rejected tokens"));
					}
				},
				_ => return Err(revert("ERC1155: transfer to non-ERC1155Receiver implementer")),
			};
		}
		Ok(())
	}

	fn safe_batch_transfer_from(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(4, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				from: Address,
				to: Address,
				ids: Vec<U256>,
				amounts: Vec<U256>,
				data: Bytes
			}
		);

		let to: H160 = to.into();
		Self::do_batch_safe_transfer_acceptance_check(
			handle,
			from,
			to,
			ids.clone(),
			amounts.clone(),
			data,
		)?;

		let serial_numbers: Vec<SerialNumber> = ids
			.iter()
			.map(|id| {
				ensure!(*id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
				Ok((*id).saturated_into())
			})
			.collect::<Result<Vec<SerialNumber>, PrecompileFailure>>()?;

		let balances: Vec<Balance> = amounts
			.iter()
			.map(|amount| {
				ensure!(
					*amount <= Balance::MAX.into(),
					revert("ERC1155: Expected amounts <= 2^128")
				);
				Ok((*amount).saturated_into())
			})
			.collect::<Result<Vec<Balance>, PrecompileFailure>>()?;

		let from: H160 = from.into();
		let res =
			Self::do_safe_transfer(collection_id, handle, from, to, serial_numbers, balances)?;

		log4(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER_BATCH,
			handle.context().caller,
			from,
			to,
			EvmDataWriter::new().write(ids).write(amounts).build(),
		)
		.record(handle)?;

		Ok(res)
	}

	// Check that target implements onERC1155BatchReceived
	// Check that caller is not a smart contract s.t. no code is inserted into
	// pallet_evm::AccountCodes except if the caller is another precompile i.e. CallPermit
	fn do_batch_safe_transfer_acceptance_check(
		handle: &mut impl PrecompileHandle,
		from: Address,
		to: H160,
		ids: Vec<U256>,
		amounts: Vec<U256>,
		data: Bytes,
	) -> Result<(), PrecompileFailure> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller_code = pallet_evm::AccountCodes::<Runtime>::get(to);
		if !(caller_code.is_empty()) {
			let operator = handle.context().caller;
			// Setup input for onErc1155BatchReceived call
			let sub_context =
				Context { address: to, caller: operator, apparent_value: Default::default() };
			let input = EvmDataWriter::new_with_selector(Action::OnErc1155Received)
				.write::<Address>(operator.into())
				.write::<Address>(from)
				.write::<Vec<U256>>(ids)
				.write::<Vec<U256>>(amounts)
				.write::<Bytes>(data)
				.build();
			let (reason, output) =
				handle.call(to, None, input.clone(), handle.gas_limit(), false, &sub_context);
			// Check response from call
			match reason {
				ExitReason::Succeed(_) => {
					if output[..4] != ON_ERC1155_BATCH_RECEIVED_FUNCTION_SELECTOR.to_vec() {
						return Err(revert("ERC1155: ERC1155Receiver rejected tokens"));
					}
				},
				_ => return Err(revert("ERC1155: transfer to non-ERC1155Receiver implementer")),
			};
		}
		Ok(())
	}

	fn do_safe_transfer(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
		from: H160,
		to: H160,
		serial_numbers: Vec<SerialNumber>,
		amounts: Vec<Balance>,
	) -> EvmResult<PrecompileOutput> {
		ensure!(
			serial_numbers.len() == amounts.len(),
			revert("ERC1155: ids and amounts length mismatch")
		);
		ensure!(to != H160::default(), revert("ERC1155: transfer to the zero address"));

		// Check approvals
		if from != handle.context().caller {
			handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
			let is_approved = pallet_token_approvals::ERC1155ApprovalsForAll::<Runtime>::get(
				Runtime::AccountId::from(from),
				(collection_id, Runtime::AccountId::from(handle.context().caller)),
			)
			.unwrap_or_default();
			ensure!(is_approved, revert("ERC1155: Caller is not token owner or approved"));
		}

		// Build input BoundedVec from serial_numbers and amounts.
		let combined = serial_numbers.into_iter().zip(amounts).collect::<Vec<_>>();
		let serial_numbers: BoundedVec<
			(SerialNumber, Balance),
			<Runtime as pallet_sft::Config>::MaxSerialsPerMint,
		> = BoundedVec::try_from(combined)
			.map_err(|_| revert("ERC1155: Too many serial numbers in one transfer."))?;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(Runtime::AccountId::from(from)).into(),
			pallet_sft::Call::<Runtime>::transfer {
				collection_id,
				serial_numbers,
				new_owner: to.into(),
			},
		)?;

		// Build output.
		Ok(succeed([]))
	}

	fn burn(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(handle, { account: Address, id: U256, value: U256 });

		let operator = H160::from(account);
		ensure!(id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
		ensure!(value <= Balance::MAX.into(), revert("ERC1155: Expected amount <= 2^128"));
		let serial_number: SerialNumber = id.saturated_into();
		let amount: Balance = value.saturated_into();

		// Check approvals
		if operator != handle.context().caller {
			handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
			let is_approved = pallet_token_approvals::ERC1155ApprovalsForAll::<Runtime>::get(
				Runtime::AccountId::from(operator),
				(collection_id, Runtime::AccountId::from(handle.context().caller)),
			)
			.unwrap_or_default();
			ensure!(is_approved, revert("ERC1155: Caller is not token owner or approved"));
		}

		// Build input BoundedVec from serial_number and amount.
		let combined = vec![(serial_number, amount)];
		let serial_numbers: BoundedVec<
			(SerialNumber, Balance),
			<Runtime as pallet_sft::Config>::MaxSerialsPerMint,
		> = BoundedVec::truncate_from(combined);

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(Runtime::AccountId::from(operator)).into(),
			pallet_sft::Call::<Runtime>::burn { collection_id, serial_numbers },
		)?;

		log4(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER_SINGLE,
			handle.context().caller,
			operator,
			H160::zero(),
			EvmDataWriter::new().write(id).write(amount).build(),
		)
		.record(handle)?;

		Ok(succeed([]))
	}

	fn burn_batch(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(4, 32)?;

		// Parse input.
		read_args!(handle, { account: Address, ids: Vec<U256>, values: Vec<U256> });

		let operator = H160::from(account);
		ensure!(ids.len() == values.len(), revert("ERC1155: ids and values length mismatch"));
		let serial_numbers: Vec<SerialNumber> = ids
			.iter()
			.map(|id| {
				ensure!(*id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
				Ok((*id).saturated_into())
			})
			.collect::<Result<Vec<SerialNumber>, PrecompileFailure>>()?;

		let balances: Vec<Balance> = values
			.iter()
			.map(|amount| {
				ensure!(
					*amount <= Balance::MAX.into(),
					revert("ERC1155: Expected values <= 2^128")
				);
				Ok((*amount).saturated_into())
			})
			.collect::<Result<Vec<Balance>, PrecompileFailure>>()?;

		// Check approvals
		if operator != handle.context().caller {
			handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
			let is_approved = pallet_token_approvals::ERC1155ApprovalsForAll::<Runtime>::get(
				Runtime::AccountId::from(operator),
				(collection_id, Runtime::AccountId::from(handle.context().caller)),
			)
			.unwrap_or_default();
			ensure!(is_approved, revert("ERC1155: Caller is not token owner or approved"));
		}

		// Build input BoundedVec from serial_number and amount.
		let combined = serial_numbers.into_iter().zip(balances).collect::<Vec<_>>();
		let serial_numbers: BoundedVec<
			(SerialNumber, Balance),
			<Runtime as pallet_sft::Config>::MaxSerialsPerMint,
		> = BoundedVec::try_from(combined)
			.map_err(|_| revert("ERC1155: Too many serial numbers in one burn."))?;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(Runtime::AccountId::from(operator)).into(),
			pallet_sft::Call::<Runtime>::burn { collection_id, serial_numbers },
		)?;

		log4(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER_BATCH,
			handle.context().caller,
			operator,
			H160::zero(),
			EvmDataWriter::new().write(ids).write(values).build(),
		)
		.record(handle)?;

		Ok(succeed([]))
	}

	fn total_supply(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(handle, { id: U256 });

		ensure!(id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
		let serial_number: SerialNumber = id.saturated_into();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let total_supply =
			pallet_sft::Pallet::<Runtime>::total_supply((collection_id, serial_number));

		Ok(succeed(EvmDataWriter::new().write(total_supply).build()))
	}

	fn exists(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(handle, { id: U256 });

		ensure!(id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
		let serial_number: SerialNumber = id.saturated_into();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let exists = pallet_sft::Pallet::<Runtime>::token_exists((collection_id, serial_number));

		Ok(succeed(EvmDataWriter::new().write(exists).build()))
	}

	fn uri(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(handle, { id: U256 });

		ensure!(id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
		let serial_number: SerialNumber = id.saturated_into();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let uri = pallet_sft::Pallet::<Runtime>::token_uri((collection_id, serial_number));
		Ok(succeed(EvmDataWriter::new().write::<Bytes>(uri.as_slice().into()).build()))
	}

	fn owner(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		match pallet_sft::Pallet::<Runtime>::get_collection_owner(collection_id) {
			Some(collection_owner) => Ok(succeed(
				EvmDataWriter::new()
					.write(Address::from(Into::<H160>::into(collection_owner)))
					.build(),
			)),
			None => Err(revert(String::from("ERC1155: Collection does not exist").as_bytes())),
		}
	}

	fn renounce_ownership(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		let origin = handle.context().caller;
		let burn_account: H160 = H160::default();

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_sft::Call::<Runtime>::set_owner {
				collection_id,
				new_owner: burn_account.into(),
			},
		)?;

		// emit OwnershipTransferred(address,address) event
		log3(
			handle.code_address(),
			SELECTOR_LOG_OWNERSHIP_TRANSFERRED,
			origin,
			burn_account,
			vec![],
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn transfer_ownership(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(handle, { new_owner: Address });
		let new_owner: H160 = new_owner.into();
		let origin = handle.context().caller;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_sft::Call::<Runtime>::set_owner { collection_id, new_owner: new_owner.into() },
		)?;

		log3(handle.code_address(), SELECTOR_LOG_OWNERSHIP_TRANSFERRED, origin, new_owner, vec![])
			.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn create_token(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;
		read_args!(handle, { name: Bytes, initial_issuance: U256, max_issuance: U256, token_owner: Address});
		// Parse name
		let name: BoundedVec<u8, <Runtime as pallet_sft::Config>::StringLimit> = name
			.as_bytes()
			.to_vec()
			.try_into()
			.map_err(|_| revert("ERC1155: Collection name exceeds the maximum length"))?;
		// Parse initial issuance
		ensure!(
			initial_issuance <= Balance::MAX.into(),
			revert("ERC1155: Expected initial issuance <= 2^128")
		);
		let initial_issuance: Balance = initial_issuance.saturated_into();
		// Parse max issuance
		ensure!(
			max_issuance <= Balance::MAX.into(),
			revert("ERC1155: Expected max issuance <= 2^128")
		);
		let max_issuance: Balance = max_issuance.saturated_into();
		// If max issuance is set to 0, we take this as no max issuance set
		let max_issuance = if max_issuance == 0 { None } else { Some(max_issuance) };
		// Parse token owner, if zero address, we take this as no owner
		let token_owner: H160 = token_owner.into();
		let token_owner: Option<Runtime::AccountId> =
			if token_owner == H160::default() { None } else { Some(token_owner.into()) };

		let serial_number = pallet_sft::Pallet::<Runtime>::do_create_token(
			handle.context().caller.into(),
			collection_id,
			name,
			initial_issuance,
			max_issuance,
			token_owner,
		);

		match serial_number {
			Ok(serial_number) => {
				log2(
					handle.code_address(),
					SELECTOR_LOG_TOKEN_CREATED,
					H256::from_uint(&U256::from(serial_number)),
					vec![],
				)
				.record(handle)?;

				Ok(succeed(EvmDataWriter::new().write(U256::from(serial_number)).build()))
			},
			Err(err) => Err(revert(
				alloc::format!("ERC1155: Create token failed {:?}", err.stripped()).as_bytes(),
			)),
		}
	}

	fn mint(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(4, 32)?;

		// Parse input.
		read_args!(handle, { to: Address, id: U256, amount: U256 });
		let receiver = H160::from(to);
		ensure!(id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
		let serial_number: SerialNumber = id.saturated_into();
		ensure!(amount <= Balance::MAX.into(), revert("ERC1155: Expected values <= 2^128"));
		let balance: Balance = amount.saturated_into();

		// Build input BoundedVec from serial_number and amount.
		let combined = vec![(serial_number, balance)];
		let serial_numbers: BoundedVec<
			(SerialNumber, Balance),
			<Runtime as pallet_sft::Config>::MaxSerialsPerMint,
		> = BoundedVec::truncate_from(combined);

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(Runtime::AccountId::from(handle.context().caller)).into(),
			pallet_sft::Call::<Runtime>::mint {
				collection_id,
				serial_numbers,
				token_owner: Some(Runtime::AccountId::from(receiver)),
			},
		)?;

		log4(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER_SINGLE,
			handle.context().caller,
			H160::zero(),
			receiver,
			EvmDataWriter::new().write(id).write(amount).build(),
		)
		.record(handle)?;

		Ok(succeed([]))
	}

	fn mint_batch(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(4, 32)?;

		// Parse input.
		read_args!(handle, { to: Address, ids: Vec<U256>, amounts: Vec<U256> });
		ensure!(amounts.len() == ids.len(), revert("ERC1155: ids and amounts length mismatch"));

		let receiver = H160::from(to);
		let serial_numbers: Vec<SerialNumber> = ids
			.iter()
			.map(|id| {
				ensure!(*id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
				Ok((*id).saturated_into())
			})
			.collect::<Result<Vec<SerialNumber>, PrecompileFailure>>()?;
		let balances: Vec<Balance> = amounts
			.iter()
			.map(|amount| {
				ensure!(
					*amount <= Balance::MAX.into(),
					revert("ERC1155: Expected amounts <= 2^128")
				);
				Ok((*amount).saturated_into())
			})
			.collect::<Result<Vec<Balance>, PrecompileFailure>>()?;

		// Build input BoundedVec from serial_number and amount.
		let combined = serial_numbers.into_iter().zip(balances).collect::<Vec<_>>();
		let serial_numbers: BoundedVec<
			(SerialNumber, Balance),
			<Runtime as pallet_sft::Config>::MaxSerialsPerMint,
		> = BoundedVec::try_from(combined)
			.map_err(|_| revert("ERC1155: Too many serial numbers in one mint."))?;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(Runtime::AccountId::from(handle.context().caller)).into(),
			pallet_sft::Call::<Runtime>::mint {
				collection_id,
				serial_numbers,
				token_owner: Some(receiver.into()),
			},
		)?;

		log4(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER_BATCH,
			handle.context().caller,
			H160::zero(),
			receiver,
			EvmDataWriter::new().write(ids).write(amounts).build(),
		)
		.record(handle)?;

		Ok(succeed([]))
	}

	fn set_max_supply(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;

		// Parse input.
		read_args!(handle, { id: U256, max_supply: U256 });

		ensure!(id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
		let serial_number: SerialNumber = id.saturated_into();

		// Parse max_supply
		if max_supply > Balance::MAX.into() {
			return Err(revert("ERC1155: Expected max_supply <= 2^128"));
		}
		let max_issuance: Balance = max_supply.saturated_into();
		let origin = handle.context().caller;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_sft::Call::<Runtime>::set_max_issuance {
				token_id: (collection_id, serial_number),
				max_issuance,
			},
		)?;

		// Emit event.
		log2(
			handle.code_address(),
			SELECTOR_LOG_MAX_SUPPLY_UPDATED,
			H256::from_uint(&max_supply),
			vec![],
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn set_base_uri(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		// Parse input.
		read_args!(handle, { base_uri: Bytes });

		let origin = handle.context().caller;
		let metadata_scheme = MetadataScheme::try_from(base_uri.0.as_slice())
			.map_err(|_| revert("ERC1155: Base uri too long."))?;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_sft::Call::<Runtime>::set_base_uri { collection_id, metadata_scheme },
		)?;

		// Emit event.
		log1(
			handle.code_address(),
			SELECTOR_LOG_BASE_URI_UPDATED,
			EvmDataWriter::new().write(base_uri).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn toggle_public_mint(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;

		read_args!(handle, { id: U256, enabled: bool });

		ensure!(id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
		let serial_number: SerialNumber = id.saturated_into();
		let token_id: TokenId = (collection_id, serial_number);

		// Dispatch call (if enough gas).
		let origin = handle.context().caller;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_sft::Call::<Runtime>::toggle_public_mint { token_id, enabled },
		)?;

		log2(
			handle.code_address(),
			SELECTOR_LOG_PUBLIC_MINT_TOGGLED,
			H256::from_uint(&U256::from(serial_number)),
			EvmDataWriter::new().write(enabled).build(),
		)
		.record(handle)?;

		Ok(succeed([]))
	}

	fn set_mint_fee(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		read_args!(handle, { id: U256, payment_asset: Address, mint_fee: U256 });

		ensure!(id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
		let serial_number: SerialNumber = id.saturated_into();
		let token_id: TokenId = (collection_id, serial_number);

		// Parse inputs
		let asset_id: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			payment_asset,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("ERC1155: Invalid payment asset address"))?;
		if mint_fee > Balance::MAX.into() {
			return Err(revert("ERC1155: Expected mint_fee <= 2^128"));
		}
		let fee: Balance = mint_fee.saturated_into();
		// If the mint fee is 0, we can assume this means no mint fee
		// Pass in None for pricing_details
		let pricing_details = match fee {
			0 => None,
			_ => Some((asset_id, fee)),
		};

		// Dispatch call (if enough gas).
		let origin = handle.context().caller;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_sft::Call::<Runtime>::set_mint_fee { token_id, pricing_details },
		)?;
		log4(
			handle.code_address(),
			SELECTOR_LOG_MINT_FEE_UPDATED,
			H256::from_uint(&U256::from(serial_number)),
			H160::from(payment_asset),
			H256::from_slice(&EvmDataWriter::new().write(mint_fee).build()),
			vec![],
		)
		.record(handle)?;

		Ok(succeed([]))
	}

	fn set_burn_auth(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				token_id: U256, burn_authority: u8
			}
		);

		if token_id > SerialNumber::MAX.into() {
			return Err(revert("ERC1155: Expected token id <= 2^32"));
		}
		let token_id: SerialNumber = token_id.saturated_into();

		if burn_authority > u8::MAX.into() {
			return Err(revert("ERC1155: Expected burn authority <= 2^8"));
		}
		let burn_authority = match TokenBurnAuthority::try_from(burn_authority) {
			Ok(b) => b,
			_ => return Err(revert("ERC1155: Could not parse burn authority")),
		};

		let origin = handle.context().caller;

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_sft::Call::<Runtime>::set_token_burn_authority {
				token_id: (collection_id, token_id),
				burn_authority,
			},
		)?;

		Ok(succeed([]))
	}

	fn issue_soulbound(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				to: Address, ids: Vec<U256>, amounts: Vec<U256>
			}
		);

		let origin = handle.context().caller;

		let receiver = H160::from(to);
		let serial_numbers: Vec<SerialNumber> = ids
			.iter()
			.map(|id| {
				ensure!(*id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
				Ok((*id).saturated_into())
			})
			.collect::<Result<Vec<SerialNumber>, PrecompileFailure>>()?;
		let balances: Vec<Balance> = amounts
			.iter()
			.map(|amount| {
				ensure!(
					*amount <= Balance::MAX.into(),
					revert("ERC1155: Expected amounts <= 2^128")
				);
				Ok((*amount).saturated_into())
			})
			.collect::<Result<Vec<Balance>, PrecompileFailure>>()?;

		// Build input BoundedVec from serial_number and amount.
		let combined = serial_numbers.into_iter().zip(balances).collect::<Vec<_>>();
		let serial_numbers: BoundedVec<
			(SerialNumber, Balance),
			<Runtime as pallet_sft::Config>::MaxSerialsPerMint,
		> = BoundedVec::try_from(combined)
			.map_err(|_| revert("ERC1155: Too many serial numbers in one issuance"))?;

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let next_issuance_id =
			pallet_sft::PendingIssuances::<Runtime>::get(collection_id).next_issuance_id;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_sft::Call::<Runtime>::issue_soulbound {
				collection_id,
				serial_numbers,
				token_owner: receiver.into(),
			},
		)?;

		log2(
			handle.code_address(),
			SELECTOR_PENDING_ISSUANCE_CREATED,
			receiver,
			EvmDataWriter::new().write(next_issuance_id).write(ids).write(amounts).build(),
		)
		.record(handle)?;

		Ok(succeed([]))
	}

	fn pending_issuances(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		read_args!(handle, { owner: Address });

		let owner: H160 = owner.into();
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let pending_issuances = pallet_sft::PendingIssuances::<Runtime>::get(collection_id)
			.get_pending_issuances(&owner.into());

		let issuance_ids = pending_issuances.iter().map(|p| U256::from(p.issuance_id)).collect();

		let issuances: Vec<(Vec<SerialNumber>, Vec<Balance>, Vec<u8>)> = pending_issuances
			.iter()
			.map(|p| -> EvmResult<(Vec<SerialNumber>, Vec<Balance>, Vec<u8>)> {
				let (serial_numbers, balances): (Vec<SerialNumber>, Vec<Balance>) =
					p.serial_numbers.clone().into_iter().unzip();

				let mut burn_auths = vec![];
				for serial_number in serial_numbers.iter() {
					handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

					let burn_auth = match pallet_sft::TokenUtilityFlags::<Runtime>::get((
						collection_id,
						*serial_number,
					))
					.burn_authority
					{
						Some(burn_auth) => burn_auth.into(),
						_ => 0 as u8,
					};

					burn_auths.push(burn_auth);
				}

				Ok((serial_numbers, balances, burn_auths))
			})
			.collect::<Vec<Result<_, _>>>()
			.into_iter()
			.collect::<Result<_, _>>()?;

		Ok(succeed(EvmDataWriter::new().write::<Vec<U256>>(issuance_ids).write(issuances).build()))
	}

	fn accept_soulbound_issuance(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		read_args!(handle, { issuance_id: U256 });

		if issuance_id > IssuanceId::MAX.into() {
			return Err(revert("ERC721: Expected issuance id <= 2^32"));
		}
		let issuance_id: IssuanceId = issuance_id.saturated_into();

		let origin = handle.context().caller;

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let collection = match pallet_sft::SftCollectionInfo::<Runtime>::get(collection_id) {
			Some(collection_info) => collection_info,
			None => return Err(revert("Collection does not exist")),
		};

		let collection_owner = collection.collection_owner;

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let pending_issuance = match pallet_sft::PendingIssuances::<Runtime>::get(collection_id)
			.get_pending_issuance(&origin.into(), issuance_id)
		{
			Some(pending_issuance) => pending_issuance,
			None => return Err(revert("Issuance does not exist")),
		};

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_sft::Call::<Runtime>::accept_soulbound_issuance { collection_id, issuance_id },
		)?;

		for (serial_number, _) in pending_issuance.serial_numbers {
			handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
			let burn_authority =
				match pallet_sft::TokenUtilityFlags::<Runtime>::get((collection_id, serial_number))
					.burn_authority
				{
					Some(burn_auth) => burn_auth.into(),
					_ => 0 as u8,
				};

			log4(
				handle.code_address(),
				SELECTOR_ISSUED,
				collection_owner.clone().into(),
				origin,
				H256::from_low_u64_be(serial_number as u64),
				EvmDataWriter::new().write(burn_authority).build(),
			)
			.record(handle)?;
		}

		Ok(succeed([]))
	}

	fn burn_as_collection_owner(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				token_owner: Address, ids: Vec<U256>, amounts: Vec<U256>
			}
		);

		let origin = handle.context().caller;

		let token_owner = H160::from(token_owner);
		let serial_numbers: Vec<SerialNumber> = ids
			.iter()
			.map(|id| {
				ensure!(*id <= u32::MAX.into(), revert("ERC1155: Expected token id <= 2^32"));
				Ok((*id).saturated_into())
			})
			.collect::<Result<Vec<SerialNumber>, PrecompileFailure>>()?;
		let balances: Vec<Balance> = amounts
			.iter()
			.map(|amount| {
				ensure!(
					*amount <= Balance::MAX.into(),
					revert("ERC1155: Expected amounts <= 2^128")
				);
				Ok((*amount).saturated_into())
			})
			.collect::<Result<Vec<Balance>, PrecompileFailure>>()?;

		// Build input BoundedVec from serial_number and amount.
		let combined = serial_numbers.into_iter().zip(balances).collect::<Vec<_>>();
		let serial_numbers: BoundedVec<
			(SerialNumber, Balance),
			<Runtime as pallet_sft::Config>::MaxSerialsPerMint,
		> = BoundedVec::try_from(combined)
			.map_err(|_| revert("ERC1155: Too many serial numbers in one burn"))?;

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_sft::Call::<Runtime>::burn_as_collection_owner {
				token_owner: token_owner.into(),
				collection_id,
				serial_numbers,
			},
		)?;

		log4(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER_BATCH,
			handle.context().caller,
			token_owner,
			H160::zero(),
			EvmDataWriter::new().write(ids).write(amounts).build(),
		)
		.record(handle)?;

		Ok(succeed([]))
	}

	fn burn_auth(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		read_args!(handle, { token_id: U256 });

		if token_id > u32::MAX.into() {
			return Err(revert("ERC721: Expected token id <= 2^32"));
		}
		let token_id: SerialNumber = token_id.saturated_into();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let burn_auth: u8 =
			match pallet_sft::TokenUtilityFlags::<Runtime>::get((collection_id, token_id))
				.burn_authority
			{
				Some(burn_authority) => burn_authority.into(),
				_ => 0, // default to TokenOwner
			};

		Ok(succeed(EvmDataWriter::new().write(burn_auth).build()))
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
