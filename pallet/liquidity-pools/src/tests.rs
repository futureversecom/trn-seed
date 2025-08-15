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
use crate::mock::{
	AssetsExt, Balances, LiquidityPools, NativeAssetId, RuntimeEvent as MockEvent, RuntimeOrigin,
	System, Test,
};
use frame_support::{assert_noop, assert_ok, weights::constants::ParityDbWeight};
use seed_pallet_common::test_prelude::*;
use seed_primitives::AccountId;
use sp_runtime::traits::{BadOrigin, Zero};

mod create_pool {
	use super::*;

	#[test]
	fn pool_creation_fails_with_next_pool_id_out_of_bounds() {
		TestExt::<Test>::default().build().execute_with(|| {
			let reward_asset_id = 1;
			let staked_asset_id = 2;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let lock_start_block = System::block_number() + 1;
			let lock_end_block = lock_start_block + reward_period;

			NextPoolId::<Test>::put(u32::MAX);

			assert_noop!(
				LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				),
				Error::<Test>::NoAvailablePoolId
			);
		});
	}

	#[test]
	fn pool_creation_fails_with_invalid_block() {
		TestExt::<Test>::default().build().execute_with(|| {
			let reward_asset_id = 1;
			let staked_asset_id = 2;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let lock_start_block = System::block_number() - 1;
			let lock_end_block = lock_start_block + reward_period;

			assert_noop!(
				LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				),
				Error::<Test>::InvalidBlockRange
			);

			let lock_start_block = System::block_number() + 1;
			let lock_end_block = lock_start_block - 1;

			assert_noop!(
				LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				),
				Error::<Test>::InvalidBlockRange
			);
		});
	}

	#[test]
	fn pool_creation_fails_without_balance_in_vault_account() {
		TestExt::<Test>::default().build().execute_with(|| {
			let reward_asset_id = 1;
			let staked_asset_id = 2;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let lock_start_block = System::block_number() + 1;
			let lock_end_block = lock_start_block + reward_period;

			assert_noop!(
				LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				),
				ArithmeticError::Underflow,
			);
		});
	}

	#[test]
	fn user_can_create_pool_successfully() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block,
				));

				let pool_id = NextPoolId::<Test>::get() - 1;

				System::assert_last_event(MockEvent::LiquidityPools(Event::PoolOpen {
					pool_id,
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block,
				}));

				assert_eq!(
					Pools::<Test>::get(pool_id),
					Some(PoolInfo {
						id: pool_id,
						creator: alice(),
						reward_asset_id,
						staked_asset_id,
						interest_rate,
						max_tokens,
						last_updated: 1,
						lock_start_block,
						lock_end_block,
						locked_amount: Zero::zero(),
						pool_status: PoolStatus::Open,
					})
				);
				assert_eq!(NextPoolId::<Test>::get(), pool_id + 1);
				assert_eq!(PoolRelationships::<Test>::get(0), None);
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &alice()), 0);
			});
	}

	#[test]
	fn admin_can_create_multiple_pools_successfully() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 200)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				let pool_id = NextPoolId::<Test>::get();

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				assert_eq!(
					Pools::<Test>::get(pool_id),
					Some(PoolInfo {
						id: pool_id,
						creator: alice(),
						reward_asset_id,
						staked_asset_id,
						interest_rate,
						max_tokens,
						last_updated: 1,
						lock_start_block,
						lock_end_block,
						locked_amount: Zero::zero(),
						pool_status: PoolStatus::Open,
					})
				);
				assert_eq!(NextPoolId::<Test>::get(), pool_id + 1);

				let pool_id = NextPoolId::<Test>::get();
				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));
				assert_eq!(
					Pools::<Test>::get(pool_id),
					Some(PoolInfo {
						id: pool_id,
						creator: alice(),
						reward_asset_id,
						staked_asset_id,
						interest_rate,
						max_tokens,
						last_updated: 1,
						lock_start_block,
						lock_end_block,
						locked_amount: Zero::zero(),
						pool_status: PoolStatus::Open,
					})
				);
				assert_eq!(NextPoolId::<Test>::get(), pool_id + 1);
			});
	}
}

mod set_pool_succession {
	use super::*;

	#[test]
	fn cannot_set_pool_succession_with_non_existent_predecessor() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let successor_id = NextPoolId::<Test>::get() - 1;
				let non_existent_predecessor_id = successor_id + 1;
				assert_noop!(
					LiquidityPools::set_pool_succession(
						RuntimeOrigin::signed(alice()),
						non_existent_predecessor_id,
						successor_id
					),
					Error::<Test>::PredecessorPoolDoesNotExist
				);
			});
	}

	#[test]
	fn cannot_set_pool_succession_with_non_existent_successor() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let predecessor_id = NextPoolId::<Test>::get() - 1;
				let non_existent_successor_id = predecessor_id + 1;
				assert_noop!(
					LiquidityPools::set_pool_succession(
						RuntimeOrigin::signed(alice()),
						predecessor_id,
						non_existent_successor_id
					),
					Error::<Test>::SuccessorPoolDoesNotExist
				);
			});
	}

	#[test]
	fn cannot_set_pool_succession_when_successor_max_tokens_less_than_predecessor() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 1000)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block,
				));

				let predecessor_id = NextPoolId::<Test>::get() - 1;

				let max_tokens = max_tokens - 1;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block,
				));

				let successor_id = NextPoolId::<Test>::get() - 1;

				assert_noop!(
					LiquidityPools::set_pool_succession(
						RuntimeOrigin::signed(alice()),
						predecessor_id,
						successor_id
					),
					Error::<Test>::SuccessorPoolSizeShouldBeGreaterThanPredecessor
				);
			});
	}

	#[test]
	fn cannot_set_pool_succession_when_successor_lock_start_block_less_than_predecessor_lock_end_block(
	) {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 1000)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block,
				));

				let predecessor_id = NextPoolId::<Test>::get() - 1;

				let max_tokens = max_tokens - 1;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block,
				));

				let successor_id = NextPoolId::<Test>::get() - 1;

				assert_noop!(
					LiquidityPools::set_pool_succession(
						RuntimeOrigin::signed(alice()),
						predecessor_id,
						successor_id
					),
					Error::<Test>::SuccessorPoolSizeShouldBeGreaterThanPredecessor
				);
			});
	}

	#[test]
	fn admin_can_set_pool_succession_successfully() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 1000)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block,
				));

				let predecessor_id = NextPoolId::<Test>::get() - 1;

				let max_tokens = max_tokens + 1;

				let lock_start_block = lock_end_block + 1;
				let lock_end_block = lock_start_block + reward_period;
				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block,
				));

				let successor_id = NextPoolId::<Test>::get() - 1;

				assert_ok!(LiquidityPools::set_pool_succession(
					RuntimeOrigin::signed(alice()),
					predecessor_id,
					successor_id
				));

				System::assert_last_event(MockEvent::LiquidityPools(Event::SetSuccession {
					predecessor_pool_id: predecessor_id,
					successor_pool_id: successor_id,
				}));

				assert_eq!(
					PoolRelationships::<Test>::get(predecessor_id),
					Some(PoolRelationship { successor_id: Some(successor_id) })
				);
			});
	}
}

