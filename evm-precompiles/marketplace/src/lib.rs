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
	types::{Listing, ListingTokens, MarketplaceId, NftListing, OfferId, SftListing},
	weights::WeightInfo,
};
use precompile_utils::{
	constants::{
		ERC1155_PRECOMPILE_ADDRESS_PREFIX, ERC20_PRECOMPILE_ADDRESS_PREFIX,
		ERC721_PRECOMPILE_ADDRESS_PREFIX,
	},
	prelude::*,
};
use seed_primitives::{AssetId, Balance, BlockNumber, CollectionUuid, SerialNumber, TokenId};
use sp_core::{H160, H256, U256};
use sp_runtime::{traits::SaturatedConversion, BoundedVec, Permill};
use sp_std::{marker::PhantomData, vec::Vec};

/// Solidity selector of the Marketplace register log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_MARKETPLACE_REGISTER: [u8; 32] =
	keccak256!("MarketplaceRegister(address,uint256,address)"); // caller_id, marketplace_id

pub const SELECTOR_LOG_FIXED_PRICE_SALE_LIST_NFT: [u8; 32] =
	keccak256!("FixedPriceSaleListNFT(address,uint256,uint256,uint256[],address,uint128)");
// seller_id, listing_id, fixed_price, serial_number_ids, collection_address, marketplace_id

pub const SELECTOR_LOG_FIXED_PRICE_SALE_LIST_SFT: [u8; 32] = keccak256!(
	"FixedPriceSaleListSFT(address,uint256,uint256,uint256[],address,uint128,uint256[])"
);
// seller_id, listing_id, fixed_price, serial_number_ids, collection_address, marketplace_id

pub const SELECTOR_LOG_FIXED_PRICE_SALE_UPDATE: [u8; 32] =
	keccak256!("FixedPriceSaleUpdate(uint256,uint256,uint256,address,uint256[],uint128)");
// collection_id, listing_id, new_price, sender, serial_number_ids, marketplace_id

pub const SELECTOR_LOG_FIXED_PRICE_SALE_COMPLETE: [u8; 32] =
	keccak256!("FixedPriceSaleComplete(uint256,uint256,uint256,address,uint256[],uint128)");
// collection_id, listing_id, fixed_price, sender, serial_number_ids, marketplace_id

pub const SELECTOR_LOG_AUCTION_OPEN_NFT: [u8; 32] =
	keccak256!("AuctionOpenNFT(uint256,uint256,uint256,address,uint256[],uint128)");
// collection_id, listing_id, reserve_price, sender, serial_number_ids, marketplace_id

pub const SELECTOR_LOG_AUCTION_OPEN_SFT: [u8; 32] =
	keccak256!("AuctionOpenSFT(uint256,uint256,uint256,address,uint256[],uint128)");
// collection_id, listing_id, reserve_price, sender, serial_number_ids, marketplace_id

pub const SELECTOR_LOG_BID: [u8; 32] = keccak256!("Bid(address,uint256,uint256,uint128)");
// bidder, listing_id, amount, marketplace_id

pub const SELECTOR_LOG_FIXED_PRICE_SALE_CLOSE: [u8; 32] =
	keccak256!("FixedPriceSaleClose(uint256,uint256,address,uint256[],uint128)");
// collectionId, listing_id, caller, series_ids, marketplace_id

pub const SELECTOR_LOG_AUCTION_CLOSE: [u8; 32] =
	keccak256!("AuctionClose(uint256,uint256,address,uint256[],uint128)");
// collectionId, listing_id, caller, series_ids, marketplace_id

pub const SELECTOR_LOG_OFFER: [u8; 32] =
	keccak256!("Offer(uint256,address,uint256,uint256,uint128)"); // offer_id, caller, collection_id, series_id, marketplace_id

pub const SELECTOR_LOG_OFFER_CANCEL: [u8; 32] =
	keccak256!("OfferCancel(uint256,address,uint256,uint256,uint128)"); // offer_id, caller, token_id, marketplace_id

pub const SELECTOR_LOG_OFFER_ACCEPT: [u8; 32] =
	keccak256!("OfferAccept(uint256,uint256,address,uint256,uint256,uint128)");
