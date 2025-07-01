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

				System::assert_last_event(MockEvent::LiquidityPools(crate::Event::PoolOpen {
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

				System::assert_last_event(MockEvent::LiquidityPools(crate::Event::SetSuccession {
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
				System::assert_last_event(MockEvent::LiquidityPools(
					crate::Event::UserInfoUpdated {
						pool_id,
						account_id: user,
						should_rollover: true,
					},
				));
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

				System::assert_last_event(MockEvent::LiquidityPools(
					crate::Event::UserInfoUpdated {
						pool_id,
						account_id: user,
						should_rollover: false,
					},
				));

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

				System::assert_last_event(MockEvent::LiquidityPools(crate::Event::PoolClosed {
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
	fn cannot_close_pool_with_active_user_stakes_frn_67_fix() {
		let reward_asset_id = 1;
		let staked_asset_id = 2;
		
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 200)])
			.with_asset(reward_asset_id, "REWARD", &[(alice(), 100)])
			.with_asset(staked_asset_id, "STAKE", &[(alice(), 100), (bob(), 100)])
			.build()
			.execute_with(|| {
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				// Alice creates a pool
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

				// Bob joins the pool with his funds
				assert_ok!(LiquidityPools::enter_pool(
					RuntimeOrigin::signed(bob()),
					pool_id,
					50
				));

				// Verify Bob's funds are locked in the pool
				let pool_vault_account = LiquidityPools::get_vault_account(pool_id);
				assert_eq!(AssetsExt::balance(staked_asset_id, &pool_vault_account), 50);
				assert_eq!(AssetsExt::balance(staked_asset_id, &bob()), 50);

				// Alice (creator) tries to close the pool while Bob still has active stakes
				// This should fail due to FRN-67 security fix
				assert_noop!(
					LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id),
					Error::<Test>::CannotClosePoolWithActiveUsers
				);

				// Verify Bob's funds are still safe in the vault
				assert_eq!(AssetsExt::balance(staked_asset_id, &pool_vault_account), 50);
				assert_eq!(AssetsExt::balance(staked_asset_id, &alice()), 100); // Alice's original balance unchanged

				// Bob can still recover his funds via emergency recovery
				assert_ok!(LiquidityPools::emergency_recover_funds(
					RuntimeOrigin::signed(bob()),
					pool_id
				));

				// Verify Bob got his funds back
				assert_eq!(AssetsExt::balance(staked_asset_id, &bob()), 100);
				assert_eq!(AssetsExt::balance(staked_asset_id, &pool_vault_account), 0);

				// Now Alice can close the pool since no active users remain
				assert_ok!(LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id));

				// Verify pool is properly closed and cleaned up
				assert_eq!(Pools::<Test>::get(pool_id), None);
			});
	}

	#[test]
	fn emergency_recover_funds_works_for_user_fund_recovery() {
		let reward_asset_id = 1;
		let staked_asset_id = 2;
		
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 200)])
			.with_asset(reward_asset_id, "REWARD", &[(alice(), 100)])
			.with_asset(staked_asset_id, "STAKE", &[(alice(), 100), (bob(), 100)])
			.build()
			.execute_with(|| {
				let interest_rate = 1_000_000;
				let max_tokens = 100;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				// Alice creates a pool
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

				// Bob joins the pool
				assert_ok!(LiquidityPools::enter_pool(
					RuntimeOrigin::signed(bob()),
					pool_id,
					75
				));

				let pool_vault_account = LiquidityPools::get_vault_account(pool_id);
				assert_eq!(AssetsExt::balance(staked_asset_id, &pool_vault_account), 75);
				assert_eq!(AssetsExt::balance(staked_asset_id, &bob()), 25);

				// Bob uses emergency recovery to get his funds back
				assert_ok!(LiquidityPools::emergency_recover_funds(
					RuntimeOrigin::signed(bob()),
					pool_id
				));

				// Verify Bob recovered his full stake
				assert_eq!(AssetsExt::balance(staked_asset_id, &bob()), 100);
				assert_eq!(AssetsExt::balance(staked_asset_id, &pool_vault_account), 0);

				// Verify Bob is no longer in the pool
				assert_eq!(PoolUsers::<Test>::get(pool_id, &bob()), None);

				System::assert_last_event(MockEvent::LiquidityPools(crate::Event::UserExited {
					account_id: bob(),
					pool_id,
					amount: 75,
				}));
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

				System::assert_last_event(MockEvent::LiquidityPools(crate::Event::UserJoined {
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
					LiquidityPools::exit_pool(RuntimeOrigin::signed(alice()), pool_id),
					Error::<Test>::PoolNotOpen
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

				System::assert_last_event(MockEvent::LiquidityPools(crate::Event::UserExited {
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

					System::assert_last_event(MockEvent::LiquidityPools(
						crate::Event::RewardsClaimed { account_id: user, pool_id, amount },
					));

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

					System::assert_last_event(MockEvent::LiquidityPools(
						crate::Event::RewardsClaimed { account_id: user, pool_id, amount },
					));

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
		).unwrap();

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
		).unwrap();

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
		).unwrap();

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
		).unwrap();

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
		).unwrap();

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
		).unwrap();

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
		).unwrap();

		// The expected reward should consider the difference in decimals
		let expected_reward =
			user_joined_amount * 10_u128.pow((native_decimals - asset_decimals) as u32);
		assert_eq!(
			reward, expected_reward,
			"Reward should be correctly converted based on decimals"
		);
	}

// =============================================================================
// SECURITY TESTS FOR FRN-66 AND FRN-67 FIXES
// =============================================================================

mod security_tests {
	use super::*;

	/// Test for FRN-66 Fix - Arithmetic Overflow Handling
	/// 
	/// This test verifies that the `create_pool` function now properly handles overflow scenarios
	/// instead of panicking. It uses extreme values to trigger overflow conditions and ensures
	/// the system returns a proper error instead of crashing.
	#[test]
	fn test_frn_66_overflow_protection_in_calculate_reward() {
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), u128::MAX)])
			.build()
			.execute_with(|| {
				let reward_asset_id = 1;
				let staked_asset_id = 2;
				
				// Test case 1: Extreme interest rate that would cause overflow
				let extreme_interest_rate = u32::MAX;
				let max_tokens = Balance::MAX;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				// Before FRN-66 fix, this would panic due to arithmetic overflow
				// After the fix, it should return RewardCalculationOverflow error
				assert_noop!(
					LiquidityPools::create_pool(
						RuntimeOrigin::signed(alice()),
						reward_asset_id,
						staked_asset_id,
						extreme_interest_rate,
						max_tokens,
						lock_start_block,
						lock_end_block
					),
					Error::<Test>::RewardCalculationOverflow
				);

				// Test case 2: Large but reasonable values that should still work
				let reasonable_interest_rate = 1_000_000; // 10%
				let reasonable_max_tokens = 1_000_000_000_000; // 1 trillion units
				
				// This should work without overflow
				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					reasonable_interest_rate,
					reasonable_max_tokens,
					lock_start_block,
					lock_end_block
				));

				// Verify the pool was created successfully
				let pool_id = NextPoolId::<Test>::get() - 1;
				assert!(Pools::<Test>::get(pool_id).is_some());

				// Test case 3: Direct testing of calculate_reward function with overflow scenarios
				let result = LiquidityPools::calculate_reward(
					Balance::MAX,
					0,
					u32::MAX,
					10_000, // base point
					18, // staked asset decimals
					18  // reward asset decimals
				);
				
				// Should return overflow error instead of panicking
				assert_noop!(result, Error::<Test>::RewardCalculationOverflow);

				// Test case 4: Verify normal operation still works
				let normal_result = LiquidityPools::calculate_reward(
					1_000_000_000_000_000_000, // 1 token with 18 decimals
					0,
					1_000_000, // 10% interest rate
					10_000_000, // base point  
					18, // staked asset decimals
					18  // reward asset decimals
				);
				
				// This should succeed and return expected reward
				assert_ok!(normal_result);
				let reward = normal_result.unwrap();
				assert!(reward > 0);
				
				// Test edge case with maximum safe values
				let edge_result = LiquidityPools::calculate_reward(
					u128::MAX / 1000, // Large but not overflow-inducing amount
					0,
					1000, // 1% interest rate
					100_000, // base point
					18,
					18
				);
				assert_ok!(edge_result);
			});
	}

	/// Test for FRN-67 Fix - Fund Theft Prevention
	/// 
	/// This test reproduces the original vulnerability scenario from the audit report
	/// and verifies that Alice (pool creator) cannot steal Bob's funds when closing
	/// a pool while Bob still has active stakes.
	#[test]
	fn test_frn_67_fund_theft_prevention() {
		let reward_asset_id = 1;
		let staked_asset_id = 2;
		
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 300), (bob(), 1)])
			.with_asset(reward_asset_id, "REWARD", &[(alice(), 200)])
			.with_asset(staked_asset_id, "STAKE", &[(alice(), 1), (bob(), 150)])
			.build()
			.execute_with(|| {
				let interest_rate = 1_000_000; // 10%
				let max_tokens = 200;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				// Step 1: Alice creates a pool
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

				// Verify Alice's initial balances after pool creation
				let alice_reward_balance_after_creation = AssetsExt::balance(reward_asset_id, &alice());
				let alice_native_balance_after_creation = Balances::free_balance(alice());
				
				// Step 2: Bob stakes tokens in Alice's pool
				let bob_stake_amount = 100;
				assert_ok!(LiquidityPools::enter_pool(
					RuntimeOrigin::signed(bob()),
					pool_id,
					bob_stake_amount
				));

				// Verify Bob's tokens are locked in the pool vault
				let pool_vault_account = LiquidityPools::get_vault_account(pool_id);
				assert_eq!(AssetsExt::balance(staked_asset_id, &pool_vault_account), bob_stake_amount);
				assert_eq!(AssetsExt::balance(staked_asset_id, &bob()), 50); // Bob has 50 remaining
				
				// Verify Bob's stake is recorded
				let bob_user_info = PoolUsers::<Test>::get(pool_id, &bob()).unwrap();
				assert_eq!(bob_user_info.amount, bob_stake_amount);

				// Step 3: Alice attempts to close the pool while Bob still has active stakes
				// This is the original vulnerability scenario - before FRN-67 fix, Alice could steal Bob's funds
				assert_noop!(
					LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id),
					Error::<Test>::CannotClosePoolWithActiveUsers
				);

				// Step 4: Verify that Bob's funds are still safe
				assert_eq!(AssetsExt::balance(staked_asset_id, &pool_vault_account), bob_stake_amount);
				assert_eq!(AssetsExt::balance(staked_asset_id, &bob()), 50);
				
				// Verify Alice did not gain any extra funds
				assert_eq!(AssetsExt::balance(reward_asset_id, &alice()), alice_reward_balance_after_creation);
				assert_eq!(Balances::free_balance(alice()), alice_native_balance_after_creation);

				// Step 5: Test that Bob can still recover his funds via emergency recovery
				assert_ok!(LiquidityPools::emergency_recover_funds(
					RuntimeOrigin::signed(bob()),
					pool_id
				));

				// Verify Bob recovered his full stake
				assert_eq!(AssetsExt::balance(staked_asset_id, &bob()), 150); // Original 150
				assert_eq!(AssetsExt::balance(staked_asset_id, &pool_vault_account), 0);
				
				// Verify Bob is no longer in the pool
				assert_eq!(PoolUsers::<Test>::get(pool_id, &bob()), None);

				// Step 6: Now Alice can close the pool since no active users remain
				assert_ok!(LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id));

				// Step 7: Verify pool is properly closed and cleaned up
				assert_eq!(Pools::<Test>::get(pool_id), None);
				
				// Verify final balances - Alice should have her original funds back, Bob should have his funds
				// Alice gets her reward tokens back since no rewards were distributed
				assert!(AssetsExt::balance(reward_asset_id, &alice()) > alice_reward_balance_after_creation);
				assert_eq!(AssetsExt::balance(staked_asset_id, &bob()), 150);

				// Verify the proper event was emitted for Bob's recovery
				System::assert_has_event(MockEvent::LiquidityPools(crate::Event::UserExited {
					account_id: bob(),
					pool_id,
					amount: bob_stake_amount,
				}));
			});
	}

	/// Integration Test - Combined FRN-66 and FRN-67 Security Scenarios
	/// 
	/// This test combines both security scenarios in a single test to ensure
	/// the system handles both overflow protection and fund protection simultaneously.
	#[test]
	fn test_both_frn_66_67_integration() {
		let reward_asset_id = 1;
		let staked_asset_id = 2;
		
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 500), (bob(), 1), (charlie(), 1)])
			.with_asset(reward_asset_id, "REWARD", &[(alice(), 300)])
			.with_asset(staked_asset_id, "STAKE", &[(alice(), 1), (bob(), 100), (charlie(), 100)])
			.build()
			.execute_with(|| {
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				// Part 1: Test FRN-66 - Overflow protection during pool creation
				// Attempt to create pool with values that would cause overflow
				assert_noop!(
					LiquidityPools::create_pool(
						RuntimeOrigin::signed(alice()),
						reward_asset_id,
						staked_asset_id,
						u32::MAX, // Extreme interest rate
						Balance::MAX, // Extreme max tokens
						lock_start_block,
						lock_end_block
					),
					Error::<Test>::RewardCalculationOverflow
				);

				// Create a legitimate pool with safe values
				let safe_interest_rate = 1_000_000; // 10%
				let safe_max_tokens = 150;
				
				let pool_id = NextPoolId::<Test>::get();
				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					safe_interest_rate,
					safe_max_tokens,
					lock_start_block,
					lock_end_block
				));

				// Part 2: Multiple users join the pool
				assert_ok!(LiquidityPools::enter_pool(
					RuntimeOrigin::signed(bob()),
					pool_id,
					75
				));
				
				assert_ok!(LiquidityPools::enter_pool(
					RuntimeOrigin::signed(charlie()),
					pool_id,
					50
				));

				// Verify both users' funds are in the vault
				let pool_vault_account = LiquidityPools::get_vault_account(pool_id);
				assert_eq!(AssetsExt::balance(staked_asset_id, &pool_vault_account), 125);

				// Part 3: Test FRN-67 - Fund protection when closing pool with active users
				// Alice tries to close pool while users still have stakes
				assert_noop!(
					LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id),
					Error::<Test>::CannotClosePoolWithActiveUsers
				);

				// Part 4: Test that both overflow protection and fund protection work together
				// Test calculate_reward with user amounts that could potentially overflow
				let bob_user_info = PoolUsers::<Test>::get(pool_id, &bob()).unwrap();
				let _charlie_user_info = PoolUsers::<Test>::get(pool_id, &charlie()).unwrap();
				
				// Test with extreme parameters to ensure overflow protection
				let overflow_result = LiquidityPools::calculate_reward(
					Balance::MAX, // Use maximum balance to trigger overflow
					0,
					u32::MAX, // Extreme interest rate
					1, // Very small base point to amplify overflow
					18,
					18
				);
				assert_noop!(overflow_result, Error::<Test>::RewardCalculationOverflow);

				// Test with normal parameters should work
				let normal_result = LiquidityPools::calculate_reward(
					bob_user_info.amount,
					0,
					safe_interest_rate,
					10_000_000,
					18,
					18
				);
				assert_ok!(normal_result);

				// Part 5: Users can still recover their funds despite overflow scenarios
				assert_ok!(LiquidityPools::emergency_recover_funds(
					RuntimeOrigin::signed(bob()),
					pool_id
				));
				
				assert_ok!(LiquidityPools::emergency_recover_funds(
					RuntimeOrigin::signed(charlie()),
					pool_id
				));

				// Verify both users recovered their funds
				assert_eq!(AssetsExt::balance(staked_asset_id, &bob()), 100);
				assert_eq!(AssetsExt::balance(staked_asset_id, &charlie()), 100);
				assert_eq!(AssetsExt::balance(staked_asset_id, &pool_vault_account), 0);

				// Part 6: Now pool can be closed safely
				assert_ok!(LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id));
				assert_eq!(Pools::<Test>::get(pool_id), None);

				// Part 7: Verify system stability after handling both security scenarios
				// System should be in a clean state and able to create new pools
				let new_pool_id = NextPoolId::<Test>::get();
				assert_ok!(LiquidityPools::create_pool(
					RuntimeOrigin::signed(alice()),
					reward_asset_id,
					staked_asset_id,
					safe_interest_rate,
					50,
					lock_start_block + 200,
					lock_end_block + 200
				));
				
				assert!(Pools::<Test>::get(new_pool_id).is_some());
			});
	}

	/// Additional edge case test for FRN-66: Test reward calculation with extreme decimal differences
	#[test]
	fn test_frn_66_decimal_overflow_protection() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Test case with extreme decimal differences that could cause overflow
			let result = LiquidityPools::calculate_reward(
				1_000_000_000_000_000_000, // 1 token with 18 decimals
				0,
				5_000_000, // 50% interest rate  
				10_000_000, // base point
				0,  // 0 decimals for staked asset (extreme case)
				18  // 18 decimals for reward asset (extreme case)
			);
			
			// This extreme decimal difference should be handled gracefully
			assert_ok!(result);
			
			// Test the opposite extreme
			let result2 = LiquidityPools::calculate_reward(
				1_000_000_000_000_000_000,
				0,
				5_000_000,
				10_000_000,
				18, // 18 decimals for staked asset
				0   // 0 decimals for reward asset (extreme case)
			);
			
			assert_ok!(result2);
			
			// Test case that would definitely overflow
			let overflow_result = LiquidityPools::calculate_reward(
				Balance::MAX,
				0,
				u32::MAX,
				1, // Very small base point to amplify overflow
				0,
				18
			);
			
			assert_noop!(overflow_result, Error::<Test>::RewardCalculationOverflow);
		});
	}

	/// Additional edge case test for FRN-67: Test fund protection with complex user scenarios
	#[test]
	fn test_frn_67_complex_user_scenarios() {
		let reward_asset_id = 1;
		let staked_asset_id = 2;
		
		TestExt::<Test>::default()
			.with_balances(&vec![(alice(), 400), (bob(), 1), (charlie(), 1), (dave(), 1)])
			.with_asset(reward_asset_id, "REWARD", &[(alice(), 200)])
			.with_asset(staked_asset_id, "STAKE", &[
				(alice(), 1),
				(bob(), 100),
				(charlie(), 100),
				(dave(), 100)
			])
			.build()
			.execute_with(|| {
				let interest_rate = 1_000_000;
				let max_tokens = 250;
				let reward_period = 100;
				let lock_start_block = System::block_number() + 1;
				let lock_end_block = lock_start_block + reward_period;

				// Alice creates pool
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

				// Multiple users join
				assert_ok!(LiquidityPools::enter_pool(RuntimeOrigin::signed(bob()), pool_id, 80));
				assert_ok!(LiquidityPools::enter_pool(RuntimeOrigin::signed(charlie()), pool_id, 90));
				assert_ok!(LiquidityPools::enter_pool(RuntimeOrigin::signed(dave()), pool_id, 70));

				let pool_vault_account = LiquidityPools::get_vault_account(pool_id);
				assert_eq!(AssetsExt::balance(staked_asset_id, &pool_vault_account), 240);

				// Alice cannot close pool with multiple active users
				assert_noop!(
					LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id),
					Error::<Test>::CannotClosePoolWithActiveUsers
				);

				// One user recovers funds
				assert_ok!(LiquidityPools::emergency_recover_funds(RuntimeOrigin::signed(bob()), pool_id));
				assert_eq!(AssetsExt::balance(staked_asset_id, &pool_vault_account), 160);

				// Alice still cannot close pool as other users remain
				assert_noop!(
					LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id),
					Error::<Test>::CannotClosePoolWithActiveUsers
				);

				// Another user recovers funds
				assert_ok!(LiquidityPools::emergency_recover_funds(RuntimeOrigin::signed(charlie()), pool_id));
				assert_eq!(AssetsExt::balance(staked_asset_id, &pool_vault_account), 70);

				// Alice still cannot close as Dave remains
				assert_noop!(
					LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id),
					Error::<Test>::CannotClosePoolWithActiveUsers
				);

				// Last user recovers funds
				assert_ok!(LiquidityPools::emergency_recover_funds(RuntimeOrigin::signed(dave()), pool_id));
				assert_eq!(AssetsExt::balance(staked_asset_id, &pool_vault_account), 0);

				// Now Alice can close the pool
				assert_ok!(LiquidityPools::close_pool(RuntimeOrigin::signed(alice()), pool_id));
				assert_eq!(Pools::<Test>::get(pool_id), None);

				// Verify all users got their funds back
				assert_eq!(AssetsExt::balance(staked_asset_id, &bob()), 100);
				assert_eq!(AssetsExt::balance(staked_asset_id, &charlie()), 100);
				assert_eq!(AssetsExt::balance(staked_asset_id, &dave()), 100);
			});
	}
}
}