mod set_pool_rollover {
	use super::*;

	#[test]
	fn set_pool_rollover_should_work() {
		let user: AccountId = create_account(1);
		let user_balance = 100;
		let staked_asset_id = 2;
		TestExt::<Test>::default()
			.with_balances(&[(alice(), user_balance)])
			.with_asset(staked_asset_id, "XRP", &[(user, user_balance)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let pool_id = NextPoolId::<Test>::get() - 1;
				let amount = 10;

				assert_ok!(LiquidityPools::enter_pool(
					RuntimeOrigin::signed(user),
					pool_id,
					amount
				));

				// Set rollover preference to true
				assert_ok!(LiquidityPools::set_pool_rollover(
					RuntimeOrigin::signed(user),
					pool_id,
					true
				));

				// Verify the rollover preference is updated
				let user_info = PoolUsers::<Test>::get(pool_id, &user).unwrap();
				assert!(user_info.should_rollover);

				// Verify the UserInfoUpdated event is emitted
				System::assert_last_event(MockEvent::LiquidityPools(Event::UserInfoUpdated {
					pool_id,
					account_id: user,
					should_rollover: true,
				}));
			});
	}

	#[test]
	fn set_pool_rollover_fails_if_pool_does_not_exist() {
		TestExt::<Test>::default().build().execute_with(|| {
			let user: AccountId = create_account(1);
			let non_existent_pool_id = 999;

			// Try to set rollover preference on a non-existent pool
			assert_noop!(
				LiquidityPools::set_pool_rollover(
					RuntimeOrigin::signed(user),
					non_existent_pool_id,
					true
				),
				Error::<Test>::PoolDoesNotExist
			);
		});
	}

	#[test]
	fn set_pool_rollover_fails_if_not_provisioning() {
		let user: AccountId = create_account(1);
		let user_balance = 100;
		let staked_asset_id = 2;
		TestExt::<Test>::default()
			.with_balances(&[(alice(), user_balance)])
			.with_asset(staked_asset_id, "XRP", &[(user, user_balance)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let pool_id = NextPoolId::<Test>::get() - 1;
				let amount = 10;

				assert_ok!(LiquidityPools::enter_pool(
					RuntimeOrigin::signed(user),
					pool_id,
					amount
				));

				let remaining_weight: Weight = ParityDbWeight::get()
					.reads(100u64)
					.saturating_add(ParityDbWeight::get().writes(100u64));
				LiquidityPools::on_idle(lock_start_block, remaining_weight);
				LiquidityPools::on_idle(lock_start_block + 1, remaining_weight);

				// Try to set rollover preference when pool is not provisioning
				assert_noop!(
					LiquidityPools::set_pool_rollover(RuntimeOrigin::signed(user), pool_id, true),
					Error::<Test>::PoolNotOpen
				);
			});
	}

	#[test]
	fn set_pool_rollover_fails_if_user_has_no_tokens_staked() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let pool_id = NextPoolId::<Test>::get();
				let pool_info = PoolInfo {
					id: pool_id,
					creator: alice(),
					reward_asset_id,
					staked_asset_id,
					interest_rate: 1_000_000,
					max_tokens: 100,
					last_updated: 1,
					lock_start_block: System::block_number() + 1,
					lock_end_block: System::block_number() + 100,
					locked_amount: Zero::zero(),
					pool_status: PoolStatus::Open,
				};
				Pools::<Test>::insert(pool_id, pool_info);
				NextPoolId::<Test>::put(pool_id + 1);

				let user: AccountId = create_account(1);

				// Try to set rollover preference when user has no tokens staked
				assert_noop!(
					LiquidityPools::set_pool_rollover(RuntimeOrigin::signed(user), pool_id, true),
					Error::<Test>::NoTokensStaked
				);
			});
	}

	#[test]
	fn set_pool_rollover_fails_due_to_bad_origin() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let pool_id = NextPoolId::<Test>::get();
				let pool_info = PoolInfo {
					id: pool_id,
					creator: alice(),
					reward_asset_id,
					staked_asset_id,
					interest_rate: 1_000_000,
					max_tokens: 100,
					last_updated: 1,
					lock_start_block: System::block_number() + 1,
					lock_end_block: System::block_number() + 100,
					locked_amount: Zero::zero(),
					pool_status: PoolStatus::Open,
				};
				Pools::<Test>::insert(pool_id, pool_info);
				NextPoolId::<Test>::put(pool_id + 1);

				let non_signed_origin = crate::tests::RuntimeOrigin::none();

				// Try to set rollover preference with a bad origin
				assert_noop!(
					LiquidityPools::set_pool_rollover(non_signed_origin, pool_id, true),
					BadOrigin
				);
			});
	}

	#[test]
	fn should_update_user_info() {
		let user: AccountId = create_account(1);
		let user_balance = 100;
		let staked_asset_id = 2;

		TestExt::<Test>::default()
			.with_balances(&[(alice(), user_balance)])
			.with_asset(staked_asset_id, "XRP", &[(user, user_balance)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block,
				));

				let pool_id = NextPoolId::<Test>::get() - 1;
				let amount = 10;

				assert_ok!(LiquidityPools::enter_pool(
					RuntimeOrigin::signed(user),
					pool_id,
					amount
				));

				assert_ok!(LiquidityPools::set_pool_rollover(
					RuntimeOrigin::signed(user),
					pool_id,
					false
				));

				System::assert_last_event(MockEvent::LiquidityPools(Event::UserInfoUpdated {
					pool_id,
					account_id: user,
					should_rollover: false,
				}));

				assert_eq!(
					PoolUsers::<Test>::get(pool_id, user),
					Some(UserInfo {
						amount,
						reward_debt: Zero::zero(),
						should_rollover: false,
						rolled_over: false
					})
				);
			});
	}

	#[test]
	fn should_not_update_for_non_existent_pool() {
		TestExt::<Test>::default().build().execute_with(|| {
			let pool_id = 1;
			let user: AccountId = create_account(1);

			assert_noop!(
				LiquidityPools::set_pool_rollover(RuntimeOrigin::signed(user), pool_id, false),
				Error::<Test>::PoolDoesNotExist
			);
		});
	}

	#[test]
	fn should_not_update_when_pool_closed() {
		let user: AccountId = create_account(1);
		let user_balance = 100;
		TestExt::<Test>::default()
			.with_balances(&[(user, user_balance), (alice(), user_balance)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				Pools::<Test>::insert(
					NextPoolId::<Test>::get(),
					PoolInfo {
						id: NextPoolId::<Test>::get(),
						creator: alice(),
						reward_asset_id,
						staked_asset_id,
						interest_rate,
						max_tokens,
						last_updated: 2,
						lock_start_block,
						lock_end_block,
						locked_amount: Zero::zero(),
						pool_status: PoolStatus::Closed,
					},
				);

				let pool_id = NextPoolId::<Test>::get();

				assert_noop!(
					LiquidityPools::set_pool_rollover(RuntimeOrigin::signed(user), pool_id, false),
					Error::<Test>::PoolNotOpen
				);
			});
	}

	#[test]
	fn should_not_update_for_user_without_tokens() {
		let user: AccountId = create_account(1);
		let user_balance = 100;
		TestExt::<Test>::default()
			.with_balances(&[(user, user_balance), (alice(), user_balance)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block,
				));

				let pool_id = NextPoolId::<Test>::get() - 1;

				assert_noop!(
					LiquidityPools::set_pool_rollover(RuntimeOrigin::signed(user), pool_id, false),
					Error::<Test>::NoTokensStaked
				);
			});
	}
}

