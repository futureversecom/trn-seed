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

use fp_evm::{PrecompileFailure, PrecompileHandle, PrecompileOutput, PrecompileResult};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	serde::__private::de::Content::U32,
	traits::OriginTrait,
};
use pallet_erc20_peg::*;
use pallet_evm::{GasWeightMapping, Precompile};
use pallet_marketplace::{
	types::{MarketplaceId, OfferId},
	weights::WeightInfo,
};
use precompile_utils::{
	constants::{ERC20_PRECOMPILE_ADDRESS_PREFIX, ERC721_PRECOMPILE_ADDRESS_PREFIX},
	prelude::*,
};
use seed_primitives::{
	AccountId, AssetId, Balance, BlockNumber, CollectionUuid, ListingId, SerialNumber, TokenId,
};
use sp_core::{H160, H256, U256};
use sp_runtime::{traits::SaturatedConversion, Permill};
use sp_std::{marker::PhantomData, vec::Vec};
/// The ID of the gas token on TRN, equivalent to ETH on ethereum
const GAS_TOKEN_ID: AssetId = 2_u32;

/// Solidity selector of the Mint log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_MINT: [u8; 32] = keccak256!("Mint(address,uint256,uint256)");

/// Solidity selector of the Burn log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_BURN: [u8; 32] = keccak256!("Burn(address,uint256,uint256,address)");

/// Solidity selector of the Swap log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_SWAP: [u8; 32] =
	keccak256!("Swap(address,uint256,uint256,uint256,uint256,address)");

/// Saturated conversion from EVM uint256 to Blocknumber
fn saturated_convert_blocknumber(input: U256) -> Result<BlockNumber, PrecompileFailure> {
	if input > BlockNumber::MAX.into() {
		return Err(revert("DEX: Input number exceeds the BlockNumber type boundary (2^32)").into())
	}
	Ok(input.saturated_into())
}

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	RegisterMarketplace = "registerMarketplace(address,U256)",
	// SellNft = "sellNft(address,address,uint256[],address,address,uint256,uint256,uint256)",
	UpdateFixedPrice = "updateFixedPrice(uint256,uint256)",
	Buy = "buy(uint256)",
	// AuctionNft = "auctionNft(address,uint256[],address,uint256,uint256,uint256)",
	Bid = "bid(uint256,uint256)",
	CancelSale = "cancelSale(uint256)",
	MakeSimpleOffer = "makeSimpleOffer(uint256,uint256,address,uint256)",
	CancelOffer = "cancelOffer(uint64)",
	AcceptOffer = "acceptOffer(uint64)",
}

/// Provides access to the Marketplace pallet
pub struct MarketplacePrecompile<Runtime>(PhantomData<Runtime>);

impl<T> Default for MarketplacePrecompile<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> Precompile for MarketplacePrecompile<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: pallet_evm::Config + frame_system::Config + pallet_marketplace::Config,
	<Runtime as frame_system::Config>::RuntimeCall:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_marketplace::Call<Runtime>>,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>,
	// <<Runtime as pallet_marketplace::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
	// From<Option<Runtime::AccountId>>,
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
				return Err(err.into())
			}

			match selector {
				Action::RegisterMarketplace => Self::register_marketplace(handle),
				// Action::SellNft => Self::sell_nft(handle),
				Action::UpdateFixedPrice => Self::update_fixed_price(handle),
				Action::Buy => Self::buy(handle),
				// Action::AuctionNft => Self::auction_nft(handle),
				Action::Bid => Self::bid(handle),
				Action::CancelSale => Self::cancel_sale(handle),
				Action::MakeSimpleOffer => Self::make_simple_offer(handle),
				Action::CancelOffer => Self::cancel_offer(handle),
				Action::AcceptOffer => Self::accept_offer(handle),
			}
		};
		return result
	}
}

impl<Runtime> MarketplacePrecompile<Runtime> {
	pub fn new() -> Self {
		Self(PhantomData)
	}
}

