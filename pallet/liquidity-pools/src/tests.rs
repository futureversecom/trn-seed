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

use crate::mock::{create_account, Assets, Balances, NativeAssetId, TEST_ASSET_ID};

use super::*;
use frame_support::{assert_noop, assert_ok, weights::constants::ParityDbWeight};
use mock::{
	LiquidityPools, RuntimeEvent as MockEvent, RuntimeOrigin as Origin, System, Test, TestExt,
};
use pallet_balances::Error as BalancesError;
use seed_primitives::AccountId;
use sp_runtime::traits::{BadOrigin, Zero};

#[test]
fn non_admin_cannot_create_pool() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;
			let alice: AccountId = create_account(1);

			assert_noop!(
				LiquidityPools::create_pool(
					Origin::signed(alice),
					asset_id,
					interest_rate,
					max_tokens,
					start_block,
					end_block
				),
				BadOrigin
			);
		});
}

#[test]
fn pool_creation_fails_with_next_pool_id_out_of_bounds() {
	TestExt::default().build().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1_000_000;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		NextPoolId::<Test>::put(u32::MAX);

		assert_noop!(
			LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			),
			Error::<Test>::NoAvailablePoolId
		);
	});
}

#[test]
fn pool_creation_fails_with_invalid_block() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() - 1;
			let end_block = start_block + reward_period;

			assert_noop!(
				LiquidityPools::create_pool(
					Origin::root(),
					asset_id,
					interest_rate,
					max_tokens,
					start_block,
					end_block
				),
				Error::<Test>::InvalidBlockRange
			);

			let start_block = System::block_number() + 1;
			let end_block = start_block - 1;

			assert_noop!(
				LiquidityPools::create_pool(
					Origin::root(),
					asset_id,
					interest_rate,
					max_tokens,
					start_block,
					end_block
				),
				Error::<Test>::InvalidBlockRange
			);
		});
}
#[test]

fn pool_creation_fails_without_balance_in_vault_account() {
	TestExt::default().build().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1_000_000;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		assert_noop!(
			LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			),
			BalancesError::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn admin_can_create_pool_successfully() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block,
			));

			let pool_id = NextPoolId::<Test>::get() - 1;

			System::assert_last_event(MockEvent::LiquidityPools(crate::Event::PoolCreated {
				pool_id,
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block,
			}));

			assert_eq!(
				Pools::<Test>::get(pool_id),
				Some(PoolInfo {
					id: pool_id,
					asset_id,
					interest_rate,
					max_tokens,
					last_updated: 1,
					start_block,
					end_block,
					locked_amount: Zero::zero(),
					pool_status: PoolStatus::Inactive,
				})
			);
			assert_eq!(NextPoolId::<Test>::get(), pool_id + 1);
			assert_eq!(PoolRelationships::<Test>::get(0), None);
			assert_eq!(Assets::balance(NativeAssetId::get(), LiquidityPools::account_id()), 0);
		});
}

#[test]
fn admin_can_create_multiple_pools_successfully() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 1000)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			let pool_id = NextPoolId::<Test>::get();

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			assert_eq!(
				Pools::<Test>::get(pool_id),
				Some(PoolInfo {
					id: pool_id,
					asset_id,
					interest_rate,
					max_tokens,
					last_updated: 1,
					start_block,
					end_block,
					locked_amount: Zero::zero(),
					pool_status: PoolStatus::Inactive,
				})
			);
			assert_eq!(NextPoolId::<Test>::get(), pool_id + 1);

			let pool_id = NextPoolId::<Test>::get();
			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));
			assert_eq!(
				Pools::<Test>::get(pool_id),
				Some(PoolInfo {
					id: pool_id,
					asset_id,
					interest_rate,
					max_tokens,
					last_updated: 1,
					start_block,
					end_block,
					locked_amount: Zero::zero(),
					pool_status: PoolStatus::Inactive,
				})
			);
			assert_eq!(NextPoolId::<Test>::get(), pool_id + 1);
		});
}

#[test]
fn non_admin_cannot_set_pool_succession() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			assert_noop!(
				LiquidityPools::set_pool_succession(Origin::signed(create_account(1)), 10, 11),
				BadOrigin
			);
		});
}

