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
	ensure,
	serde::__private::de::Content::U32,
	traits::OriginTrait,
};
use pallet_erc20_peg::*;
use pallet_evm::{GasWeightMapping, Precompile};
use pallet_marketplace::{
	types::{AuctionClosureReason, FixedPriceClosureReason, Listing, MarketplaceId, OfferId},
	weights::WeightInfo,
	Error::NotForFixedPriceSale,
};
use precompile_utils::{
	constants::{ERC20_PRECOMPILE_ADDRESS_PREFIX, ERC721_PRECOMPILE_ADDRESS_PREFIX},
	prelude::*,
};
use seed_primitives::{
	AccountId, AssetId, Balance, BlockNumber, CollectionUuid, ListingId, SerialNumber, TokenId,
};
use sp_core::{H160, H256, U256};
use sp_runtime::{traits::SaturatedConversion, BoundedVec, Permill};
use sp_std::{marker::PhantomData, vec::Vec};

//Self::deposit_event(Event::<T>::MarketplaceRegister {
// 			account: marketplace_account,
// 			entitlement,
// 			marketplace_id,
// 		});
/// Solidity selector of the Marketplace register log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_MARKETPLACE_REGISTER: [u8; 32] =
	keccak256!("MarketplaceRegister(address,uint256,uint256)");

pub const SELECTOR_LOG_FIXED_PRICE_SALE_LIST: [u8; 32] =
	keccak256!("FixedPriceSaleList(address,uint256,uint256)");

pub const SELECTOR_LOG_FIXED_PRICE_SALE_UPDATE: [u8; 32] =
	keccak256!("FixedPriceSaleUpdate(uint256,uint256,uint256)"); // collection_id, listing_id, new_price

pub const SELECTOR_LOG_FIXED_PRICE_SALE_COMPLETE: [u8; 32] =
	keccak256!("FixedPriceSaleComplete(uint256,uint256,uint256)"); // collection_id, listing_id, fixed_price

pub const SELECTOR_LOG_AUCTION_OPEN: [u8; 32] = keccak256!("AuctionOpen(uint256,uint256,uint256)"); // collection_id, listing_id, reserve_price

pub const SELECTOR_LOG_BID: [u8; 32] = keccak256!("Bid(address,uint256,uint256)");

pub const SELECTOR_LOG_FIXED_PRICE_SALE_CLOSE: [u8; 32] =
	keccak256!("FixedPriceSaleClose(address,uint256)");

pub const SELECTOR_LOG_AUCTION_CLOSE: [u8; 32] = keccak256!("AuctionClose(address,uint256)");

pub const SELECTOR_LOG_OFFER: [u8; 32] = keccak256!("Offer(uint256,address,uint256)");

pub const SELECTOR_LOG_OFFER_CANCEL: [u8; 32] = keccak256!("OfferCancel(uint256, uint256)");