impl<Runtime> MarketplacePrecompile<Runtime>
where
	Runtime::AccountId: From<H160> + Into<H160>,
	Runtime: pallet_marketplace::Config + pallet_evm::Config,
	Runtime: ErcIdConversion<CollectionUuid, EvmId = Address>
		+ ErcIdConversion<AssetId, EvmId = Address>,
	<Runtime as frame_system::Config>::RuntimeCall:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_marketplace::Call<Runtime>>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<Runtime::AccountId>>,
{
	fn register_marketplace(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				marketplace_account: Address,
				entitlement: U256
			}
		);

		let marketplace_account: H160 = marketplace_account.into();
		let marketplace_account: Option<Runtime::AccountId> =
			if marketplace_account == H160::default() {
				None
			} else {
				Some(marketplace_account.into())
			};

		let entitlement: u32 = entitlement.saturated_into();
		let entitlement: Permill = Permill::from_parts(entitlement);

		// Build output.
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller: Runtime::AccountId = handle.context().caller.into();
		// let result = pallet_marketplace::Pallet::<Runtime>::register_marketplace(
		// 	&caller,
		// 	marketplace_account,
		// 	entitlement,
		// );
		// // Handle error case
		// if let Err(err) = result {
		// 	return Err(revert(
		// 		alloc::format!("Marketplace: register marketplace failed {:?}", err.stripped())
		// 			.as_bytes()
		// 			.to_vec(),
		// 	))
		// };
		// Ok(succeed([]))
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_marketplace::Call::<Runtime>::register_marketplace {
				marketplace_account,
				entitlement,
			},
		)?;
		Ok(succeed([]))
	}

	// fn sell_nft(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
	// 	handle.record_log_costs_manual(1, 32)?;
	// 	read_args!(
	// 		handle,
	// 		{
	// 			beneficiary: Address,
	// 			collection_address: Address,
	// 			serial_numbers: Vec<U256>,
	// 			buyer: Address,
	// 			payment_asset: Address,
	// 			fixed_price: U256,
	// 			duration: U256,
	// 			marketplace_id: U256
	// 		}
	// 	);
	// 	// Parse beneficiary
	// 	let beneficiary: H160 = beneficiary.into();
	// 	// Parse asset_id
	// 	let payment_asset: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
	// 		payment_asset,
	// 		ERC20_PRECOMPILE_ADDRESS_PREFIX,
	// 	)
	// 	.ok_or_else(|| revert("PEG: Invalid asset address"))?;
	// 	let marketplace_id: u32 = marketplace_id.saturated_into();
	// 	let duration = Some(saturated_convert_blocknumber(duration)?.into());
	// 	let fixed_price: Balance = fixed_price.saturated_into();
	// 	let collection_id: CollectionUuid =
	// 		<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
	// 			collection_address,
	// 			ERC721_PRECOMPILE_ADDRESS_PREFIX,
	// 		)
	// 		.ok_or_else(|| revert("MARKETPLACE: Invalid collection address"))?;
	// 	pub const MaxTokensPerListing: u32 = 1000;
	// 	let vec_serial_numbers = serial_numbers
	// 		.into_iter()
	// 		.map(|serial_numbers| {
	// 			if serial_numbers > u32::MAX.into() {
	// 				return Err(revert("ERC1155: Expected token id <= 2^32").into())
	// 			}
	// 			Ok(serial_numbers.saturated_into())
	// 		});
	// 	// let serial_numbers: BoundedVec<SerialNumber, u32> =
	// 	// 	BoundedVec::try_from(vec_serial_numbers);
	// 	let serial_numbers: BoundedVec<SerialNumber, u32> =
	// 		BoundedVec::try_from(vec_serial_numbers).unwrap();
	// 	handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
	// 	let caller: Runtime::AccountId = handle.context().caller.into();
	//
	// 	RuntimeHelper::<Runtime>::try_dispatch(
	// 		handle,
	// 		None.into(),
	// 		pallet_marketplace::Call::<Runtime>::sell_nft {
	// 			collection_id,
	// 			serial_numbers,
	// 			buyer,
	// 			payment_asset,
	// 			fixed_price,
	// 			duration,
	// 			marketplace_id
	// 		},
	// 	)?;
	// 	Ok(succeed([]))
	// }

	fn update_fixed_price(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				listing_id: U256,
				new_price: U256
			}
		);
		let listing_id: u128 = listing_id.saturated_into();
		let new_price: Balance = new_price.saturated_into();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost().saturating_mul(3))?;
		let caller: Runtime::AccountId = handle.context().caller.into();
		// let result = pallet_marketplace::Pallet::<Runtime>::update_fixed_price(
		// 	&caller,
		// 	listing_id,
		// 	fixed_price,
		// );
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_marketplace::Call::<Runtime>::update_fixed_price { listing_id, new_price },
		)?;

		// Build output.
		Ok(succeed([]))
	}

	fn buy(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(handle, { listing_id: U256 });
		let listing_id: u128 = listing_id.saturated_into();
		let caller: Runtime::AccountId = handle.context().caller.into();
		// let result = pallet_marketplace::Pallet::<Runtime>::buy(&caller, listing_id);
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_marketplace::Call::<Runtime>::buy { listing_id },
		)?;

		// Build output.
		Ok(succeed([]))
	}

	// fn auction_nft(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
	// 	handle.record_log_costs_manual(4, 32)?;
	//
	// 	// Parse input.
	// 	read_args!(
	// 		handle,
	// 		{
	// 			collection_address: Address,
	// 			serial_numbers: Vec<U256>,
	// 			payment_asset: Address,
	// 			reserve_price: U256,
	// 			duration: U256,
	// 			marketplace_id: U256
	// 		}
	// 	);
	// 	let marketplace_id: u32 = marketplace_id.saturated_into();
	// 	let duration = Some(saturated_convert_blocknumber(duration)?.into());
	// 	let reserve_price: Balance = reserve_price.saturated_into();
	// 	let collection_id: CollectionUuid =
	// 		<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
	// 			collection_address,
	// 			ERC721_PRECOMPILE_ADDRESS_PREFIX,
	// 		)
	// 		.ok_or_else(|| revert("MARKETPLACE: Invalid collection address"))?;
	// 	let serial_numbers: SerialNumber = serial_numbers.saturated_into();
	// 	// Parse asset_id
	// 	let payment_asset: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
	// 		payment_asset,
	// 		ERC20_PRECOMPILE_ADDRESS_PREFIX,
	// 	)
	// 	.ok_or_else(|| revert("PEG: Invalid asset address"))?;
	//
	// 	RuntimeHelper::<Runtime>::try_dispatch(
	// 		handle,
	// 		None.into(),
	// 		pallet_marketplace::Call::<Runtime>::auction_nft {
	// 			origin: handle.context().caller.into(),
	// 			collection_id,
	// 			serial_numbers,
	// 			payment_asset,
	// 			reserve_price,
	// 			duration,
	// 			marketplace_id
	// 		},
	// 	)?;
	//
	// 	// Build output.
	// 	Ok(succeed([]))
	// }

	fn bid(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(
			handle,
			{
				listing_id: U256,
				amount: U256
			}
		);

		let listing_id: u128 = listing_id.saturated_into();
		let amount: Balance = amount.saturated_into();
		// let caller: Runtime::AccountId = handle.context().caller.into();
		// let result = pallet_marketplace::Pallet::<Runtime>::bid(caller, listing_id, amount);

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_marketplace::Call::<Runtime>::bid { listing_id, amount },
		)?;

		// Build output.
		Ok(succeed([]))
	}

	fn cancel_sale(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(handle, { listing_id: U256 });

		let listing_id: u128 = listing_id.saturated_into();
		// let caller: Runtime::AccountId = handle.context().caller.into();
		// let result = pallet_marketplace::Pallet::<Runtime>::cancel_sale(caller, listing_id);
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_marketplace::Call::<Runtime>::cancel_sale { listing_id },
		)?;

		// Build output.
		Ok(succeed([]))
	}

	fn make_simple_offer(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				collection_address: Address,
				serial_number: U256,
				amount: U256,
				asset_id: Address,
				marketplace_id: U256
			}
		);

		let marketplace_id: u32 = marketplace_id.saturated_into();
		let marketplace_id: Option<u32> =
			if marketplace_id == u32::default() { None } else { Some(marketplace_id) };

		let amount: Balance = amount.saturated_into();
		let collection_id: CollectionUuid =
			<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
				collection_address,
				ERC721_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("MARKETPLACE: Invalid collection address"))?;
		let serial_number: SerialNumber = serial_number.saturated_into();
		let token_id: TokenId = (collection_id, serial_number);
		// Parse asset_id
		let asset_id: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			asset_id,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("MARKETPLACE: Invalid asset address"))?;
		// let caller: Runtime::AccountId = handle.context().caller.into();
		// let result = pallet_marketplace::Pallet::<Runtime>::make_simple_offer(
		// 	caller,
		// 	token_id,
		// 	amount,
		// 	asset_id,
		// 	marketplace_id,
		// );
		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_marketplace::Call::<Runtime>::make_simple_offer {
				token_id,
				amount,
				asset_id,
				marketplace_id,
			},
		)?;

		// Build output.
		Ok(succeed([]))
	}

	fn cancel_offer(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		// Parse input.
		read_args!(handle, { offer_id: U256 });

		let offer_id: OfferId = offer_id.saturated_into();

		// Return either the approved account or zero address if no account is approved
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_marketplace::Call::<Runtime>::cancel_offer { offer_id },
		)?;
		Ok(succeed([]))
	}

	fn accept_offer(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		// Parse input.
		read_args!(handle, { offer_id: U256 });

		let offer_id: OfferId = offer_id.saturated_into();

		// Return either the approved account or zero address if no account is approved
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		// let caller: Runtime::AccountId = handle.context().caller.into();
		// let result = pallet_marketplace::Pallet::<Runtime>::cancel_offer(caller, offer_id);
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_marketplace::Call::<Runtime>::accept_offer { offer_id },
		)?;
		Ok(succeed([]))
	}
}
