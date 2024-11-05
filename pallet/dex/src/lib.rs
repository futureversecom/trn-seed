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
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::unused_unit)]
#![allow(clippy::collapsible_if)]
extern crate alloc;

pub use pallet::*;

use alloc::string::ToString;
use frame_support::{
	pallet_prelude::*,
	traits::{
		fungibles::{self, metadata::Inspect as MetadataInspect, Inspect, Mutate},
		tokens::{Fortitude, Precision, Preservation},
	},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use seed_pallet_common::CreateExt;
use seed_primitives::{AssetId, Balance};
use serde::{Deserialize, Serialize};
use sp_core::{H160, U256};
use sp_runtime::{
	traits::{AccountIdConversion, Zero},
	ArithmeticError, DispatchError, FixedU128, RuntimeDebug, SaturatedConversion,
};
use sp_std::{cmp::min, convert::TryInto, prelude::*, vec};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod types;
use types::SafeMath;
pub use types::TradingPair;
pub mod weights;
pub use weights::WeightInfo;
pub type Price = FixedU128;
pub type ExchangeRate = FixedU128;
pub type Ratio = FixedU128;
pub type Rate = FixedU128;

/// Status for TradingPair
#[derive(
	Clone,
	Copy,
	Encode,
	Decode,
	RuntimeDebug,
	PartialEq,
	Eq,
	MaxEncodedLen,
	TypeInfo,
	Serialize,
	Deserialize,
)]
pub enum TradingPairStatus {
	/// Default status,
	/// can withdraw liquidity, re-enable and list this trading pair.
	NotEnabled,
	/// TradingPair is Enabled,
	/// can add/remove liquidity, trading and disable this trading pair.
	Enabled,
}

impl Default for TradingPairStatus {
	fn default() -> Self {
		Self::NotEnabled
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::config]
	pub trait Config: frame_system::Config
	where
		<Self as frame_system::Config>::AccountId: From<H160>,
	{
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Trading fee rate
		/// The first item of the tuple is the numerator of the fee rate, second
		/// item is the denominator, fee_rate = numerator / denominator,
		/// use (u32, u32) over `Rate` type to minimize internal division
		/// operation.
		#[pallet::constant]
		type GetExchangeFee: Get<(u32, u32)>;

		/// The limit for length of trading path
		#[pallet::constant]
		type TradingPathLimit: Get<u32>;

		/// The DEX's burn id, to provide for a redundant, unredeemable minter/burner address.
		#[pallet::constant]
		type DEXBurnPalletId: Get<PalletId>;

		/// Liquidity pair default token decimals
		#[pallet::constant]
		type LPTokenDecimals: Get<u8>;

		/// The default FeeTo account
		#[pallet::constant]
		type DefaultFeeTo: Get<Option<PalletId>>;

		/// Weight information for the extrinsic call in this module.
		type WeightInfo: WeightInfo;

		/// Currency implementation to deal with assets on DEX.
		type MultiCurrency: CreateExt<AccountId = Self::AccountId>
			+ fungibles::Inspect<Self::AccountId, AssetId = AssetId>
			+ fungibles::metadata::Inspect<Self::AccountId>
			+ fungibles::Mutate<Self::AccountId, Balance = Balance>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Trading pair must be in Enabled status
		MustBeEnabled,
		/// Trading pair must be in NotEnabled status
		MustBeNotEnabled,
		/// Insufficient input amount
		InsufficientInputAmount,
		/// Must provide non-zero amount of liquidity
		InvalidInputAmounts,
		/// Insufficent amount
		InsufficientAmount,
		/// Insufficient asset_a liquidity amount
		InsufficientAmountA,
		/// Insufficient asset_b liquidity amount
		InsufficientAmountB,
		/// Insufficient liquidity burnt
		InsufficientLiquidityBurnt,
		/// Insufficient withdraw amount for token A
		InsufficientWithdrawnAmountA,
		/// Insufficient withdraw amount for token B
		InsufficientWithdrawnAmountB,
		/// Insufficient output amount
		InsufficientOutputAmount,
		/// The increment of liquidity is invalid
		InvalidLiquidityIncrement,
		/// Invalid constant product K
		InvalidConstantProduct,
		// Identical token address
		IdenticalTokenAddress,
		/// Invalid Asset id
		InvalidAssetId,
		/// Invalid trading path length
		InvalidTradingPathLength,
		/// Target amount is less to min_target_amount
		InsufficientTargetAmount,
		/// Supply amount is more than max_supply_amount
		ExcessiveSupplyAmount,
		/// Liquidity is not enough
		InsufficientLiquidity,
		/// The supply amount is zero
		ZeroSupplyAmount,
		/// The target amount is zero
		ZeroTargetAmount,
		/// The Liquidity Provider token does not exist
		LiquidityProviderTokenNotCreated,
		/// The deadline has been missed
		ExpiredDeadline,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config>
	where
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		/// Set FeeTo account success. \[fee_to]
		FeeToSet(Option<T::AccountId>),
		/// Add provision success. \[who, asset_id_0, contribution_0,
		/// asset_id_1, contribution_1\]
		AddProvision(T::AccountId, AssetId, Balance, AssetId, Balance),
		/// Add liquidity success. \[who, asset_id_0, reserve_0_increment,
		/// asset_id_1, reserve_1_increment, share_increment, to\]
		AddLiquidity(T::AccountId, AssetId, Balance, AssetId, Balance, Balance, T::AccountId),
		/// Remove liquidity from the trading pool success. \[who,
		/// asset_id_0, reserve_0_decrement, asset_id_1, reserve_1_decrement,
		/// share_decrement, to\]
		RemoveLiquidity(T::AccountId, AssetId, Balance, AssetId, Balance, Balance, T::AccountId),
		/// Use supply Asset to swap target Asset. \[trader, trading_path,
		/// supply_Asset_amount, target_Asset_amount, to\]
		Swap(T::AccountId, Vec<AssetId>, Balance, Balance, T::AccountId),
		/// Enable trading pair. \[trading_pair\]
		EnableTradingPair(TradingPair),
		/// Disable trading pair. \[trading_pair\]
		DisableTradingPair(TradingPair),
		/// Provisioning trading pair convert to Enabled. \[trading_pair,
		/// pool_0_amount, pool_1_amount, total_share_amount\]
		ProvisioningToEnabled(TradingPair, Balance, Balance, Balance),
	}

