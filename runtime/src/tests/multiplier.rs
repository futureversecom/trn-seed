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

use super::*;
use crate::{
	AdjustmentVariable, MaximumMultiplier, MinimumMultiplier, Multiplier, RuntimeBlockWeights,
	TargetBlockFullness, TargetedFeeAdjustment, Weight,
};
use frame_support::dispatch::DispatchClass;
use sp_runtime::{
	assert_eq_error_rate,
	traits::{Convert, One, Zero},
	FixedPointNumber,
};
use std::ops::Add;

fn max_normal() -> Weight {
	RuntimeBlockWeights::get()
		.get(DispatchClass::Normal)
		.max_total
		.unwrap_or_else(|| RuntimeBlockWeights::get().max_block)
}

fn min_multiplier() -> Multiplier {
	MinimumMultiplier::get()
}

fn target() -> Weight {
	TargetBlockFullness::get() * max_normal()
}

// update based on runtime impl.
fn runtime_multiplier_update(fm: Multiplier) -> Multiplier {
	TargetedFeeAdjustment::<
		Runtime,
		TargetBlockFullness,
		AdjustmentVariable,
		MinimumMultiplier,
		MaximumMultiplier,
	>::convert(fm)
}

// update based on reference impl.
fn truth_value_update(block_weight: Weight, previous: Multiplier) -> Multiplier {
	let accuracy = Multiplier::accuracy() as f64;
	let previous_float = previous.into_inner() as f64 / accuracy;
	// bump if it is zero.
	let previous_float = previous_float.max(min_multiplier().into_inner() as f64 / accuracy);

	// maximum tx weight
	let m = max_normal().ref_time() as f64;
	// block weight always truncated to max weight
	let block_weight = (block_weight.ref_time() as f64).min(m);
	let v: f64 = AdjustmentVariable::get().to_float();

	// Ideal saturation in terms of weight
	let ss = target().ref_time() as f64;
	// Current saturation in terms of weight
	let s = block_weight;

	let t1 = v * (s / m - ss / m);
	let t2 = v.powi(2) * (s / m - ss / m).powi(2) / 2.0;
	let next_float = previous_float * (1.0 + t1 + t2);
	Multiplier::from_float(next_float)
}

fn run_with_system_weight<F>(w: Weight, mut assertions: F)
where
	F: FnMut(),
{
	let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
		.build_storage()
		.unwrap()
		.into();
	t.execute_with(|| {
		System::set_block_consumed_resources(w, 0);
		assertions()
	});
}

#[test]
fn truth_value_update_poc_works() {
	let fm = Multiplier::saturating_from_rational(1, 2);
	let test_set = vec![
		(Weight::from_all(0), fm),
		(Weight::from_all(100), fm),
		(Weight::from_all(1000), fm),
		(target(), fm),
		(max_normal() / 2, fm),
		(max_normal(), fm),
	];
	test_set.into_iter().for_each(|(w, fm)| {
		run_with_system_weight(w, || {
			assert_eq_error_rate!(
				truth_value_update(w, fm),
				runtime_multiplier_update(fm),
				// Error is only 1 in 100^18
				Multiplier::from_inner(100),
			);
		})
	})
}

#[test]
fn multiplier_can_grow_from_zero() {
	// if the min is too small, then this will not change, and we are doomed forever.
	// the block ref time is 1/100th bigger than target.
	run_with_system_weight(target().set_ref_time(target().ref_time() * 101 / 100), || {
		let next = runtime_multiplier_update(min_multiplier());
		assert!(next > min_multiplier(), "{:?} !> {:?}", next, min_multiplier());
	});

	// the block proof size is 1/100th bigger than target.
	run_with_system_weight(target().set_proof_size((target().proof_size() / 100) * 101), || {
		let next = runtime_multiplier_update(min_multiplier());
		assert!(next > min_multiplier(), "{:?} !> {:?}", next, min_multiplier());
	})
}

#[test]
fn multiplier_cannot_go_below_limit() {
	// will not go any further below even if block is empty.
	run_with_system_weight(Weight::zero(), || {
		let next = runtime_multiplier_update(min_multiplier());
		assert_eq!(next, min_multiplier());
	})
}

#[test]
fn weight_mul_grow_on_big_block() {
	run_with_system_weight(target() * 2, || {
		let mut original = Multiplier::zero();
		let mut next = Multiplier::default();

		(0..1_000).for_each(|_| {
			next = runtime_multiplier_update(original);
			assert_eq_error_rate!(
				next,
				truth_value_update(target() * 2, original),
				Multiplier::from_inner(100),
			);
			// must always increase
			assert!(next > original, "{:?} !>= {:?}", next, original);
			original = next;
		});
	});
}

#[test]
fn weight_mul_decrease_on_small_block() {
	run_with_system_weight(target() / 2, || {
		let mut original = Multiplier::saturating_from_rational(1, 2);
		let mut next;

		for _ in 0..100 {
			// decreases
			next = runtime_multiplier_update(original);
			assert!(next < original, "{:?} !<= {:?}", next, original);
			original = next;
		}
	})
}

#[test]
fn weight_to_fee_should_not_overflow_on_large_weights() {
	let kb = Weight::from_all(1024u64);
	let mb = kb.mul(kb.ref_time());
	let max_fm = Multiplier::saturating_from_integer(i128::MAX);

	// check that for all values it can compute, correctly.
	vec![
		Weight::zero(),
		Weight::from_all(1u64),
		Weight::from_all(10u64),
		Weight::from_all(1000u64),
		kb,
		kb.mul(10u64),
		kb.mul(100u64),
		mb,
		mb.mul(10u64),
		Weight::from_all(2147483647u64),
		Weight::from_all(4294967295u64),
		RuntimeBlockWeights::get().max_block.div(2u64),
		RuntimeBlockWeights::get().max_block,
		Weight::MAX.div(2u64),
		Weight::MAX,
	]
	.into_iter()
	.for_each(|i| {
		run_with_system_weight(i, || {
			let next = runtime_multiplier_update(Multiplier::one());
			let truth = truth_value_update(i, Multiplier::one());
			assert_eq_error_rate!(truth, next, Multiplier::from_inner(50_000_000));
		});
	});

	// Some values that are all above the target and will cause an increase.
	let t = target();
	vec![t.add(Weight::from_all(100u64)), t.mul(2u64), t.mul(4u64)]
		.into_iter()
		.for_each(|i| {
			run_with_system_weight(i, || {
				let fm = runtime_multiplier_update(max_fm);
				// won't grow. The convert saturates everything.
				assert_eq!(fm, max_fm);
			})
		});
}
