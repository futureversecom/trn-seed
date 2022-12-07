#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

use frame_support::{
	pallet_prelude::*,
	weights::WeightToFee,
};
use frame_system::pallet_prelude::*;

use seed_primitives::Balance;

use sp_core::U256;
use sp_runtime::{Perbill, Permill};

use core::ops::Mul;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

mod benchmarking;

pub trait BaseFeeThreshold {
	fn lower() -> Permill;
	fn ideal() -> Permill;
	fn upper() -> Permill;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Threshold: BaseFeeThreshold;
		type DefaultEvmBaseFeePerGas: Get<U256>;
		type DefaultEvmElasticity: Get<Permill>;
		type WeightToFeeReduction: Get<Perbill>;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub base_fee_per_gas: U256,
		pub elasticity: Permill,
		_marker: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> GenesisConfig<T> {
		pub fn new(base_fee_per_gas: U256, elasticity: Permill) -> Self {
			Self {
				base_fee_per_gas,
				elasticity,
				_marker: PhantomData,
			}
		}
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				base_fee_per_gas: T::DefaultEvmBaseFeePerGas::get(),
				elasticity: T::DefaultEvmElasticity::get(),
				_marker: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			<EvmBaseFeePerGas<T>>::put(self.base_fee_per_gas);
			<EvmElasticity<T>>::put(self.elasticity);
		}
	}

	#[pallet::type_value]
	pub fn DefaultEvmBaseFeePerGas<T: Config>() -> U256 {
		T::DefaultEvmBaseFeePerGas::get()
	}

	#[pallet::type_value]
	pub fn DefaultWeightToFeeReduction<T: Config>() -> Perbill {
		T::WeightToFeeReduction::get()
	}

	#[pallet::storage]
	#[pallet::getter(fn base_fee_per_gas)]
	pub type EvmBaseFeePerGas<T> = StorageValue<_, U256, ValueQuery, DefaultEvmBaseFeePerGas<T>>;

	#[pallet::storage]
	#[pallet::getter(fn extrinsic_weight_to_fee)]
	pub type ExtrinsicWeightToFee<T> = StorageValue<_, Perbill, ValueQuery, DefaultWeightToFeeReduction<T>>;

	#[pallet::type_value]
	pub fn DefaultElasticity<T: Config>() -> Permill {
		T::DefaultEvmElasticity::get()
	}

	#[pallet::storage]
	#[pallet::getter(fn elasticity)]
	pub type EvmElasticity<T> = StorageValue<_, Permill, ValueQuery, DefaultElasticity<T>>;


	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		BaseFeeOverflow,
	}

	#[pallet::error]
	pub enum Error<T> {
		BaseFeeOverflow,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: T::BlockNumber) -> Weight {
			// Register the Weight used on_finalize.
			// 	- One storage read to get the block_weight.
			// 	- One storage read to get the Elasticity.
			// 	- One write to EvmBaseFeePerGas.
			let db_weight = <T as frame_system::Config>::DbWeight::get();
			db_weight.reads(2).saturating_add(db_weight.write)
		}

		fn on_finalize(_n: <T as frame_system::Config>::BlockNumber) {
			if <EvmElasticity<T>>::get().is_zero() {
				// Zero elasticity means constant EvmBaseFeePerGas.
				return;
			}

			let lower = T::Threshold::lower();
			let upper = T::Threshold::upper();
			// `target` is the ideal congestion of the network where the base fee should remain unchanged.
			// Under normal circumstances the `target` should be 50%.
			// If we go below the `target`, the base fee is linearly decreased by the Elasticity delta of lower~target.
			// If we go above the `target`, the base fee is linearly increased by the Elasticity delta of upper~target.
			// The base fee is fully increased (default 12.5%) if the block is upper full (default 100%).
			// The base fee is fully decreased (default 12.5%) if the block is lower empty (default 0%).
			let weight = <frame_system::Pallet<T>>::block_weight();
			let max_weight = <<T as frame_system::Config>::BlockWeights>::get().max_block;

			// We convert `weight` into block fullness and ensure we are within the lower and upper bound.
			let weight_used =
				Permill::from_rational(weight.total(), max_weight).clamp(lower, upper);
			// After clamp `weighted_used` is always between `lower` and `upper`.
			// We scale the block fullness range to the lower/upper range, and the usage represents the
			// actual percentage within this new scale.
			let usage = (weight_used - lower) / (upper - lower);

			// Target is our ideal block fullness.
			let target = T::Threshold::ideal();
			if usage > target {
				// Above target, increase.
				let coef = Permill::from_parts((usage.deconstruct() - target.deconstruct()) * 2u32);
				// How much of the Elasticity is used to mutate base fee.
				let coef = <EvmElasticity<T>>::get() * coef;
				<EvmBaseFeePerGas<T>>::mutate(|bf| {
					if let Some(scaled_basefee) = bf.checked_mul(U256::from(coef.deconstruct())) {
						// Normalize to GWEI.
						let increase = scaled_basefee
							.checked_div(U256::from(1_000_000))
							.unwrap_or_else(U256::zero);
						*bf = bf.saturating_add(increase);
					} else {
						Self::deposit_event(Event::<T>::BaseFeeOverflow);
					}
				});
			} else if usage < target {
				// Below target, decrease.
				let coef = Permill::from_parts((target.deconstruct() - usage.deconstruct()) * 2u32);
				// How much of the Elasticity is used to mutate base fee.
				let coef = <EvmElasticity<T>>::get() * coef;
				<EvmBaseFeePerGas<T>>::mutate(|bf| {
					if let Some(scaled_basefee) = bf.checked_mul(U256::from(coef.deconstruct())) {
						// Normalize to GWEI.
						let decrease = scaled_basefee
							.checked_div(U256::from(1_000_000))
							.unwrap_or_else(U256::zero);
						*bf = bf.saturating_sub(decrease);
					} else {
						Self::deposit_event(Event::<T>::BaseFeeOverflow);
					}
				});
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1000_000 as Weight)]
		pub fn set_evm_base_fee(origin: OriginFor<T>, value: U256) -> DispatchResult {
			ensure_root(origin)?;
			<EvmBaseFeePerGas<T>>::put(value);
			Ok(())
		}

		#[pallet::weight(1000_000 as Weight)]
		pub fn set_extrinsic_base_fee(origin: OriginFor<T>, value: Perbill) -> DispatchResult {
			ensure_root(origin)?;
			ExtrinsicWeightToFee::<T>::put(value);
			Ok(())
		}

		// For local testing. Charge some low gas
		#[pallet::weight(207_555_000 as Weight)]
		pub fn debug_charge_feee(origin: OriginFor<T>) -> DispatchResult {
			Ok(())
		}
	}

	// Substrate extrinsics fee control
	impl<T> WeightToFee for Pallet<T>
	where
		T: Config,
	{
		type Balance = Balance;
		fn weight_to_fee(weight: &Weight) -> Balance {
			Self::extrinsic_weight_to_fee().mul(*weight as Balance)
		}
	}

}

impl<T: Config> fp_evm::FeeCalculator for Pallet<T> {
	fn min_gas_price() -> (U256, Weight) {
		(<EvmBaseFeePerGas<T>>::get(), T::DbWeight::get().reads(1))
	}
}