mod close_pool {
	use super::*;

	#[test]
	fn cannot_close_non_existent_pool() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				assert_noop!(
					LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), 10),
					Error::<Test>::PoolDoesNotExist
				);
			});
	}

	#[test]
	fn not_pool_creator_cannot_close_pool() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				let pool_id = NextPoolId::<Test>::get();
				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				assert_noop!(
					LiquidityPools::close_pool(RuntimeOrigin::signed(bob()), pool_id),
					Error::<Test>::NotPoolCreator
				);
			});
	}

	#[test]
	fn admin_can_close_pool_successfully() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				assert_eq!(Balances::free_balance(alice()), 0);

				let pool_id = NextPoolId::<Test>::get() - 1;

				assert_eq!(
					Pools::<Test>::get(pool_id),
					Some(PoolInfo {
						id: pool_id,
						creator: alice(),
						reward_asset_id,
						staked_asset_id,
						interest_rate,
						max_tokens,
						last_updated: 1,
						lock_start_block,
						lock_end_block,
						locked_amount: Zero::zero(),
						pool_status: PoolStatus::Open,
					})
				);

				assert_ok!(LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id));

				System::assert_last_event(MockEvent::LiquidityPools(Event::PoolClosed {
					pool_id,
					reward_asset_amount: 100,
					staked_asset_amount: 0,
					receiver: alice(),
				}));

				assert_eq!(Pools::<Test>::get(pool_id), None);
				assert_eq!(RolloverPivot::<Test>::get(pool_id), vec![]);
				assert_eq!(PoolRelationships::<Test>::get(pool_id), None);
				assert_eq!(Balances::free_balance(alice()), 100);
			});
	}

	#[test]
	fn cannot_close_already_closed_pool() {
		let user: AccountId = create_account(12);
		let user_balance = 100;
		let staked_asset_id = 2;

		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.with_asset(staked_asset_id, "XRP", &[(user, user_balance)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				let pool_id = NextPoolId::<Test>::get();
				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				// Some user enters the pool, meaning it will stay alive but in the closed state
				assert_ok!(LiquidityPools::enter_pool(
					RuntimeOrigin::signed(user),
					pool_id,
					user_balance
				));

				// Close pool first time successfully
				assert_ok!(LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id));
				System::assert_last_event(MockEvent::LiquidityPools(Event::PoolClosed {
					pool_id,
					reward_asset_amount: 0,
					staked_asset_amount: 100,
					receiver: alice(),
				}));

				assert_eq!(Pools::<Test>::get(pool_id).unwrap().pool_status, PoolStatus::Closed);
				assert!(!RolloverPivot::<Test>::contains_key(pool_id));
				assert!(!PoolRelationships::<Test>::contains_key(pool_id));
				assert_eq!(Balances::free_balance(alice()), 100);
				// Alice does not get refunded the users staked asset
				assert_eq!(AssetsExt::balance(staked_asset_id, &alice()), 0);

				// Try to close the pool again fails as it is already closed
				assert_noop!(
					LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id),
					Error::<Test>::PoolAlreadyClosed
				);
			});
	}

	// This is a weird test, but it ensures that if a user enters a pool, then exits it, the pool
	// will correctly close and remove the pool from storage immediately
	#[test]
	fn pool_with_no_staked_balance_closes_fully() {
		let user: AccountId = create_account(12);
		let user_balance = 100;
		let staked_asset_id = 2;

		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.with_asset(staked_asset_id, "XRP", &[(user, user_balance)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				let pool_id = NextPoolId::<Test>::get();
				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				// Some user enters the pool, adding to locked balance
				assert_ok!(LiquidityPools::enter_pool(
					RuntimeOrigin::signed(user),
					pool_id,
					user_balance
				));
				assert_eq!(Pools::<Test>::get(pool_id).unwrap().locked_amount, user_balance);
				assert_eq!(AssetsExt::balance(staked_asset_id, &user), 0);

				// User exits the pool, removing their locked balance
				assert_ok!(LiquidityPools::exit_pool(RuntimeOrigin::signed(user), pool_id,));
				assert_eq!(Pools::<Test>::get(pool_id).unwrap().locked_amount, 0);
				assert_eq!(AssetsExt::balance(staked_asset_id, &user), user_balance);

				// Close pool successfully and remove from storage
				assert_ok!(LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id));
				System::assert_last_event(MockEvent::LiquidityPools(Event::PoolClosed {
					pool_id,
					reward_asset_amount: 0,
					staked_asset_amount: 0,
					receiver: alice(),
				}));

				assert!(!Pools::<Test>::contains_key(pool_id));
				assert!(!RolloverPivot::<Test>::contains_key(pool_id));
				assert!(!PoolRelationships::<Test>::contains_key(pool_id));
				assert_eq!(Balances::free_balance(alice()), 100);
			});
	}
}

mod enter_pool {
	use super::*;