	#[pallet::type_value]
	pub fn DefaultFeeTo<T: Config>() -> Option<T::AccountId>
	where
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		T::DefaultFeeTo::get().map(|v| v.into_account_truncating())
	}

	/// FeeTo account where network fees are deposited
	#[pallet::storage]
	#[pallet::getter(fn fee_to)]
	pub type FeeTo<T: Config> = StorageValue<_, Option<T::AccountId>, ValueQuery, DefaultFeeTo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn lp_token_id)]
	pub type TradingPairLPToken<T: Config> =
		StorageMap<_, Twox64Concat, TradingPair, Option<AssetId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn liquidity_pool)]
	pub type LiquidityPool<T: Config> =
		StorageMap<_, Twox64Concat, TradingPair, (Balance, Balance), ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn liquidity_pool_last_k)]
	pub type LiquidityPoolLastK<T: Config> = StorageMap<_, Twox64Concat, AssetId, U256, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn trading_pair_statuses)]
	pub type TradingPairStatuses<T: Config> =
		StorageMap<_, Twox64Concat, TradingPair, TradingPairStatus, ValueQuery>;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> where
		<T as frame_system::Config>::AccountId: From<H160>
	{
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		/// Set the `FeeTo` account. This operation requires root access.
		/// - note: analogous to Uniswapv2 `setFeeTo`
		///
		/// - `fee_to`: the new account or None assigned to FeeTo.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_fee_to())]
		#[transactional]
		pub fn set_fee_to(
			origin: OriginFor<T>,
			fee_to: Option<T::AccountId>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			FeeTo::<T>::put(&fee_to);

			Self::deposit_event(Event::FeeToSet(fee_to));

			Ok(().into())
		}

		/// Trading with DEX, swap with exact supply amount. Specify your input; retrieve variable
		/// output.
		/// - note: analogous to Uniswapv2 `swapExactTokensForTokens`
		///
		/// - `path`: trading path.
		/// - `amount_in`: exact supply amount.
		/// - `amount_out_min`: acceptable minimum target amount.
		/// - `to`: The recipient of the swapped token asset. The caller is the default recipient if
		///   it is set to None.
		/// - `deadline`: The deadline of executing this extrinsic. The deadline won't be checked if
		///   it is set to None
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::swap_with_exact_supply())]
		#[transactional]
		pub fn swap_with_exact_supply(
			origin: OriginFor<T>,
			#[pallet::compact] amount_in: Balance,
			#[pallet::compact] amount_out_min: Balance,
			path: Vec<AssetId>,
			to: Option<T::AccountId>,
			deadline: Option<BlockNumberFor<T>>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			Self::do_swap_with_exact_supply(
				&who,
				amount_in,
				amount_out_min,
				&path,
				to.unwrap_or(who.clone()), /* set the caller as token recipient if
				                            * it is None */
				deadline,
			)?;
			Ok(().into())
		}

		/// Trading with DEX, swap with exact target amount. Specify your output; supply variable
		/// input.
		/// - note: analogous to Uniswapv2 `swapTokensForExactTokens`
		///
		/// - `amount_out`: exact target amount.
		/// - `amount_in_max`: acceptable maximum supply amount.
		/// - `path`: trading path.
		/// - `to`: The recipient of the swapped token asset. The caller is the default recipient if
		///   it is set to None.
		/// - `deadline`: The deadline of executing this extrinsic. The deadline won't be checked if
		///   it is set to None
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::swap_with_exact_target())]
		#[transactional]
		pub fn swap_with_exact_target(
			origin: OriginFor<T>,
			#[pallet::compact] amount_out: Balance,
			#[pallet::compact] amount_in_max: Balance,
			path: Vec<AssetId>,
			to: Option<T::AccountId>,
			deadline: Option<BlockNumberFor<T>>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			Self::do_swap_with_exact_target(
				&who,
				amount_out,
				amount_in_max,
				&path,
				to.unwrap_or(who.clone()), /* set the caller as token recipient if
				                            * it is None */
				deadline,
			)?;
			Ok(().into())
		}

		/// Add liquidity to Enabled trading pair, or add provision to Provisioning trading pair.
		/// - Add liquidity success will issue shares in current price which decided by the
		///   liquidity scale. Shares are temporarily not
		/// allowed to transfer and trade, it represents the proportion of
		/// assets in liquidity pool.
		/// - Add provision success will record the provision, issue shares to caller in the initial
		///   price when trading pair convert to Enabled.
		/// - Creates and enables TradingPair LP token if it does not exist for trading pair.
		/// - Fails to add liquidity for `NotEnabled` trading pair.
		///
		/// - `token_a`: Asset id A.
		/// - `token_b`: Asset id B.
		/// - `amount_a_desired`: amount a desired to add.
		/// - `amount_b_desired`: amount b desired to add.
		/// - `amount_a_min`: amount a minimum willing to add.
		/// - `amount_b_min`: amount b minimum willing to add.
		/// - `to`: The recipient of the LP token. The caller is the default recipient if it is set
		///   to None.
		/// - `deadline`: The deadline of executing this extrinsic. The deadline won't be checked if
		///   it is set to None
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::add_liquidity())]
		#[transactional]
		pub fn add_liquidity(
			origin: OriginFor<T>,
			token_a: AssetId,
			token_b: AssetId,
			#[pallet::compact] amount_a_desired: Balance,
			#[pallet::compact] amount_b_desired: Balance,
			#[pallet::compact] amount_a_min: Balance,
			#[pallet::compact] amount_b_min: Balance,
			to: Option<T::AccountId>,
			deadline: Option<BlockNumberFor<T>>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			Self::do_add_liquidity(
				&who,
				token_a,
				token_b,
				amount_a_desired,
				amount_b_desired,
				amount_a_min,
				amount_b_min,
				to.unwrap_or(who.clone()), /* set the caller as LP recipient if
				                            * it is None */
				deadline,
			)?;
			Ok(().into())
		}

		/// Remove liquidity from specific liquidity pool in the form of burning
		/// shares, and withdrawing currencies in trading pairs from liquidity
		/// pool in proportion, and withdraw liquidity incentive interest.
		/// - note: liquidity can still be withdrawn for `NotEnabled` trading pairs.
		///
		/// - `token_a`: Asset id A.
		/// - `token_b`: Asset id B.
		/// - `liquidity`: liquidity amount to remove.
		/// - `amount_a_min`: minimum amount of asset A to be withdrawn from LP token.
		/// - `amount_b_min`: minimum amount of asset B to be withdrawn from LP token.
		/// - `to`: The recipient of the withdrawn token assets. The caller is the default recipient
		///   if it is set to None.
		/// - `deadline`: The deadline of executing this extrinsic. The deadline won't be checked if
		///   it is set to None
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::remove_liquidity())]
		#[transactional]
		pub fn remove_liquidity(
			origin: OriginFor<T>,
			token_a: AssetId,
			token_b: AssetId,
			#[pallet::compact] liquidity: Balance,
			#[pallet::compact] amount_a_min: Balance,
			#[pallet::compact] amount_b_min: Balance,
			to: Option<T::AccountId>,
			deadline: Option<BlockNumberFor<T>>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			Self::do_remove_liquidity(
				&who,
				token_a,
				token_b,
				liquidity,
				amount_a_min,
				amount_b_min,
				to.unwrap_or(who.clone()), /* set the caller as token recipient if
				                            * it is None */
				deadline,
			)?;
			Ok(().into())
		}

		/// Re enable a `NotEnabled` trading pair.
		/// - Requires LP token to be created and in the `NotEnabled` status
		/// - Only root can enable a disabled trading pair
		///
		/// - `token_a`: Asset id A.
		/// - `token_b`: Asset id B.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::reenable_trading_pair())]
		#[transactional]
		pub fn reenable_trading_pair(
			origin: OriginFor<T>,
			token_a: AssetId,
			token_b: AssetId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let trading_pair = TradingPair::new(token_a, token_b);

			ensure!(
				Self::lp_token_id(trading_pair).is_some(),
				Error::<T>::LiquidityProviderTokenNotCreated
			);

			match Self::trading_pair_statuses(trading_pair) {
				TradingPairStatus::Enabled => return Err(Error::<T>::MustBeNotEnabled.into()),
				// will enabled Disabled trading_pair
				TradingPairStatus::NotEnabled => {
					TradingPairStatuses::<T>::insert(trading_pair, TradingPairStatus::Enabled);
					Self::deposit_event(Event::EnableTradingPair(trading_pair));
				},
			};
			Ok(().into())
		}

		/// Disable an `Enabled` trading pair.
		/// - Requires LP token to be created and in the `Enabled` status
		/// - Only root can disable trading pair
		///
		/// - `token_a`: Asset id A.
		/// - `token_b`: Asset id B.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::disable_trading_pair())]
		#[transactional]
		pub fn disable_trading_pair(
			origin: OriginFor<T>,
			token_a: AssetId,
			token_b: AssetId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let trading_pair = TradingPair::new(token_a, token_b);

			ensure!(
				Self::lp_token_id(trading_pair).is_some(),
				Error::<T>::LiquidityProviderTokenNotCreated
			);

			match Self::trading_pair_statuses(trading_pair) {
				// will disable Enabled trading_pair
				TradingPairStatus::Enabled => {
					TradingPairStatuses::<T>::insert(trading_pair, TradingPairStatus::NotEnabled);
					Self::deposit_event(Event::DisableTradingPair(trading_pair));
				},
				TradingPairStatus::NotEnabled => return Err(Error::<T>::MustBeEnabled.into()),
			};
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<H160>,
{
	pub fn burn_account_id() -> T::AccountId {
		T::DEXBurnPalletId::get().into_account_truncating()
	}

	/// Given some amount of an asset and pair reserves, returns an equivalent amount of the other
	/// asset
	pub fn quote(
		amount_a: U256,
		reserve_a: u128,
		reserve_b: u128,
	) -> sp_std::result::Result<U256, DispatchError> {
		// require(amountA > 0, "UniswapV2Library: INSUFFICIENT_AMOUNT");
		// require(reserveA > 0 && reserveB > 0, "UniswapV2Library: INSUFFICIENT_LIQUIDITY");
		// amountB = amountA.mul(reserveB) / reserveA;
		ensure!(amount_a.gt(&U256::zero()), Error::<T>::InsufficientAmount);
		ensure!(reserve_a > 0_u128 && reserve_b > 0_u128, Error::<T>::InsufficientLiquidity);
		let amount_b = amount_a.mul(U256::from(reserve_b))?.div(U256::from(reserve_a))?;
		Ok(amount_b)
	}

	fn create_lp_token(trading_pair: &TradingPair) -> Result<AssetId, DispatchError> {
		let symbol_a_truncated: Vec<u8> =
			T::MultiCurrency::symbol(trading_pair.0).into_iter().take(20).collect();
		let symbol_b_truncated: Vec<u8> =
			T::MultiCurrency::symbol(trading_pair.1).into_iter().take(20).collect();

		// name: b"LP " + symbol_a_truncated + b" " + symbol_b_truncated
		let mut lp_token_name =
			Vec::with_capacity(22 + symbol_a_truncated.len() + symbol_b_truncated.len());
		lp_token_name.extend_from_slice(b"LP ");
		lp_token_name.extend_from_slice(&symbol_a_truncated);
		lp_token_name.push(b' ');
		lp_token_name.extend_from_slice(&symbol_b_truncated);

		let token_a_bytes = trading_pair.0.to_string().into_bytes();
		let token_b_bytes = trading_pair.1.to_string().into_bytes();

		// symbol: b"LP-" + token_a_bytes + b"-" + token_b_bytes
		let mut lp_token_symbol =
			Vec::with_capacity(3 + token_a_bytes.len() + 1 + token_b_bytes.len());
		lp_token_symbol.extend_from_slice(b"LP-");
		lp_token_symbol.extend_from_slice(&token_a_bytes);
		lp_token_symbol.push(b'-');
		lp_token_symbol.extend_from_slice(&token_b_bytes);

		let lp_asset_id = T::MultiCurrency::create_with_metadata(
			&trading_pair.pool_address(),
			lp_token_name,
			lp_token_symbol,
			T::LPTokenDecimals::get(),
			None,
		)?;
		TradingPairLPToken::<T>::insert(trading_pair, Some(lp_asset_id));
		TradingPairStatuses::<T>::insert(trading_pair, TradingPairStatus::Enabled);
		Ok(lp_asset_id)
	}

	pub fn do_add_liquidity(
		who: &T::AccountId,
		token_a: AssetId,
		token_b: AssetId,
		amount_a_desired: Balance,
		amount_b_desired: Balance,
		amount_a_min: Balance,
		amount_b_min: Balance,
		to: T::AccountId,
		deadline: Option<BlockNumberFor<T>>,
	) -> sp_std::result::Result<(Balance, Balance, Balance), DispatchError> {
		const MINIMUM_LIQUIDITY_AMOUNT: u128 = 1000_u128; // for 18 decimals -> 1000; hence for 6 decimals -> 10

		ensure!(token_a != token_b, Error::<T>::IdenticalTokenAddress);
		ensure!(amount_a_desired > 0 && amount_b_desired > 0, Error::<T>::InvalidInputAmounts);

		// Check if the deadline is met when the `deadline` parameter is not None
		if let Some(deadline_block) = deadline {
			let current_block_number = frame_system::Pallet::<T>::block_number();
			ensure!(deadline_block >= current_block_number, Error::<T>::ExpiredDeadline);
		}

		let trading_pair = TradingPair::new(token_a, token_b);
		// create trading pair & lp token if non-existent
		let lp_share_asset_id = match Self::lp_token_id(trading_pair) {
			Some(lp_id) => lp_id,
			None => Self::create_lp_token(&trading_pair)?,
		};

		ensure!(
			matches!(Self::trading_pair_statuses(trading_pair), TradingPairStatus::Enabled),
			Error::<T>::MustBeEnabled,
		);

		// match trading-pair to inputs - to match reserves in liquidity pool
		let (
			token_a,
			token_b,
			amount_a_desired,
			amount_b_desired,
			amount_a_min,
			amount_b_min,
			is_swapped,
		) = if token_a == trading_pair.0 {
			(
				token_a,
				token_b,
				U256::from(amount_a_desired),
				U256::from(amount_b_desired),
				U256::from(amount_a_min),
				U256::from(amount_b_min),
				false,
			)
		} else {
			(
				token_b,
				token_a,
				U256::from(amount_b_desired),
				U256::from(amount_a_desired),
				U256::from(amount_b_min),
				U256::from(amount_a_min),
				true,
			)
		};

		let (reserve_a, reserve_b) = LiquidityPool::<T>::get(trading_pair);

		// _addLiquidity func in uniswap-v2-router
		let (amount_a, amount_b) = if reserve_a.is_zero() && reserve_b.is_zero() {
			(amount_a_desired, amount_b_desired)
		} else {
			// let amount_b_optimal = UniswapV2Library.quote(amountADesired, reserveA,
			// reserveB);
			let amount_b_optimal = Self::quote(amount_a_desired, reserve_a, reserve_b)?;
			if amount_b_optimal <= amount_b_desired {
				ensure!(amount_b_optimal >= amount_b_min, Error::<T>::InsufficientAmountB);
				(amount_a_desired, amount_b_optimal)
			} else {
				// uint256 amountAOptimal = UniswapV2Library.quote(amountBDesired, reserveB,
				// reserveA);
				let amount_a_optimal = Self::quote(amount_b_desired, reserve_b, reserve_a)?;
				ensure!(amount_a_optimal <= amount_a_desired, Error::<T>::InsufficientAmount); // TODO - verify assert
				ensure!(amount_a_optimal >= amount_a_min, Error::<T>::InsufficientAmountA);
				(amount_a_optimal, amount_b_desired)
			}
		};

		let pool_address = trading_pair.pool_address();
		T::MultiCurrency::transfer(
			token_a,
			who,
			&pool_address,
			amount_a.saturated_into(),
			Preservation::Expendable,
		)?;
		T::MultiCurrency::transfer(
			token_b,
			who,
			&pool_address,
			amount_b.saturated_into(),
			Preservation::Expendable,
		)?;

		let balance_0 = T::MultiCurrency::balance(token_a, &pool_address);
		let balance_1 = T::MultiCurrency::balance(token_b, &pool_address);
		let amount_0 = balance_0.sub(reserve_a)?;
		let amount_1 = balance_1.sub(reserve_b)?;

		let total_supply = U256::from(T::MultiCurrency::total_issuance(lp_share_asset_id));

		let liquidity: Balance = if total_supply.is_zero() {
			// liquidity = Math.sqrt(amount0.mul(amount1)).sub(MINIMUM_LIQUIDITY);
			let liquidity = (U256::from(amount_0).mul(U256::from(amount_1)))?
				.integer_sqrt()
				.sub(U256::from(MINIMUM_LIQUIDITY_AMOUNT))?
				.saturated_into();
			// mint 0 address MINIMUM_LIQUIDITY_AMOUNT - required to increase total issuance
			T::MultiCurrency::mint_into(
				lp_share_asset_id,
				&Self::burn_account_id(),
				MINIMUM_LIQUIDITY_AMOUNT,
			)?;
			liquidity
		} else {
			// liquidity = Math.min(amount0.mul(_totalSupply) / _reserve0,
			// amount1.mul(_totalSupply) / _reserve1);
			min(
				U256::from(amount_0).mul(total_supply)?.div(U256::from(reserve_a))?,
				U256::from(amount_1).mul(total_supply)?.div(U256::from(reserve_b))?,
			)
			.saturated_into()
		};

		ensure!(!liquidity.is_zero(), Error::<T>::InvalidLiquidityIncrement);

		// mint lp tokens to the LP to
		T::MultiCurrency::mint_into(lp_share_asset_id, &to, liquidity)?;

		LiquidityPool::<T>::try_mutate(trading_pair, |(reserve_a, reserve_b)| -> DispatchResult {
			// update reserves
			*reserve_a = balance_0;
			*reserve_b = balance_1;

			Self::deposit_event(Event::AddLiquidity(
				who.clone(),
				trading_pair.0,
				amount_0,
				trading_pair.1,
				amount_1,
				liquidity,
				to,
			));
			Ok(())
		})?;

		// update the k_last value if FeeTo is set
		if FeeTo::<T>::get().is_some() {
			let (reserve_a, reserve_b) = LiquidityPool::<T>::get(trading_pair);
			let _ = LiquidityPoolLastK::<T>::try_mutate(lp_share_asset_id, |k| -> DispatchResult {
				// update k to the product of the updated reserve_a and reserve_b
				*k = U256::from(reserve_a).mul(U256::from(reserve_b))?;
				Ok(())
			});
		}

		if is_swapped {
			Ok((amount_1, amount_0, liquidity))
		} else {
			Ok((amount_0, amount_1, liquidity))
		}
	}

	#[transactional]
	pub fn do_remove_liquidity(
		who: &T::AccountId,
		token_a: AssetId,
		token_b: AssetId,
		liquidity: Balance,
		amount_a_min: Balance,
		amount_b_min: Balance,
		to: T::AccountId,
		deadline: Option<BlockNumberFor<T>>,
	) -> sp_std::result::Result<(Balance, Balance), DispatchError> {
		// Check if the deadline is met when the `deadline` parameter is not None
		if let Some(deadline_block) = deadline {
			let current_block_number = frame_system::Pallet::<T>::block_number();
			ensure!(deadline_block >= current_block_number, Error::<T>::ExpiredDeadline);
		}

		let trading_pair = TradingPair::new(token_a, token_b);
		let lp_share_asset_id =
			Self::lp_token_id(trading_pair).ok_or(Error::<T>::InvalidAssetId)?;

		ensure!(token_a != token_b, Error::<T>::IdenticalTokenAddress);

		// transfer lp tokens to dex
		let pool_address = trading_pair.pool_address();
		T::MultiCurrency::transfer(
			lp_share_asset_id,
			who,
			&pool_address,
			liquidity,
			Preservation::Expendable,
		)?;

		// match trading-pair to inputs - to match reserves in liquidity pool
		let (token_a, token_b, amount_a_min, amount_b_min, is_swapped) =
			if token_a == trading_pair.0 {
				(token_a, token_b, amount_a_min, amount_b_min, false)
			} else {
				(token_b, token_a, amount_b_min, amount_a_min, true)
			};

		let mut balance_0 = T::MultiCurrency::balance(token_a, &pool_address);
		let mut balance_1 = T::MultiCurrency::balance(token_b, &pool_address);
		let liquidity = T::MultiCurrency::balance(lp_share_asset_id, &pool_address);

		let total_supply = T::MultiCurrency::total_issuance(lp_share_asset_id);

		// amount0 = liquidity.mul(balance0) / _totalSupply;
		let amount_0 = U256::from(liquidity)
			.mul(U256::from(balance_0))?
			.div(U256::from(total_supply))?
			.saturated_into();

		// amount1 = liquidity.mul(balance1) / _totalSupply;
		let amount_1 = U256::from(liquidity)
			.mul(U256::from(balance_1))?
			.div(U256::from(total_supply))?
			.saturated_into();

		ensure!(amount_0 > 0 && amount_1 > 0, Error::<T>::InsufficientLiquidityBurnt);
		ensure!(amount_0 >= amount_a_min, Error::<T>::InsufficientWithdrawnAmountA);
		ensure!(amount_1 >= amount_b_min, Error::<T>::InsufficientWithdrawnAmountB);

		T::MultiCurrency::burn_from(
			lp_share_asset_id,
			&pool_address,
			liquidity,
			Precision::Exact,
			Fortitude::Polite,
		)?;
		T::MultiCurrency::transfer(
			token_a,
			&pool_address,
			&to,
			amount_0,
			Preservation::Expendable,
		)?;
		T::MultiCurrency::transfer(
			token_b,
			&pool_address,
			&to,
			amount_1,
			Preservation::Expendable,
		)?;

		balance_0 = T::MultiCurrency::balance(token_a, &pool_address);
		balance_1 = T::MultiCurrency::balance(token_b, &pool_address);

		LiquidityPool::<T>::try_mutate(trading_pair, |(reserve_0, reserve_1)| -> DispatchResult {
			*reserve_0 = balance_0;
			*reserve_1 = balance_1;

			Self::deposit_event(Event::RemoveLiquidity(
				who.clone(),
				trading_pair.0,
				amount_0,
				trading_pair.1,
				amount_1,
				liquidity,
				to,
			));
			Ok(())
		})?;

		// update the k_last value if FeeTo is set
		if FeeTo::<T>::get().is_some() {
			let (reserve_a, reserve_b) = LiquidityPool::<T>::get(trading_pair);
			let _ = LiquidityPoolLastK::<T>::try_mutate(lp_share_asset_id, |k| -> DispatchResult {
				// update k to the product of the updated reserve_a and reserve_b
				*k = U256::from(reserve_a).mul(U256::from(reserve_b))?;
				Ok(())
			});
		}

		if is_swapped {
			Ok((amount_1, amount_0))
		} else {
			Ok((amount_0, amount_1))
		}
	}

	pub fn get_lp_token_id(
		token_a: AssetId,
		token_b: AssetId,
	) -> sp_std::result::Result<AssetId, DispatchError> {
		let trading_pair = TradingPair::new(token_a, token_b);
		Self::lp_token_id(trading_pair).ok_or(Error::<T>::InvalidAssetId.into())
	}

	pub fn get_liquidity(token_a: AssetId, token_b: AssetId) -> (Balance, Balance) {
		let trading_pair = TradingPair::new(token_a, token_b);
		let (reserve_0, reserve_1) = Self::liquidity_pool(trading_pair);
		if token_a == trading_pair.0 {
			(reserve_0, reserve_1)
		} else {
			(reserve_1, reserve_0)
		}
	}

	pub fn get_trading_pair_status(token_a: AssetId, token_b: AssetId) -> TradingPairStatus {
		let trading_pair = TradingPair::new(token_a, token_b);
		Self::trading_pair_statuses(trading_pair)
	}

	/// Given an input amount of an asset and pair reserves, returns the maximum output amount of
	/// the other asset
	pub fn get_amount_out(
		amount_in: Balance,
		reserve_in: Balance,
		reserve_out: Balance,
	) -> sp_std::result::Result<Balance, DispatchError> {
		ensure!(amount_in > 0, Error::<T>::InsufficientInputAmount);
		ensure!(reserve_in > 0 && reserve_out > 0, Error::<T>::InsufficientLiquidity);

		// uniswapv2 getAmountOut code ⬇︎
		// uint256 amountInWithFee = amountIn.mul(997);
		// uint256 numerator = amountInWithFee.mul(reserveOut);
		// uint256 denominator = reserveIn.mul(1000).add(amountInWithFee);
		// amountOut = numerator / denominator;

		let (fee_numerator, fee_denominator) = T::GetExchangeFee::get(); // 3 / 1000 = 0.3%

		let amount_in_with_fee =
			U256::from(amount_in).mul(U256::from(fee_denominator.sub(fee_numerator)?))?;
		let numerator = amount_in_with_fee.mul(U256::from(reserve_out))?;
		let denominator = U256::from(reserve_in)
			.mul(U256::from(fee_denominator))?
			.add(amount_in_with_fee)?;
		let amount_out = (numerator.div(denominator)?).saturated_into();

		Ok(amount_out)
	}

	/// Get how much supply amount will be paid for specific target amount.
	pub fn get_amount_in(
		amount_out: Balance,
		reserve_in: Balance,
		reserve_out: Balance,
	) -> sp_std::result::Result<Balance, DispatchError> {
		ensure!(amount_out > 0, Error::<T>::InsufficientOutputAmount);
		ensure!(reserve_in > 0 && reserve_out > 0, Error::<T>::InsufficientLiquidity);

		// uint256 numerator = reserveIn.mul(amountOut).mul(1000);
		// uint256 denominator = reserveOut.sub(amountOut).mul(997);
		// amountIn = (numerator / denominator).add(1);

		let (fee_numerator, fee_denominator) = T::GetExchangeFee::get(); // 3 / 1000 = 0.3%
		let numerator = U256::from(reserve_in)
			.mul(U256::from(amount_out))?
			.mul(U256::from(fee_denominator))?;
		let denominator = U256::from(reserve_out)
			.sub(U256::from(amount_out))?
			.mul(U256::from(fee_denominator.sub(fee_numerator)?))?;
		let amount_in = numerator.div(denominator)?.add(U256::from(1u32))?.saturated_into();

		Ok(amount_in)
	}

	pub fn get_amounts_out(
		amount_in: Balance,
		path: &[AssetId],
	) -> sp_std::result::Result<Vec<Balance>, DispatchError> {
		let path_length = path.len();
		ensure!(
			path_length >= 2 && path_length <= T::TradingPathLimit::get().saturated_into(),
			Error::<T>::InvalidTradingPathLength
		);
		let mut amounts: Vec<Balance> = vec![Zero::zero(); path_length];
		amounts[0] = amount_in;

		let mut i: usize = 0;
		while i < path_length - 1 {
			// trading pair in path must be enabled
			ensure!(
				matches!(
					Self::trading_pair_statuses(TradingPair::new(path[i], path[i + 1])),
					TradingPairStatus::Enabled
				),
				Error::<T>::MustBeEnabled
			);

			let (reserve_in, reserve_out) = Self::get_liquidity(path[i], path[i + 1]);

			// might not need this check - as sufficient checks occur when adding/removing liquidity
			ensure!(
				!reserve_in.is_zero() && !reserve_out.is_zero(),
				Error::<T>::InsufficientLiquidity
			);

			let amount_out = Self::get_amount_out(amounts[i], reserve_in, reserve_out)?;
			ensure!(!amount_out.is_zero(), Error::<T>::ZeroTargetAmount);
			amounts[i + 1] = amount_out;

			i += 1;
		}

		Ok(amounts)
	}

	pub fn get_amounts_in(
		amount_out: Balance,
		path: &[AssetId],
	) -> sp_std::result::Result<Vec<Balance>, DispatchError> {
		let path_length = path.len();
		ensure!(
			path_length >= 2 && path_length <= T::TradingPathLimit::get().saturated_into(),
			Error::<T>::InvalidTradingPathLength
		);
		let mut amounts: Vec<Balance> = vec![Zero::zero(); path_length];
		amounts[path_length - 1] = amount_out;

		let mut i: usize = path_length - 1;
		while i > 0 {
			// trading pair in path must be enabled
			ensure!(
				matches!(
					Self::trading_pair_statuses(TradingPair::new(path[i - 1], path[i])),
					TradingPairStatus::Enabled
				),
				Error::<T>::MustBeEnabled
			);

			let (reserve_in, reserve_out) = Self::get_liquidity(path[i - 1], path[i]);

			// might not need this check - as sufficient checks occur when adding/removing liquidity
			ensure!(
				!reserve_in.is_zero() && !reserve_out.is_zero(),
				Error::<T>::InsufficientLiquidity
			);

			let amount_in = Self::get_amount_in(amounts[i], reserve_in, reserve_out)?;
			ensure!(!amount_in.is_zero(), Error::<T>::ZeroSupplyAmount);
			amounts[i - 1] = amount_in;

			i -= 1;
		}

		Ok(amounts)
	}

	// Uniswapv2 `_swap` implementation in rust
	// TODO: may need re-entrancy lock for this function
	fn _swap(
		amounts: &[Balance],
		path: &[AssetId],
		to: &T::AccountId,
	) -> sp_std::result::Result<Vec<(Balance, Balance, Balance, Balance)>, DispatchError> {
		let mut i: usize = 0;
		// build the result vec for precompile events
		let mut res = Vec::new();
		while i < path.len() - 1 {
			let (input, output) = (path[i], path[i + 1]);
			let amount_out = amounts[i + 1];

			ensure!(input != output, Error::<T>::IdenticalTokenAddress);

			let trading_pair = TradingPair::new(input, output);

			// (uint amount0Out, uint amount1Out) = input == token0 ? (uint(0), amountOut) :
			// (amountOut, uint(0))
			let (amount_0_out, amount_1_out) =
				if input == trading_pair.0 { (0, amount_out) } else { (amount_out, 0) };

			// address to = i < path.length - 2 ? UniswapV2Library.pairFor(factory, output, path[i +
			// 2]) : _to;
			let to = if i < path.len() - 2 {
				TradingPair::new(output, path[i + 2]).pool_address()
			} else {
				to.clone()
			};

			let pool_address = trading_pair.pool_address();

			// IUniswapV2Pair(UniswapV2Library.pairFor(factory, input, output)).swap(amount0Out,
			// amount1Out, to, new bytes(0));

			ensure!(amount_0_out > 0 || amount_1_out > 0, Error::<T>::InsufficientOutputAmount);

			let (reserve_0, reserve_1) = LiquidityPool::<T>::get(trading_pair);

			ensure!(
				amount_0_out < reserve_0 && amount_1_out < reserve_1,
				Error::<T>::InsufficientLiquidity
			);

			// require(to != _token0 && to != _token1, "UniswapV2: INVALID_TO");
			// ^ dont need this check as AssetId is different to AccountId

			if amount_0_out > 0 {
				// optimistically transfer tokens
				T::MultiCurrency::transfer(
					trading_pair.0,
					&pool_address,
					&to,
					amount_0_out,
					Preservation::Expendable,
				)?;
			}
			if amount_1_out > 0 {
				// optimistically transfer tokens
				T::MultiCurrency::transfer(
					trading_pair.1,
					&pool_address,
					&to,
					amount_1_out,
					Preservation::Expendable,
				)?;
			}

			let balance_0 = T::MultiCurrency::balance(trading_pair.0, &pool_address);
			let balance_1 = T::MultiCurrency::balance(trading_pair.1, &pool_address);

			// uint256 amount0In = balance0 > _reserve0 - amount0Out ? balance0 - (_reserve0
			// - amount0Out) : 0; uint256 amount1In = balance1 > _reserve1 - amount1Out ?
			// balance1
			// - (_reserve1 - amount1Out) : 0; require(amount0In > 0 || amount1In > 0,
			// "UniswapV2: INSUFFICIENT_INPUT_AMOUNT");
			let subtractor = U256::from(reserve_0).sub(U256::from(amount_0_out))?.saturated_into();
			let amount_0_in = if balance_0 > subtractor {
				U256::from(balance_0).sub(U256::from(subtractor))?.saturated_into()
			} else {
				0u128
			};

			let subtractor = U256::from(reserve_1).sub(U256::from(amount_1_out))?.saturated_into();
			let amount_1_in = if balance_1 > subtractor {
				U256::from(balance_1).sub(U256::from(subtractor))?.saturated_into()
			} else {
				0u128
			};

			ensure!(amount_0_in > 0 || amount_1_in > 0, Error::<T>::InsufficientInputAmount);

			// scope for reserve{0,1}Adjusted, avoids stack too deep errors
			// uint256 balance0Adjusted = balance0.mul(1000).sub(amount0In.mul(3));
			// uint256 balance1Adjusted = balance1.mul(1000).sub(amount1In.mul(3));
			// require(balance0Adjusted.mul(balance1Adjusted) >=
			// uint256(_reserve0).mul(_reserve1).mul(1000**2), "UniswapV2: K");

			let (fee_numerator, fee_denominator) = T::GetExchangeFee::get(); // -> 3 / 1000 = 0.3%
			let balance_0_adjusted = U256::from(balance_0)
				.mul(U256::from(fee_denominator))?
				.sub(U256::from(amount_0_in).mul(U256::from(fee_numerator))?)?;
			let balance_1_adjusted = U256::from(balance_1)
				.mul(U256::from(fee_denominator))?
				.sub(U256::from(amount_1_in).mul(U256::from(fee_numerator))?)?;

			ensure!(
				balance_0_adjusted.mul(balance_1_adjusted)?
					>= U256::from(reserve_0).mul(U256::from(reserve_1))?.mul(
						U256::from(fee_denominator)
							.checked_pow(U256::from(2_u32))
							.ok_or(ArithmeticError::Overflow)?
					)?,
				Error::<T>::InvalidConstantProduct
			);

			let _ = LiquidityPool::<T>::try_mutate(
				trading_pair,
				|(reserve_0, reserve_1)| -> DispatchResult {
					*reserve_0 = balance_0;
					*reserve_1 = balance_1;
					Ok(())
				},
			);

			// check if FeeTo is Some before doing the network fee related calculation
			if FeeTo::<T>::get().is_some() {
				// get the lp shared asset id
				let lp_share_asset_id =
					Self::lp_token_id(trading_pair).ok_or(Error::<T>::InvalidAssetId)?;

				// get the updated reserve values of the trading pair
				let (reserve_a, reserve_b) = LiquidityPool::<T>::get(trading_pair);

				// call mint_fee function here to collect network fees since the last collection and
				// send it to the FeeTo account
				let fee_on = Self::mint_fee(lp_share_asset_id, reserve_a, reserve_b)?;

				// update the k_last value if fee_on is true
				if fee_on {
					let _ = LiquidityPoolLastK::<T>::try_mutate(
						lp_share_asset_id,
						|k| -> DispatchResult {
							// update k to the product of the updated reserve_a and reserve_b
							*k = U256::from(reserve_a).mul(U256::from(reserve_b))?;
							Ok(())
						},
					);
				}
			}

			res.push((amount_0_in, amount_1_in, amount_0_out, amount_1_out));

			i += 1;
		}
		Ok(res)
	}

	/// Ensured atomic.
	#[transactional]
	pub fn do_swap_with_exact_supply(
		who: &T::AccountId,
		amount_in: Balance,
		min_amount_out: Balance,
		path: &[AssetId],
		to: T::AccountId,
		deadline: Option<BlockNumberFor<T>>,
	) -> sp_std::result::Result<
		(Vec<Balance>, Vec<(Balance, Balance, Balance, Balance)>),
		DispatchError,
	> {
		// Check if the deadline is met when the `deadline` parameter is not None
		if let Some(deadline_block) = deadline {
			let current_block_number = frame_system::Pallet::<T>::block_number();
			ensure!(deadline_block >= current_block_number, Error::<T>::ExpiredDeadline);
		}

		let amounts = Self::get_amounts_out(amount_in, path)?;

		// INSUFFICIENT_OUTPUT_AMOUNT
		ensure!(amounts[amounts.len() - 1] >= min_amount_out, Error::<T>::InsufficientTargetAmount);
		let trading_pair = TradingPair::new(path[0], path[1]);
		// transfer tokens to module account (uniswapv2 trading pair)
		let pool_address = trading_pair.pool_address();

		T::MultiCurrency::transfer(
			path[0],
			who,
			&pool_address,
			amounts[0],
			Preservation::Expendable,
		)?;

		let swap_res = Self::_swap(&amounts, path, &to)?;
		Self::deposit_event(Event::Swap(
			who.clone(),
			path.to_vec(),
			amount_in,
			amounts[amounts.len() - 1],
			to,
		));
		Ok((amounts, swap_res))
	}

	/// Ensured atomic.
	#[transactional]
	pub fn do_swap_with_exact_target(
		who: &T::AccountId,
		amount_out: Balance,
		amount_in_max: Balance,
		path: &[AssetId],
		to: T::AccountId,
		deadline: Option<BlockNumberFor<T>>,
	) -> sp_std::result::Result<
		(Vec<Balance>, Vec<(Balance, Balance, Balance, Balance)>),
		DispatchError,
	> {
		// Check if the deadline is met when the `deadline` parameter is not None
		if let Some(deadline_block) = deadline {
			let current_block_number = frame_system::Pallet::<T>::block_number();
			ensure!(deadline_block >= current_block_number, Error::<T>::ExpiredDeadline);
		}

		let amounts = Self::get_amounts_in(amount_out, path)?;

		// EXCESSIVE_INPUT_AMOUNT
		ensure!(amounts[0] <= amount_in_max, Error::<T>::ExcessiveSupplyAmount);
		let trading_pair = TradingPair::new(path[0], path[1]);
		let pool_address = trading_pair.pool_address();
		T::MultiCurrency::transfer(
			path[0],
			who,
			&pool_address,
			amounts[0],
			Preservation::Expendable,
		)?;

		let swap_res = Self::_swap(&amounts, path, &to)?;
		Self::deposit_event(Event::Swap(who.clone(), path.to_vec(), amounts[0], amount_out, to));
		Ok((amounts, swap_res))
	}

	/// Send the network fee to the `FeeTo` account
	/// This function is analogous to Uniswapv2 `_mintFee`
	fn mint_fee(
		lp_share_asset_id: AssetId,
		reserve_a: Balance,
		reserve_b: Balance,
	) -> sp_std::result::Result<bool, DispatchError> {
		let k_last = LiquidityPoolLastK::<T>::get(lp_share_asset_id);
		if let Some(fee_to) = FeeTo::<T>::get() {
			if !k_last.is_zero() {
				let root_k =
					U256::from(reserve_a).saturating_mul(U256::from(reserve_b)).integer_sqrt();
				let root_k_last = k_last.integer_sqrt();

				if root_k.gt(&root_k_last) {
					// calculate the amount with the formula
					// liquidity = total_supply * (sqrt(k)-sqrt(k_last))/(5*sqrt(k)+sqrt(k_last))
					let total_supply =
						U256::from(T::MultiCurrency::total_issuance(lp_share_asset_id));
					let numerator = total_supply.mul(root_k.sub(root_k_last)?)?;
					let denominator = root_k.mul(U256::from(5))?.add(root_k_last)?;
					let liquidity = numerator.div(denominator)?;

					if liquidity.gt(&U256::zero()) {
						// mint lp tokens to the FeeTo account
						T::MultiCurrency::mint_into(
							lp_share_asset_id,
							&fee_to,
							liquidity.saturated_into(),
						)?;
					}
				}
			}
			Ok(true)
		} else if !k_last.is_zero() {
			let _ = LiquidityPoolLastK::<T>::try_mutate(lp_share_asset_id, |k| -> DispatchResult {
				*k = U256::zero();
				Ok(())
			});
			Ok(false)
		} else {
			Ok(false)
		}
	}
}
