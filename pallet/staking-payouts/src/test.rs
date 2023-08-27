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

#![cfg(test)]
use super::*;
use crate::mock::{
	bond_nominator, bond_validator, current_total_payout_for_duration, new_test_ext,
	reward_time_per_era, test_accounts::*, Balances, ExtBuilder, Origin,
	RewardOnUnbalanceWasCalled, Staking, StakingPayout, System, TestRuntime,
};
use frame_support::{assert_ok, traits::Hooks};
use pallet_staking::Validators;
use seed_primitives::{AccountId, AccountId20};
use sp_runtime::{assert_eq_error_rate, Perbill};

fn alice() -> AccountId {
	AccountId20([1; 20])
}

// #[test]
// fn payout_period_id_increments() {
// 	new_test_ext().execute_with(|| {});
// }

// #[test]
// fn iterates_per_block_validators() {
// 	env_logger::init();
// 	new_test_ext().execute_with(|| {
// 		let block = System::block_number();
// 		StakingPayout::on_initialize(block);
// 		// assert_eq!(CurrentValidatorIter::<TestRuntime>::get(), Some(alice()));
// 	});
// }

// #[test]
// fn test_payout_stakers() {
// 	// Test that payout_stakers work in general, including that only the top
// 	// `T::MaxNominatorRewardedPerValidator` nominators are rewarded.
// 	new_test_ext().execute_with(|| {
// 		let balance = 1000;
// 		// Track the exposure of the validator and all nominators.
// 		let mut total_exposure = balance;
// 		// Track the exposure of the validator and the nominators that will get paid out.
// 		let mut payout_exposure = balance;
// 		// Create a validator:
// 		bond_validator(STASH_ONE, CONTROLLER_ONE, balance); // Default(64)
// 		assert_eq!(Validators::<TestRuntime>::count(), 1);

// 		// // Create nominators, targeting stash of validators
// 		for i in 0..100 {
// 			let bond_amount = balance + i as BalanceOf<TestRuntime>;
// 			// bond_nominator(1000 + i, 100 + i, bond_amount, vec![11]);
// 			bond_nominator(
// 				// AccountId20([1000 + i; 20]),
// 				// Try this...
// 				AccountId20([100 + i; 20]),
// 				AccountId20([100 + i; 20]),
// 				bond_amount,
// 				vec![AccountId20([11; 20])],
// 			);
// 			total_exposure += bond_amount;
// 			if i >= 36 {
// 				payout_exposure += bond_amount;
// 			};
// 		}
// 		let payout_exposure_part = Perbill::from_rational(payout_exposure, total_exposure);
// 		////////

// 		mock::start_active_era(1);
// 		// assert_eq!( pallet_staking::CurrentEra::<TestRuntime>::get(), Some(1));

// 		// Staking::reward_by_ids(vec![(AccountId20([11; 20]), 1)]);

// 		// // compute and ensure the reward amount is greater than zero.
// 		// let payout = current_total_payout_for_duration(reward_time_per_era());
// 		// let actual_paid_out = payout_exposure_part * payout;

// 		// mock::start_active_era(2);

// 		// let pre_payout_total_issuance = Balances::total_issuance();
// 		// RewardOnUnbalanceWasCalled::set(false);

// 		// assert_ok!(Staking::payout_stakers(Origin::signed(AccountId20([1; 20])), STASH_ONE, 1));
// 		// // TODO:  Check payout was made here

// 		// let current_block = System::block_number();
// 		// StakingPayout::on_initialize(current_block);

// 		// assert_eq!(CurrentValidatorIter::<TestRuntime>::get(), Some(AccountId20([1; 20])));

// 		////////

// 		// assert_eq_error_rate!(
// 		// 	Balances::total_issuance(),
// 		// 	pre_payout_total_issuance + actual_paid_out,
// 		// 	1
// 		// );
// 		// assert!(RewardOnUnbalanceWasCalled::get());

