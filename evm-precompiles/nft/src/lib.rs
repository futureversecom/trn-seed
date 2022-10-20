#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use fp_evm::{PrecompileHandle, PrecompileOutput, PrecompileResult};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::Precompile;
use pallet_nft::{
	CollectionNameType, MetadataScheme, OriginChain, RoyaltiesSchedule, TokenCount, WeightInfo,
};
use precompile_utils::prelude::*;
use seed_primitives::CollectionUuid;
use sp_core::{H160, U256};
use sp_runtime::{traits::SaturatedConversion, Permill};
use sp_std::{marker::PhantomData, vec::Vec};

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	/// Create a new NFT collection
	/// name, max_issuance, metadata_type, metadata_path, royalty_addresses, royalty_entitlements
	InitializeCollection = "initializeCollection(bytes,uint32,uint8,bytes,address[],uint32[])",
	/// Mint an NFT in a collection
	/// collection_id, quantity, owner
	Mint = "mint(uint32,uint32,address)",
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
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::Call: From<pallet_nft::Call<Runtime>>,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
{
	fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
		let result = {
			let selector = match handle.read_selector() {
				Ok(selector) => selector,
				Err(e) => return Err(e.into()),
			};

			if let Err(err) = handle.check_function_modifier(FunctionModifier::NonPayable) {
				return Err(err.into())
			}

			match selector {
				Action::InitializeCollection => Self::initialize_collection(handle),
				Action::Mint => Self::mint(handle),
			}
		};
		return result
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
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::Call: From<pallet_nft::Call<Runtime>>,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
{
	fn initialize_collection(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input.
		read_args!(
			handle,
			{
				name: Bytes,
				max_issuance: U256,
				metadata_type: U256,
				metadata_path: Bytes,
				royalty_addresses: Vec<Address>,
				royalty_entitlements: Vec<U256>
			}
		);

		// Parse name
		let name: CollectionNameType = name.as_bytes().to_vec();

		// Parse max issuance
		// If max issuance is 0, we assume no max issuance is set
		if max_issuance > u32::MAX.into() {
			return Err(revert("expected max_issuance <= 2^32").into())
		}
		let max_issuance: TokenCount = max_issuance.saturated_into();
		let max_issuance: Option<TokenCount> = match max_issuance {
			0 => None,
			n => Some(n),
		};

		// Parse Metadata
		if metadata_type > u8::MAX.into() {
			return Err(revert("Invalid metadata_type, expected u8").into())
		}
		let metadata_type: u8 = metadata_type.saturated_into();
		let metadata_path: Vec<u8> = metadata_path.as_bytes().to_vec();
		let metadata_scheme = MetadataScheme::from_index(metadata_type, metadata_path)
			.map_err(|_| revert("Invalid metadata_type, expected u8 <= 3"))?;

		// Parse royalties
		if royalty_addresses.len() != royalty_entitlements.len() {
			return Err(revert("Royalty addresses and entitlements must be the same length").into())
		}
		let royalty_entitlements = royalty_entitlements.into_iter().map(|entitlement| {
			let entitlement: u32 = entitlement.saturated_into();
			Permill::from_parts(entitlement)
		});
		let royalties_schedule: Option<RoyaltiesSchedule<Runtime::AccountId>> =
			if royalty_addresses.len() > 0 {
				let entitlements = royalty_addresses
					.into_iter()
					.map(|address| {
						let address: H160 = address.into();
						address.into()
					})
					.zip(royalty_entitlements)
					.collect();
				Some(RoyaltiesSchedule { entitlements })
			} else {
				None
			};

		let origin = handle.context().caller;
		handle.record_cost(<Runtime as pallet_nft::Config>::WeightInfo::create_collection())?;

		// Dispatch call (if enough gas).
		let collection_id = pallet_nft::Pallet::<Runtime>::do_create_collection(
			origin.into(),
			name,
			0, // Initial issuance is set to 0
			max_issuance,
			None, // Token owner set to None
			metadata_scheme,
			royalties_schedule,
			OriginChain::Root,
		);

		// Build output.
		match collection_id {
			Ok(collection_id) =>
				Ok(succeed(EvmDataWriter::new().write(U256::from(collection_id)).build())),
			Err(err) => Err(revert(
				alloc::format!("Initialize collection failed {:?}", err.stripped())
					.as_bytes()
					.to_vec(),
			)),
		}
	}

	fn mint(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input.
		read_args!(
			handle,
			{
				collection_id: U256,
				quantity: U256,
				owner: Address
			}
		);

		// Parse collection id
		if collection_id > CollectionUuid::MAX.into() {
			return Err(revert("expected collection ID <= 2^32").into())
		}
		let collection_id: CollectionUuid = collection_id.saturated_into();

		// Parse quantity
		if quantity > TokenCount::MAX.into() {
			return Err(revert("expected quantity <= 2^32").into())
		}
		let quantity: TokenCount = quantity.saturated_into();

		// Parse owner
		let owner: H160 = owner.into();
		let token_owner: Option<Runtime::AccountId> =
			if owner == H160::default() { None } else { Some(owner.into()) };

		let origin = handle.context().caller;

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_nft::Call::<Runtime>::mint { collection_id, quantity, token_owner },
		)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}
}
