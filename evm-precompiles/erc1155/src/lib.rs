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

use core::convert::TryFrom;
use fp_evm::{PrecompileHandle, PrecompileOutput};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	ensure,
	traits::OriginTrait,
};
use pallet_evm::{Context, ExitReason, PrecompileFailure, PrecompileSet};
use pallet_nft::traits::NFTExt;
use sp_core::{H160, U256};
use sp_runtime::{traits::SaturatedConversion, BoundedVec};
use sp_std::{marker::PhantomData, vec, vec::Vec};

use precompile_utils::{constants::ERC1155_PRECOMPILE_ADDRESS_PREFIX, prelude::*};
use seed_primitives::{CollectionUuid, EthAddress, SerialNumber, TokenCount, TokenId};

/// Solidity selector of the TransferSingle log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_TRANSFER_SINGLE: [u8; 32] =
	keccak256!("TransferSingle(address,address,address,uint256,uint256)");

/// Solidity selector of the TransferBatch log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_TRANSFER_BATCH: [u8; 32] =
	keccak256!("TransferBatch(address,address,address,uint256[],uint256[])");

/// Solidity selector of the Approval log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_APPROVAL: [u8; 32] = keccak256!("ApprovalForAll(address,address,bool)");

/// Solidity selector of the URI log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_URI: [u8; 32] = keccak256!("URI(string,uint256)");

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
	// ERC1155 metadata URI extensions
	Uri = "uri(uint256)",
	// TRN extensions
	Mint = "mint(address,uint256,uint256)",
	MintBatch = "mintBatch(address,uint256[],uint256[])",
}

/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
/// 1024-2047 Precompiles that are not in Ethereum Mainnet but are neither Root specific
/// 2048-4095 Seed specific precompiles
/// SFT precompile addresses can only fall between
/// 	0xBBBBBBBB00000000000000000000000000000000 - 0xBBBBBBBBFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
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
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::Call: From<pallet_sft::Call<Runtime>> + From<pallet_token_approvals::Call<Runtime>>,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	<<Runtime as frame_system::Config>::Call as Dispatchable>::Origin: OriginTrait,
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

					// if let Err(err) = handle.check_function_modifier(match selector {
					// 	Action::Approve |
					// 	Action::SafeTransferFrom |
					// 	Action::TransferFrom |
					// 	Action::SafeTransferFromCallData => FunctionModifier::NonPayable,
					// 	_ => FunctionModifier::View,
					// }) {
					// 	return Some(Err(err.into()))
					// }

					match selector {
						// Core ERC1155
						Action::BalanceOf => Self::balance_of(collection_id, handle),
						Action::BalanceOfBatch => Self::balance_of_batch(collection_id, handle),
						_ => Self::balance_of(collection_id, handle),
					}
				};
				return Some(result)
			}
		}
		None
	}

	fn is_precompile(&self, address: H160) -> bool {
		if let Some(collection_id) =
			Runtime::evm_id_to_runtime_id(Address(address), ERC1155_PRECOMPILE_ADDRESS_PREFIX)
		{
			// Check whether the collection exists
			pallet_sft::Pallet::<Runtime>::collection_exists(collection_id)
		} else {
			false
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
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::Call: From<pallet_sft::Call<Runtime>> + From<pallet_token_approvals::Call<Runtime>>,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	<<Runtime as frame_system::Config>::Call as Dispatchable>::Origin: OriginTrait,
{
	fn balance_of(
		collection_id: CollectionUuid,
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		read_args!(handle, { owner: Address, id: U256 });

		// Parse args
		let owner: H160 = owner.into();
		ensure!(id > u32::MAX.into(), revert("ERC721: Expected token id <= 2^32"));
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
					return Err(revert("ERC1155: Expected token id <= 2^32").into())
				}
				Ok(id.saturated_into())
			})
			.collect::<Result<Vec<SerialNumber>, PrecompileFailure>>()?;

		// Get balance from SFT pallet for each
		let mut balances: Vec<U256> = vec![];
		owners.iter().zip(ids.iter()).for_each(|(owner, id)| {
			// Record one read cost per token
			handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost()).unwrap();
			let balance = pallet_sft::Pallet::<Runtime>::balance_of(
				&Runtime::AccountId::from(*owner),
				(collection_id, *id),
			);
			balances.push(U256::from(balance));
		});

		Ok(succeed(EvmDataWriter::new().write(balances).build()))
	}
}
