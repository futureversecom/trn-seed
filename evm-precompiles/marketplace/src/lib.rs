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
};
use pallet_evm::{GasWeightMapping, Precompile};
use pallet_marketplace::{
	types::{Listing, MarketplaceId, OfferId},
	weights::WeightInfo,
};
use precompile_utils::{
	constants::{ERC20_PRECOMPILE_ADDRESS_PREFIX, ERC721_PRECOMPILE_ADDRESS_PREFIX},
	prelude::*,
};
use seed_primitives::{AssetId, Balance, BlockNumber, CollectionUuid, SerialNumber, TokenId};
use sp_core::{H160, H256, U256};
use sp_runtime::{traits::SaturatedConversion, BoundedVec, Permill};
use sp_std::{marker::PhantomData, vec::Vec};

/// Solidity selector of the Marketplace register log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_MARKETPLACE_REGISTER: [u8; 32] =
	keccak256!("MarketplaceRegister(address,uint256,address)"); // caller_id, marketplace_id

pub const SELECTOR_LOG_FIXED_PRICE_SALE_LIST: [u8; 32] =
	keccak256!("FixedPriceSaleList(address,uint256,uint256,uint256[],address)"); // seller_id, listing_id, fixed_price, serial_number_ids, collection_address

pub const SELECTOR_LOG_FIXED_PRICE_SALE_UPDATE: [u8; 32] =
	keccak256!("FixedPriceSaleUpdate(uint256,uint256,uint256,address,uint256[])"); // collection_id, listing_id, new_price, sender, serial_number_ids

pub const SELECTOR_LOG_FIXED_PRICE_SALE_COMPLETE: [u8; 32] =
	keccak256!("FixedPriceSaleComplete(uint256,uint256,uint256,address,uint256[])"); // collection_id, listing_id, fixed_price, sender, serial_number_ids

pub const SELECTOR_LOG_AUCTION_OPEN: [u8; 32] =
	keccak256!("AuctionOpen(uint256,uint256,uint256,address,uint256[])"); // collection_id, listing_id, reserve_price, sender, serial_number_ids

pub const SELECTOR_LOG_BID: [u8; 32] = keccak256!("Bid(address,uint256,uint256)"); // bidder, listing_id, amount
pub const SELECTOR_LOG_FIXED_PRICE_SALE_CLOSE: [u8; 32] =
	keccak256!("FixedPriceSaleClose(uint256,uint256,address,uint256[])"); // collectionId, listing_id, caller, series_ids

pub const SELECTOR_LOG_AUCTION_CLOSE: [u8; 32] =
	keccak256!("AuctionClose(uint256,uint256,address,uint256[])"); // collectionId, listing_id, caller, series_ids

pub const SELECTOR_LOG_OFFER: [u8; 32] = keccak256!("Offer(uint256,address,uint256,uint256)"); // offer_id, caller, collection_id, series_id

pub const SELECTOR_LOG_OFFER_CANCEL: [u8; 32] =
	keccak256!("OfferCancel(uint256,address,uint256,uint256)"); // offer_id, caller, token_id

pub const SELECTOR_LOG_OFFER_ACCEPT: [u8; 32] =
	keccak256!("OfferAccept(uint256,uint256,address,uint256,uint256)"); // offer_id, amount, caller, collection_id, series_id