// offer_id, amount, caller, collection_id, series_id, marketplace_id

/// Saturated conversion from EVM uint256 to Blocknumber
fn saturated_convert_blocknumber(input: U256) -> Result<BlockNumber, PrecompileFailure> {
	if input > BlockNumber::MAX.into() {
		return Err(revert(
			"Marketplace: Input number exceeds the BlockNumber type boundary (2^32)",
		));
	}
	Ok(input.saturated_into())
}

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	RegisterMarketplace = "registerMarketplace(address,uint256)",
	#[deprecated(
		note = "Use `sellNft(address,uint256[],address,address,uint256,uint256,uint32)` instead"
	)]
	SellNftWithMarketplaceId =
		"sellNftWithMarketplaceId(address,uint256[],address,address,uint256,uint256,uint32)",
	SellNft = "sellNft(address,uint256[],address,address,uint256,uint256,uint32)",
	SellSft = "sellSft(address,uint256[],uint256[],address,address,uint256,uint256,uint32)", /* collection_address, serial_number_ids, quantities, buyer, payment_asset, fixed_price, duration, marketplace_id */

	UpdateFixedPrice = "updateFixedPrice(uint128,uint256)",
	Buy = "buy(uint128)",
	#[deprecated(
		note = "Use `auctionNft(address,uint256[],address,uint256,uint256,uint32)` instead"
	)]
	AuctionNftWithMarketplaceId =
		"auctionNftWithMarketplaceId(address,uint256[],address,uint256,uint256,uint32)",
	AuctionNft = "auctionNft(address,uint256[],address,uint256,uint256,uint32)",

	AuctionSft = "auctionSft(address,uint256[],uint256[],address,uint256,uint256,uint32)",

	Bid = "bid(uint128,uint256)",
	CancelSale = "cancelSale(uint128)",
	#[deprecated(note = "Use `makeSimpleOffer(address,uint32,uint256,address,uint32)` instead")]
	MakeSimpleOfferWithMarketplaceId =
		"makeSimpleOfferWithMarketplaceId(address,uint32,uint256,address,uint32)",
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