pub const SELECTOR_LOG_OFFER_ACCEPT: [u8; 32] =
	keccak256!("OfferCancel(uint256, uint256, uint256)");

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
	SellNft = "sellNft(address,address,uint256[],address,address,uint256,uint256,uint256)",
	UpdateFixedPrice = "updateFixedPrice(uint256,uint256)",
	Buy = "buy(uint256)",
	AuctionNft = "auctionNft(address,uint256[],address,uint256,uint256,uint256)",
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
				Action::SellNft => Self::sell_nft(handle),
				Action::UpdateFixedPrice => Self::update_fixed_price(handle),
				Action::Buy => Self::buy(handle),
				Action::AuctionNft => Self::auction_nft(handle),
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
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				marketplace_account: Address,
				entitlement: U256
			}
		);

		let marketplace_account_h160: H160 = marketplace_account.into();
		let marketplace_account_h160: Option<Runtime::AccountId> =
			if marketplace_account_h160 == H160::default() {
				None
			} else {
				Some(marketplace_account_h160.into())
			};

		let entitlement: u32 = entitlement.saturated_into();
		ensure!(
			entitlement <= u32::MAX.into(),
			revert("Marketplace: Expected entitlement <= 2^32")
		);
		let entitlement: Permill = Permill::from_parts(entitlement);
		let caller: Runtime::AccountId = handle.context().caller.into();
		let marketplace_id = pallet_marketplace::Pallet::<Runtime>::do_register_marketplace(
			caller,
			marketplace_account_h160,
			entitlement,
		)
		.map_err(|e| {
			revert(alloc::format!("Marketplace: Dispatched call failed with error: {:?}", e))
		})?;
		ensure!(
			marketplace_id <= u32::MAX.into(),
			revert("Marketplace: Expected marketplace id <= 2^32")
		);

		let marketplace_id = H256::from_low_u64_be(marketplace_id as u64);

		log3(
			handle.code_address(),
			SELECTOR_LOG_MARKETPLACE_REGISTER,
			// marketplace_account.into(),
			handle.context().caller,
			// <H160 as Into<Address>>::into(marketplace_account.into()),

			// H256::from(entitlement),
			marketplace_id,
			vec![],
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(marketplace_id).build()))
	}

	fn sell_nft(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;
		read_args!(
			handle,
			{
				beneficiary: Address,
				collection_address: Address,
				serial_numbers: Vec<U256>,
				buyer: Address,
				payment_asset: Address,
				fixed_price: U256,
				duration: U256,
				marketplace_id: U256
			}
		);
		// Parse beneficiary
		let beneficiary: H160 = beneficiary.into();
		// Parse asset_id
		let payment_asset: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			payment_asset,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("MARKETPLACE: Invalid payment asset address"))?;
		let marketplace_id: u32 = marketplace_id.saturated_into();
		let duration = Some(saturated_convert_blocknumber(duration)?.into());
		let fixed_price: Balance = fixed_price.saturated_into();
		let collection_id: CollectionUuid =
			<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
				collection_address,
				ERC721_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("MARKETPLACE: Invalid collection address"))?;

		let serials_unbounded = serial_numbers
			.into_iter()
			.map(|serial_number| {
				if serial_number > SerialNumber::MAX.into() {
					return Err(revert("MARKETPLACE: Expected serial_number <= 2^128").into())
				}
				let serial_number: SerialNumber = serial_number.saturated_into();
				Ok(serial_number)
			})
			.collect::<Result<Vec<SerialNumber>, PrecompileFailure>>()?;

		let serial_numbers = BoundedVec::try_from(serials_unbounded).unwrap();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let buyer: H160 = buyer.into();
		let buyer: Option<Runtime::AccountId> =
			if buyer == H160::default() { None } else { Some(buyer.into()) };
		let marketplace_id: u32 = marketplace_id.saturated_into();
		let marketplace_id: Option<u32> =
			if marketplace_id == u32::default() { None } else { Some(marketplace_id) };

		let caller: Runtime::AccountId = handle.context().caller.into();
		let listing_id = pallet_marketplace::Pallet::<Runtime>::do_sell_nft(
			caller,
			collection_id,
			serial_numbers,
			buyer,
			payment_asset,
			fixed_price,
			duration,
			marketplace_id,
		)
		.map_err(|e| {
			revert(alloc::format!("Marketplace: Dispatched call failed with error: {:?}", e))
		})?;

		let listing_id = H256::from_low_u64_be(listing_id as u64);
		let fixed_price = H256::from_low_u64_be(fixed_price as u64);
		log4(
			handle.code_address(),
			SELECTOR_LOG_FIXED_PRICE_SALE_LIST,
			handle.context().caller, //seller
			listing_id,
			fixed_price,
			vec![],
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(listing_id).build()))
	}

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
		//Self::do_update_fixed_price(who, listing_id, new_price)
		let caller: Runtime::AccountId = handle.context().caller.into();
		let _ = pallet_marketplace::Pallet::<Runtime>::do_update_fixed_price(
			caller, listing_id, new_price,
		);
		let listing = pallet_marketplace::Pallet::<Runtime>::get_listing_detail(listing_id)
			.or_else(|_| Err(revert("MARKETPLACE: NotForFixedPriceSale")))?;
		let listing = match listing {
			Listing::FixedPrice(listing) => listing,
			_ => return Err(revert("Not fixed price")),
		};
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost().saturating_mul(3))?;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_marketplace::Call::<Runtime>::update_fixed_price { listing_id, new_price },
		)?;
		let collection_id = H256::from_low_u64_be(listing.collection_id as u64);
		let listing_id = H256::from_low_u64_be(listing_id as u64);

		log4(
			handle.code_address(),
			SELECTOR_LOG_FIXED_PRICE_SALE_UPDATE,
			collection_id,
			listing_id,
			H256::from_slice(&EvmDataWriter::new().write(new_price).build()),
			vec![],
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn buy(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(handle, { listing_id: U256 });
		let listing_id: u128 = listing_id.saturated_into();
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_marketplace::Call::<Runtime>::buy { listing_id },
		)?;
		let caller: Runtime::AccountId = handle.context().caller.into(); // caller is the buyer
		let _ = pallet_marketplace::Pallet::<Runtime>::do_buy(caller, listing_id);
		let listing = pallet_marketplace::Pallet::<Runtime>::get_listing_detail(listing_id)
			.or_else(|_| Err(revert("MARKETPLACE: NotForFixedPriceSale")))?;
		let listing = match listing {
			Listing::FixedPrice(listing) => listing,
			_ => return Err(revert("Not fixed price")),
		};
		let collection_id = H256::from_low_u64_be(listing.collection_id as u64);
		let listing_id = H256::from_low_u64_be(listing_id as u64);

		let seller = listing.seller;
		log4(
			handle.code_address(),
			SELECTOR_LOG_FIXED_PRICE_SALE_COMPLETE,
			collection_id,
			listing_id,
			H256::from_slice(&EvmDataWriter::new().write(listing.fixed_price).build()),
			vec![],
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn auction_nft(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(4, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				collection_address: Address,
				serial_numbers: Vec<U256>,
				payment_asset: Address,
				reserve_price: U256,
				duration: U256,
				marketplace_id: U256
			}
		);
		let marketplace_id: u32 = marketplace_id.saturated_into();
		let duration = Some(saturated_convert_blocknumber(duration)?.into());
		let reserve_price: Balance = reserve_price.saturated_into();
		let collection_id: CollectionUuid =
			<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
				collection_address,
				ERC721_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("MARKETPLACE: Invalid collection address"))?;
		let serials_unbounded = serial_numbers
			.into_iter()
			.map(|serial_number| {
				if serial_number > SerialNumber::MAX.into() {
					return Err(revert("MARKETPLACE: Expected serial_number <= 2^128").into())
				}
				let serial_number: SerialNumber = serial_number.saturated_into();
				Ok(serial_number)
			})
			.collect::<Result<Vec<SerialNumber>, PrecompileFailure>>()?;

		let serial_numbers = BoundedVec::try_from(serials_unbounded).unwrap();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		let marketplace_id: u32 = marketplace_id.saturated_into();
		let marketplace_id: Option<u32> =
			if marketplace_id == u32::default() { None } else { Some(marketplace_id) };
		// Parse asset_id
		let payment_asset: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			payment_asset,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("MARKETPLACE: Invalid payment asset address"))?;

		let caller: Runtime::AccountId = handle.context().caller.into();
		let listing_id = pallet_marketplace::Pallet::<Runtime>::do_auction_nft(
			caller,
			collection_id,
			serial_numbers,
			payment_asset,
			reserve_price,
			duration,
			marketplace_id,
		)
		.map_err(|e| {
			revert(alloc::format!("Marketplace: Dispatched call failed with error: {:?}", e))
		})?;
		let listing = pallet_marketplace::Pallet::<Runtime>::get_listing_detail(listing_id)
			.or_else(|_| Err(revert("MARKETPLACE: NotForFixedPriceSale")))?;
		let listing = match listing {
			Listing::Auction(listing) => listing,
			_ => return Err(revert("Not fixed price")),
		};
		let seller = listing.seller;
		let collection_id = H256::from_low_u64_be(collection_id as u64);
		let listing_id = H256::from_low_u64_be(listing_id as u64);
		log4(
			handle.code_address(),
			SELECTOR_LOG_AUCTION_OPEN,
			collection_id,
			listing_id,
			H256::from_slice(&EvmDataWriter::new().write(reserve_price).build()),
			vec![],
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

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

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_marketplace::Call::<Runtime>::bid { listing_id, amount },
		)?;

		let caller: Runtime::AccountId = handle.context().caller.into(); // caller is the bidder
		let listing_id = H256::from_low_u64_be(listing_id as u64);
		log4(
			handle.code_address(),
			SELECTOR_LOG_BID,
			handle.context().caller, //bidder
			listing_id,
			H256::from_slice(&EvmDataWriter::new().write(amount).build()),
			vec![],
		)
		.record(handle)?;

		Ok(succeed([]))
	}

	fn cancel_sale(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(handle, { listing_id: U256 });

		let caller: Runtime::AccountId = handle.context().caller.into();

		let listing_id: u128 = listing_id.saturated_into();
		let listing = pallet_marketplace::Pallet::<Runtime>::get_listing_detail(listing_id);
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_marketplace::Call::<Runtime>::cancel_sale { listing_id },
		)?;
		let listing_id = H256::from_low_u64_be(listing_id as u64);
		match listing {
			Ok(Listing::FixedPrice(sale)) => {
				let reason = FixedPriceClosureReason::VendorCancelled;
				log3(
					handle.code_address(),
					SELECTOR_LOG_FIXED_PRICE_SALE_CLOSE,
					handle.context().caller,
					listing_id,
					// EvmDataWriter::new().write(reason).build(),
					vec![],
				)
				.record(handle)?;
			},
			Ok(Listing::Auction(auction)) => {
				let reason = AuctionClosureReason::VendorCancelled;
				log3(
					handle.code_address(),
					SELECTOR_LOG_AUCTION_CLOSE,
					handle.context().caller,
					listing_id,
					// EvmDataWriter::new().write(reason).build(),
					vec![],
				)
				.record(handle)?;
			},
			_ => return Err(revert("Not valid")),
		}

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

		let caller: Runtime::AccountId = handle.context().caller.into(); // caller is the buyer
		let offer_id = pallet_marketplace::Pallet::<Runtime>::do_make_simple_offer(
			caller,
			token_id,
			amount,
			asset_id,
			marketplace_id,
		)
		.map_err(|e| {
			revert(alloc::format!("Marketplace: Dispatched call failed with error: {:?}", e))
		})?;
		let offer_id = H256::from_low_u64_be(offer_id as u64);

		log3(
			handle.code_address(),
			SELECTOR_LOG_OFFER,
			offer_id,
			handle.context().caller,
			EvmDataWriter::new().write(token_id).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(offer_id).build()))
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
		let offer = pallet_marketplace::Pallet::<Runtime>::get_offer_detail(offer_id);
		let offer_id = H256::from_low_u64_be(offer_id as u64);
		log2(
			handle.code_address(),
			SELECTOR_LOG_OFFER_CANCEL,
			offer_id,
			EvmDataWriter::new().write(offer.unwrap().token_id).build(),
		)
		.record(handle)?;
		Ok(succeed([]))
	}

	fn accept_offer(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;

		// Parse input.
		read_args!(handle, { offer_id: U256 });

		let offer_id: OfferId = offer_id.saturated_into();

		// Return either the approved account or zero address if no account is approved
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			None.into(),
			pallet_marketplace::Call::<Runtime>::accept_offer { offer_id },
		)?;
		let offer = pallet_marketplace::Pallet::<Runtime>::get_offer_detail(offer_id).unwrap();
		let offer_id = H256::from_low_u64_be(offer_id as u64);
		log3(
			handle.code_address(),
			SELECTOR_LOG_OFFER_ACCEPT,
			offer_id,
			H256::from_slice(&EvmDataWriter::new().write(offer.amount).build()),
			EvmDataWriter::new().write(offer.token_id).build(),
		)
		.record(handle)?;
		Ok(succeed([]))
	}
}
