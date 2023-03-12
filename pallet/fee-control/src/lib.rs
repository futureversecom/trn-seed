/* Copyright 2021-2022 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */

#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

use seed_primitives::Balance;

use frame_support::traits::OnRuntimeUpgrade;
use sp_core::U256;
use sp_runtime::Perbill;
use types::{CalculatorErrors, ConfigOp, DecimalBalance, FeeControlData, FeeMultiplierCalculator};

use core::ops::Mul;
#[cfg(test)]
mod tests;

mod weights;
pub use weights::WeightInfo;
mod migrations;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod types;

#[frame_support::pallet]
pub mod pallet {
	use sp_runtime::Permill;

	use super::*;

	pub trait BaseFeeThreshold {
		fn lower() -> Permill;
		fn ideal() -> Permill;
		fn upper() -> Permill;
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Interface to access weight values.
		type WeightInfo: WeightInfo;
		/// Origin that can control this pallet.
		type CallOrigin: EnsureOrigin<Self::Origin>;

		type Threshold: BaseFeeThreshold;
		type DefaultBaseFeePerGas: Get<U256>;
		type DefaultElasticity: Get<Permill>;

		/// To get the value of one XRP.
		#[pallet::constant]
		type OneXRP: Get<Balance>;
		/// Get XRP Evm scale factor.
		#[pallet::constant]
		type EvmXRPScaleFactor: Get<Balance>;
		/// Default Weight multiplier.
		#[pallet::constant]
		type WeightMultiplier: Get<Perbill>;
		/// Default Length multiplier.
		#[pallet::constant]
		type LengthMultiplier: Get<Balance>;
		/// Default EVM base fee.
		#[pallet::constant]
		type EvmBaseFeePerGas: Get<U256>;
		/// Default TX price.
		#[pallet::constant]
		type OutputTxPrice: Get<Balance>;
		/// Default Length price.
		#[pallet::constant]
		type OutputLenPrice: Get<Balance>;

		/// Input TX weight.
		type InputTxWeight: Get<Weight>;
		/// Input Gas limit.
		type InputGasLimit: Get<U256>;
	}

	#[pallet::type_value]
	pub fn DefaultWeightToFeeReduction<T: Config>() -> FeeControlData {
		let length_multiplier = DecimalBalance::new(T::LengthMultiplier::get(), Perbill::zero());
		FeeControlData {
			weight_multiplier: T::WeightMultiplier::get(),
			length_multiplier,
			output_len_fee: T::OutputLenPrice::get(),
			output_tx_fee: T::OutputTxPrice::get(),
			input_tx_weight: T::InputTxWeight::get(),
			evm_base_fee: T::EvmBaseFeePerGas::get(),
			input_gas_limit: T::InputGasLimit::get(),
			is_locked: false,
			refresh_data: true,
		}
	}

	#[pallet::storage]
	pub type SettingsAndMultipliers<T> =
		StorageValue<_, FeeControlData, ValueQuery, DefaultWeightToFeeReduction<T>>;

	#[pallet::type_value]
	pub fn DefaultElasticity<T: Config>() -> Permill {
		T::DefaultElasticity::get()
	}

	#[pallet::storage]
	pub type EvmElasticity<T> = StorageValue<_, Permill, ValueQuery, DefaultElasticity<T>>;

	#[pallet::type_value]
	pub fn DefaultBaseFeePerGas<T: Config>() -> U256 {
		T::DefaultBaseFeePerGas::get()
	}

	#[pallet::storage]
	pub type BaseFeePerGas<T> = StorageValue<_, U256, ValueQuery, DefaultBaseFeePerGas<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T> {
		/// Was not able to automatically set new EVM fee
		EvmBaseFeeOverflow,
		/// New settings and multipliers have been applied.
		NewSettingsHaveBeenApplied,
		/// New XRP Price has been set.
		NewXRPPrice { value: Balance },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// One XRP cannot be zero.
		OneXRPCannotBeZero,
		/// XRP Price cannot be zero.
		XRPPriceCannotBeZero,
		/// EVM XRP Scale Factor cannot be zero.
		EvmXrpScaleFactorCannotBeZero,
		/// Input TX Weight cannot be zero.
		InputTxWeightCannotBeZero,
		/// Input Gas Limit cannot be zero.
		InputGasLimitCannotBeZero,
		/// Output TX fee cannot be zero.
		OutputTxFeeCannotBeZero,
		/// Output len fee cannot be zero.
		OutputLenFeeCannotBeZero,
		/// Weight Multiplier quotient cannot be zero.
		WeightMultiplierQuotientCannotBeZero,
		/// One Weight value cannot be worth more than one XRP.
		OneWeightCannotBeWorthMoreThanOneXRP,
		/// Something went wrong with calculating the length multiplier.
		LengthMultiplierCalculationError,
		/// Something went wrong with calculating the evm multiplier.
		EvmMultiplierCalculationError,
		/// Cannot update multipliers with lock on.
		CannotUpdateMultipliersWithLockOn,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<(), &'static str> {
			log::info!("Pre Upgrade.");
			migrations::v2::MigrationV2::<T>::pre_upgrade()
		}