#[test]
fn cannot_set_pool_succession_with_non_existent_predecessor() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let successor_id = NextPoolId::<Test>::get() - 1;
			let non_existent_predecessor_id = successor_id + 1;
			assert_noop!(
				LiquidityPools::set_pool_succession(
					Origin::root(),
					non_existent_predecessor_id,
					successor_id
				),
				Error::<Test>::PredecessorPoolDoesNotExist
			);
		});
}

#[test]
fn cannot_set_pool_succession_with_non_existent_successor() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let predecessor_id = NextPoolId::<Test>::get() - 1;
			let non_existent_successor_id = predecessor_id + 1;
			assert_noop!(
				LiquidityPools::set_pool_succession(
					Origin::root(),
					predecessor_id,
					non_existent_successor_id
				),
				Error::<Test>::SuccessorPoolDoesNotExist
			);
		});
}

#[test]
fn cannot_set_pool_succession_when_successor_max_tokens_less_than_predecessor() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 1000)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block,
			));

			let predecessor_id = NextPoolId::<Test>::get() - 1;

			let max_tokens = max_tokens - 1;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block,
			));

			let successor_id = NextPoolId::<Test>::get() - 1;

			assert_noop!(
				LiquidityPools::set_pool_succession(Origin::root(), predecessor_id, successor_id),
				Error::<Test>::SuccessorPoolSizeShouldBeGreaterThanPredecessor
			);
		});
}

#[test]
fn admin_can_set_pool_succession_successfully() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 1000)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block,
			));

			let predecessor_id = NextPoolId::<Test>::get() - 1;

			let max_tokens = max_tokens + 1;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block,
			));

			let successor_id = NextPoolId::<Test>::get() - 1;

			assert_ok!(LiquidityPools::set_pool_succession(
				Origin::root(),
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

#[test]
fn set_pool_rollover_should_work() {
	let user: AccountId = create_account(1);
	let user_balance = 100;
	TestExt::default()
		.with_balances(&[(user, user_balance), (LiquidityPools::account_id(), user_balance)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);
			LiquidityPools::on_idle(start_block + 1, remaining_weight);

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			assert_ok!(LiquidityPools::join_pool(Origin::signed(user), pool_id, amount));

			// Set rollover preference to true
			assert_ok!(LiquidityPools::set_pool_rollover(Origin::signed(user), pool_id, true));

			// Verify the rollover preference is updated
			let user_info = PoolUsers::<Test>::get(pool_id, &user).unwrap();
			assert!(user_info.should_rollover);

			// Verify the UserInfoUpdated event is emitted
			System::assert_last_event(MockEvent::LiquidityPools(crate::Event::UserInfoUpdated {
				pool_id,
				account_id: user,
				should_rollover: true,
			}));
		});
}

#[test]
fn set_pool_rollover_fails_if_pool_does_not_exist() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let user: AccountId = create_account(1);
			let non_existent_pool_id = 999;

			// Try to set rollover preference on a non-existent pool
			assert_noop!(
				LiquidityPools::set_pool_rollover(Origin::signed(user), non_existent_pool_id, true),
				Error::<Test>::PoolDoesNotExist
			);
		});
}

#[test]
fn set_pool_rollover_fails_if_not_provisioning() {
	let user: AccountId = create_account(1);
	let user_balance = 100;
	TestExt::default()
		.with_balances(&[(user, user_balance), (LiquidityPools::account_id(), user_balance)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);
			LiquidityPools::on_idle(start_block + 1, remaining_weight);

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			assert_ok!(LiquidityPools::join_pool(Origin::signed(user), pool_id, amount));

			LiquidityPools::on_idle(end_block, remaining_weight);

			// Try to set rollover preference when pool is not provisioning
			assert_noop!(
				LiquidityPools::set_pool_rollover(Origin::signed(user), pool_id, true),
				Error::<Test>::PoolNotActive
			);
		});
}

