// Copyright 2019-2022 Centrality Investments Ltd.
// This file is part of CENNZnet.

// CENNZnet is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// CENNZnet is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with CENNZnet.  If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use fp_evm::{ExitSucceed, PrecompileFailure, PrecompileHandle, PrecompileOutput};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::OriginTrait,
};
use pallet_evm::PrecompileSet;
use sp_core::{H160, U256};
use sp_runtime::traits::SaturatedConversion;
use sp_std::marker::PhantomData;

use precompile_utils::{constants::ERC721_PRECOMPILE_ADDRESS_PREFIX, prelude::*, ExitRevert};
use seed_primitives::{CollectionUuid, SerialNumber};

/// Solidity selector of the Transfer log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_TRANSFER: [u8; 32] = keccak256!("Transfer(address,address,uint256)");

/// Solidity selector of the Transfer log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_APPROVAL: [u8; 32] = keccak256!("Approval(address,address,uint256)");

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
}

/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
/// 1024-2047 Precompiles that are not in Ethereum Mainnet but are neither CENNZnet specific
/// 2048-4095 CENNZnet specific precompiles
/// NFT precompile addresses can only fall between
/// 	0xAAAAAAAA00000000000000000000000000000000 - 0xAAAAAAAAFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
/// The precompile for NFT series (X,Y) where X & Y are a u32 (i.e.8 bytes), if 0XFFFFFFFF +
/// Bytes(CollectionId) + Bytes(SeriesId) In order to route the address to Erc721Precompile<R>, we
/// check whether the CollectionId + SeriesId exist in crml-nft pallet

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
	Runtime: pallet_nft::Config + pallet_evm::Config + frame_system::Config,
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::Call: From<pallet_nft::Call<Runtime>>,
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
						Err(e) => return Some(Err(e)),
					};

					if let Err(err) = handle.check_function_modifier(match selector {
						Action::Approve |
						Action::SafeTransferFrom |
						Action::TransferFrom |
						Action::SafeTransferFromCallData => FunctionModifier::NonPayable,
						_ => FunctionModifier::View,
					}) {
						return Some(Err(err))
					}

					match selector {
						Action::OwnerOf => Self::owner_of(collection_id, handle),
						Action::BalanceOf => Self::balance_of(collection_id, handle),
						Action::TransferFrom => Self::transfer_from(collection_id, handle),
						Action::Name => Self::name(collection_id, handle),
						Action::Symbol => Self::symbol(collection_id, handle),
						Action::TokenURI => Self::token_uri(collection_id, handle),
						Action::Approve |
						Action::GetApproved |
						Action::SafeTransferFrom |
						Action::SafeTransferFromCallData |
						Action::IsApprovedForAll |
						Action::SetApprovalForAll => return Some(Err(error("function not implemented yet").into())),
					}
				};
				return Some(result)
			}
		}
		None
	}

	fn is_precompile(&self, address: H160) -> bool {
		if let Some(collection_id) =
			Runtime::evm_id_to_runtime_id(Address(address), ERC721_PRECOMPILE_ADDRESS_PREFIX)
		{
			// route to NFT module only if the (collection, series) exists
			pallet_nft::Pallet::<Runtime>::collection_exists(collection_id)
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
	Runtime: pallet_nft::Config + pallet_evm::Config + frame_system::Config,
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::Call: From<pallet_nft::Call<Runtime>>,
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
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input.
		let mut input = handle.read_input()?;
		input.expect_arguments(1)?;
		let serial_number: U256 = input.read::<U256>()?;

		// For now we only support Ids < u32 max
		// since `u32` is the native `SerialNumber` type used by the NFT module.
		// it's not possible for the module to issue Ids larger than this
		if serial_number > u32::MAX.into() {
			return Err(error("expected token id <= 2^32").into())
		}
		let serial_number: SerialNumber = serial_number.saturated_into();

		// Build output.
		match pallet_nft::Pallet::<Runtime>::token_owner(collection_id, serial_number) {
			Some(owner_account_id) => Ok(PrecompileOutput {
				exit_status: ExitSucceed::Returned,
				output: EvmDataWriter::new()
					.write(Address::from(Into::<H160>::into(owner_account_id)))
					.build(),
			}),
			None => Err(PrecompileFailure::Revert {
				exit_status: ExitRevert::Reverted,
				output: alloc::format!("Token does not exist").as_bytes().to_vec(),
			}),
		}
	}

	fn balance_of(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Read input.
		let mut input = handle.read_input()?;
		input.expect_arguments(1)?;

		let owner: H160 = input.read::<Address>()?.into();

		// Fetch info.
		let amount: U256 = match pallet_nft::Pallet::<Runtime>::token_balance::<Runtime::AccountId>(
			owner.into(),
		) {
			Some(balance_map) => U256::from(*(balance_map.get(&collection_id).unwrap_or(&0))),
			None => U256::zero(),
		};

		// Build output.
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(amount).build(),
		})
	}

	fn transfer_from(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		let mut input = handle.read_input()?;
		input.expect_arguments(3)?;

		let from: H160 = input.read::<Address>()?.into();
		let to: H160 = input.read::<Address>()?.into();
		let serial_number = input.read::<U256>()?;

		// For now we only support Ids < u32 max
		// since `u32` is the native `SerialNumber` type used by the NFT module.
		// it's not possible for the module to issue Ids larger than this
		if serial_number > u32::MAX.into() {
			return Err(error("expected token id <= 2^32").into())
		}
		let serial_number: SerialNumber = serial_number.saturated_into();
		let token_id = (collection_id, serial_number);
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Build call with origin.
		// TODO Implement token_approvals check
		if handle.context().caller == from {
			// Dispatch call (if enough gas).
			RuntimeHelper::<Runtime>::try_dispatch(
				handle,
				Some(from.into()).into(),
				pallet_nft::Call::<Runtime>::transfer { token_id, new_owner: to.into() },
			)?;
		} else {
			return Err(error("caller not approved").into())
		}

		log3(
			handle.code_address(),
			SELECTOR_LOG_TRANSFER,
			handle.context().caller,
			to,
			EvmDataWriter::new().write(serial_number).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(true).build(),
		})
	}

	fn name(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		match pallet_nft::Pallet::<Runtime>::collection_info(collection_id) {
			Some(collection_info) => Ok(PrecompileOutput {
				exit_status: ExitSucceed::Returned,
				output: EvmDataWriter::new()
					.write::<Bytes>(collection_info.name.as_slice().into())
					.build(),
			}),
			None => Err(PrecompileFailure::Revert {
				exit_status: ExitRevert::Reverted,
				output: alloc::format!("Collection does not exist").as_bytes().to_vec(),
			}),
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
			Some(collection_info) => Ok(PrecompileOutput {
				exit_status: ExitSucceed::Returned,
				output: EvmDataWriter::new()
					.write::<Bytes>(collection_info.name.as_slice().into())
					.build(),
			}),
			None => Err(PrecompileFailure::Revert {
				exit_status: ExitRevert::Reverted,
				output: alloc::format!("Collection does not exist").as_bytes().to_vec(),
			}),
		}
	}

	fn token_uri(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		let mut input = handle.read_input()?;
		input.expect_arguments(1)?;
		let serial_number = input.read::<U256>()?;

		// For now we only support Ids < u32 max
		// since `u32` is the native `SerialNumber` type used by the NFT module.
		// it's not possible for the module to issue Ids larger than this
		if serial_number > u32::MAX.into() {
			return Err(error("expected token id <= 2^32").into())
		}
		let serial_number: SerialNumber = serial_number.saturated_into();

		// Build output.
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new()
				.write::<Bytes>(
					pallet_nft::Pallet::<Runtime>::token_uri((collection_id, serial_number))
						.as_slice()
						.into(),
				)
				.build(),
		})
	}
}