		fn on_runtime_upgrade() -> Weight {
			let current = Pallet::<T>::current_storage_version();
			let onchain = Pallet::<T>::on_chain_storage_version();

			log::info!(
				"Running migration with current storage version {current:?} / onchain {onchain:?}"
			);

			let mut weight = T::DbWeight::get().reads(1);
			// If you are running Fork Porcini script, make sure that you set the current storage
			// version to 0. The reason for this is because we started this pallet with storage
			// version set to 1 but this was just in code and have actually never set it to
			// that value in the db.
			// Because of that Porcini and Root don't have a `StorageVersion` value in the db at all
			// and when you query it you can the default value which is 0. If you scrap the remote
			// chain you won't get the value 0 since it's not stored inside the db.
			if onchain == 0 {
				weight += migrations::v2::MigrationV2::<T>::on_runtime_upgrade();
			} else {
				log::info!("No migration was done");
			}

			weight
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade() -> Result<(), &'static str> {
			log::info!("Post Upgrade.");
			migrations::v2::MigrationV2::<T>::post_upgrade()
		}

		fn on_initialize(_: T::BlockNumber) -> Weight {
			// Register the Weight used on_finalize.
			// 	- One storage read to get the block_weight.
			// 	- One storage read to get the Elasticity.
			// 	- One write to BaseFeePerGas.
			let db_weight = <T as frame_system::Config>::DbWeight::get();
			db_weight.reads_writes(2, 1)
		}

		fn on_finalize(_n: <T as frame_system::Config>::BlockNumber) {
			if <EvmElasticity<T>>::get().is_zero() {
				// Zero elasticity means constant BaseFeePerGas.
				return
			}

			let lower = T::Threshold::lower();
			let upper = T::Threshold::upper();
			// `target` is the ideal congestion of the network where the base fee should remain
			// unchanged. Under normal circumstances the `target` should be 50%.
			// If we go below the `target`, the base fee is linearly decreased by the Elasticity
			// delta of lower~target. If we go above the `target`, the base fee is linearly
			// increased by the Elasticity delta of upper~target. The base fee is fully increased
			// (default 12.5%) if the block is upper full (default 100%). The base fee is fully
			// decreased (default 12.5%) if the block is lower empty (default 0%).
			let weight = <frame_system::Pallet<T>>::block_weight();
			let max_weight = <<T as frame_system::Config>::BlockWeights>::get().max_block;

			// We convert `weight` into block fullness and ensure we are within the lower and upper
			// bound.
			let weight_used =
				Permill::from_rational(weight.total(), max_weight).clamp(lower, upper);
			// After clamp `weighted_used` is always between `lower` and `upper`.
			// We scale the block fullness range to the lower/upper range, and the usage represents
			// the actual percentage within this new scale.
			let usage = (weight_used - lower) / (upper - lower);

			// Target is our ideal block fullness.
			let target = T::Threshold::ideal();
			if usage > target {
				// Above target, increase.
				let coef = Permill::from_parts((usage.deconstruct() - target.deconstruct()) * 2u32);
				// How much of the Elasticity is used to mutate base fee.
				let coef = <EvmElasticity<T>>::get() * coef;
				<BaseFeePerGas<T>>::mutate(|bf| {
					if let Some(scaled_basefee) = bf.checked_mul(U256::from(coef.deconstruct())) {
						// Normalize to GWEI.
						let increase = scaled_basefee
							.checked_div(U256::from(1_000_000))
							.unwrap_or_else(U256::zero);
						*bf = bf.saturating_add(increase);
					} else {
						Self::deposit_event(Event::EvmBaseFeeOverflow);
					}
				});
			} else if usage < target {
				// Below target, decrease.
				let coef = Permill::from_parts((target.deconstruct() - usage.deconstruct()) * 2u32);
				// How much of the Elasticity is used to mutate base fee.
				let coef = <EvmElasticity<T>>::get() * coef;
				<BaseFeePerGas<T>>::mutate(|bf| {
					if let Some(scaled_basefee) = bf.checked_mul(U256::from(coef.deconstruct())) {
						// Normalize to GWEI.
						let decrease = scaled_basefee
							.checked_div(U256::from(1_000_000))
							.unwrap_or_else(U256::zero);
						*bf = bf.saturating_sub(decrease);
					} else {
						Self::deposit_event(Event::EvmBaseFeeOverflow);
					}
				});
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::set_settings())]
		pub fn set_settings(
			origin: OriginFor<T>,
			weight_multiplier: ConfigOp<Perbill>,
			length_multiplier: ConfigOp<DecimalBalance>,
			evm_base_fee: ConfigOp<U256>,
			input_tx_weight: ConfigOp<Weight>,
			input_gas_limit: ConfigOp<U256>,
			output_tx_fee: ConfigOp<Balance>,
			output_len_fee: ConfigOp<Balance>,
			is_locked: ConfigOp<bool>,
			refresh_data: ConfigOp<bool>,
		) -> DispatchResult {
			T::CallOrigin::ensure_origin(origin)?;

			SettingsAndMultipliers::<T>::mutate(|x| {
				x.weight_multiplier = weight_multiplier.new_or_existing(x.weight_multiplier);
				x.length_multiplier =
					length_multiplier.new_or_existing(x.length_multiplier.clone());
				x.evm_base_fee = evm_base_fee.new_or_existing(x.evm_base_fee);
				x.output_tx_fee = output_tx_fee.new_or_existing(x.output_tx_fee);
				x.input_tx_weight = input_tx_weight.new_or_existing(x.input_tx_weight);
				x.output_len_fee = output_len_fee.new_or_existing(x.output_len_fee);
				x.input_gas_limit = input_gas_limit.new_or_existing(x.input_gas_limit);
				x.is_locked = is_locked.new_or_existing(x.is_locked);
				x.refresh_data = refresh_data.new_or_existing(x.refresh_data);
			});

			Self::deposit_event(Event::<T>::NewSettingsHaveBeenApplied);

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::set_xrp_price())]
		pub fn set_xrp_price(
			origin: OriginFor<T>,
			#[pallet::compact] xrp_price: Balance,
		) -> DispatchResult {
			T::CallOrigin::ensure_origin(origin)?;

			SettingsAndMultipliers::<T>::try_mutate(|x| -> DispatchResult {
				ensure!(!x.is_locked, Error::<T>::CannotUpdateMultipliersWithLockOn);

				let one_xrp = T::OneXRP::get();
				let evm_xrp_scale_factor = T::EvmXRPScaleFactor::get();

				if x.refresh_data {
					x.input_gas_limit = T::InputGasLimit::get();
					x.input_tx_weight = T::InputTxWeight::get();
					x.refresh_data = false;
				}

				let weight_multiplier = FeeMultiplierCalculator::weight_multiplier(
					one_xrp,
					xrp_price,
					x.input_tx_weight,
					x.output_tx_fee,
				)
				.map_err(|x| Error::<T>::from(x))?;

				let length_multiplier = FeeMultiplierCalculator::length_multiplier(
					one_xrp,
					xrp_price,
					x.output_len_fee,
				)
				.map_err(|x| Error::<T>::from(x))?;

				let evm_base_fee = FeeMultiplierCalculator::evm_base_fee(
					one_xrp,
					xrp_price,
					evm_xrp_scale_factor,
					x.input_gas_limit,
					x.output_tx_fee,
				)
				.map_err(|x| Error::<T>::from(x))?;

				x.weight_multiplier = weight_multiplier;
				x.length_multiplier = length_multiplier;
				x.evm_base_fee = evm_base_fee;

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::NewXRPPrice { value: xrp_price });
			Ok(())
		}
	}
}