#[test]
fn set_pool_rollover_fails_if_user_has_no_tokens_staked() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let pool_id = NextPoolId::<Test>::get();
			let pool_info = PoolInfo {
				id: pool_id,
				asset_id: 1,
				interest_rate: 1_000_000,
				max_tokens: 100,
				last_updated: 1,
				start_block: System::block_number() + 1,
				end_block: System::block_number() + 100,
				locked_amount: Zero::zero(),
				pool_status: PoolStatus::Provisioning,
			};
			Pools::<Test>::insert(pool_id, pool_info);
			NextPoolId::<Test>::put(pool_id + 1);

			let user: AccountId = create_account(1);

			// Try to set rollover preference when user has no tokens staked
			assert_noop!(
				LiquidityPools::set_pool_rollover(Origin::signed(user), pool_id, true),
				Error::<Test>::NoTokensStaked
			);
		});
}

#[test]
fn set_pool_rollover_fails_due_to_bad_origin() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let pool_id = NextPoolId::<Test>::get();
			let pool_info = PoolInfo {
				id: pool_id,
				asset_id: 1,
				interest_rate: 1_000_000,
				max_tokens: 100,
				last_updated: 1,
				start_block: System::block_number() + 1,
				end_block: System::block_number() + 100,
				locked_amount: Zero::zero(),
				pool_status: PoolStatus::Provisioning,
			};
			Pools::<Test>::insert(pool_id, pool_info);
			NextPoolId::<Test>::put(pool_id + 1);

			let non_signed_origin = Origin::none();

			// Try to set rollover preference with a bad origin
			assert_noop!(
				LiquidityPools::set_pool_rollover(non_signed_origin, pool_id, true),
				BadOrigin
			);
		});
}

#[test]
fn non_admin_cannot_close_pool() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			assert_noop!(
				LiquidityPools::close_pool(Origin::signed(create_account(1)), 0),
				BadOrigin
			);
		});
}

#[test]
fn cannot_close_non_existent_pool() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			assert_noop!(
				LiquidityPools::close_pool(Origin::root(), 10),
				Error::<Test>::PoolDoesNotExist
			);
		});
}

#[test]
fn admin_can_close_pool_successfully() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let pool_id = NextPoolId::<Test>::get() - 1;

			assert_eq!(
				Pools::<Test>::get(pool_id),
				Some(PoolInfo {
					id: pool_id,
					asset_id,
					interest_rate,
					max_tokens,
					last_updated: 1,
					start_block,
					end_block,
					locked_amount: Zero::zero(),
					pool_status: PoolStatus::Inactive,
				})
			);

			assert_ok!(LiquidityPools::close_pool(Origin::root(), pool_id));

			System::assert_last_event(MockEvent::LiquidityPools(crate::Event::PoolClosed {
				pool_id,
			}));

			assert_eq!(Pools::<Test>::get(pool_id), None);
			assert_eq!(RolloverPivot::<Test>::get(pool_id), vec![]);
			assert_eq!(PoolRelationships::<Test>::get(pool_id), None);
		});
}

#[test]
fn invalid_origin_cannot_join_pool() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			assert_noop!(LiquidityPools::join_pool(Origin::none(), 0, 100), BadOrigin);
		});
}

#[test]
fn cannot_join_non_existent_pool() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			assert_noop!(
				LiquidityPools::join_pool(Origin::signed(create_account(1)), 0, 100),
				Error::<Test>::PoolDoesNotExist
			);
		});
}

#[test]
fn cannot_join_pool_after_end_block() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let pool_id = NextPoolId::<Test>::get() - 1;

			System::set_block_number(reward_period + System::block_number() + 1);

			assert_noop!(
				LiquidityPools::join_pool(Origin::signed(create_account(1)), pool_id, 10),
				Error::<Test>::PoolNotActive
			);
		});
}

#[test]
fn cannot_join_pool_if_token_limit_exceeded() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);
			LiquidityPools::on_idle(start_block + 1, remaining_weight);

			let pool_id = NextPoolId::<Test>::get() - 1;

			assert_noop!(
				LiquidityPools::join_pool(
					Origin::signed(create_account(1)),
					pool_id,
					max_tokens + 1
				),
				Error::<Test>::StakingLimitExceeded
			);
		});
}