/// Saturated conversion from EVM uint256 to Blocknumber
fn saturated_convert_blocknumber(input: U256) -> Result<BlockNumber, PrecompileFailure> {
	if input > BlockNumber::MAX.into() {
		return Err(
			revert("Marketplace: Input number exceeds the BlockNumber type boundary (2^32)").into()
		)
	}
	Ok(input.saturated_into())
}

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	RegisterMarketplace = "registerMarketplace(address,uint256)",
	SellNft = "sellNft(address,uint256[],address,address,uint256,uint256,uint32)",
	UpdateFixedPrice = "updateFixedPrice(uint128,uint256)",
	Buy = "buy(uint128)",
	AuctionNft = "auctionNft(address,uint256[],address,uint256,uint256,uint256)",
	Bid = "bid(uint128,uint256)",
	CancelSale = "cancelSale(uint128)",
	MakeSimpleOffer = "makeSimpleOffer(address,uint32,uint256,address,uint32)",
	CancelOffer = "cancelOffer(uint64)",
	AcceptOffer = "acceptOffer(uint64)",
	GetMarketplaceAccount = "getMarketplaceAccount(uint32)",
	GetListingFromId = "getListingFromId(uint128)",
	GetOfferFromId = "getOfferFromId(uint64)",
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

			if let Err(err) = handle.check_function_modifier(match selector {
				Action::RegisterMarketplace |
				Action::SellNft |
				Action::UpdateFixedPrice |
				Action::AuctionNft |
				Action::Bid |
				Action::CancelSale |
				Action::MakeSimpleOffer |
				Action::CancelOffer |
				Action::AcceptOffer => FunctionModifier::NonPayable,
				Action::Buy => FunctionModifier::Payable, // user would need to pay to buy nft
				_ => FunctionModifier::View,
			}) {
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
				Action::GetMarketplaceAccount => Self::get_marketplace_account(handle),
				Action::GetListingFromId => Self::get_listing_from_id(handle),
				Action::GetOfferFromId => Self::get_offer_from_id(handle),
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
		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_marketplace::Config>::WeightInfo::register_marketplace(),
		))?;
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
			handle.context().caller,
			marketplace_id,
			EvmDataWriter::new().write(marketplace_account).build(),
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
				collection_address: Address,
				serial_number_ids: Vec<U256>,
				buyer: Address,
				payment_asset: Address,
				fixed_price: U256,
				duration: U256,
				marketplace_id: U256
			}
		);
		// Parse asset_id
		let payment_asset: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			payment_asset,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("MARKETPLACE: Invalid payment asset address"))?;
		ensure!(
			marketplace_id <= u32::MAX.into(),
			revert("Marketplace: Expected marketplace id <= 2^32")
		);
		let marketplace_id: u32 = marketplace_id.saturated_into();
		let duration = Some(saturated_convert_blocknumber(duration)?.into());
		ensure!(
			fixed_price <= u128::MAX.into(),
			revert("Marketplace: Expected fixed price <= 2^128")
		);
		let fixed_price: Balance = fixed_price.saturated_into();
		let collection_id: CollectionUuid =
			<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
				collection_address,
				ERC721_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("Marketplace: Invalid collection address"))?;

		let serials_unbounded = serial_number_ids
			.clone()
			.into_iter()
			.map(|serial_number| {
				if serial_number > SerialNumber::MAX.into() {
					return Err(revert("Marketplace: Expected serial_number <= 2^32").into())
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
		let marketplace_id: Option<u32> =
			if marketplace_id == u32::default() { None } else { Some(marketplace_id) };

		let caller: Runtime::AccountId = handle.context().caller.into();
		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_marketplace::Config>::WeightInfo::sell(),
		))?;
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
		ensure!(
			listing_id <= u128::MAX.into(),
			revert("Marketplace: Expected listing id <= 2^128")
		);

		log4(
			handle.code_address(),
			SELECTOR_LOG_FIXED_PRICE_SALE_LIST,
			handle.context().caller, //seller
			H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
			H256::from_slice(&EvmDataWriter::new().write(fixed_price).build()),
			EvmDataWriter::new().write(serial_number_ids).write(collection_address).build(),
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
		ensure!(
			listing_id <= u128::MAX.into(),
			revert("Marketplace: Expected listing id <= 2^128")
		);
		let listing_id: u128 = listing_id.saturated_into();

		ensure!(new_price <= u128::MAX.into(), revert("Marketplace: Expected new price <= 2^128"));
		let new_price: Balance = new_price.saturated_into();
		let caller: Runtime::AccountId = handle.context().caller.into();
		let _ = pallet_marketplace::Pallet::<Runtime>::do_update_fixed_price(
			caller, listing_id, new_price,
		);
		let listing = match pallet_marketplace::Pallet::<Runtime>::get_listing_detail(listing_id) {
			Ok(Listing::FixedPrice(listing)) => listing,
			_ => return Err(revert("Not fixed price")),
		};
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost().saturating_mul(3))?;
		let origin = handle.context().caller;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_marketplace::Call::<Runtime>::update_fixed_price { listing_id, new_price },
		)?;
		let collection_id = H256::from_low_u64_be(listing.collection_id as u64);

		let caller: H160 = caller.into();
		log4(
			handle.code_address(),
			SELECTOR_LOG_FIXED_PRICE_SALE_UPDATE,
			collection_id,
			H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
			H256::from_slice(&EvmDataWriter::new().write(new_price).build()),
			EvmDataWriter::new()
				.write(Address::from(caller))
				.write(listing.serial_numbers.into_inner())
				.build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn buy(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(handle, { listing_id: U256 });
		ensure!(
			listing_id <= u128::MAX.into(),
			revert("Marketplace: Expected listing id <= 2^128")
		);
		let listing_id: u128 = listing_id.saturated_into();

		let caller: Runtime::AccountId = handle.context().caller.into(); // caller is the buyer
		let listing = pallet_marketplace::Pallet::<Runtime>::do_buy(caller, listing_id)
			.or_else(|_| Err(revert("Marketplace: NotForFixedPriceSale")))?;

		let collection_id = H256::from_low_u64_be(listing.collection_id as u64);

		let seller = listing.seller;
		let seller: H160 = seller.into();
		log4(
			handle.code_address(),
			SELECTOR_LOG_FIXED_PRICE_SALE_COMPLETE,
			collection_id,
			H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
			H256::from_slice(&EvmDataWriter::new().write(listing.fixed_price).build()),
			EvmDataWriter::new()
				.write(Address::from(seller))
				.write(listing.serial_numbers.into_inner())
				.build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn auction_nft(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				collection_address: Address,
				serial_number_ids: Vec<U256>,
				payment_asset: Address,
				reserve_price: U256,
				duration: U256,
				marketplace_id: U256
			}
		);
		ensure!(
			marketplace_id <= u32::MAX.into(),
			revert("Marketplace: Expected marketplace id <= 2^32")
		);
		let marketplace_id: u32 = marketplace_id.saturated_into();
		let duration = Some(saturated_convert_blocknumber(duration)?.into());
		ensure!(
			reserve_price <= Balance::MAX.into(),
			revert("Marketplace: Expected reserve_price <= 2^128")
		);
		let reserve_price: Balance = reserve_price.saturated_into();
		let collection_id: CollectionUuid =
			<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
				collection_address,
				ERC721_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("Marketplace: Invalid collection address"))?;
		ensure!(
			collection_id <= u32::MAX.into(),
			revert("Marketplace: Expected collection id <= 2^32")
		);
		let serials_unbounded = serial_number_ids
			.clone()
			.into_iter()
			.map(|serial_number| {
				if serial_number > SerialNumber::MAX.into() {
					return Err(revert("Marketplace: Expected serial_number <= 2^32").into())
				}
				let serial_number: SerialNumber = serial_number.saturated_into();
				Ok(serial_number)
			})
			.collect::<Result<Vec<SerialNumber>, PrecompileFailure>>()?;

		let serial_numbers = BoundedVec::try_from(serials_unbounded).unwrap();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		ensure!(
			marketplace_id <= u32::MAX.into(),
			revert("Marketplace: Expected marketplace_id <= 2^32")
		);
		let marketplace_id: u32 = marketplace_id.saturated_into();
		let marketplace_id: Option<u32> =
			if marketplace_id == u32::default() { None } else { Some(marketplace_id) };
		// Parse asset_id
		let payment_asset: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			payment_asset,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("Marketplace: Invalid payment asset address"))?;

		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_marketplace::Config>::WeightInfo::auction(),
		))?;

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
		let collection_id = H256::from_low_u64_be(collection_id as u64);
		// let listing_id = convert_u128_to_h256(listing_id);
		log4(
			handle.code_address(),
			SELECTOR_LOG_AUCTION_OPEN,
			collection_id,
			H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
			H256::from_slice(&EvmDataWriter::new().write(reserve_price).build()),
			EvmDataWriter::new()
				.write(Address::from(handle.context().caller))
				.write(serial_number_ids)
				.build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn bid(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(4, 32)?;
		// Parse input.
		read_args!(
			handle,
			{
				listing_id: U256,
				amount: U256
			}
		);
		ensure!(
			listing_id <= u128::MAX.into(),
			revert("Marketplace: Expected listing_id <= 2^128")
		);
		let listing_id: u128 = listing_id.saturated_into();
		ensure!(amount <= u128::MAX.into(), revert("Marketplace: Expected amount <= 2^128"));
		let amount: Balance = amount.saturated_into();
		let origin = handle.context().caller;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_marketplace::Call::<Runtime>::bid { listing_id, amount },
		)?;

		// let listing_id = convert_u128_to_h256(listing_id);
		log4(
			handle.code_address(),
			SELECTOR_LOG_BID,
			handle.context().caller, //bidder
			H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
			H256::from_slice(&EvmDataWriter::new().write(amount).build()),
			alloc::vec![],
		)
		.record(handle)?;

		Ok(succeed([]))
	}

	fn cancel_sale(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;
		// Parse input.
		read_args!(handle, { listing_id: U256 });

		ensure!(
			listing_id <= u128::MAX.into(),
			revert("Marketplace: Expected listing id <= 2^128")
		);
		let listing_id: u128 = listing_id.saturated_into();

		let origin = handle.context().caller;

		let listing = pallet_marketplace::Pallet::<Runtime>::get_listing_detail(listing_id)
			.or_else(|_| Err(revert("Marketplace: listing details not found")))?;
		let (collection_id, serial_numbers) = match listing.clone() {
			Listing::FixedPrice(listing) => (listing.collection_id, listing.serial_numbers),
			Listing::Auction(listing) => (listing.collection_id, listing.serial_numbers),
		};
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_marketplace::Call::<Runtime>::cancel_sale { listing_id },
		)?;
		let collection_id = H256::from_low_u64_be(collection_id as u64);
		// let listing_id = convert_u128_to_h256(listing_id);
		match listing {
			Listing::FixedPrice(_sale) => {
				log3(
					handle.code_address(),
					SELECTOR_LOG_FIXED_PRICE_SALE_CLOSE,
					collection_id,
					H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
					EvmDataWriter::new()
						.write(Address::from(handle.context().caller))
						.write(serial_numbers.into_inner())
						.build(),
				)
				.record(handle)?;
			},
			Listing::Auction(_auction) => {
				log3(
					handle.code_address(),
					SELECTOR_LOG_AUCTION_CLOSE,
					collection_id,
					H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
					EvmDataWriter::new()
						.write(Address::from(handle.context().caller))
						.write(serial_numbers.into_inner())
						.build(),
				)
				.record(handle)?;
			},
			_ => return Err(revert("Not valid")),
		}

		// Build output.
		Ok(succeed([]))
	}

	fn make_simple_offer(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

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
		ensure!(
			marketplace_id <= u32::MAX.into(),
			revert("Marketplace: Expected marketplace id <= 2^32")
		);
		let marketplace_id: Option<u32> =
			if marketplace_id == u32::default() { None } else { Some(marketplace_id) };
		ensure!(amount <= u128::MAX.into(), revert("Marketplace: Expected amount <= 2^128"));
		let amount: Balance = amount.saturated_into();
		let collection_id: CollectionUuid =
			<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
				collection_address,
				ERC721_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("Marketplace: Invalid collection address"))?;
		ensure!(
			collection_id <= u32::MAX.into(),
			revert("Marketplace: Expected collection_id <= 2^32")
		);
		ensure!(
			serial_number <= u32::MAX.into(),
			revert("Marketplace: Expected serial_number <= 2^32")
		);
		let serial_number: SerialNumber = serial_number.saturated_into();
		let token_id: TokenId = (collection_id, serial_number);
		// Parse asset_id
		let asset_id: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			asset_id,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("Marketplace: Invalid asset address"))?;
		ensure!(asset_id <= u32::MAX.into(), revert("Marketplace: Expected asset_id <= 2^32"));

		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_marketplace::Config>::WeightInfo::make_simple_offer(),
		))?;

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
		// let offer_id = H256::from_low_u64_be(offer_id);

		log3(
			handle.code_address(),
			SELECTOR_LOG_OFFER,
			H256::from_slice(&EvmDataWriter::new().write(offer_id).build()),
			handle.context().caller,
			EvmDataWriter::new().write(collection_id).write(serial_number).build(),
			// EvmDataWriter::new().write(token_id).build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(offer_id).build()))
	}

	fn cancel_offer(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;

		// Parse input.
		read_args!(handle, { offer_id: U256 });

		ensure!(offer_id <= u64::MAX.into(), revert("Marketplace: Expected offer_id <= 2^64"));
		let offer_id: OfferId = offer_id.saturated_into();
		let offer = pallet_marketplace::Pallet::<Runtime>::get_offer_detail(offer_id)
			.or_else(|_| Err(revert("Marketplace: Offer details not found")))?;

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let origin = handle.context().caller;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_marketplace::Call::<Runtime>::cancel_offer { offer_id },
		)?;
		let (collection_id, serial_number) = offer.token_id;
		let offer_id = H256::from_low_u64_be(offer_id);
		log3(
			handle.code_address(),
			SELECTOR_LOG_OFFER_CANCEL,
			offer_id,
			handle.context().caller,
			EvmDataWriter::new().write(collection_id).write(serial_number).build(),
		)
		.record(handle)?;
		Ok(succeed([]))
	}

	fn accept_offer(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(handle, { offer_id: U256 });

		ensure!(offer_id <= u64::MAX.into(), revert("Marketplace: Expected offer_id <= 2^64"));
		let offer_id: OfferId = offer_id.saturated_into();
		let offer = pallet_marketplace::Pallet::<Runtime>::get_offer_detail(offer_id)
			.or_else(|_| Err(revert("Marketplace: Offer details not found")))?;

		// Return either the approved account or zero address if no account is approved
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let origin = handle.context().caller;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_marketplace::Call::<Runtime>::accept_offer { offer_id },
		)?;
		let offer_id = H256::from_low_u64_be(offer_id);
		let (collection_id, serial_number) = offer.token_id;
		log4(
			handle.code_address(),
			SELECTOR_LOG_OFFER_ACCEPT,
			offer_id,
			H256::from_slice(&EvmDataWriter::new().write(offer.amount).build()),
			handle.context().caller,
			EvmDataWriter::new().write(collection_id).write(serial_number).build(),
		)
		.record(handle)?;
		Ok(succeed([]))
	}

	fn get_marketplace_account(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(handle, { marketplace_id: U256 });

		// handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		ensure!(
			marketplace_id <= u32::MAX.into(),
			revert("Marketplace: Expected marketplace id <= 2^32")
		);
		let marketplace_id: MarketplaceId = marketplace_id.saturated_into();
		let Some(marketplace_account) = pallet_marketplace::RegisteredMarketplaces::<Runtime>::get(marketplace_id) else {
			return Err(revert("Marketplace: The account_id hasn't been registered as a marketplace"));
		};
		let marketplace_account_h160: H160 = marketplace_account.account.into();
		Ok(succeed(EvmDataWriter::new().write(Address::from(marketplace_account_h160)).build()))
	}

	fn get_listing_from_id(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(handle, { listing_id: U256 });
		ensure!(
			listing_id <= u128::MAX.into(),
			revert("Marketplace: Expected listing id <= 2^128")
		);
		let listing_id: u128 = listing_id.saturated_into();

		let listing = pallet_marketplace::Pallet::<Runtime>::get_listing_detail(listing_id)
			.or_else(|_| Err(revert("Marketplace: listing details not found")))?;
		let (collection_id, serial_numbers, price, payment_asset) = match listing {
			Listing::FixedPrice(listing) => (
				listing.collection_id,
				listing.serial_numbers,
				listing.fixed_price,
				listing.payment_asset,
			),
			Listing::Auction(listing) => (
				listing.collection_id,
				listing.serial_numbers,
				listing.reserve_price,
				listing.payment_asset,
			),
		};
		Ok(succeed(
			EvmDataWriter::new()
				.write::<u32>(collection_id)
				.write::<Vec<u32>>(serial_numbers.into_inner())
				.write::<u128>(price)
				.write::<u32>(payment_asset)
				// .write::<u32>(marketplace_id.unwrap())
				.build(),
		))
	}

	fn get_offer_from_id(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		// Parse input.
		read_args!(handle, { offer_id: U256 });
		ensure!(offer_id <= u64::MAX.into(), revert("Marketplace: Expected offer_id <= 2^64"));
		let offer_id: OfferId = offer_id.saturated_into();

		let offer = pallet_marketplace::Pallet::<Runtime>::get_offer_detail(offer_id);
		if offer.is_err() {
			return Err(revert("Marketplace: Offer details not found"))
		}
		let offer = offer.unwrap();
		let (collection_id, serial_number) = offer.token_id;
		let buyer: H160 = offer.buyer.into();

		Ok(succeed(
			EvmDataWriter::new()
				.write::<u32>(collection_id)
				.write::<u32>(serial_number)
				.write::<u128>(offer.amount)
				.write::<Address>(Address::from(buyer))
				// .write::<u32>(offer.marketplace_id.unwrap())
				.build(),
		))
	}
}