impl<T: Config> fp_evm::FeeCalculator for Pallet<T> {
	fn min_gas_price() -> (U256, Weight) {
		(Self::base_fee_per_gas(), T::DbWeight::get().reads(1))
	}
}

impl<T: Config> Pallet<T> {
	pub fn weight_to_fee(weight: &Weight) -> Balance {
		let multiplier = SettingsAndMultipliers::<T>::get().weight_multiplier;
		multiplier.mul(*weight as Balance)
	}

	pub fn length_to_fee(weight: &Weight) -> Balance {
		let multiplier = SettingsAndMultipliers::<T>::get().length_multiplier;
		multiplier.mul(*weight as Balance)
	}

	pub fn base_fee_per_gas() -> U256 {
		// TODO: Think of how to reconcile this custom base fee & and base fee which was calculated
		// off of congestion
		SettingsAndMultipliers::<T>::get().evm_base_fee
	}
}

impl<T: Config> From<CalculatorErrors> for Error<T> {
	fn from(value: CalculatorErrors) -> Self {
		use CalculatorErrors as CE;
		match value {
			CE::OneXRPCannotBeZero => Self::OneXRPCannotBeZero,
			CE::XRPPriceCannotBeZero => Self::XRPPriceCannotBeZero,
			CE::EvmXrpScaleFactorCannotBeZero => Self::EvmXrpScaleFactorCannotBeZero,
			CE::InputTxWeightCannotBeZero => Self::InputTxWeightCannotBeZero,
			CE::InputGasLimitCannotBeZero => Self::InputGasLimitCannotBeZero,
			CE::OutputTxFeeCannotBeZero => Self::OutputTxFeeCannotBeZero,
			CE::OutputLenFeeCannotBeZero => Self::OutputLenFeeCannotBeZero,
			CE::WeightMultiplierQuotientCannotBeZero => Self::WeightMultiplierQuotientCannotBeZero,
			CE::OneWeightCannotBeWorthMoreThanOneXRP => Self::OneWeightCannotBeWorthMoreThanOneXRP,
			CE::LengthMultiplierCalculationError => Self::LengthMultiplierCalculationError,
			CE::EvmMultiplierCalculationError => Self::EvmMultiplierCalculationError,
		}
	}
}
