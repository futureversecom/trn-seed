#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use fp_evm::{PrecompileHandle, PrecompileOutput};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::OriginTrait,
};
use pallet_evm::{Context, ExitReason, PrecompileSet};
use pallet_nft::TokenCount;
use sp_core::{H160, U256};
use sp_runtime::traits::SaturatedConversion;
use sp_std::{marker::PhantomData, vec::Vec};

use precompile_utils::{constants::ERC721_PRECOMPILE_ADDRESS_PREFIX, prelude::*};
use seed_primitives::{CollectionUuid, SerialNumber, TokenId};

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

/// Solidity selector of the onERC721Received(address,address,uint256,bytes) function
pub const ON_ERC721_RECEIVED_FUNCTION_SELECTOR: [u8; 4] = [0x15, 0x0b, 0x7a, 0x02];

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
	// The Root Network extensions
	// Mint an NFT in a collection
	// quantity, receiver
	Mint = "mint(address,uint32)",
	OwnedTokens = "ownedTokens(address,uint16,uint32)",
	// Selector used by SafeTransferFrom function
	OnErc721Received = "onERC721Received(address,address,uint256,bytes)",
}

/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
/// 1024-2047 Precompiles that are not in Ethereum Mainnet but are neither CENNZnet specific
/// 2048-4095 Seed specific precompiles
/// NFT precompile addresses can only fall between
/// 	0xAAAAAAAA00000000000000000000000000000000 - 0xAAAAAAAAFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
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
		+ pallet_token_approvals::Config,
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::Call: From<pallet_nft::Call<Runtime>> + From<pallet_token_approvals::Call<Runtime>>,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	<<Runtime as frame_system::Config>::Call as Dispatchable>::Origin: OriginTrait,
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
						| Action::SafeTransferFromCallData => FunctionModifier::NonPayable,
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
						// The Root Network extensions
						Action::Mint => Self::mint(collection_id, handle),
						Action::OwnedTokens => Self::owned_tokens(collection_id, handle),
						_ => {
							return Some(Err(revert("ERC721: Function not implemented yet").into()))
						},
					}
				};
				return Some(result);
			}
		}
		None
	}

	fn is_precompile(&self, address: H160) -> bool {
		if let Some(collection_id) =
			Runtime::evm_id_to_runtime_id(Address(address), ERC721_PRECOMPILE_ADDRESS_PREFIX)
		{
			// Check whether the collection exists
			if !pallet_nft::Pallet::<Runtime>::collection_exists(collection_id) {
				return false
			}
			// Check whether this precompile address has some code stored within the EVM pallet
			if pallet_evm::Pallet::<Runtime>::is_account_empty(&address) {
				// No code exists, let's add some filler code, this will mean the precompile address passes
				// checks that reference an address's byte code i.e. EXTCODESIZE
				pallet_evm::Pallet::<Runtime>::create_account(address, b"ERC721 precompile".to_vec());
			}
			true
		} else {
			false
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
		+ pallet_token_approvals::Config,
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::Call: From<pallet_nft::Call<Runtime>> + From<pallet_token_approvals::Call<Runtime>>,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	<<Runtime as frame_system::Config>::Call as Dispatchable>::Origin: OriginTrait,
{
	/// Returns the CENNZnet address which owns the given token
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
			return Err(revert("ERC721: Expected token id <= 2^32").into());
		}
		let serial_number: SerialNumber = serial_number.saturated_into();

		// Build output.
		match pallet_nft::Pallet::<Runtime>::token_owner(collection_id, serial_number) {
			Some(owner_account_id) => Ok(succeed(
				EvmDataWriter::new()
					.write(Address::from(Into::<H160>::into(owner_account_id)))
					.build(),
			)),
			None => Err(revert(alloc::format!("ERC721: Token does not exist").as_bytes().to_vec())),
		}
	}

	fn balance_of(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Read input.
		read_args!(handle, { owner: Address });
		let owner: H160 = owner.into();

		// Build output.
		Ok(succeed(
			EvmDataWriter::new()
				.write(U256::from(pallet_nft::Pallet::<Runtime>::token_balance_of(
					owner.into(),
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
			return Err(revert("ERC721: Expected token id <= 2^32").into());
		}
		let serial_number: SerialNumber = serial_number.saturated_into();
		let token_id = (collection_id, serial_number);
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		// Get token approval
		let approved_account: Option<Runtime::AccountId> =
			pallet_token_approvals::Pallet::<Runtime>::erc721_approvals(token_id);

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		// Get collection approval
		let approved_for_all_account =
			pallet_token_approvals::Pallet::<Runtime>::erc721_approvals_for_all(
				Runtime::AccountId::from(from),
				collection_id,
			);

		// Build call with origin, check account is approved
		if handle.context().caller == from
			|| Some(Runtime::AccountId::from(handle.context().caller)) == approved_account
			|| Some(Runtime::AccountId::from(handle.context().caller)) == approved_for_all_account
		{
			// Dispatch call (if enough gas).
			RuntimeHelper::<Runtime>::try_dispatch(
				handle,
				Some(Runtime::AccountId::from(from)).into(),
				pallet_nft::Call::<Runtime>::transfer { token_id, new_owner: to.into() },
			)?;
		} else {
			return Err(revert("ERC721: Caller not approved").into());
		}

		log3(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER,
			from,
			to,
			EvmDataWriter::new().write(serial_number).build(),
		)
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
			return Err(revert("ERC721: Expected token id <= 2^32").into());
		}
		let serial_number: SerialNumber = serial_number.saturated_into();
		let token_id = (collection_id, serial_number);
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		// Get token approval
		let approved_account: Option<Runtime::AccountId> =
			pallet_token_approvals::Pallet::<Runtime>::erc721_approvals(token_id);

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		// Get collection approval
		let approved_for_all_account =
			pallet_token_approvals::Pallet::<Runtime>::erc721_approvals_for_all(
				Runtime::AccountId::from(from),
				collection_id,
			);

		// Check approvals/ ownership
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		if Some(Runtime::AccountId::from(handle.context().caller))
			!= pallet_nft::Pallet::<Runtime>::token_owner(collection_id, serial_number)
			&& Some(Runtime::AccountId::from(handle.context().caller)) != approved_account
			&& Some(Runtime::AccountId::from(handle.context().caller)) != approved_for_all_account
		{
			return Err(revert("ERC721: Caller not approved").into());
		}

		// Check that target implements onERC721Received

		// Check that caller is not a smart contract s.t. no code is inserted into
		// pallet_evm::AccountCodes except if the caller is another precompile i.e. CallPermit
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller_code = pallet_evm::Pallet::<Runtime>::account_codes(to);
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
						return Err(
							revert("ERC721: transfer to non ERC721Receiver implementer").into()
						);
					}
				},
				_ => {
					return Err(revert("ERC721: transfer to non ERC721Receiver implementer").into())
				},
			};
		}

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(Runtime::AccountId::from(from)).into(),
			pallet_nft::Call::<Runtime>::transfer { token_id, new_owner: to.into() },
		)?;

		log3(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER,
			from,
			to,
			EvmDataWriter::new().write(serial_number).build(),
		)
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
			return Err(revert("ERC721: Expected token id <= 2^32").into());
		}
		let serial_number: SerialNumber = serial_number.saturated_into();

		let token_id: TokenId = (collection_id, serial_number);
		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_token_approvals::Call::<Runtime>::erc721_approval {
				caller: handle.context().caller.into(),
				operator_account: to.into(),
				token_id,
			},
		)?;

		log3(
			handle.code_address(),
			SELECTOR_LOG_APPROVAL,
			handle.context().caller,
			to,
			EvmDataWriter::new().write(serial_number).build(),
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
			return Err(revert("ERC721: Expected token id <= 2^32").into());
		}
		let serial_number: SerialNumber = serial_number.saturated_into();
		match pallet_token_approvals::Pallet::<Runtime>::erc721_approvals((
			collection_id,
			serial_number,
		)) {
			Some(approved_account) => Ok(succeed(
				EvmDataWriter::new()
					.write(Address::from(Into::<H160>::into(approved_account)))
					.build(),
			)),
			None => Ok(succeed(alloc::format!("ERC721: No accounts approved").as_bytes().to_vec())),
		}
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
		let is_approved = match pallet_token_approvals::Pallet::<Runtime>::erc721_approvals_for_all(
			owner,
			collection_id,
		) {
			Some(approved_account) => approved_account == operator,
			None => false,
		};

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

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_token_approvals::Call::<Runtime>::erc721_approval_for_all {
				caller: handle.context().caller.into(),
				operator_account: operator.clone().into(),
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

		match pallet_nft::Pallet::<Runtime>::collection_info(collection_id) {
			Some(collection_info) => Ok(succeed(
				EvmDataWriter::new()
					.write::<Bytes>(collection_info.name.as_slice().into())
					.build(),
			)),
			None => {
				Err(revert(alloc::format!("ERC721: Collection does not exist").as_bytes().to_vec()))
			},
		}
	}

	fn symbol(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Build output.
		// TODO: Returns same as name
		match pallet_nft::Pallet::<Runtime>::collection_info(collection_id) {
			Some(collection_info) => Ok(succeed(
				EvmDataWriter::new()
					.write::<Bytes>(collection_info.name.as_slice().into())
					.build(),
			)),
			None => {
				Err(revert(alloc::format!("ERC721: Collection does not exist").as_bytes().to_vec()))
			},
		}
	}

	fn token_uri(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		read_args!(handle, { serial_number: U256 });

		// For now we only support Ids < u32 max
		// since `u32` is the native `SerialNumber` type used by the NFT module.
		// it's not possible for the module to issue Ids larger than this
		if serial_number > u32::MAX.into() {
			return Err(revert("ERC721: Expected token id <= 2^32").into());
		}
		let serial_number: SerialNumber = serial_number.saturated_into();

		// Build output.
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
			return Err(revert("ERC721: Expected quantity <= 2^32").into());
		}
		let quantity: TokenCount = quantity.saturated_into();
		let origin = handle.context().caller;

		// emit transfer events - quantity times
		// reference impl: https://github.com/chiru-labs/ERC721A/blob/1843596cf863557fcd3bf0105222a7c29690af5c/contracts/ERC721A.sol#L789
		let serial_number =
			pallet_nft::Pallet::<Runtime>::next_serial_number(collection_id).unwrap_or_default();
		for token_id in serial_number..(serial_number.saturating_add(quantity)) {
			log3(
				handle.code_address(),
				SELECTOR_LOG_TRANSFER,
				origin,
				to,
				EvmDataWriter::new().write(token_id).build(),
			)
			.record(handle)?;
		}

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

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
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
			return Err(revert("ERC721: Expected limit <= 2^32").into());
		}
		let limit: u16 = limit.saturated_into();
		if cursor > SerialNumber::MAX.into() {
			return Err(revert("ERC721: Expected cursor <= 2^32").into());
		}
		let cursor: SerialNumber = cursor.saturated_into();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let (new_cursor, collected_tokens) = pallet_nft::Pallet::<Runtime>::owned_tokens_paginated(
			collection_id,
			&owner.into(),
			cursor,
			limit,
		);
		// Build output.
		Ok(succeed(
			EvmDataWriter::new()
				.write::<u32>(new_cursor)
				.write::<Vec<u32>>(collected_tokens)
				.build(),
		))
	}

	fn owner(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		match pallet_nft::Pallet::<Runtime>::collection_info(collection_id) {
			Some(collection_info) => Ok(succeed(
				EvmDataWriter::new()
					.write(Address::from(Into::<H160>::into(collection_info.owner)))
					.build(),
			)),
			None => {
				Err(revert(alloc::format!("ERC721: Collection does not exist").as_bytes().to_vec()))
			},
		}
	}

	fn renounce_ownership(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
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
		log2(
			handle.code_address(),
			SELECTOR_LOG_OWNERSHIP_TRANSFERRED,
			origin,
			EvmDataWriter::new().write(Address::from(burn_account)).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn transfer_ownership(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		let origin = handle.context().caller;

		// Parse input.
		read_args!(handle, { new_owner: Address });
		let new_owner: H160 = new_owner.into();

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_nft::Call::<Runtime>::set_owner { collection_id, new_owner: new_owner.into() },
		)?;

		log2(
			handle.code_address(),
			SELECTOR_LOG_OWNERSHIP_TRANSFERRED,
			origin,
			EvmDataWriter::new().write(Address::from(new_owner)).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}
}