#[test]
fn cannot_join_pool_without_sufficient_root_balance() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);
			LiquidityPools::on_idle(start_block + 1, remaining_weight);

			let pool_id = NextPoolId::<Test>::get() - 1;

			assert_noop!(
				LiquidityPools::join_pool(Origin::signed(create_account(1)), pool_id, 10),
				BalancesError::<Test>::InsufficientBalance
			);
		});
}

#[test]
fn can_join_pool_successfully() {
	let user: AccountId = create_account(1);
	let user_balance = 100;
	TestExt::default()
		.with_balances(&[(user, user_balance), (LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);
			LiquidityPools::on_idle(start_block + 1, remaining_weight);

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			assert_ok!(LiquidityPools::join_pool(Origin::signed(user), pool_id, amount));

			System::assert_last_event(MockEvent::LiquidityPools(crate::Event::UserJoined {
				account_id: user,
				pool_id,
				amount,
			}));

			assert_eq!(Balances::free_balance(user), user_balance - amount);

			assert_eq!(
				Pools::<Test>::get(pool_id),
				Some(PoolInfo {
					id: pool_id,
					asset_id,
					interest_rate,
					max_tokens,
					last_updated: 2,
					start_block,
					end_block,
					locked_amount: amount,
					pool_status: PoolStatus::Provisioning,
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

#[test]
fn can_refund_back_when_pool_is_done() {
	let user: AccountId = create_account(1);
	let user_balance = 100;
	let vault_balance = 1000;
	TestExt::default()
		.with_balances(&[(user, user_balance), (LiquidityPools::account_id(), vault_balance)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 500;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			assert_eq!(
				Balances::free_balance(LiquidityPools::account_id()),
				vault_balance - max_tokens
			);

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			assert_ok!(LiquidityPools::join_pool(Origin::signed(user), pool_id, amount));

			LiquidityPools::on_idle(end_block, remaining_weight);

			assert_eq!(
				Balances::free_balance(LiquidityPools::account_id()),
				vault_balance - amount
			);
		});
}

#[test]
fn invalid_origin_cannot_exit_pool() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			assert_noop!(LiquidityPools::exit_pool(Origin::none(), 0), BadOrigin);
		});
}

#[test]
fn cannot_exit_non_existent_pool() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			assert_noop!(
				LiquidityPools::exit_pool(Origin::signed(create_account(1)), 0),
				Error::<Test>::PoolDoesNotExist
			);
		});
}

#[test]
fn cannot_exit_pool_with_wrong_pool_status() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let pool_id = NextPoolId::<Test>::get() - 1;

			assert_noop!(
				LiquidityPools::exit_pool(Origin::signed(create_account(1)), pool_id),
				Error::<Test>::PoolNotActive
			);
		});
}

#[test]
fn cannot_exit_pool_without_previously_depositing_token() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let pool_id = NextPoolId::<Test>::get() - 1;

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);

			assert_noop!(
				LiquidityPools::exit_pool(Origin::signed(create_account(1)), pool_id),
				Error::<Test>::NoTokensStaked
			);

			PoolUsers::<Test>::insert(pool_id, create_account(1), UserInfo::default());

			assert_noop!(
				LiquidityPools::exit_pool(Origin::signed(create_account(1)), pool_id),
				Error::<Test>::NoTokensStaked
			);
		});
}

#[test]
fn can_exit_pool_successfully() {
	let user: AccountId = create_account(1);
	let user_balance = 100;
	TestExt::default()
		.with_balances(&[(user, user_balance), (LiquidityPools::account_id(), user_balance)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);
			LiquidityPools::on_idle(start_block + 1, remaining_weight);

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			assert_ok!(LiquidityPools::join_pool(Origin::signed(user), pool_id, amount));

			assert_ok!(LiquidityPools::exit_pool(Origin::signed(user), pool_id));

			System::assert_last_event(MockEvent::LiquidityPools(crate::Event::UserExited {
				account_id: user,
				pool_id,
				amount,
			}));

			assert_eq!(Balances::free_balance(user), user_balance);

			assert_eq!(
				Pools::<Test>::get(pool_id),
				Some(PoolInfo {
					id: pool_id,
					asset_id,
					interest_rate,
					max_tokens,
					last_updated: 2,
					start_block,
					end_block,
					locked_amount: Zero::zero(),
					pool_status: PoolStatus::Provisioning,
				})
			);

			assert_eq!(PoolUsers::<Test>::get(pool_id, user), None);
		});
}