	#[test]
	fn invalid_origin_cannot_enter_pool() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_noop!(LiquidityPools::enter_pool(RuntimeOrigin::none(), 0, 100), BadOrigin);
		});
	}

	#[test]
	fn cannot_join_non_existent_pool() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				assert_noop!(
					LiquidityPools::enter_pool(RuntimeOrigin::signed(alice()), 0, 100),
					Error::<Test>::PoolDoesNotExist
				);
			});
	}

	#[test]
	fn cannot_enter_pool_after_lock_end_block() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				let pool_id = NextPoolId::<Test>::get();
				Pools::<Test>::insert(
					pool_id,
					PoolInfo {
						id: pool_id,
						creator: alice(),
						reward_asset_id,
						staked_asset_id,
						interest_rate,
						max_tokens,
						last_updated: 1,
						lock_start_block,
						lock_end_block,
						locked_amount: Zero::zero(),
						pool_status: PoolStatus::Closed,
					},
				);

				assert_noop!(
					LiquidityPools::enter_pool(RuntimeOrigin::signed(alice()), pool_id, 10),
					Error::<Test>::PoolNotOpen
				);
			});
	}

	#[test]
	fn cannot_enter_pool_if_token_limit_exceeded() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let pool_id = NextPoolId::<Test>::get() - 1;

				assert_noop!(
					LiquidityPools::enter_pool(
						RuntimeOrigin::signed(alice()),
						pool_id,
						max_tokens + 1
					),
					Error::<Test>::StakingLimitExceeded
				);
			});
	}

	#[test]
	fn cannot_enter_pool_when_not_provisioning() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let pool_id = NextPoolId::<Test>::get() - 1;
				let amount = 10;

				// Simulate the pool moving to a different state
				Pools::<Test>::mutate(pool_id, |pool| {
					*pool = Some(PoolInfo {
						pool_status: PoolStatus::Closed, // Not Provisioning
						..pool.clone().unwrap()
					});
				});

				assert_noop!(
					LiquidityPools::enter_pool(RuntimeOrigin::signed(alice()), pool_id, amount),
					Error::<Test>::PoolNotOpen
				);
			});
	}

	#[test]
	fn cannot_enter_pool_without_sufficient_root_balance() {
		let staked_asset_id = 2;
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.with_asset(staked_asset_id, "XRP", &[(alice(), 0)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let pool_id = NextPoolId::<Test>::get() - 1;

				assert_noop!(
					LiquidityPools::enter_pool(RuntimeOrigin::signed(alice()), pool_id, 10),
					ArithmeticError::Underflow
				);
			});
	}

	#[test]
	fn can_enter_pool_successfully() {
		let user: AccountId = create_account(1);
		let user_balance = 100;
		let staked_asset_id = 2;

		TestExt::<Test>::default()
			.with_balances(&[(alice(), 100)])
			.with_asset(staked_asset_id, "XRP", &[(user, user_balance)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let pool_id = NextPoolId::<Test>::get() - 1;
				let amount = 10;

				assert_ok!(LiquidityPools::enter_pool(
					RuntimeOrigin::signed(user),
					pool_id,
					amount
				));

				System::assert_last_event(MockEvent::LiquidityPools(Event::UserJoined {
					account_id: user,
					pool_id,
					amount,
				}));

				assert_eq!(AssetsExt::balance(staked_asset_id, &user), user_balance - amount);

				assert_eq!(
					Pools::<Test>::get(pool_id),
					Some(PoolInfo {
						id: pool_id,
						creator: alice(),
						reward_asset_id,
						staked_asset_id,
						interest_rate,
						max_tokens,
						last_updated: 1,
						lock_start_block,
						lock_end_block,
						locked_amount: amount,
						pool_status: PoolStatus::Open,
					})
				);

				assert_eq!(
					PoolUsers::<Test>::get(pool_id, user),
					Some(UserInfo {
						amount,
						reward_debt: Zero::zero(),
						should_rollover: true,
						rolled_over: false
					})
				);
			});
	}
}

#[test]
fn can_refund_back_when_pool_is_done() {
	let user: AccountId = create_account(1);
	let user_balance = 100;
	let vault_balance = 1000;
	let staked_asset_id = 2;

	TestExt::<Test>::default()
		.with_balances(&[(alice(), vault_balance)])
		.with_asset(staked_asset_id, "XRP", &[(user, user_balance)])
		.build()
		.execute_with(|| {
			let reward_asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 500;
			let reward_period = 100;
			let lock_start_block = System::block_number() + 1;
			let lock_end_block = lock_start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				RuntimeOrigin::signed(alice()),
				reward_asset_id,
				staked_asset_id,
				interest_rate,
				max_tokens,
				lock_start_block,
				lock_end_block
			));

			assert_eq!(AssetsExt::balance(reward_asset_id, &alice()), vault_balance);

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			assert_ok!(LiquidityPools::enter_pool(RuntimeOrigin::signed(user), pool_id, amount));

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(lock_start_block, remaining_weight);

			LiquidityPools::on_idle(lock_end_block, remaining_weight);

			assert_eq!(AssetsExt::balance(reward_asset_id, &alice()), vault_balance);
		});
}

mod exit_pool {
	use super::*;

