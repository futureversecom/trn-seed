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

use core::convert::TryFrom;
use fp_evm::{IsPrecompileResult, PrecompileHandle, PrecompileOutput};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::OriginTrait,
};
use pallet_evm::{Context, ExitReason, PrecompileSet};
use precompile_utils::{
	constants::{ERC20_PRECOMPILE_ADDRESS_PREFIX, ERC721_PRECOMPILE_ADDRESS_PREFIX},
	prelude::*,
};
use seed_pallet_common::{utils::TokenBurnAuthority, NFTExt};
use seed_primitives::{
	AssetId, Balance, CollectionUuid, EthAddress, IssuanceId, SerialNumber, TokenCount, TokenId,
};
use sp_core::{Encode, H160, H256, U256};
use sp_runtime::{traits::SaturatedConversion, BoundedVec};
use sp_std::{marker::PhantomData, vec, vec::Vec};

/// Solidity selector of the Transfer log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_TRANSFER: [u8; 32] = keccak256!("Transfer(address,address,uint256)");

/// Solidity selector of the Approval log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_APPROVAL: [u8; 32] = keccak256!("Approval(address,address,uint256)");

/// Solidity selector of the Approval for all log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_APPROVAL_FOR_ALL: [u8; 32] =
	keccak256!("ApprovalForAll(address,address,bool)");

/// Solidity selector of the OwnershipTransferred log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_OWNERSHIP_TRANSFERRED: [u8; 32] =
	keccak256!("OwnershipTransferred(address,address)");

/// Solidity selector of the XLS20CompatibilityEnabled log, which is the Keccak of the Log
/// signature.
pub const SELECTOR_LOG_XLS20_ENABLED: [u8; 32] = keccak256!("XLS20CompatibilityEnabled()");

/// Solidity selector of the Xls20MintReRequested log, which is the Keccak of the Log
/// signature.
pub const SELECTOR_LOG_XLS20_RE_REQUESTED: [u8; 32] = keccak256!("XLS20MintReRequested(uint256)");

pub const MAX_SUPPLY_UPDATED: [u8; 32] = keccak256!("MaxSupplyUpdated(uint32)");

pub const BASE_URI_UPDATED: [u8; 32] = keccak256!("BaseURIUpdated(string)");

pub const SELECTOR_LOG_PUBLIC_MINT_TOGGLED: [u8; 32] = keccak256!("PublicMintToggled(bool)");

pub const SELECTOR_LOG_MINT_FEE_UPDATED: [u8; 32] = keccak256!("MintFeeUpdated(address,uint128)");

pub const SELECTOR_PENDING_ISSUANCE_CREATED: [u8; 32] =
	keccak256!("PendingIssuanceCreated(address,uint256,u8)");

pub const SELECTOR_ISSUED: [u8; 32] = keccak256!("Issued(address,address,uint256,uint8)");

/// Solidity selector of the onERC721Received(address,address,uint256,bytes) function
pub const ON_ERC721_RECEIVED_FUNCTION_SELECTOR: [u8; 4] = [0x15, 0x0b, 0x7a, 0x02];