#[test]
fn claim_reward_should_work() {
	let mut assets = vec![];
	let user_balance = 100;
	for account_id in 1..100 {
		let user: AccountId = create_account(account_id);
		assets.push((TEST_ASSET_ID, user, user_balance));
	}

	let balances = vec![(LiquidityPools::account_id(), user_balance * 100)];

	TestExt::default()
		.with_balances(&balances)
		.with_assets(&assets)
		.build()
		.execute_with(|| {
			let asset_id = TEST_ASSET_ID;
			let interest_rate = 1_000_000;
			let max_tokens = 100 * 50;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);
			LiquidityPools::on_idle(start_block + 1, remaining_weight);

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			for account_id in 1..100 {
				let user: AccountId = create_account(account_id);
				assert_ok!(LiquidityPools::join_pool(Origin::signed(user), pool_id, amount));
			}

			LiquidityPools::on_idle(end_block, remaining_weight);
			System::set_block_number(end_block + 1);

			for account_id in 1..100 {
				let user: AccountId = create_account(account_id);
				assert_ok!(LiquidityPools::claim_reward(Origin::signed(user), pool_id));

				System::assert_last_event(MockEvent::LiquidityPools(
					crate::Event::RewardsClaimed { account_id: user, pool_id, amount },
				));
				assert_ok!(LiquidityPools::claim_reward(Origin::signed(user), pool_id));

				assert_eq!(Assets::balance(asset_id, user), user_balance - amount);
				assert_eq!(Balances::free_balance(user), amount);
			}
		});
}

#[test]
fn claim_reward_should_work_when_not_rollover() {
	let mut assets = vec![];
	let user_balance = 100;
	for account_id in 1..100 {
		let user: AccountId = create_account(account_id);
		assets.push((TEST_ASSET_ID, user, user_balance));
	}

	let balances = vec![(LiquidityPools::account_id(), user_balance * 100)];

	TestExt::default()
		.with_balances(&balances)
		.with_assets(&assets)
		.build()
		.execute_with(|| {
			let asset_id = TEST_ASSET_ID;
			let interest_rate = 1_000_000;
			let max_tokens = 100 * 50;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);
			LiquidityPools::on_idle(start_block + 1, remaining_weight);

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			for account_id in 1..100 {
				let user: AccountId = create_account(account_id);
				assert_ok!(LiquidityPools::join_pool(Origin::signed(user), pool_id, amount));
				assert_ok!(LiquidityPools::set_pool_rollover(Origin::signed(user), pool_id, false));
			}

			LiquidityPools::on_idle(end_block, remaining_weight);
			System::set_block_number(end_block + 1);

			for account_id in 1..100 {
				let user: AccountId = create_account(account_id);
				assert_ok!(LiquidityPools::claim_reward(Origin::signed(user), pool_id));

				System::assert_last_event(MockEvent::LiquidityPools(
					crate::Event::RewardsClaimed { account_id: user, pool_id, amount },
				));

				assert_eq!(Assets::balance(asset_id, user), user_balance);
				assert_eq!(Balances::free_balance(user), amount);
			}
		});
}

#[test]
fn claim_reward_should_fail_if_no_tokens_staked() {
	let user_balance = 100;
	let user: AccountId = create_account(1);
	let assets = vec![(TEST_ASSET_ID, user, user_balance)];

	let balances = vec![(LiquidityPools::account_id(), user_balance)];

	TestExt::default()
		.with_balances(&balances)
		.with_balances(&[(user, user_balance), (LiquidityPools::account_id(), user_balance)])
		.with_assets(&assets)
		.build()
		.execute_with(|| {
			let asset_id = TEST_ASSET_ID;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let pool_id = NextPoolId::<Test>::get() - 1;

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);
			LiquidityPools::on_idle(start_block + 1, remaining_weight);

			assert_noop!(
				LiquidityPools::claim_reward(Origin::signed(user), pool_id),
				Error::<Test>::NoTokensStaked
			);
		});
}