	#[test]
	fn invalid_origin_cannot_exit_pool() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_noop!(LiquidityPools::exit_pool(RuntimeOrigin::none(), 0), BadOrigin);
		});
	}

	#[test]
	fn cannot_exit_non_existent_pool() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				assert_noop!(
					LiquidityPools::exit_pool(RuntimeOrigin::signed(alice()), 0),
					Error::<Test>::PoolDoesNotExist
				);
			});
	}

	#[test]
	fn cannot_exit_pool_when_not_joined() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let pool_id = NextPoolId::<Test>::get() - 1;
				Pools::<Test>::mutate(NextPoolId::<Test>::get() - 1, |pool| {
					*pool = Some(PoolInfo {
						pool_status: PoolStatus::Open, // Not Provisioning
						..pool.clone().unwrap()
					});
				});

				assert_noop!(
					LiquidityPools::exit_pool(RuntimeOrigin::signed(alice()), pool_id),
					Error::<Test>::NoTokensStaked
				);
			});
	}

	#[test]
	fn cannot_exit_pool_with_wrong_pool_status() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let pool_id = NextPoolId::<Test>::get();
				let mut pool_info = PoolInfo {
					id: pool_id,
					creator: alice(),
					pool_status: PoolStatus::Renewing,
					..Default::default()
				};

				// Cannot exit when pool in Renewing status
				Pools::<Test>::insert(pool_id, &pool_info);
				assert_noop!(
					LiquidityPools::exit_pool(RuntimeOrigin::signed(alice()), pool_id),
					Error::<Test>::CannotExitPool
				);

				// Cannot exit when pool in Matured status
				pool_info.pool_status = PoolStatus::Matured;
				Pools::<Test>::insert(pool_id, &pool_info);
				assert_noop!(
					LiquidityPools::exit_pool(RuntimeOrigin::signed(alice()), pool_id),
					Error::<Test>::CannotExitPool
				);

				// Cannot exit when pool in Started status
				pool_info.pool_status = PoolStatus::Started;
				Pools::<Test>::insert(pool_id, &pool_info);
				assert_noop!(
					LiquidityPools::exit_pool(RuntimeOrigin::signed(alice()), pool_id),
					Error::<Test>::CannotExitPool
				);
			});
	}

	#[test]
	fn cannot_exit_pool_without_previously_depositing_token() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let pool_id = NextPoolId::<Test>::get() - 1;

				assert_noop!(
					LiquidityPools::exit_pool(RuntimeOrigin::signed(alice()), pool_id),
					Error::<Test>::NoTokensStaked
				);

				PoolUsers::<Test>::insert(pool_id, create_account(1), UserInfo::default());

				assert_noop!(
					LiquidityPools::exit_pool(RuntimeOrigin::signed(alice()), pool_id),
					Error::<Test>::NoTokensStaked
				);
			});
	}

	#[test]
	fn can_exit_closed_pool_successfully() {
		let user_count = 10;
		let mut users: Vec<AccountId> = Vec::with_capacity(user_count);
		let mut user_balances: Vec<(AccountId, u128)> = Vec::with_capacity(user_count);
		let user_balance = 100;
		let mut total_balance: u128 = 0;
		for i in 1..=user_count {
			let user: AccountId = create_account(i as u64 + 10);
			let balance = user_balance * i as u128;
			total_balance += balance;
			users.push(user);
			user_balances.push((user, balance));
		}
		let staked_asset_id = 2;
		let reward_asset_id = 3;
		let max_tokens = 100000;

		TestExt::<Test>::default()
			.with_xrp_balances(&user_balances)
			.with_asset(reward_asset_id, "REW", &[(alice(), max_tokens)])
			.build()
			.execute_with(|| {
				let interest_rate = 1_000_000;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				let pool_id = NextPoolId::<Test>::get();
				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));
				assert_eq!(AssetsExt::balance(reward_asset_id, &alice()), 0);

				// Enter the pool with multiple users
				for (user, balance) in &user_balances {
					assert_ok!(LiquidityPools::enter_pool(
						RuntimeOrigin::signed(*user),
						pool_id,
						*balance
					));
				}

				// Close the pool
				assert_ok!(LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id));
				assert_eq!(Pools::<Test>::get(pool_id).unwrap().pool_status, PoolStatus::Closed);
				System::assert_last_event(MockEvent::LiquidityPools(Event::PoolClosed {
					pool_id,
					reward_asset_amount: max_tokens,
					staked_asset_amount: total_balance, // All staked assets
					receiver: alice(),
				}));
				// Sorry Alice, you don't get the users staked assets
				assert_eq!(AssetsExt::balance(staked_asset_id, &alice()), 0);
				assert_eq!(AssetsExt::balance(reward_asset_id, &alice()), max_tokens);

				// Exit the pool for each user
				for (user, amount) in &user_balances {
					assert!(Pools::<Test>::contains_key(pool_id)); // Pool should still exist
					assert_ok!(LiquidityPools::exit_pool(RuntimeOrigin::signed(*user), pool_id));
					System::assert_last_event(MockEvent::LiquidityPools(Event::UserExited {
						account_id: *user,
						pool_id,
						amount: *amount,
					}));
					assert_eq!(AssetsExt::balance(staked_asset_id, user), *amount);
					assert!(PoolUsers::<Test>::get(pool_id, user).is_none());
				}
				// After all users exit, the pool should be removed automatically
				assert!(!Pools::<Test>::contains_key(pool_id));
			});
	}

	#[test]
	fn exiting_open_pool_does_not_remove() {
		let user: AccountId = create_account(1);
		let user_balance = 100;
		let staked_asset_id = 2;
		let reward_asset_id = 3;
		let max_tokens = 100000;

		TestExt::<Test>::default()
			.with_xrp_balances(&[(user, user_balance)])
			.with_asset(reward_asset_id, "REW", &[(alice(), max_tokens)])
			.build()
			.execute_with(|| {
				let interest_rate = 1_000_000;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				let pool_id = NextPoolId::<Test>::get();
				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				// Enter and exit the pool
				assert_ok!(LiquidityPools::enter_pool(
					RuntimeOrigin::signed(user),
					pool_id,
					user_balance
				));
				assert_ok!(LiquidityPools::exit_pool(RuntimeOrigin::signed(user), pool_id));

				// Pool is still Open despite having zero locked amount
				assert_eq!(Pools::<Test>::get(pool_id).unwrap().pool_status, PoolStatus::Open);
				assert_eq!(Pools::<Test>::get(pool_id).unwrap().locked_amount, 0);
			});
	}

	#[test]
	fn can_exit_pool_successfully() {
		let user: AccountId = create_account(1);
		let user_balance = 100;
		let staked_asset_id = 2;

		TestExt::<Test>::default()
			.with_balances(&[(alice(), 100)])
			.with_asset(staked_asset_id, "XRP", &[(user, user_balance)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let pool_id = NextPoolId::<Test>::get() - 1;
				let amount = 10;

				assert_ok!(LiquidityPools::enter_pool(
					RuntimeOrigin::signed(user),
					pool_id,
					amount
				));

				assert_ok!(LiquidityPools::exit_pool(RuntimeOrigin::signed(user), pool_id));

				System::assert_last_event(MockEvent::LiquidityPools(Event::UserExited {
					account_id: user,
					pool_id,
					amount,
				}));

				assert_eq!(AssetsExt::balance(staked_asset_id, &user), user_balance);

				assert_eq!(
					Pools::<Test>::get(pool_id),
					Some(PoolInfo {
						id: pool_id,
						creator: alice(),
						reward_asset_id,
						staked_asset_id,
						interest_rate,
						max_tokens,
						last_updated: 1,
						lock_start_block,
						lock_end_block,
						locked_amount: Zero::zero(),
						pool_status: PoolStatus::Open,
					})
				);

				assert_eq!(PoolUsers::<Test>::get(pool_id, user), None);
			});
	}
}

mod claim_reward {
	use super::*;