/// Interface IDs for the ERC165, ERC721, ERC721Metadata, ERC721Burnable, Ownable, and TRN721 interfaces
pub const ERC165_INTERFACE_IDS: &[u32] = &[
	0x01ffc9a7, // ERC165
	0x80ac58cd, // ERC721
	0x5b5e139f, // ERC721Metadata
	0x42966c68, // ERC721Burnable
	0x0e083076, // Ownable
	0x2a4288ec, // TRN721
];

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	BalanceOf = "balanceOf(address)",
	OwnerOf = "ownerOf(uint256)",
	TransferFrom = "transferFrom(address,address,uint256)",
	SafeTransferFrom = "safeTransferFrom(address,address,uint256)",
	SafeTransferFromCallData = "safeTransferFrom(address,address,uint256,bytes)",
	Approve = "approve(address,uint256)",
	GetApproved = "getApproved(uint256)",
	IsApprovedForAll = "isApprovedForAll(address,address)",
	SetApprovalForAll = "setApprovalForAll(address,bool)",
	// Metadata extensions
	Name = "name()",
	Symbol = "symbol()",
	TokenURI = "tokenURI(uint256)",
	// Ownable - https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/access/Ownable.sol
	Owner = "owner()",
	RenounceOwnership = "renounceOwnership()",
	TransferOwnership = "transferOwnership(address)",
	// ERC721Burnable - https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/token/ERC721/extensions/ERC721Burnable.sol
	Burn = "burn(uint256)",
	// The Root Network extensions
	TotalSupply = "totalSupply()",
	Mint = "mint(address,uint32)",
	SetMaxSupply = "setMaxSupply(uint32)",
	SetBaseURI = "setBaseURI(bytes)",
	OwnedTokens = "ownedTokens(address,uint16,uint32)",
	TogglePublicMint = "togglePublicMint(bool)",
	SetMintFee = "setMintFee(address,uint128)",
	// Selector used by SafeTransferFrom function
	OnErc721Received = "onERC721Received(address,address,uint256,bytes)",
	// XLS-20 extensions
	EnableXls20Compatibility = "enableXls20Compatibility()",
	ReRequestXls20Mint = "reRequestXls20Mint(uint32[])",
	// ERC165 - https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/utils/introspection/ERC165.sol
	SupportsInterface = "supportsInterface(bytes4)",
	// ERC5484 Soulbound tokens
	IssueSoulbound = "issueSoulbound(address,uint32,uint8)",
	AcceptSoulboundIssuance = "acceptSouldboundIssuance(uint32)",
	PendingIssuances = "pendingIssuances(address)",
	BurnAuth = "burnAuth(uint256)",
}

/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
/// 1024-2047 Precompiles that are not in Ethereum Mainnet but are neither Root specific
/// 2048-4095 Seed specific precompiles
/// NFT precompile addresses can only fall between
///     0xAAAAAAAA00000000000000000000000000000000 - 0xAAAAAAAAFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
/// The precompile for NFT series X where X is a u32 (i.e.4 bytes), if 0XFFFFFFFF +
/// Bytes(CollectionUuid) In order to route the address to Erc721Precompile<R>, we
/// check whether the CollectionUuid exists in pallet-nft

/// This means that every address that starts with 0xAAAAAAAA will go through an additional db read,
/// but the probability for this to happen is 2^-32 for random addresses
pub struct Erc721PrecompileSet<Runtime>(PhantomData<Runtime>);