#[test]
fn claim_reward_should_fail_if_pool_does_not_exist() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let pool_id = 1;
			let user: AccountId = create_account(1);
			let amount = 10;

			assert_noop!(
				LiquidityPools::join_pool(Origin::signed(user), pool_id, amount),
				Error::<Test>::PoolDoesNotExist
			);
		});
}

#[test]
fn claim_reward_should_fail_if_pool_status_is_not_done() {
	let user_balance = 100;
	let user: AccountId = create_account(1);
	let assets = vec![(TEST_ASSET_ID, user, user_balance)];

	let balances = vec![(LiquidityPools::account_id(), user_balance)];

	TestExt::default()
		.with_balances(&balances)
		.with_assets(&assets)
		.build()
		.execute_with(|| {
			let asset_id = TEST_ASSET_ID;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let pool_id = NextPoolId::<Test>::get() - 1;

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);
			LiquidityPools::on_idle(start_block + 1, remaining_weight);

			assert_ok!(LiquidityPools::join_pool(Origin::signed(user), pool_id, 10));

			assert_noop!(
				LiquidityPools::claim_reward(Origin::signed(user), pool_id),
				Error::<Test>::NotReadyForClaimingReward
			);
		});
}

#[test]
fn should_update_user_info() {
	let user: AccountId = create_account(1);
	let user_balance = 100;
	TestExt::default()
		.with_balances(&[(user, user_balance), (LiquidityPools::account_id(), user_balance)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block,
			));

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);
			LiquidityPools::on_idle(start_block + 1, remaining_weight);

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			assert_ok!(LiquidityPools::join_pool(Origin::signed(user), pool_id, amount));

			assert_ok!(LiquidityPools::set_pool_rollover(Origin::signed(user), pool_id, false));

			System::assert_last_event(MockEvent::LiquidityPools(crate::Event::UserInfoUpdated {
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
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let pool_id = 1;
			let user: AccountId = create_account(1);

			assert_noop!(
				LiquidityPools::set_pool_rollover(Origin::signed(user), pool_id, false),
				Error::<Test>::PoolDoesNotExist
			);
		});
}

#[test]
fn should_not_update_when_pool_closed() {
	let user: AccountId = create_account(1);
	let user_balance = 100;
	TestExt::default()
		.with_balances(&[(user, user_balance), (LiquidityPools::account_id(), user_balance)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block,
			));

			let pool_id = NextPoolId::<Test>::get() - 1;

			assert_noop!(
				LiquidityPools::set_pool_rollover(Origin::signed(user), pool_id, false),
				Error::<Test>::PoolNotActive
			);
		});
}

#[test]
fn should_not_update_for_user_without_tokens() {
	let user: AccountId = create_account(1);
	let user_balance = 100;
	TestExt::default()
		.with_balances(&[(user, user_balance), (LiquidityPools::account_id(), user_balance)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block,
			));

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);
			LiquidityPools::on_idle(start_block + 1, remaining_weight);

			let pool_id = NextPoolId::<Test>::get() - 1;

			assert_noop!(
				LiquidityPools::set_pool_rollover(Origin::signed(user), pool_id, false),
				Error::<Test>::NoTokensStaked
			);
		});
}