	#[test]
	fn claim_reward_should_work() {
		let user_balance = 100;
		let initial_balance = user_balance * 100;

		let endowments = (1..100)
			.map(|account_id| (create_account(account_id), user_balance))
			.collect::<Vec<_>>();

		TestExt::<Test>::default()
			.with_balances(&[(alice(), initial_balance)])
			.configure_root()
			.with_asset(XRP_ASSET_ID, "XRP", &endowments)
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = XRP_ASSET_ID;
				let interest_rate = 1_000_000;
				let max_tokens = 100 * 50;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				let pool_id = NextPoolId::<Test>::get();

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let amount = 10;

				for account_id in 1..100 {
					let user: AccountId = create_account(account_id);
					assert_ok!(LiquidityPools::enter_pool(
						RuntimeOrigin::signed(user),
						pool_id,
						amount
					));
				}

				// progress time to end of reward period
				let remaining_weight: Weight = ParityDbWeight::get()
					.reads(100u64)
					.saturating_add(ParityDbWeight::get().writes(100u64));
				LiquidityPools::on_idle(lock_start_block, remaining_weight);
				LiquidityPools::on_idle(lock_start_block + 1, remaining_weight);
				LiquidityPools::on_idle(lock_end_block, remaining_weight);

				System::set_block_number(lock_end_block + 1);
				assert_ok!(LiquidityPools::rollover_unsigned(
					RuntimeOrigin::none(),
					pool_id,
					System::block_number()
				));

				for account_id in 1..100 {
					let user: AccountId = create_account(account_id);
					assert_ok!(LiquidityPools::claim_reward(RuntimeOrigin::signed(user), pool_id));

					System::assert_last_event(MockEvent::LiquidityPools(Event::RewardsClaimed {
						account_id: user,
						pool_id,
						amount,
					}));

					assert_eq!(AssetsExt::balance(staked_asset_id, &user), user_balance);
					assert_eq!(AssetsExt::balance(reward_asset_id, &user), amount);
				}
			});
	}

	#[test]
	fn claim_reward_should_work_when_not_rollover() {
		let user_balance = 100;
		let initial_balance = user_balance * 100;

		let endowments = (1..100)
			.map(|account_id| (create_account(account_id), user_balance))
			.collect::<Vec<_>>();

		TestExt::<Test>::default()
			.configure_root()
			.with_balances(&[(alice(), initial_balance)])
			.with_asset(XRP_ASSET_ID, "XRP", &endowments)
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = XRP_ASSET_ID;
				let interest_rate = 1_000_000;
				let max_tokens = 100 * 50;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				let pool_id = NextPoolId::<Test>::get();
				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let amount = 10;
				for account_id in 1..100 {
					let user: AccountId = create_account(account_id);
					assert_ok!(LiquidityPools::enter_pool(
						RuntimeOrigin::signed(user),
						pool_id,
						amount
					));
					assert_ok!(LiquidityPools::set_pool_rollover(
						RuntimeOrigin::signed(user),
						pool_id,
						false
					));
				}

				// progress time to end of reward period
				let remaining_weight: Weight = ParityDbWeight::get()
					.reads(100u64)
					.saturating_add(ParityDbWeight::get().writes(100u64));
				LiquidityPools::on_idle(lock_start_block, remaining_weight);
				LiquidityPools::on_idle(lock_start_block + 1, remaining_weight);
				LiquidityPools::on_idle(lock_end_block, remaining_weight);
				System::set_block_number(lock_end_block + 1);

				for account_id in 1..100 {
					let user: AccountId = create_account(account_id);
					assert_ok!(LiquidityPools::claim_reward(RuntimeOrigin::signed(user), pool_id));

					System::assert_last_event(MockEvent::LiquidityPools(Event::RewardsClaimed {
						account_id: user,
						pool_id,
						amount,
					}));

					assert_eq!(AssetsExt::balance(staked_asset_id, &user), user_balance);
					assert_eq!(AssetsExt::balance(reward_asset_id, &user), amount);
				}
			});
	}

	#[test]
	fn claim_reward_should_fail_if_no_tokens_staked() {
		let user: AccountId = create_account(1);
		let user_balance = 100;

		TestExt::<Test>::default()
			.with_balances(&[(user, user_balance), (alice(), user_balance)])
			.with_asset(XRP_ASSET_ID, "XRP", &[(user, user_balance)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = XRP_ASSET_ID;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				let pool_id = NextPoolId::<Test>::get();

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				// progress time to end of reward period
				let remaining_weight: Weight = ParityDbWeight::get()
					.reads(100u64)
					.saturating_add(ParityDbWeight::get().writes(100u64));
				LiquidityPools::on_idle(lock_start_block, remaining_weight);
				LiquidityPools::on_idle(lock_end_block, remaining_weight);

				assert_noop!(
					LiquidityPools::claim_reward(RuntimeOrigin::signed(user), pool_id),
					Error::<Test>::NoTokensStaked
				);
			});
	}

	#[test]
	fn claim_reward_should_fail_if_pool_does_not_exist() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let pool_id = 1;
				let user: AccountId = create_account(1);
				let amount = 10;

				assert_noop!(
					LiquidityPools::enter_pool(RuntimeOrigin::signed(user), pool_id, amount),
					Error::<Test>::PoolDoesNotExist
				);
			});
	}

	#[test]
	fn claim_reward_should_fail_if_pool_status_is_not_done() {
		let user_balance = 100;
		let user: AccountId = create_account(1);

		TestExt::<Test>::default()
			.with_balances(&[(alice(), user_balance)])
			.with_asset(XRP_ASSET_ID, "XRP", &[(user, user_balance)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = XRP_ASSET_ID;
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let pool_id = NextPoolId::<Test>::get() - 1;

				assert_ok!(LiquidityPools::enter_pool(RuntimeOrigin::signed(user), pool_id, 10));

				let remaining_weight: Weight = ParityDbWeight::get()
					.reads(100u64)
					.saturating_add(ParityDbWeight::get().writes(100u64));
				LiquidityPools::on_idle(lock_start_block, remaining_weight);
				LiquidityPools::on_idle(lock_start_block + 1, remaining_weight);

				assert_noop!(
					LiquidityPools::claim_reward(RuntimeOrigin::signed(user), pool_id),
					Error::<Test>::NotReadyForClaimingReward
				);
			});
	}
}

mod rollover_unsigned {
	use super::*;