impl<T> Default for Erc721PrecompileSet<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> PrecompileSet for Erc721PrecompileSet<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: pallet_nft::Config
		+ pallet_evm::Config
		+ frame_system::Config
		+ pallet_token_approvals::Config
		+ pallet_xls20::Config,
	<Runtime as frame_system::Config>::RuntimeCall:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_nft::Call<Runtime>>
		+ From<pallet_xls20::Call<Runtime>>
		+ From<pallet_token_approvals::Call<Runtime>>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<Runtime::AccountId>>,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: OriginTrait,
{
	fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<EvmResult<PrecompileOutput>> {
		// Convert target `address` into it's runtime NFT Id
		if let Some(collection_id) = Runtime::evm_id_to_runtime_id(
			Address(handle.code_address()),
			ERC721_PRECOMPILE_ADDRESS_PREFIX,
		) {
			// 'collection name' is empty when the collection doesn't exist yet
			if pallet_nft::Pallet::<Runtime>::collection_exists(collection_id) {
				let result = {
					let selector = match handle.read_selector() {
						Ok(selector) => selector,
						Err(e) => return Some(Err(e.into())),
					};

					if let Err(err) = handle.check_function_modifier(match selector {
						Action::Approve
						| Action::SafeTransferFrom
						| Action::TransferFrom
						| Action::SafeTransferFromCallData
						| Action::SetApprovalForAll
						| Action::SetMaxSupply
						| Action::RenounceOwnership
						| Action::TransferOwnership
						| Action::SetBaseURI
						| Action::EnableXls20Compatibility
						| Action::ReRequestXls20Mint
						| Action::TogglePublicMint
						| Action::SetMintFee
						| Action::Mint => FunctionModifier::NonPayable,
						_ => FunctionModifier::View,
					}) {
						return Some(Err(err.into()));
					}

					match selector {
						// Core ERC721
						Action::OwnerOf => Self::owner_of(collection_id, handle),
						Action::BalanceOf => Self::balance_of(collection_id, handle),
						Action::Approve => Self::approve(collection_id, handle),
						Action::GetApproved => Self::get_approved(collection_id, handle),
						Action::TransferFrom => Self::transfer_from(collection_id, handle),
						Action::SafeTransferFrom => Self::safe_transfer_from(collection_id, handle),
						Action::SafeTransferFromCallData => {
							Self::safe_transfer_from_call_data(collection_id, handle)
						},
						Action::IsApprovedForAll => {
							Self::is_approved_for_all(collection_id, handle)
						},
						Action::SetApprovalForAll => {
							Self::set_approval_for_all(collection_id, handle)
						},
						// ERC721-Metadata
						Action::Name => Self::name(collection_id, handle),
						Action::Symbol => Self::symbol(collection_id, handle),
						Action::TokenURI => Self::token_uri(collection_id, handle),
						// Ownable
						Action::Owner => Self::owner(collection_id, handle),
						Action::RenounceOwnership => {
							Self::renounce_ownership(collection_id, handle)
						},
						Action::TransferOwnership => {
							Self::transfer_ownership(collection_id, handle)
						},
						// Burnable
						Action::Burn => Self::burn(collection_id, handle),
						// The Root Network extensions
						Action::TotalSupply => Self::total_supply(collection_id, handle),
						Action::Mint => Self::mint(collection_id, handle),
						Action::SetMaxSupply => Self::set_max_supply(collection_id, handle),
						Action::SetBaseURI => Self::set_base_uri(collection_id, handle),
						Action::OwnedTokens => Self::owned_tokens(collection_id, handle),
						Action::TogglePublicMint => Self::toggle_public_mint(collection_id, handle),
						Action::SetMintFee => Self::set_mint_fee(collection_id, handle),
						// XLS-20 extensions
						Action::EnableXls20Compatibility => {
							Self::enable_xls20_compatibility(collection_id, handle)
						},
						Action::ReRequestXls20Mint => {
							Self::re_request_xls20_mint(collection_id, handle)
						},
						// ERC165
						Action::SupportsInterface => Self::supports_interface(handle),
						// ERC5484
						Action::IssueSoulbound => Self::issue_soulbound(collection_id, handle),
						Action::PendingIssuances => Self::pending_issuances(collection_id, handle),
						Action::AcceptSoulboundIssuance => {
							Self::accept_soulbound_issuance(collection_id, handle)
						},
						Action::BurnAuth => Self::burn_auth(collection_id, handle),
						_ => return Some(Err(revert("ERC721: Function not implemented"))),
					}
				};
				return Some(result);
			}
		}
		None
	}

	fn is_precompile(&self, address: H160, _remaining_gas: u64) -> IsPrecompileResult {
		if let Some(collection_id) =
			Runtime::evm_id_to_runtime_id(Address(address), ERC721_PRECOMPILE_ADDRESS_PREFIX)
		{
			let extra_cost = RuntimeHelper::<Runtime>::db_read_gas_cost();
			// Check whether the collection exists
			IsPrecompileResult::Answer {
				is_precompile: pallet_nft::Pallet::<Runtime>::collection_exists(collection_id),
				extra_cost,
			}
		} else {
			IsPrecompileResult::Answer { is_precompile: false, extra_cost: 0 }
		}
	}
}

impl<Runtime> Erc721PrecompileSet<Runtime> {
	pub fn new() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> Erc721PrecompileSet<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: pallet_nft::Config
		+ pallet_evm::Config
		+ frame_system::Config
		+ pallet_token_approvals::Config
		+ pallet_xls20::Config,
	<Runtime as frame_system::Config>::RuntimeCall:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_nft::Call<Runtime>>
		+ From<pallet_xls20::Call<Runtime>>
		+ From<pallet_token_approvals::Call<Runtime>>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<Runtime::AccountId>>,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: OriginTrait,
{
	/// Returns the Root address which owns the given token
	/// An error is returned if the token doesn't exist
	fn owner_of(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		// Parse input.
		read_args!(handle, { serial_number: U256 });

		// For now we only support Ids < u32 max
		// since `u32` is the native `SerialNumber` type used by the NFT module.
		// it's not possible for the module to issue Ids larger than this
		if serial_number > u32::MAX.into() {
			return Err(revert("ERC721: Expected token id <= 2^32"));
		}
		let serial_number: SerialNumber = serial_number.saturated_into();

		// Build output.
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		match pallet_nft::Pallet::<Runtime>::get_token_owner(&(collection_id, serial_number)) {
			Some(owner_account_id) => Ok(succeed(
				EvmDataWriter::new()
					.write(Address::from(Into::<H160>::into(owner_account_id)))
					.build(),
			)),
			None => Err(revert("ERC721: Token does not exist")),
		}
	}

	fn balance_of(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		// Read input.
		read_args!(handle, { owner: Address });
		let owner: H160 = owner.into();

		// Build output.
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		Ok(succeed(
			EvmDataWriter::new()
				.write(U256::from(pallet_nft::Pallet::<Runtime>::token_balance_of(
					&owner.into(),
					collection_id,
				)))
				.build(),
		))
	}

	fn transfer_from(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				from: Address,
				to: Address,
				serial_number: U256
			}
		);
		let from: H160 = from.into();
		let to: H160 = to.into();