#[test]
fn rollover_should_work() {
	let mut assets = vec![];
	let user_balance = 100;
	let user_amount = 100;
	let opt_out_rollover_amount = 10;
	for account_id in 1..=user_amount + opt_out_rollover_amount {
		let user: AccountId = create_account(account_id);
		assets.push((TEST_ASSET_ID, user, user_balance));
	}

	let balances = vec![(LiquidityPools::account_id(), user_balance * 100)];

	TestExt::default()
		.with_balances(&balances)
		.with_assets(&assets)
		.build()
		.execute_with(|| {
			let asset_id = TEST_ASSET_ID;
			let interest_rate = 1_000_000;
			let max_tokens = 100 * 50;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);
			LiquidityPools::on_idle(start_block + 1, remaining_weight);

			let predecessor_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			for account_id in 1..=user_amount {
				let user: AccountId = create_account(account_id);
				assert_ok!(LiquidityPools::join_pool(Origin::signed(user), predecessor_id, amount));
			}
			// 10 user opt-out rollover should be left over when rollover
			for account_id in user_amount + 1..=user_amount + opt_out_rollover_amount {
				let user: AccountId = create_account(account_id);
				assert_ok!(LiquidityPools::join_pool(Origin::signed(user), predecessor_id, amount));
				assert_ok!(LiquidityPools::set_pool_rollover(
					Origin::signed(user),
					predecessor_id,
					false
				));
			}

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			assert_eq!(
				Pools::<Test>::get(predecessor_id),
				Some(PoolInfo {
					id: predecessor_id,
					asset_id,
					interest_rate,
					max_tokens,
					last_updated: 2,
					start_block,
					end_block,
					locked_amount: amount * ((user_amount + opt_out_rollover_amount) as u128),
					pool_status: PoolStatus::Provisioning
				})
			);

			let successor_id = NextPoolId::<Test>::get() - 1;

			assert_ok!(LiquidityPools::set_pool_succession(
				Origin::root(),
				predecessor_id,
				successor_id
			));

			// Simulate rollover process
			System::set_block_number(reward_period);

			// Give some time for the rollover to be processed
			for _block_bump in 1..100 {
				LiquidityPools::on_idle(System::block_number(), remaining_weight);
				System::set_block_number(System::block_number() + 1);

				assert_ok!(LiquidityPools::rollover_unsigned(
					Origin::none(),
					predecessor_id,
					System::block_number()
				));
			}

			assert_eq!(
				Pools::<Test>::get(predecessor_id),
				Some(PoolInfo {
					id: predecessor_id,
					asset_id,
					interest_rate,
					max_tokens,
					last_updated: 113,
					start_block,
					end_block,
					locked_amount: opt_out_rollover_amount as u128 * amount,
					pool_status: PoolStatus::Done
				})
			);
		});
}

#[test]
fn rollover_should_work_when_exceeding_successor_pool_maxtokens() {
	let mut assets = vec![];
	let user_balance = 10000;
	let user_amount = 100;
	for account_id in 1..=user_amount {
		let user: AccountId = create_account(account_id);
		assets.push((TEST_ASSET_ID, user, user_balance));
	}

	let balances = vec![(LiquidityPools::account_id(), user_balance * 100)];

	TestExt::default()
		.with_balances(&balances)
		.with_assets(&assets)
		.build()
		.execute_with(|| {
			let asset_id = TEST_ASSET_ID;
			let interest_rate = 1_000_000;
			let max_tokens = 100 * 50;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let predecessor_id = NextPoolId::<Test>::get() - 1;

			let end_block_2 = end_block + 100;
			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block_2
			));

			let successor_id = NextPoolId::<Test>::get() - 1;

			let remaining_weight: Weight = ParityDbWeight::get()
				.reads(100u64)
				.saturating_add(ParityDbWeight::get().writes(100u64));
			LiquidityPools::on_idle(start_block, remaining_weight);
			LiquidityPools::on_idle(start_block + 1, remaining_weight);

			let amount = 10;

			for account_id in 1..=user_amount {
				let user: AccountId = create_account(account_id);
				assert_ok!(LiquidityPools::join_pool(Origin::signed(user), predecessor_id, amount));
			}

			// Join the successor pool
			assert_ok!(LiquidityPools::join_pool(
				Origin::signed(create_account(1)),
				successor_id,
				max_tokens - amount * (user_amount as u128 / 2),
			));

			assert_eq!(
				Pools::<Test>::get(predecessor_id),
				Some(PoolInfo {
					id: predecessor_id,
					asset_id,
					interest_rate,
					max_tokens,
					last_updated: 2,
					start_block,
					end_block,
					locked_amount: amount * ((user_amount) as u128),
					pool_status: PoolStatus::Provisioning
				})
			);

			assert_ok!(LiquidityPools::set_pool_succession(
				Origin::root(),
				predecessor_id,
				successor_id
			));

			// Simulate rollover process
			System::set_block_number(reward_period);

			// Give some time for the rollover to be processed
			for _block_bump in 1..100 {
				LiquidityPools::on_idle(System::block_number(), remaining_weight);
				System::set_block_number(System::block_number() + 1);

				assert_ok!(LiquidityPools::rollover_unsigned(
					Origin::none(),
					predecessor_id,
					System::block_number()
				));
			}

			assert_eq!(
				Pools::<Test>::get(predecessor_id),
				Some(PoolInfo {
					id: predecessor_id,
					asset_id,
					interest_rate,
					max_tokens,
					last_updated: 107,
					start_block,
					end_block,
					locked_amount: user_amount as u128 / 2 * amount,
					pool_status: PoolStatus::Done
				})
			);

			assert_eq!(
				Pools::<Test>::get(successor_id),
				Some(PoolInfo {
					id: successor_id,
					asset_id,
					interest_rate,
					max_tokens,
					last_updated: 107,
					start_block,
					end_block: end_block_2,
					locked_amount: max_tokens,
					pool_status: PoolStatus::Provisioning
				})
			);
		});
}