	#[test]
	fn rollover_should_work() {
		let user_balance = 100;
		let user_amount = 100;
		let opt_out_rollover_amount = 10;

		let endowments = (1..=user_amount + opt_out_rollover_amount)
			.map(|account_id| (create_account(account_id), user_balance))
			.collect::<Vec<_>>();

		TestExt::<Test>::default()
			.configure_root()
			.with_balances(&[(alice(), user_balance * 100)])
			.with_asset(XRP_ASSET_ID, "XRP", &endowments)
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = XRP_ASSET_ID;
				let interest_rate = 1_000_000;
				let max_tokens = 100 * 50;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));
				let predecessor_id = NextPoolId::<Test>::get() - 1;

				let amount = 10;

				// 100 users default opt-in to rollover
				for account_id in 1..=user_amount {
					let user: AccountId = create_account(account_id);
					assert_ok!(LiquidityPools::enter_pool(
						RuntimeOrigin::signed(user),
						predecessor_id,
						amount
					));
				}
				// 10 users explicitly opted out of rollover
				for account_id in (user_amount + 1)..=(user_amount + opt_out_rollover_amount) {
					let user: AccountId = create_account(account_id);
					assert_ok!(LiquidityPools::enter_pool(
						RuntimeOrigin::signed(user),
						predecessor_id,
						amount
					));
					assert_ok!(LiquidityPools::set_pool_rollover(
						RuntimeOrigin::signed(user),
						predecessor_id,
						false
					));
				}

				// Check that the pool is open and has accumulated the correct amount of locked tokens
				assert_eq!(
					Pools::<Test>::get(predecessor_id),
					Some(PoolInfo {
						id: predecessor_id,
						creator: alice(),
						reward_asset_id,
						staked_asset_id,
						interest_rate,
						max_tokens,
						last_updated: 1,
						lock_start_block: 2,
						lock_end_block: 102,
						locked_amount: amount * ((user_amount + opt_out_rollover_amount) as u128),
						pool_status: PoolStatus::Open
					})
				);

				// Create successor pool for the next period
				let lock_start_block = lock_end_block + 1;
				let lock_end_block = lock_start_block + reward_period;
				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));
				let successor_id = NextPoolId::<Test>::get() - 1;

				// Set the successor pool for the next period
				assert_ok!(LiquidityPools::set_pool_succession(
					RuntimeOrigin::signed(alice()),
					predecessor_id,
					successor_id
				));

				// Progress time to end of reward period
				let remaining_weight: Weight = ParityDbWeight::get()
					.reads(100u64)
					.saturating_add(ParityDbWeight::get().writes(100u64));
				LiquidityPools::on_idle(lock_start_block, remaining_weight);
				LiquidityPools::on_idle(lock_start_block + 1, remaining_weight);

				// Simulate rollover process
				System::set_block_number(reward_period);

				// Give some time for the rollover to be processed
				for _block_bump in 1..110 {
					LiquidityPools::on_idle(System::block_number(), remaining_weight);
					System::set_block_number(System::block_number() + 1);

					assert_ok!(LiquidityPools::rollover_unsigned(
						RuntimeOrigin::none(),
						predecessor_id,
						System::block_number()
					));
				}

				assert_eq!(
					Pools::<Test>::get(predecessor_id),
					Some(PoolInfo {
						id: predecessor_id,
						creator: alice(),
						reward_asset_id,
						staked_asset_id,
						interest_rate,
						max_tokens,
						last_updated: 111,
						lock_start_block: 2,
						lock_end_block: 102,
						locked_amount: opt_out_rollover_amount as u128 * amount,
						pool_status: PoolStatus::Matured
					})
				);

				// 100 user default opt-in rollover should be not be refunded joined asset amount
				for account_id in 1..=user_amount {
					let user: AccountId = create_account(account_id);
					assert_ok!(LiquidityPools::claim_reward(
						RuntimeOrigin::signed(user),
						predecessor_id
					));
					assert_eq!(AssetsExt::balance(staked_asset_id, &user), user_balance - amount);
				}
				// 10 user opt-out rollover should be refunded joined asset amount
				for account_id in (user_amount + 1)..=(user_amount + opt_out_rollover_amount) {
					let user: AccountId = create_account(account_id);
					assert_ok!(LiquidityPools::claim_reward(
						RuntimeOrigin::signed(user),
						predecessor_id
					));
					assert_eq!(AssetsExt::balance(staked_asset_id, &user), user_balance);
				}
			});
	}

	#[test]
	fn rollover_should_work_when_exceeding_successor_pool_maxtokens() {
		let user_balance = 10_000;
		let user_amount = 100;

		let endowments = (1..=user_amount)
			.map(|account_id| (create_account(account_id), user_balance))
			.collect::<Vec<_>>();

		TestExt::<Test>::default()
			.with_balances(&[(alice(), user_balance * 100)])
			.with_asset(XRP_ASSET_ID, "XRP", &endowments)
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = XRP_ASSET_ID;
				let interest_rate = 1_000_000;
				let max_tokens = 100 * 50;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block
				));

				let predecessor_id = NextPoolId::<Test>::get() - 1;

				let lock_start_block = lock_end_block + 1;
				let lock_end_block_2 = lock_end_block + 100;
				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					interest_rate,
					max_tokens,
					lock_start_block,
					lock_end_block_2
				));

				let successor_id = NextPoolId::<Test>::get() - 1;

				let amount = 10;

				for account_id in 1..=user_amount {
					let user: AccountId = create_account(account_id);
					assert_ok!(LiquidityPools::enter_pool(
						RuntimeOrigin::signed(user),
						predecessor_id,
						amount
					));
				}

				// Join the successor pool
				assert_ok!(LiquidityPools::enter_pool(
					RuntimeOrigin::signed(endowments[0].0),
					successor_id,
					max_tokens - amount * (user_amount as u128 / 2),
				));

				assert_eq!(
					Pools::<Test>::get(predecessor_id),
					Some(PoolInfo {
						id: predecessor_id,
						creator: alice(),
						reward_asset_id,
						staked_asset_id,
						interest_rate,
						max_tokens,
						last_updated: 1,
						lock_start_block: 2,
						lock_end_block: 102,
						locked_amount: amount * ((user_amount) as u128),
						pool_status: PoolStatus::Open
					})
				);

				assert_ok!(LiquidityPools::set_pool_succession(
					RuntimeOrigin::signed(alice()),
					predecessor_id,
					successor_id
				));

				let remaining_weight: Weight = ParityDbWeight::get()
					.reads(100u64)
					.saturating_add(ParityDbWeight::get().writes(100u64));
				LiquidityPools::on_idle(lock_start_block, remaining_weight);
				LiquidityPools::on_idle(lock_start_block + 1, remaining_weight);

				// Simulate rollover process
				System::set_block_number(reward_period);

				// Give some time for the rollover to be processed
				for _block_bump in 1..100 {
					LiquidityPools::on_idle(System::block_number(), remaining_weight);
					System::set_block_number(System::block_number() + 1);

					assert_ok!(LiquidityPools::rollover_unsigned(
						RuntimeOrigin::none(),
						predecessor_id,
						System::block_number()
					));
				}

				assert_eq!(
					Pools::<Test>::get(predecessor_id),
					Some(PoolInfo {
						id: predecessor_id,
						creator: alice(),
						reward_asset_id,
						staked_asset_id,
						interest_rate,
						max_tokens,
						last_updated: 105,
						lock_start_block: 2,
						lock_end_block: 102,
						locked_amount: user_amount as u128 / 2 * amount,
						pool_status: PoolStatus::Matured
					})
				);

				assert_eq!(
					Pools::<Test>::get(successor_id),
					Some(PoolInfo {
						id: successor_id,
						creator: alice(),
						reward_asset_id,
						staked_asset_id,
						interest_rate,
						max_tokens,
						last_updated: 105,
						lock_start_block,
						lock_end_block: lock_end_block_2,
						locked_amount: max_tokens,
						pool_status: PoolStatus::Started
					})
				);
			});
	}

	#[test]
	fn rollover_should_fail_when_pool_not_exist() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				assert_noop!(
					LiquidityPools::rollover_unsigned(
						RuntimeOrigin::none(),
						1,
						System::block_number()
					),
					Error::<Test>::PoolDoesNotExist
				);
			});
	}

	#[test]
	fn rollover_should_fail_when_successor_pool_not_exist() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 100)])
			.build()
			.execute_with(|| {
				let precessor_pool_id = 1;
				Pools::<Test>::insert(
					precessor_pool_id,
					PoolInfo {
						id: precessor_pool_id,
						creator: alice(),
						reward_asset_id: 1,
						staked_asset_id: XRP_ASSET_ID,
						interest_rate: 1_000_000,
						max_tokens: 100,
						last_updated: 0,
						lock_start_block: 0,
						lock_end_block: 0,
						locked_amount: 0,
						pool_status: PoolStatus::Renewing,
					},
				);

				let successor_pool_id = 2;
				PoolRelationships::<Test>::insert(
					&precessor_pool_id,
					PoolRelationship { successor_id: Some(successor_pool_id) },
				);

				assert_noop!(
					LiquidityPools::rollover_unsigned(
						RuntimeOrigin::none(),
						precessor_pool_id,
						System::block_number()
					),
					Error::<Test>::PoolDoesNotExist
				);
			});
	}
}