#[allow(deprecated)]
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
		let selector = match handle.read_selector() {
			Ok(selector) => selector,
			Err(e) => return Err(e.into()),
		};

		if let Err(err) = handle.check_function_modifier(match selector {
			Action::RegisterMarketplace
			| Action::SellNft
			| Action::SellNftWithMarketplaceId
			| Action::SellSft
			| Action::UpdateFixedPrice
			| Action::AuctionNft
			| Action::AuctionNftWithMarketplaceId
			| Action::AuctionSft
			| Action::Bid
			| Action::Buy
			| Action::CancelSale
			| Action::MakeSimpleOffer
			| Action::MakeSimpleOfferWithMarketplaceId
			| Action::CancelOffer
			| Action::AcceptOffer => FunctionModifier::NonPayable,
			_ => FunctionModifier::View,
		}) {
			return Err(err.into());
		}

		match selector {
			Action::RegisterMarketplace => Self::register_marketplace(handle),
			Action::SellNftWithMarketplaceId => Self::sell_nft(handle),
			Action::SellNft => Self::sell_nft(handle),
			Action::SellSft => Self::sell_sft(handle),
			Action::UpdateFixedPrice => Self::update_fixed_price(handle),
			Action::Buy => Self::buy(handle),
			Action::AuctionNftWithMarketplaceId => Self::auction_nft_with_marketplace_id(handle),
			Action::AuctionNft => Self::auction_nft_with_marketplace_id(handle),
			Action::AuctionSft => Self::auction_sft_with_marketplace_id(handle),
			Action::Bid => Self::bid(handle),
			Action::CancelSale => Self::cancel_sale(handle),
			Action::MakeSimpleOfferWithMarketplaceId => {
				Self::make_simple_offer_with_marketplace_id(handle)
			},
			Action::MakeSimpleOffer => Self::make_simple_offer_with_marketplace_id(handle),
			Action::CancelOffer => Self::cancel_offer(handle),
			Action::AcceptOffer => Self::accept_offer(handle),
			Action::GetMarketplaceAccount => Self::get_marketplace_account(handle),
			Action::GetListingFromId => Self::get_listing_from_id(handle),
			Action::GetOfferFromId => Self::get_offer_from_id(handle),
		}
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
		handle.record_log_costs_manual(2, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				marketplace_account: Address,
				entitlement: U256
			}
		);

		let marketplace_account_h160: H160 = marketplace_account.into();
		let marketplace_account_option: Option<Runtime::AccountId> =
			if marketplace_account_h160 == H160::default() {
				None
			} else {
				Some(marketplace_account_h160.into())
			};

		ensure!(
			entitlement <= u32::MAX.into(),
			revert("Marketplace: Expected entitlement <= 2^32")
		);
		let entitlement: u32 = entitlement.saturated_into();
		let entitlement: Permill = Permill::from_parts(entitlement);
		let caller: Runtime::AccountId = handle.context().caller.into();
		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_marketplace::Config>::WeightInfo::register_marketplace(),
		))?;
		let marketplace_id = pallet_marketplace::Pallet::<Runtime>::do_register_marketplace(
			caller,
			marketplace_account_option,
			entitlement,
		)
		.map_err(|e| {
			revert(alloc::format!("Marketplace: Dispatched call failed with error: {:?}", e))
		})?;

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
		ensure!(
			marketplace_id <= u32::MAX.into(),
			revert("Marketplace: Expected marketplace id <= 2^32")
		);
		let marketplace_id: Option<MarketplaceId> = match marketplace_id {
			i if i == U256::zero() => None,
			_ => Some(marketplace_id.saturated_into()),
		};

		let payment_asset: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			payment_asset,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("Marketplace NFT: Invalid payment asset address"))?;

		let duration: BlockNumber = saturated_convert_blocknumber(duration)?;
		let duration = match duration {
			0 => None,
			n => Some(n),
		};
		ensure!(
			fixed_price <= u128::MAX.into(),
			revert("Marketplace NFT: Expected fixed price <= 2^128")
		);
		let fixed_price: Balance = fixed_price.saturated_into();

		let collection_id: CollectionUuid =
			<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
				collection_address,
				ERC721_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("Marketplace NFT: Invalid collection address"))?;

		let serials_unbounded = serial_number_ids
			.clone()
			.into_iter()
			.map(|serial_number| {
				if serial_number > SerialNumber::MAX.into() {
					return Err(revert("Marketplace NFT: Expected serial_number <= 2^32"));
				}
				let serial_number: SerialNumber = serial_number.saturated_into();
				Ok(serial_number)
			})
			.collect::<Result<Vec<SerialNumber>, PrecompileFailure>>()?;

		let serial_numbers: BoundedVec<SerialNumber, Runtime::MaxTokensPerListing> =
			BoundedVec::try_from(serials_unbounded)
				.map_err(|_| revert("Marketplace NFT: Too many serial numbers"))?;
		let tokens = ListingTokens::Nft(NftListing { collection_id, serial_numbers });

		let buyer: H160 = buyer.into();
		let buyer: Option<Runtime::AccountId> =
			if buyer == H160::default() { None } else { Some(buyer.into()) };

		let caller: Runtime::AccountId = handle.context().caller.into();
		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_marketplace::Config>::WeightInfo::sell_nft(
				serial_number_ids.len() as u32
			),
		))?;
		let listing_id = pallet_marketplace::Pallet::<Runtime>::do_sell(
			caller,
			tokens,
			buyer,
			payment_asset,
			fixed_price,
			duration.map(Into::into),
			marketplace_id,
		)
		.map_err(|e| {
			revert(alloc::format!("Marketplace NFT: Dispatched call failed with error: {:?}", e))
		})?;
		log4(
			handle.code_address(),
			SELECTOR_LOG_FIXED_PRICE_SALE_LIST_NFT,
			handle.context().caller, //seller
			H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
			H256::from_slice(&EvmDataWriter::new().write(fixed_price).build()),
			EvmDataWriter::new()
				.write(serial_number_ids)
				.write(collection_address)
				.write(marketplace_id.unwrap_or_default())
				.build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(listing_id).build()))
	}

	fn sell_sft(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;
		read_args!(
			handle,
			{
				collection_address: Address,
				serial_number_ids: Vec<U256>,
				quantities: Vec<U256>,
				buyer: Address,
				payment_asset: Address,
				fixed_price: U256,
				duration: U256,
				marketplace_id: U256
			}
		);
		ensure!(
			marketplace_id <= u32::MAX.into(),
			revert("Marketplace: Expected marketplace id <= 2^32")
		);
		let marketplace_id: Option<MarketplaceId> = match marketplace_id {
			i if i == U256::zero() => None,
			_ => Some(marketplace_id.saturated_into()),
		};

		// Parse asset_id
		let payment_asset: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			payment_asset,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("Marketplace SFT: Invalid payment asset address"))?;

		let duration: BlockNumber = saturated_convert_blocknumber(duration)?;
		let duration = match duration {
			0 => None,
			n => Some(n),
		};
		ensure!(
			fixed_price <= u128::MAX.into(),
			revert("Marketplace SFT: Expected fixed price <= 2^128")
		);
		let fixed_price: Balance = fixed_price.saturated_into();
		let collection_id: CollectionUuid =
			<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
				collection_address,
				ERC1155_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("Marketplace SFT: Invalid collection address"))?;
		ensure!(
			serial_number_ids.len() == quantities.len(),
			revert("Marketplace SFT: Expected serial number ids and quantities array to be equal")
		);

		let serials_unbounded = serial_number_ids
			.clone()
			.into_iter()
			.zip(quantities.clone())
			.map(|(serial_number, quantity)| {
				if serial_number > SerialNumber::MAX.into() {
					return Err(revert("Marketplace SFT: Expected serial_number <= 2^32"));
				}
				if quantity > Balance::MAX.into() {
					return Err(revert("Marketplace SFT: Expected quantity <= 2^128"));
				}
				let serial_number: SerialNumber = serial_number.saturated_into();
				let quantity: Balance = quantity.saturated_into();
				Ok((serial_number, quantity))
			})
			.collect::<Result<Vec<(SerialNumber, Balance)>, PrecompileFailure>>()?;

		let serial_numbers: BoundedVec<(SerialNumber, Balance), Runtime::MaxTokensPerListing> =
			BoundedVec::try_from(serials_unbounded)
				.map_err(|_| revert("Marketplace: Too many serial numbers"))?;
		let tokens = ListingTokens::Sft(SftListing { collection_id, serial_numbers });
		let buyer: H160 = buyer.into();
		let buyer: Option<Runtime::AccountId> =
			if buyer == H160::default() { None } else { Some(buyer.into()) };

		let caller: Runtime::AccountId = handle.context().caller.into();
		// Manually record gas
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_marketplace::Config>::WeightInfo::sell_sft(
				serial_number_ids.len() as u32
			),
		))?;
		let listing_id = pallet_marketplace::Pallet::<Runtime>::do_sell(
			caller,
			tokens,
			buyer,
			payment_asset,
			fixed_price,
			duration.map(Into::into),
			marketplace_id,
		)
		.map_err(|e| {
			revert(alloc::format!("Marketplace SFT: Dispatched call failed with error: {:?}", e))
		})?;
		log4(
			handle.code_address(),
			SELECTOR_LOG_FIXED_PRICE_SALE_LIST_SFT,
			handle.context().caller, //seller
			H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
			H256::from_slice(&EvmDataWriter::new().write(fixed_price).build()),
			EvmDataWriter::new()
				.write(serial_number_ids)
				.write(collection_address)
				.write(marketplace_id.unwrap_or_default())
				.write::<Vec<U256>>(quantities)
				.build(),
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
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let listing = match pallet_marketplace::Pallet::<Runtime>::get_listing_detail(listing_id) {
			Ok(Listing::FixedPrice(listing)) => listing,
			_ => return Err(revert("Not fixed price")),
		};
		let origin = handle.context().caller;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_marketplace::Call::<Runtime>::update_fixed_price { listing_id, new_price },
		)?;

		let (collection_id, serial_numbers) = Self::split_listing_tokens(listing.tokens)?;
		let collection_id = H256::from_low_u64_be(collection_id as u64);
		let marketplace_id = listing.marketplace_id;
		let caller: H160 = caller.into();

		log4(
			handle.code_address(),
			SELECTOR_LOG_FIXED_PRICE_SALE_UPDATE,
			collection_id,
			H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
			H256::from_slice(&EvmDataWriter::new().write(new_price).build()),
			EvmDataWriter::new()
				.write(Address::from(caller))
				.write(serial_numbers)
				.write(marketplace_id.unwrap_or_default())
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
		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_marketplace::Config>::WeightInfo::buy(),
		))?;

		let caller: Runtime::AccountId = handle.context().caller.into(); // caller is the buyer

		// Dispatch call
		let maybe_listing = pallet_marketplace::Pallet::<Runtime>::do_buy(caller, listing_id);

		// Build output.
		match maybe_listing {
			Ok(listing) => {
				let (collection_id, serial_numbers) = Self::split_listing_tokens(listing.tokens)?;
				let collection_id = H256::from_low_u64_be(collection_id as u64);
				let marketplace_id = listing.marketplace_id.unwrap_or_default();

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
						.write(serial_numbers)
						.write(marketplace_id)
						.build(),
				)
				.record(handle)?;

				// Build output.
				Ok(succeed([]))
			},
			Err(err) => Err(revert(
				alloc::format!("Marketplace: buy nft failed {:?}", err.stripped()).as_bytes(),
			)),
		}
	}

	fn auction_nft_with_marketplace_id(
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
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

		let duration: BlockNumber = saturated_convert_blocknumber(duration)?;
		let duration = match duration {
			0 => None,
			n => Some(n),
		};
		ensure!(
			reserve_price <= Balance::MAX.into(),
			revert("Marketplace NFT: Expected reserve_price <= 2^128")
		);
		let reserve_price: Balance = reserve_price.saturated_into();

		let collection_id: CollectionUuid =
			<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
				collection_address,
				ERC721_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("Marketplace NFT: Invalid collection address"))?;

		let serials_unbounded = serial_number_ids
			.clone()
			.into_iter()
			.map(|serial_number| {
				if serial_number > SerialNumber::MAX.into() {
					return Err(revert("Marketplace NFT: Expected serial_number <= 2^32"));
				}
				let serial_number: SerialNumber = serial_number.saturated_into();
				Ok(serial_number)
			})
			.collect::<Result<Vec<SerialNumber>, PrecompileFailure>>()?;

		let serial_numbers: BoundedVec<SerialNumber, Runtime::MaxTokensPerListing> =
			BoundedVec::try_from(serials_unbounded)
				.map_err(|_| revert("Marketplace NFT: Too many serial numbers"))?;
		let tokens = ListingTokens::Nft(NftListing { collection_id, serial_numbers });

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse asset_id
		let payment_asset: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			payment_asset,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("Marketplace NFT: Invalid payment asset address"))?;

		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_marketplace::Config>::WeightInfo::auction_nft(
				serial_number_ids.len() as u32,
			),
		))?;

		let caller: Runtime::AccountId = handle.context().caller.into();
		let listing_id = pallet_marketplace::Pallet::<Runtime>::do_auction(
			caller,
			tokens,
			payment_asset,
			reserve_price,
			duration.map(Into::into),
			Some(marketplace_id),
		)
		.map_err(|e| {
			revert(alloc::format!("Marketplace: Dispatched call failed with error: {:?}", e))
		})?;
		let collection_id = H256::from_low_u64_be(collection_id as u64);
		log4(
			handle.code_address(),
			SELECTOR_LOG_AUCTION_OPEN_NFT,
			collection_id,
			H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
			H256::from_slice(&EvmDataWriter::new().write(reserve_price).build()),
			EvmDataWriter::new()
				.write(Address::from(handle.context().caller))
				.write(serial_number_ids)
				.write(marketplace_id)
				.build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn auction_sft_with_marketplace_id(
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;

		// Parse input.
		read_args!(
			handle,
			{
				collection_address: Address,
				serial_number_ids: Vec<U256>,
				quantities: Vec<U256>,
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

		let duration: BlockNumber = saturated_convert_blocknumber(duration)?;
		let duration = match duration {
			0 => None,
			n => Some(n),
		};
		ensure!(
			reserve_price <= Balance::MAX.into(),
			revert("Marketplace SFT: Expected reserve_price <= 2^128")
		);
		let reserve_price: Balance = reserve_price.saturated_into();

		let collection_id: CollectionUuid =
			<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
				collection_address,
				ERC1155_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("Marketplace: Invalid collection address"))?;

		ensure!(
			serial_number_ids.len() == quantities.len(),
			revert("Marketplace: Expected serial number ids and quantities array to be equal")
		);

		let serials_unbounded = serial_number_ids
			.clone()
			.into_iter()
			.zip(quantities.clone())
			.map(|(serial_number, quantity)| {
				if serial_number > SerialNumber::MAX.into() {
					return Err(revert("Marketplace: Expected serial_number <= 2^32"));
				}
				if quantity > Balance::MAX.into() {
					return Err(revert("Marketplace: Expected quantity <= 2^128"));
				}
				let serial_number: SerialNumber = serial_number.saturated_into();
				let quantity: Balance = quantity.saturated_into();
				Ok((serial_number, quantity))
			})
			.collect::<Result<Vec<(SerialNumber, Balance)>, PrecompileFailure>>()?;

		let serial_numbers: BoundedVec<(SerialNumber, Balance), Runtime::MaxTokensPerListing> =
			BoundedVec::try_from(serials_unbounded)
				.map_err(|_| revert("Marketplace: Too many serial numbers"))?;
		let tokens = ListingTokens::Sft(SftListing { collection_id, serial_numbers });

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse asset_id
		let payment_asset: AssetId = <Runtime as ErcIdConversion<AssetId>>::evm_id_to_runtime_id(
			payment_asset,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.ok_or_else(|| revert("Marketplace SFT: Invalid payment asset address"))?;

		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_marketplace::Config>::WeightInfo::auction_sft(
				serial_number_ids.len() as u32,
			),
		))?;

		let caller: Runtime::AccountId = handle.context().caller.into();
		let listing_id = pallet_marketplace::Pallet::<Runtime>::do_auction(
			caller,
			tokens,
			payment_asset,
			reserve_price,
			duration.map(Into::into),
			Some(marketplace_id),
		)
		.map_err(|e| {
			revert(alloc::format!("Marketplace SFT: Dispatched call failed with error: {:?}", e))
		})?;
		let collection_id = H256::from_low_u64_be(collection_id as u64);
		log4(
			handle.code_address(),
			SELECTOR_LOG_AUCTION_OPEN_SFT,
			collection_id,
			H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
			H256::from_slice(&EvmDataWriter::new().write(reserve_price).build()),
			EvmDataWriter::new()
				.write(Address::from(handle.context().caller))
				.write(serial_number_ids)
				.write(marketplace_id)
				.build(),
		)
		.record(handle)?;

		// Build output.
		Ok(succeed([]))
	}

	fn bid(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(3, 32)?;
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
		let listing = match pallet_marketplace::Pallet::<Runtime>::get_listing_detail(listing_id) {
			Ok(Listing::Auction(listing)) => listing,
			_ => return Err(revert("NotForAuction")),
		};
		ensure!(amount <= u128::MAX.into(), revert("Marketplace: Expected amount <= 2^128"));
		let amount: Balance = amount.saturated_into();
		let origin = handle.context().caller;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_marketplace::Call::<Runtime>::bid { listing_id, amount },
		)?;

		let marketplace_id = listing.marketplace_id.unwrap_or_default();
		log4(
			handle.code_address(),
			SELECTOR_LOG_BID,
			handle.context().caller, //bidder
			H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
			H256::from_slice(&EvmDataWriter::new().write(amount).build()),
			EvmDataWriter::new().write(marketplace_id).build(),
		)
		.record(handle)?;

		Ok(succeed([]))
	}

	fn cancel_sale(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(2, 32)?;
		// Parse input.
		read_args!(handle, { listing_id: U256 });

		ensure!(
			listing_id <= u128::MAX.into(),
			revert("Marketplace: Expected listing id <= 2^128")
		);
		let listing_id: u128 = listing_id.saturated_into();

		let origin = handle.context().caller;
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let listing = pallet_marketplace::Pallet::<Runtime>::get_listing_detail(listing_id)
			.map_err(|_| revert("Marketplace: listing details not found"))?;

		let Ok((collection_id, serial_numbers)) = (match listing.clone() {
			Listing::FixedPrice(listing) => Self::split_listing_tokens(listing.tokens),
			Listing::Auction(listing) => Self::split_listing_tokens(listing.tokens),
		}) else {
			return Err(revert("Marketplace: Expected NFT tokens"));
		};

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_marketplace::Call::<Runtime>::cancel_sale { listing_id },
		)?;
		let collection_id = H256::from_low_u64_be(collection_id as u64);
		match listing {
			Listing::FixedPrice(sale) => {
				let marketplace_id = sale.marketplace_id.unwrap_or_default();
				log3(
					handle.code_address(),
					SELECTOR_LOG_FIXED_PRICE_SALE_CLOSE,
					collection_id,
					H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
					EvmDataWriter::new()
						.write(Address::from(handle.context().caller))
						.write(serial_numbers)
						.write(marketplace_id)
						.build(),
				)
				.record(handle)?;
			},
			Listing::Auction(auction) => {
				let marketplace_id = auction.marketplace_id.unwrap_or_default();
				log3(
					handle.code_address(),
					SELECTOR_LOG_AUCTION_CLOSE,
					collection_id,
					H256::from_slice(&EvmDataWriter::new().write(listing_id).build()),
					EvmDataWriter::new()
						.write(Address::from(handle.context().caller))
						.write(serial_numbers)
						.write(marketplace_id)
						.build(),
				)
				.record(handle)?;
			},
		}

		// Build output.
		Ok(succeed([]))
	}

	fn make_simple_offer_with_marketplace_id(
		handle: &mut impl PrecompileHandle,
	) -> EvmResult<PrecompileOutput> {
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
		ensure!(amount <= u128::MAX.into(), revert("Marketplace: Expected amount <= 2^128"));
		let amount: Balance = amount.saturated_into();
		let collection_id: CollectionUuid =
			<Runtime as ErcIdConversion<CollectionUuid>>::evm_id_to_runtime_id(
				collection_address,
				ERC721_PRECOMPILE_ADDRESS_PREFIX,
			)
			.ok_or_else(|| revert("Marketplace: Invalid collection address"))?;
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

		handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
			<Runtime as pallet_marketplace::Config>::WeightInfo::make_simple_offer(),
		))?;

		let caller: Runtime::AccountId = handle.context().caller.into(); // caller is the buyer
		let offer_id = pallet_marketplace::Pallet::<Runtime>::do_make_simple_offer(
			caller,
			token_id,
			amount,
			asset_id,
			Some(marketplace_id),
			None,
		)
		.map_err(|e| {
			revert(alloc::format!("Marketplace: Dispatched call failed with error: {:?}", e))
		})?;

		log3(
			handle.code_address(),
			SELECTOR_LOG_OFFER,
			H256::from_slice(&EvmDataWriter::new().write(offer_id).build()),
			handle.context().caller,
			EvmDataWriter::new()
				.write(collection_id)
				.write(serial_number)
				.write(marketplace_id)
				.build(),
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
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let offer = pallet_marketplace::Pallet::<Runtime>::get_offer_detail(offer_id)
			.map_err(|_| revert("Marketplace: Offer details not found"))?;

		let origin = handle.context().caller;
		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin.into()).into(),
			pallet_marketplace::Call::<Runtime>::cancel_offer { offer_id },
		)?;
		let (collection_id, serial_number) = offer.token_id;
		let offer_id = H256::from_low_u64_be(offer_id);
		let marketplace_id = offer.marketplace_id.unwrap_or_default();
		log3(
			handle.code_address(),
			SELECTOR_LOG_OFFER_CANCEL,
			offer_id,
			handle.context().caller,
			EvmDataWriter::new()
				.write(collection_id)
				.write(serial_number)
				.write(marketplace_id)
				.build(),
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
			.map_err(|_| revert("Marketplace: Offer details not found"))?;

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
		let marketplace_id = offer.marketplace_id.unwrap_or_default();
		log4(
			handle.code_address(),
			SELECTOR_LOG_OFFER_ACCEPT,
			offer_id,
			H256::from_slice(&EvmDataWriter::new().write(offer.amount).build()),
			handle.context().caller,
			EvmDataWriter::new()
				.write(collection_id)
				.write(serial_number)
				.write(marketplace_id)
				.build(),
		)
		.record(handle)?;

		Ok(succeed([]))
	}

	fn get_marketplace_account(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;
		// Parse input.
		read_args!(handle, { marketplace_id: U256 });

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		ensure!(
			marketplace_id <= u32::MAX.into(),
			revert("Marketplace: Expected marketplace id <= 2^32")
		);
		let marketplace_id: MarketplaceId = marketplace_id.saturated_into();
		let Some(marketplace_account) =
			pallet_marketplace::RegisteredMarketplaces::<Runtime>::get(marketplace_id)
		else {
			return Err(revert(
				"Marketplace: This MarketplaceId does not have a registered AccountId",
			));
		};
		let marketplace_account_h160: H160 = marketplace_account.account.into();
		Ok(succeed(EvmDataWriter::new().write(Address::from(marketplace_account_h160)).build()))
	}

	fn get_listing_from_id(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;
		// Parse input.
		read_args!(handle, { listing_id: U256 });
		ensure!(
			listing_id <= u128::MAX.into(),
			revert("Marketplace: Expected listing id <= 2^128")
		);
		let listing_id: u128 = listing_id.saturated_into();
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let listing = pallet_marketplace::Pallet::<Runtime>::get_listing_detail(listing_id)
			.map_err(|_| revert("Marketplace: listing details not found"))?;
		match listing {
			Listing::FixedPrice(listing) => {
				let (collection_id, serial_numbers) = Self::split_listing_tokens(listing.tokens)?;
				Ok(succeed(
					EvmDataWriter::new()
						.write::<Bytes>("fixed_price_listing_for_nft".as_bytes().into())
						.write::<u32>(collection_id)
						.write::<Vec<u32>>(serial_numbers)
						.write::<u128>(listing.fixed_price)
						.write::<u32>(listing.payment_asset)
						.build(),
				))
			},
			Listing::Auction(listing) => {
				let (collection_id, serial_numbers) = Self::split_listing_tokens(listing.tokens)?;
				Ok(succeed(
					EvmDataWriter::new()
						.write::<Bytes>("auction_listing_for_nft".as_bytes().into())
						.write::<u32>(collection_id)
						.write::<Vec<u32>>(serial_numbers)
						.write::<u128>(listing.reserve_price)
						.write::<u32>(listing.payment_asset)
						.build(),
				))
			},
		}
	}

	fn get_offer_from_id(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_log_costs_manual(1, 32)?;
		// Parse input.
		read_args!(handle, { offer_id: U256 });
		ensure!(offer_id <= u64::MAX.into(), revert("Marketplace: Expected offer_id <= 2^64"));
		let offer_id: OfferId = offer_id.saturated_into();

		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let offer = pallet_marketplace::Pallet::<Runtime>::get_offer_detail(offer_id)
			.map_err(|e| revert(alloc::format!("Marketplace: Offer details not found {:?}", e)))?;

		let (collection_id, serial_number) = offer.token_id;
		let buyer: H160 = offer.buyer.into();

		Ok(succeed(
			EvmDataWriter::new()
				.write::<u32>(collection_id)
				.write::<u32>(serial_number)
				.write::<u128>(offer.amount)
				.write::<Address>(Address::from(buyer))
				.build(),
		))
	}

	// Split the listing tokens into a collection_id and a list of Serial numbers
	fn split_listing_tokens(
		tokens: ListingTokens<Runtime>,
	) -> Result<(CollectionUuid, Vec<SerialNumber>), PrecompileFailure> {
		match tokens {
			ListingTokens::Nft(tokens) => {
				let collection_id = tokens.collection_id;
				let serial_numbers = tokens.serial_numbers.into_inner();
				Ok((collection_id, serial_numbers))
			},
			ListingTokens::Sft(tokens) => {
				let collection_id = tokens.collection_id;
				// let serial_numbers = tokens.serial_numbers.into_inner();
				let serial_numbers = tokens
					.serial_numbers
					.clone()
					.into_iter()
					.map(|(serial_number, _quantity)| serial_number)
					.collect();
				Ok((collection_id, serial_numbers))
			},
		}
	}
}