		// For now we only support Ids < u32 max
		// since `u32` is the native `SerialNumber` type used by the NFT module.
		// it's not possible for the module to issue Ids larger than this
		if serial_number > u32::MAX.into() {
			return Err(revert("ERC721: Expected token id <= 2^32"));
		}
		let serial_number: SerialNumber = serial_number.saturated_into();

		// Check approvals/ ownership
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost().saturating_mul(3))?;
		if pallet_token_approvals::Pallet::<Runtime>::is_approved_or_owner(
			(collection_id, serial_number),
			Runtime::AccountId::from(handle.context().caller),
		) {
			let serial_numbers_unbounded: Vec<SerialNumber> = vec![serial_number];
			let serial_numbers: BoundedVec<
				SerialNumber,
				<Runtime as pallet_nft::Config>::TransferLimit,
			> = BoundedVec::try_from(serial_numbers_unbounded).expect("Should not fail");
			// Dispatch call (if enough gas).
			RuntimeHelper::<Runtime>::try_dispatch(
				handle,
				Some(Runtime::AccountId::from(from)).into(),
				pallet_nft::Call::<Runtime>::transfer {
					collection_id,
					serial_numbers,
					new_owner: to.into(),
				},
			)?;
		} else {
			return Err(revert("ERC721: Caller not approved"));
		}

		let serial_number = H256::from_low_u64_be(serial_number as u64);
		log4(handle.code_address(), SELECTOR_LOG_TRANSFER, from, to, serial_number, vec![])
			.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn safe_transfer_from(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				from: Address,
				to: Address,
				serial_number: U256
			}
		);
		let data: &[u8] = b"";
		Self::do_safe_transfer(collection_id, handle, from, to, serial_number, Bytes::from(data))
	}

	fn safe_transfer_from_call_data(
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
				serial_number: U256,
				data: Bytes
			}
		);
		Self::do_safe_transfer(collection_id, handle, from, to, serial_number, data)
	}

	fn do_safe_transfer(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
		from: Address,
		to: Address,
		serial_number: U256,
		data: Bytes,
	) -> EvmResult<PrecompileOutput> {
		let from: H160 = from.into();
		let to: H160 = to.into();

		// For now we only support Ids < u32 max
		// since `u32` is the native `SerialNumber` type used by the NFT module.
		// it's not possible for the module to issue Ids larger than this
		if serial_number > u32::MAX.into() {
			return Err(revert("ERC721: Expected token id <= 2^32"));
		}
		let serial_number: SerialNumber = serial_number.saturated_into();
		let token_id: TokenId = (collection_id, serial_number);

		// Check approvals/ ownership
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost().saturating_mul(3))?;
		if !pallet_token_approvals::Pallet::<Runtime>::is_approved_or_owner(
			token_id,
			Runtime::AccountId::from(handle.context().caller),
		) {
			return Err(revert("ERC721: Caller not approved"));
		}

		// Check that target implements onERC721Received
		// Check that caller is not a smart contract s.t. no code is inserted into
		// pallet_evm::AccountCodes except if the caller is another precompile i.e. CallPermit
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller_code = pallet_evm::AccountCodes::<Runtime>::get(to);
		if !(caller_code.is_empty()) {
			// Setup input for onErc721Received call
			let sub_context = Context {
				address: to,
				caller: handle.context().caller,
				apparent_value: Default::default(),
			};
			let input = EvmDataWriter::new_with_selector(Action::OnErc721Received)
				.write::<Address>(from.into())
				.write::<Address>(to.into())
				.write::<U256>(serial_number.into())
				.write::<Bytes>(data)
				.build();
			let (reason, output) =
				handle.call(to, None, input.clone(), handle.gas_limit(), false, &sub_context);
			// Check response from call
			match reason {
				ExitReason::Succeed(_) => {
					if output[..4] != ON_ERC721_RECEIVED_FUNCTION_SELECTOR.to_vec() {
						return Err(revert("ERC721: transfer to non ERC721Receiver implementer"));
					}
				},
				_ => return Err(revert("ERC721: transfer to non ERC721Receiver implementer")),
			};
		}

		// Dispatch call (if enough gas).
		let serial_numbers_unbounded: Vec<SerialNumber> = vec![serial_number];
		let serial_numbers: BoundedVec<
			SerialNumber,
			<Runtime as pallet_nft::Config>::TransferLimit,
		> = BoundedVec::try_from(serial_numbers_unbounded).expect("Should not fail");

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(Runtime::AccountId::from(from)).into(),
			pallet_nft::Call::<Runtime>::transfer {
				collection_id,
				serial_numbers,
				new_owner: to.into(),
			},
		)?;

		let serial_number = H256::from_low_u64_be(serial_number as u64);
		log4(handle.code_address(), SELECTOR_LOG_TRANSFER, from, to, serial_number, vec![])
			.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn approve(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				to: Address,
				serial_number: U256
			}
		);
		let to: H160 = to.into();

		// For now we only support Ids < u32 max
		// since `u32` is the native `SerialNumber` type used by the NFT module.
		// it's not possible for the module to issue Ids larger than this
		if serial_number > u32::MAX.into() {
			return Err(revert("ERC721: Expected token id <= 2^32"));
		}
		let serial_number: SerialNumber = serial_number.saturated_into();

		let token_id: TokenId = (collection_id, serial_number);
		let caller = handle.context().caller;
		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(Runtime::AccountId::from(caller)).into(),
			pallet_token_approvals::Call::<Runtime>::erc721_approval {
				operator_account: to.into(),
				token_id,
			},
		)?;

		let serial_number = H256::from_low_u64_be(serial_number as u64);
		log4(
			handle.code_address(),
			SELECTOR_LOG_APPROVAL,
			handle.context().caller,
			to,
			serial_number,
			vec![],
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn get_approved(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		// Parse input.
		read_args!(handle, { serial_number: U256 });
		// For now we only support Ids < u32 max
		// since `u32` is the native `SerialNumber` type used by the NFT module.
		// it's not possible for the module to issue Ids larger than this
		if serial_number > u32::MAX.into() {
			return Err(revert("ERC721: Expected token id <= 2^32"));
		}
		let serial_number: SerialNumber = serial_number.saturated_into();

		// Return either the approved account or zero address if no account is approved
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let approved_account: H160 = match pallet_token_approvals::ERC721Approvals::<Runtime>::get(
			(collection_id, serial_number),
		) {
			Some(approved_account) => (approved_account).into(),
			None => H160::default(),
		};

		Ok(succeed(EvmDataWriter::new().write(Address::from(approved_account)).build()))
	}

	fn is_approved_for_all(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;

		// Parse input.
		read_args!(handle, { owner: Address, operator: Address });
		let owner: Runtime::AccountId = H160::from(owner).into();
		let operator: Runtime::AccountId = H160::from(operator).into();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let is_approved = pallet_token_approvals::ERC721ApprovalsForAll::<Runtime>::get(
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
		handle.record_log_costs_manual(2, 32)?;

		// Parse input.
		read_args!(handle, { operator: Address, approved: bool });
		let operator = H160::from(operator);
		let caller = handle.context().caller;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(Runtime::AccountId::from(caller)).into(),
			pallet_token_approvals::Call::<Runtime>::erc721_approval_for_all {
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

	fn name(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		match pallet_nft::CollectionInfo::<Runtime>::get(collection_id) {
			Some(collection_info) => Ok(succeed(
				EvmDataWriter::new()
					.write::<Bytes>(collection_info.name.as_slice().into())
					.build(),
			)),
			None => Err(revert("ERC721: Collection does not exist")),
		}
	}

	fn symbol(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Build output.
		// TODO: Returns same as name
		match pallet_nft::CollectionInfo::<Runtime>::get(collection_id) {
			Some(collection_info) => Ok(succeed(
				EvmDataWriter::new()
					.write::<Bytes>(collection_info.name.as_slice().into())
					.build(),
			)),
			None => Err(revert("ERC721: Collection does not exist")),
		}
	}

	fn token_uri(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;
		read_args!(handle, { serial_number: U256 });

		// For now we only support Ids < u32 max
		// since `u32` is the native `SerialNumber` type used by the NFT module.
		// it's not possible for the module to issue Ids larger than this
		if serial_number > u32::MAX.into() {
			return Err(revert("ERC721: Expected token id <= 2^32"));
		}
		let serial_number: SerialNumber = serial_number.saturated_into();

		// Build output.
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		Ok(succeed(
			EvmDataWriter::new()
				.write::<Bytes>(
					pallet_nft::Pallet::<Runtime>::token_uri((collection_id, serial_number))
						.as_slice()
						.into(),
				)
				.build(),
		))
	}

	fn total_supply(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		match pallet_nft::CollectionInfo::<Runtime>::get(collection_id) {
			Some(collection_info) => Ok(succeed(
				EvmDataWriter::new()
					.write::<U256>(collection_info.collection_issuance.into())
					.build(),
			)),
			None => Err(revert("ERC721: Collection does not exist")),
		}
	}

	fn mint(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				to: Address,
				quantity: U256
			}
		);
		let to: H160 = to.into();

		// Parse quantity
		if quantity > TokenCount::MAX.into() {
			return Err(revert("ERC721: Expected quantity <= 2^32"));
		}
		let quantity: TokenCount = quantity.saturated_into();
		let origin = handle.context().caller;

		// emit transfer events - quantity times
		// reference impl: https://github.com/chiru-labs/ERC721A/blob/1843596cf863557fcd3bf0105222a7c29690af5c/contracts/ERC721A.sol#L789
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let serial_number = match pallet_nft::CollectionInfo::<Runtime>::get(collection_id) {
			Some(collection_info) => collection_info.next_serial_number,
			None => return Err(revert("Collection does not exist")),
		};

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_nft::Call::<Runtime>::mint {
				collection_id,
				quantity,
				token_owner: Some(to.into()),
			},
		)?;

		for token_id in serial_number..(serial_number.saturating_add(quantity)) {
			let token_id = H256::from_low_u64_be(token_id as u64);
			log4(
				handle.code_address(),
				SELECTOR_LOG_TRANSFER,
				EthAddress::zero(),
				to,
				token_id,
				vec![],
			)
			.record(handle)?;
		}

		// Build output.
		Ok(succeed([]))
	}

	fn set_max_supply(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		// Parse input.
		read_args!(handle, { max_supply: U256 });

		// Parse max_supply
		if max_supply > TokenCount::MAX.into() {
			return Err(revert("ERC721: Expected max_supply <= 2^32"));
		}
		let max_issuance: TokenCount = max_supply.saturated_into();
		let origin = handle.context().caller;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_nft::Call::<Runtime>::set_max_issuance { collection_id, max_issuance },
		)?;

		// Emit event.
		log1(
			handle.code_address(),
			MAX_SUPPLY_UPDATED,
			EvmDataWriter::new().write(max_supply).build(),
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

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_nft::Call::<Runtime>::set_base_uri {
				collection_id,
				base_uri: base_uri.0.to_vec(),
			},
		)?;

		// Emit event.
		log1(handle.code_address(), BASE_URI_UPDATED, EvmDataWriter::new().write(base_uri).build())
			.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn owned_tokens(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		read_args!(handle, { owner: Address, limit: U256, cursor: U256 });

		// Parse inputs
		let owner: H160 = owner.into();
		if limit > u16::MAX.into() {
			return Err(revert("ERC721: Expected limit <= 2^32"));
		}
		let limit: u16 = limit.saturated_into();
		if cursor > SerialNumber::MAX.into() {
			return Err(revert("ERC721: Expected cursor <= 2^32"));
		}
		let cursor: SerialNumber = cursor.saturated_into();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let (new_cursor, total_owned, collected_tokens) =
			pallet_nft::Pallet::<Runtime>::owned_tokens(
				collection_id,
				&owner.into(),
				cursor,
				limit,
			);
		// Build output.
		Ok(succeed(
			EvmDataWriter::new()
				.write::<u32>(new_cursor)
				.write::<u32>(total_owned)
				.write::<Vec<u32>>(collected_tokens)
				.build(),
		))
	}

	fn toggle_public_mint(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		read_args!(handle, { enabled: bool });

		// Dispatch call (if enough gas).
		let origin = handle.context().caller;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_nft::Call::<Runtime>::toggle_public_mint { collection_id, enabled },
		)?;

		log1(
			handle.code_address(),
			SELECTOR_LOG_PUBLIC_MINT_TOGGLED,
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

		read_args!(handle, { payment_asset: Address, mint_fee: U256 });

		// Parse inputs
		let asset_id: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			payment_asset,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("ERC721: Invalid payment asset address"))?;
		if mint_fee > Balance::MAX.into() {
			return Err(revert("ERC721: Expected mint_fee <= 2^128"));
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
			pallet_nft::Call::<Runtime>::set_mint_fee { collection_id, pricing_details },
		)?;

		log3(
			handle.code_address(),
			SELECTOR_LOG_MINT_FEE_UPDATED,
			H160::from(payment_asset),
			H256::from_slice(&EvmDataWriter::new().write(mint_fee).build()),
			vec![],
		)
		.record(handle)?;

		Ok(succeed([]))
	}

	fn owner(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		match pallet_nft::CollectionInfo::<Runtime>::get(collection_id) {
			Some(collection_info) => Ok(succeed(
				EvmDataWriter::new()
					.write(Address::from(Into::<H160>::into(collection_info.owner)))
					.build(),
			)),
			None => Err(revert("ERC721: Collection does not exist")),
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
			pallet_nft::Call::<Runtime>::set_owner {
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
			pallet_nft::Call::<Runtime>::set_owner { collection_id, new_owner: new_owner.into() },
		)?;

		log3(handle.code_address(), SELECTOR_LOG_OWNERSHIP_TRANSFERRED, origin, new_owner, vec![])
			.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn burn(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(4, 32)?;

		// Parse input.
		read_args!(handle, { serial_number: U256 });

		// For now we only support Ids < u32 max
		// since `u32` is the native `SerialNumber` type used by the NFT module.
		// it's not possible for the module to issue Ids larger than this
		if serial_number > u32::MAX.into() {
			return Err(revert("ERC721: Expected token id <= 2^32"));
		}
		let serial_number: SerialNumber = serial_number.saturated_into();
		let token_id: TokenId = (collection_id, serial_number);
		let origin = handle.context().caller;

		// Check if caller is approved
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost().saturating_mul(3))?;
		if !pallet_token_approvals::Pallet::<Runtime>::is_approved_or_owner(
			token_id,
			Runtime::AccountId::from(origin),
		) {
			return Err(revert("ERC721: Caller not approved"));
		}

		// Get token owner and call burn from the owner address
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let owner = match pallet_nft::Pallet::<Runtime>::get_token_owner(&token_id) {
			Some(owner) => owner,
			None => return Err(revert("ERC721: Token does not exist")),
		};
		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(owner.clone()).into(),
			pallet_nft::Call::<Runtime>::burn { token_id: (collection_id, serial_number) },
		)?;

		// Record transfer log to zero address
		let serial_number = H256::from_low_u64_be(serial_number as u64);
		log4(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER,
			owner.into(),
			H160::default(),
			serial_number,
			vec![],
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn enable_xls20_compatibility(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		let origin = handle.context().caller;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_xls20::Call::<Runtime>::enable_xls20_compatibility { collection_id },
		)?;

		log0(handle.code_address(), SELECTOR_LOG_XLS20_ENABLED).record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn re_request_xls20_mint(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;

		// Parse input.
		read_args!(handle, { serial_numbers: Vec<U256> });
		let origin = handle.context().caller;

		// Convert serial numbers from U256 -> u32
		// Fails if overflow (Although should not happen)
		let mut serial_numbers_unbounded: Vec<SerialNumber> = vec![];
		for serial_number in serial_numbers {
			if serial_number > u32::MAX.into() {
				return Err(revert("XLS20: Expected serial_number <= 2^32"));
			}
			serial_numbers_unbounded.push(serial_number.saturated_into())
		}
		let serial_numbers: BoundedVec<
			SerialNumber,
			<Runtime as pallet_xls20::Config>::MaxTokensPerXls20Mint,
		> = BoundedVec::try_from(serial_numbers_unbounded).expect("Should not fail");

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_xls20::Call::<Runtime>::re_request_xls20_mint {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			},
		)?;

		// Drop log for every serial number requested
		for serial_number in serial_numbers.iter() {
			let token_id: TokenId = (collection_id, *serial_number);
			log1(
				handle.code_address(),
				SELECTOR_LOG_XLS20_RE_REQUESTED,
				EvmDataWriter::new().write(token_id).build(),
			)
			.record(handle)?;
		}

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
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
				to: Address,
				quantity: U256,
				burn_authority: u8
			}
		);
		let to: H160 = to.into();

		// Parse quantity
		if quantity > TokenCount::MAX.into() {
			return Err(revert("ERC721: Expected quantity <= 2^32"));
		}
		let quantity: TokenCount = quantity.saturated_into();

		if burn_authority > u8::MAX.into() {
			return Err(revert("ERC721: Expected burn authority <= 2^8"));
		}
		let burn_authority = match TokenBurnAuthority::try_from(burn_authority) {
			Ok(b) => b,
			_ => return Err(revert("ERC721: Could not parse burn authority")),
		};

		let origin = handle.context().caller;

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let next_issuance_id = pallet_nft::NextIssuanceId::<Runtime>::get();

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_nft::Call::<Runtime>::issue_soulbound {
				collection_id,
				quantity,
				token_owner: to.into(),
				burn_authority,
			},
		)?;

		log2(
			handle.code_address(),
			SELECTOR_PENDING_ISSUANCE_CREATED,
			to,
			EvmDataWriter::new()
				.write(next_issuance_id)
				.write(quantity)
				.write(<TokenBurnAuthority as Into<u8>>::into(burn_authority))
				.build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn pending_issuances(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		read_args!(handle, { owner: Address });

		let owner: H160 = owner.into();

		let mut iter = pallet_nft::PendingIssuances::<Runtime>::iter_prefix((
			collection_id,
			Runtime::AccountId::from(owner),
		));

		let mut issuance_ids: Vec<IssuanceId> = Vec::new();
		let mut issuances: Vec<(U256, u8)> = Vec::new();

		while let Some((p, q)) = iter.next() {
			// Record gas cost before processing the item
			handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

			// Process and store the values
			issuance_ids.push(p);
			issuances.push((U256::from(q.quantity), q.burn_authority.into()));
		}

		Ok(succeed(EvmDataWriter::new().write(issuance_ids).write(issuances).build()))
	}

	fn accept_soulbound_issuance(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		read_args!(handle, { issuance_id: U256 });

		if issuance_id > IssuanceId::MAX.into() {
			return Err(revert("ERC721: Expected issuance id <= 2^64"));
		}
		let issuance_id: IssuanceId = issuance_id.saturated_into();

		let origin = handle.context().caller;

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let collection = match pallet_nft::CollectionInfo::<Runtime>::get(collection_id) {
			Some(collection_info) => collection_info,
			None => return Err(revert("Collection does not exist")),
		};

		let collection_owner = collection.owner;
		let serial_number = collection.next_serial_number;

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let Some(pending_issuance) = pallet_nft::PendingIssuances::<Runtime>::get((
			collection_id,
			Runtime::AccountId::from(origin),
			issuance_id,
		)) else {
			return Err(revert("Issuance does not exist"));
		};

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_nft::Call::<Runtime>::accept_soulbound_issuance { collection_id, issuance_id },
		)?;

		for sn in serial_number..(serial_number + pending_issuance.quantity) {
			log4(
				handle.code_address(),
				SELECTOR_ISSUED,
				collection_owner.clone().into(),
				origin,
				H256::from_low_u64_be(sn as u64),
				EvmDataWriter::new()
					.write(<TokenBurnAuthority as Into<u8>>::into(pending_issuance.burn_authority))
					.build(),
			)
			.record(handle)?;
		}

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
		let serial_number: SerialNumber = token_id.saturated_into();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let token_info = match pallet_nft::TokenInfo::<Runtime>::get(collection_id, serial_number) {
			Some(token_info) => token_info,
			None => return Err(revert("ERC721: Token does not exist")),
		};
		let burn_auth: u8 = match token_info.utility_flags.burn_authority {
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