// 		// // Top 64 nominators of validator 11 automatically paid out, including the validator
// 		// // Validator payout goes to controller.
// 		// assert!(Balances::free_balance(&10) > balance);
// 		// for i in 36..100 {
// 		// 	assert!(Balances::free_balance(&(100 + i)) > balance + i as Balance);
// 		// }
// 		// // The bottom 36 do not
// 		// for i in 0..36 {
// 		// 	assert_eq!(Balances::free_balance(&(100 + i)), balance + i as Balance);
// 		// }

// 		// // We track rewards in `claimed_rewards` vec
// 		// assert_eq!(
// 		// 	Staking::ledger(&10),
// 		// 	Some(StakingLedger {
// 		// 		stash: 11,
// 		// 		total: 1000,
// 		// 		active: 1000,
// 		// 		unlocking: Default::default(),
// 		// 		claimed_rewards: vec![1]
// 		// 	})
// 		// );

// 		// for i in 3..16 {
// 		// 	Staking::reward_by_ids(vec![(11, 1)]);

// 		// 	// compute and ensure the reward amount is greater than zero.
// 		// 	let payout = current_total_payout_for_duration(reward_time_per_era());
// 		// 	let actual_paid_out = payout_exposure_part * payout;
// 		// 	let pre_payout_total_issuance = Balances::total_issuance();

// 		// 	mock::start_active_era(i);
// 		// 	RewardOnUnbalanceWasCalled::set(false);
// 		// 	assert_ok!(Staking::payout_stakers(Origin::signed(1337), 11, i - 1));
// 		// 	assert_eq_error_rate!(
// 		// 		Balances::total_issuance(),
// 		// 		pre_payout_total_issuance + actual_paid_out,
// 		// 		1
// 		// 	);
// 		// 	assert!(RewardOnUnbalanceWasCalled::get());
// 		// }

// 		// // We track rewards in `claimed_rewards` vec
// 		// assert_eq!(
// 		// 	Staking::ledger(&10),
// 		// 	Some(StakingLedger {
// 		// 		stash: 11,
// 		// 		total: 1000,
// 		// 		active: 1000,
// 		// 		unlocking: Default::default(),
// 		// 		claimed_rewards: (1..=14).collect()
// 		// 	})
// 		// );

// 		// for i in 16..100 {
// 		// 	Staking::reward_by_ids(vec![(11, 1)]);
// 		// 	// compute and ensure the reward amount is greater than zero.
// 		// 	let _ = current_total_payout_for_duration(reward_time_per_era());
// 		// 	mock::start_active_era(i);
// 		// }

// 		// // We clean it up as history passes
// 		// assert_ok!(Staking::payout_stakers(Origin::signed(1337), 11, 15));
// 		// assert_ok!(Staking::payout_stakers(Origin::signed(1337), 11, 98));
// 		// assert_eq!(
// 		// 	Staking::ledger(&10),
// 		// 	Some(StakingLedger {
// 		// 		stash: 11,
// 		// 		total: 1000,
// 		// 		active: 1000,
// 		// 		unlocking: Default::default(),
// 		// 		claimed_rewards: vec![15, 98]
// 		// 	})
// 		// );

// 		// // Out of order claims works.
// 		// assert_ok!(Staking::payout_stakers(Origin::signed(1337), 11, 69));
// 		// assert_ok!(Staking::payout_stakers(Origin::signed(1337), 11, 23));
// 		// assert_ok!(Staking::payout_stakers(Origin::signed(1337), 11, 42));
// 		// assert_eq!(
// 		// 	Staking::ledger(&10),
// 		// 	Some(StakingLedger {
// 		// 		stash: 11,
// 		// 		total: 1000,
// 		// 		active: 1000,
// 		// 		unlocking: Default::default(),
// 		// 		claimed_rewards: vec![15, 23, 42, 69, 98]
// 		// 	})
// 		// );
// 	});
// }

// #[test]
// fn current_iter_resets_to_zero_until_next_payout() {}