mod calculate_reward {
	use super::*;

	#[test]
	fn test_calculate_reward_basic() {
		// Test with basic values where no overflow or saturation should occur
		let user_joined_amount: Balance = 1000;
		let interest_rate: u32 = 10_000; // 100% in basis points
		let reward_debt: Balance = 0;
		let asset_decimals: u8 = 10;
		let native_decimals: u8 = 10;
		let interest_rate_base_point: u32 = 10000;

		let reward = LiquidityPools::calculate_reward(
			user_joined_amount,
			reward_debt,
			interest_rate,
			interest_rate_base_point,
			asset_decimals,
			native_decimals,
		)
			.unwrap();

		assert_eq!(reward, user_joined_amount); // Reward should be equal to the staked amount for 100%
	}

	#[test]
	fn test_calculate_reward_with_debt() {
		// Test calculation when there is existing reward debt
		let user_joined_amount: Balance = 1000;
		let interest_rate: u32 = 5000; // 50% in basis points
		let reward_debt: Balance = 250; // Existing debt
		let asset_decimals: u8 = 10;
		let native_decimals: u8 = 10;
		let interest_rate_base_point: u32 = 10_000;

		let reward = LiquidityPools::calculate_reward(
			user_joined_amount,
			reward_debt,
			interest_rate,
			interest_rate_base_point,
			asset_decimals,
			native_decimals,
		)
			.unwrap();

		let expected_reward = (user_joined_amount / 2) - reward_debt; // Half of the amount minus debt
		assert_eq!(reward, expected_reward);
	}

	#[test]
	fn test_calculate_reward_decimal_conversion() {
		// Test the conversion between different decimal places
		let user_joined_amount: Balance = 1000;
		let interest_rate: u32 = 10000; // 100% in basis points
		let reward_debt: Balance = 0;
		let asset_decimals: u8 = 8; // Asset has less decimals
		let native_decimals: u8 = 10; // Native token has more decimals
		let interest_rate_base_point: u32 = 10000;

		let reward = LiquidityPools::calculate_reward(
			user_joined_amount,
			reward_debt,
			interest_rate,
			interest_rate_base_point,
			asset_decimals,
			native_decimals,
		)
			.unwrap();

		let expected_reward = user_joined_amount * 100; // Account for the decimal difference
		assert_eq!(reward, expected_reward);
	}

	#[test]
	fn test_calculate_reward_zero_interest() {
		// Test for zero interest rate resulting in no reward
		let user_joined_amount: Balance = 1000;
		let interest_rate: u32 = 0; // 0% interest rate
		let reward_debt: Balance = 0;
		let asset_decimals: u8 = 10;
		let native_decimals: u8 = 10;
		let interest_rate_base_point: u32 = 10000;

		let reward = LiquidityPools::calculate_reward(
			user_joined_amount,
			reward_debt,
			interest_rate,
			interest_rate_base_point,
			asset_decimals,
			native_decimals,
		)
			.unwrap();

		assert_eq!(reward, 0); // Reward should be zero
	}

	#[test]
	fn test_calculate_reward_less_than_debt() {
		// Test when the reward debt is greater than the calculated reward
		let user_joined_amount: Balance = 1000;
		let interest_rate: u32 = 5000; // 50% interest rate
		let reward_debt: Balance = 600; // Reward debt greater than half of the staked amount
		let asset_decimals: u8 = 10;
		let native_decimals: u8 = 10;
		let interest_rate_base_point: u32 = 10000;

		let reward = LiquidityPools::calculate_reward(
			user_joined_amount,
			reward_debt,
			interest_rate,
			interest_rate_base_point,
			asset_decimals,
			native_decimals,
		)
			.unwrap();

		assert!(
			reward.is_zero(),
			"Reward should be zero or negative, therefore it's zero after saturation"
		);
	}

	#[test]
	fn test_calculate_reward_overflow() {
		// Test for overflow conditions
		let user_joined_amount: Balance = Balance::max_value() / 2;
		let interest_rate: u32 = 20000; // 200% interest rate
		let reward_debt: Balance = 0;
		let asset_decimals: u8 = 10;
		let native_decimals: u8 = 10;
		let interest_rate_base_point: u32 = 10000;

		let reward = LiquidityPools::calculate_reward(
			user_joined_amount,
			reward_debt,
			interest_rate,
			interest_rate_base_point,
			asset_decimals,
			native_decimals,
		)
			.unwrap();

		// Ensure the reward does not exceed the maximum balance after calculation
		assert!(reward <= Balance::max_value(), "Reward should not overflow");
	}

	#[test]
	fn test_calculate_reward_decimal_conversion_issues() {
		// Test for conversion issues with different decimals
		let user_joined_amount: Balance = 1000;
		let interest_rate: u32 = 10000; // 100% interest rate
		let reward_debt: Balance = 0;
		let asset_decimals: u8 = 6; // Asset has fewer decimals
		let native_decimals: u8 = 18; // Native token has more decimals
		let interest_rate_base_point: u32 = 10000;

		let reward = LiquidityPools::calculate_reward(
			user_joined_amount,
			reward_debt,
			interest_rate,
			interest_rate_base_point,
			asset_decimals,
			native_decimals,
		)
			.unwrap();

		// The expected reward should consider the difference in decimals
		let expected_reward =
			user_joined_amount * 10_u128.pow((native_decimals - asset_decimals) as u32);
		assert_eq!(
			reward, expected_reward,
			"Reward should be correctly converted based on decimals"
		);
	}

	#[test]
	fn test_calculate_reward_fails_with_overflow() {
		TestExt::<Test>::default().build().execute_with(|| {
			let interest_rate = u32::MAX;
			let max_tokens = Balance::MAX;
			let reward_debt: Balance = 0;
			let asset_decimals: u8 = 6;
			let native_decimals: u8 = 6;
			let interest_rate_base_point: u32 = 1;

			// This should cause an overflow in the calculation as
			// (max_tokens * interest_rate) / interest_rate_base_point cannot fit into u128
			assert_noop!(
				LiquidityPools::calculate_reward(
					max_tokens,
					reward_debt,
					interest_rate,
					interest_rate_base_point,
					asset_decimals,
					native_decimals,
				),
				Error::<Test>::RewardCalculationOverflow
			);
		});
	}
}
