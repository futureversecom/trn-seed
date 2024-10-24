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

use fp_evm::{PrecompileHandle, PrecompileOutput, PrecompileResult};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	ensure,
};
use pallet_evm::{GasWeightMapping, Precompile};
use pallet_nft::WeightInfo;
use precompile_utils::{constants::ERC721_PRECOMPILE_ADDRESS_PREFIX, prelude::*};
use seed_primitives::{
	CollectionUuid, CrossChainCompatibility, MetadataScheme, OriginChain, RoyaltiesSchedule,
	TokenCount, MAX_COLLECTION_ENTITLEMENTS, MAX_ENTITLEMENTS,
};
use sp_core::{H160, U256};
use sp_runtime::{
	traits::{ConstU32, SaturatedConversion},
	BoundedVec, Permill,
};
use sp_std::{marker::PhantomData, vec::Vec};

/// Solidity selector of the InitializeCollection log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_INITIALIZE_COLLECTION: [u8; 32] =
	keccak256!("InitializeCollection(address,address)"); // collection_owner, collection_address

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	/// Create a new NFT collection
	/// collection_owner, name, max_issuance, metadata_path, royalty_addresses,
	/// royalty_entitlements
	InitializeCollection = "initializeCollection(address,bytes,uint32,bytes,address[],uint32[])",
}

/// Provides access to the NFT pallet
pub struct NftPrecompile<Runtime>(PhantomData<Runtime>);

impl<T> Default for NftPrecompile<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> Precompile for NftPrecompile<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: frame_system::Config + pallet_nft::Config + pallet_evm::Config,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	<Runtime as frame_system::Config>::RuntimeCall:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_nft::Call<Runtime>>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<Runtime::AccountId>>,
{
	fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
		let result = {
			let selector = match handle.read_selector() {
				Ok(selector) => selector,
				Err(e) => return Err(e.into()),
			};

			if let Err(err) = handle.check_function_modifier(FunctionModifier::NonPayable) {
				return Err(err.into());
			}

			match selector {
				Action::InitializeCollection => Self::initialize_collection(handle),
			}
		};
		return result;
	}
}

impl<Runtime> NftPrecompile<Runtime> {
	pub fn new() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> NftPrecompile<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: frame_system::Config + pallet_nft::Config + pallet_evm::Config,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	<Runtime as frame_system::Config>::RuntimeCall:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_nft::Call<Runtime>>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<Runtime::AccountId>>,
{
	fn initialize_collection(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(7, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				collection_owner: Address,
				name: Bytes,
				max_issuance: U256,
				metadata_path: Bytes,
				royalty_addresses: Vec<Address>,
				royalty_entitlements: Vec<U256>
			}
		);

		// Parse owner
		let collection_owner: H160 = collection_owner.into();
		// Parse name
		let name: BoundedVec<u8, <Runtime as pallet_nft::Config>::StringLimit> = name
			.as_bytes()
			.to_vec()
			.try_into()
			.map_err(|_| revert("NFT: Collection name exceeds the maximum length"))?;

		// Parse max issuance
		// If max issuance is 0, we assume no max issuance is set
		if max_issuance > u32::MAX.into() {
			return Err(revert("NFT: Expected max_issuance <= 2^32").into());
		}
		let max_issuance: TokenCount = max_issuance.saturated_into();
		let max_issuance: Option<TokenCount> = match max_issuance {
			0 => None,
			n => Some(n),
		};

		// Parse Metadata
		let metadata_scheme: MetadataScheme =
			metadata_path.as_bytes().try_into().map_err(|str_err| {
				revert(alloc::format!("{}: {:?}", "NFT: Invalid metadata_path", str_err))
			})?;

		// Parse royalties
		if royalty_addresses.len() != royalty_entitlements.len() {
			return Err(
				revert("NFT: Royalty addresses and entitlements must be the same length").into()
			);
		}
		let royalty_entitlements = royalty_entitlements.into_iter().map(|entitlement| {
			let entitlement: u32 = entitlement.saturated_into();
			Permill::from_parts(entitlement)
		});
		let royalties_schedule: Option<RoyaltiesSchedule<Runtime::AccountId>> =
			if royalty_addresses.len() > 0 {
				let entitlements_unbounded: Vec<(Runtime::AccountId, Permill)> = royalty_addresses
					.into_iter()
					.map(|address| H160::from(address).into())
					.zip(royalty_entitlements)
					.collect();
				ensure!(
					entitlements_unbounded.len() <= MAX_COLLECTION_ENTITLEMENTS as usize,
					revert("NFT: Too many royalty entitlements provided")
				);
				let entitlements: BoundedVec<
					(Runtime::AccountId, Permill),
					ConstU32<MAX_ENTITLEMENTS>,
				> = BoundedVec::truncate_from(entitlements_unbounded);

				Some(RoyaltiesSchedule { entitlements })
			} else {
				None
			};

		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_nft::Config>::WeightInfo::create_collection(),
		))?;

		// Dispatch call
		let collection_id = pallet_nft::Pallet::<Runtime>::do_create_collection(
			collection_owner.into(),
			name,
			0, // Initial issuance is set to 0
			max_issuance,
			None, // Token owner set to None
			metadata_scheme,
			royalties_schedule,
			OriginChain::Root,
			CrossChainCompatibility::default(),
		);

		// Build output.
		match collection_id {
			Ok(collection_id) => {
				let precompile_address =
					Runtime::runtime_id_to_evm_id(collection_id, ERC721_PRECOMPILE_ADDRESS_PREFIX);

				log2(
					handle.code_address(),
					SELECTOR_LOG_INITIALIZE_COLLECTION,
					collection_owner,
					EvmDataWriter::new().write(precompile_address).build(),
				)
				.record(handle)?;

				Ok(succeed(
					EvmDataWriter::new()
						.write(precompile_address)
						.write(U256::from(collection_id))
						.build(),
				))
			},
			Err(err) => Err(revert(
				alloc::format!("NFT: Initialize collection failed {:?}", err.stripped())
					.as_bytes()
					.to_vec(),
			)),
		}
	}
}