#[test]
fn rollover_should_fail_when_pool_not_exist() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			assert_noop!(
				LiquidityPools::rollover_unsigned(Origin::none(), 1, System::block_number()),
				Error::<Test>::PoolDoesNotExist
			);
		});
}

#[test]
fn rollover_should_fail_when_successor_pool_not_exist() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let precessor_pool_id = 1;
			Pools::<Test>::insert(
				precessor_pool_id,
				PoolInfo {
					id: precessor_pool_id,
					asset_id: 1,
					interest_rate: 1_000_000,
					max_tokens: 100,
					last_updated: 0,
					start_block: 0,
					end_block: 0,
					locked_amount: 0,
					pool_status: PoolStatus::RollingOver,
				},
			);

			let successor_pool_id = 2;
			PoolRelationships::<Test>::insert(
				&precessor_pool_id,
				PoolRelationship { successor_id: Some(successor_pool_id) },
			);

			assert_noop!(
				LiquidityPools::rollover_unsigned(
					Origin::none(),
					precessor_pool_id,
					System::block_number()
				),
				Error::<Test>::PoolDoesNotExist
			);
		});
}

#[test]
fn cannot_join_pool_when_not_provisioning() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			// Simulate the pool moving to a different state
			Pools::<Test>::mutate(pool_id, |pool| {
				*pool = Some(PoolInfo {
					pool_status: PoolStatus::Inactive, // Not Provisioning
					..pool.clone().unwrap()
				});
			});

			assert_noop!(
				LiquidityPools::join_pool(Origin::signed(create_account(1)), pool_id, amount),
				Error::<Test>::PoolNotActive
			);
		});
}

#[test]
fn cannot_exit_pool_when_not_joined() {
	TestExt::default()
		.with_balances(&vec![(LiquidityPools::account_id(), 100)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1_000_000;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(LiquidityPools::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let pool_id = NextPoolId::<Test>::get() - 1;
			Pools::<Test>::mutate(NextPoolId::<Test>::get() - 1, |pool| {
				*pool = Some(PoolInfo {
					pool_status: PoolStatus::Provisioning, // Not Provisioning
					..pool.clone().unwrap()
				});
			});

			assert_noop!(
				LiquidityPools::exit_pool(Origin::signed(create_account(1)), pool_id),
				Error::<Test>::NoTokensStaked
			);
		});
}

#[test]
fn test_calculate_reward_basic() {
	// Test with basic values where no overflow or saturation should occur
	let user_joined_amount: Balance = 1000;
	let interest_rate: u32 = 10000; // 100% in basis points
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
	);

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
	let interest_rate_base_point: u32 = 10000;

	let reward = LiquidityPools::calculate_reward(
		user_joined_amount,
		reward_debt,
		interest_rate,
		interest_rate_base_point,
		asset_decimals,
		native_decimals,
	);

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
	);

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
	);

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
	);

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
	);

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
	);

	// The expected reward should consider the difference in decimals
	let expected_reward =
		user_joined_amount * 10_u128.pow((native_decimals - asset_decimals) as u32);
	assert_eq!(reward, expected_reward, "Reward should be correctly converted based on decimals");
}
