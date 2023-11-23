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

use crate::mock::{create_account, Assets, Balances, TEST_ASSET_ID};

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
	new_test_ext, Incentive, RuntimeEvent as MockEvent, RuntimeOrigin as Origin, System, Test,
	TestExt,
};
use pallet_balances::Error as BalancesError;
use seed_primitives::AccountId;
use sp_runtime::traits::{BadOrigin, Zero};

#[test]
fn non_admin_cannot_create_pool() {
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;
		let alice: AccountId = create_account(1);

		assert_noop!(
			Incentive::create_pool(
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
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		NextPoolId::<Test>::put(u32::MAX);

		assert_noop!(
			Incentive::create_pool(
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
fn admin_can_create_pool_successfully() {
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		assert_ok!(Incentive::create_pool(
			Origin::root(),
			asset_id,
			interest_rate,
			max_tokens,
			start_block,
			end_block,
		));

		let pool_id = NextPoolId::<Test>::get() - 1;

		System::assert_last_event(MockEvent::Incentive(crate::Event::PoolCreated(
			pool_id,
			asset_id,
			interest_rate,
			max_tokens,
			start_block,
			end_block,
		)));

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
	});
}

#[test]
fn admin_can_create_multiple_pools_successfully() {
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		assert_ok!(Incentive::create_pool(
			Origin::root(),
			asset_id,
			interest_rate,
			max_tokens,
			start_block,
			end_block
		));

		assert_ok!(Incentive::create_pool(
			Origin::root(),
			asset_id,
			interest_rate,
			max_tokens,
			start_block,
			end_block
		));
	});
}

#[test]
fn non_admin_cannot_set_incentive_pool_succession() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Incentive::set_incentive_pool_succession(Origin::signed(create_account(1)), 10, 11),
			BadOrigin
		);
	});
}

#[test]
fn cannot_set_incentive_pool_succession_with_non_existent_predecessor() {
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		assert_ok!(Incentive::create_pool(
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
			Incentive::set_incentive_pool_succession(
				Origin::root(),
				non_existent_predecessor_id,
				successor_id
			),
			Error::<Test>::PredecessorPoolDoesNotExist
		);
	});
}

#[test]
fn cannot_set_incentive_pool_succession_with_non_existent_successor() {
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		assert_ok!(Incentive::create_pool(
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
			Incentive::set_incentive_pool_succession(
				Origin::root(),
				predecessor_id,
				non_existent_successor_id
			),
			Error::<Test>::SuccessorPoolDoesNotExist
		);
	});
}

#[test]
fn cannot_set_incentive_pool_succession_when_successor_max_tokens_less_than_predecessor() {
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		assert_ok!(Incentive::create_pool(
			Origin::root(),
			asset_id,
			interest_rate,
			max_tokens,
			start_block,
			end_block,
		));

		let predecessor_id = NextPoolId::<Test>::get() - 1;

		let max_tokens = max_tokens - 1;

		assert_ok!(Incentive::create_pool(
			Origin::root(),
			asset_id,
			interest_rate,
			max_tokens,
			start_block,
			end_block,
		));

		let successor_id = NextPoolId::<Test>::get() - 1;

		assert_noop!(
			Incentive::set_incentive_pool_succession(Origin::root(), predecessor_id, successor_id),
			Error::<Test>::SuccessorPoolSizeShouldBeGreaterThanPredecessor
		);
	});
}

#[test]
fn admin_can_set_incentive_pool_succession_successfully() {
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		assert_ok!(Incentive::create_pool(
			Origin::root(),
			asset_id,
			interest_rate,
			max_tokens,
			start_block,
			end_block,
		));

		let predecessor_id = NextPoolId::<Test>::get() - 1;

		let max_tokens = max_tokens + 1;

		assert_ok!(Incentive::create_pool(
			Origin::root(),
			asset_id,
			interest_rate,
			max_tokens,
			start_block,
			end_block,
		));

		let successor_id = NextPoolId::<Test>::get() - 1;

		assert_ok!(Incentive::set_incentive_pool_succession(
			Origin::root(),
			predecessor_id,
			successor_id
		));

		System::assert_last_event(MockEvent::Incentive(crate::Event::SetSuccession(
			predecessor_id,
			successor_id,
		)));

		assert_eq!(
			PoolRelationships::<Test>::get(predecessor_id),
			Some(PoolRelationship { predecessor_id: None, successor_id: Some(successor_id) })
		);
	});
}

#[test]
fn non_admin_cannot_close_pool() {
	new_test_ext().execute_with(|| {
		assert_noop!(Incentive::close_pool(Origin::signed(create_account(1)), 0), BadOrigin);
	});
}

#[test]
fn cannot_close_non_existent_pool() {
	new_test_ext().execute_with(|| {
		// 假设1不存在于存储中
		assert_noop!(Incentive::close_pool(Origin::root(), 10), Error::<Test>::PoolDoesNotExist);
	});
}

#[test]
fn admin_can_close_pool_successfully() {
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		assert_ok!(Incentive::create_pool(
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

		assert_ok!(Incentive::close_pool(Origin::root(), pool_id));

		System::assert_last_event(MockEvent::Incentive(crate::Event::PoolClosed(pool_id)));

		assert_eq!(Pools::<Test>::get(pool_id), None);
	});
}

#[test]
fn invalid_origin_cannot_join_pool() {
	new_test_ext().execute_with(|| {
		assert_noop!(Incentive::join_pool(Origin::none(), 0, 100), BadOrigin);
	});
}

#[test]
fn cannot_join_non_existent_pool() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Incentive::join_pool(Origin::signed(create_account(1)), 0, 100),
			Error::<Test>::PoolDoesNotExist
		);
	});
}

#[test]
fn cannot_join_pool_after_end_block() {
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		assert_ok!(Incentive::create_pool(
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
			Incentive::join_pool(Origin::signed(create_account(1)), pool_id, 10),
			Error::<Test>::PoolNotActive
		);
	});
}

#[test]
fn cannot_join_pool_if_token_limit_exceeded() {
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		assert_ok!(Incentive::create_pool(
			Origin::root(),
			asset_id,
			interest_rate,
			max_tokens,
			start_block,
			end_block
		));

		Incentive::on_initialize(start_block);
		Incentive::on_initialize(start_block + 1);

		let pool_id = NextPoolId::<Test>::get() - 1;

		assert_noop!(
			Incentive::join_pool(Origin::signed(create_account(1)), pool_id, max_tokens + 1),
			Error::<Test>::StakingLimitExceeded
		);
	});
}

#[test]
fn cannot_join_pool_without_sufficient_root_balance() {
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		assert_ok!(Incentive::create_pool(
			Origin::root(),
			asset_id,
			interest_rate,
			max_tokens,
			start_block,
			end_block
		));

		Incentive::on_initialize(start_block);
		Incentive::on_initialize(start_block + 1);

		let pool_id = NextPoolId::<Test>::get() - 1;

		assert_noop!(
			Incentive::join_pool(Origin::signed(create_account(1)), pool_id, 10),
			BalancesError::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn can_join_pool_successfully() {
	let user: AccountId = create_account(1);
	let user_balance = 100;
	TestExt::default()
		.with_balances(&[(user, user_balance)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(Incentive::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			Incentive::on_initialize(start_block);
			Incentive::on_initialize(start_block + 1);

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			assert_ok!(Incentive::join_pool(Origin::signed(user), pool_id, amount));

			System::assert_last_event(MockEvent::Incentive(crate::Event::UserJoined(
				user, pool_id, amount,
			)));

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
fn invalid_origin_cannot_exit_pool() {
	new_test_ext().execute_with(|| {
		assert_noop!(Incentive::exit_pool(Origin::none(), 0), BadOrigin);
	});
}

#[test]
fn cannot_exit_non_existent_pool() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Incentive::exit_pool(Origin::signed(create_account(1)), 0),
			Error::<Test>::PoolDoesNotExist
		);
	});
}

#[test]
fn cannot_exit_pool_without_previously_depositing_token() {
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		assert_ok!(Incentive::create_pool(
			Origin::root(),
			asset_id,
			interest_rate,
			max_tokens,
			start_block,
			end_block
		));

		let pool_id = NextPoolId::<Test>::get() - 1;

		Incentive::on_initialize(start_block);

		assert_noop!(
			Incentive::exit_pool(Origin::signed(create_account(1)), pool_id),
			Error::<Test>::NoTokensStaked
		);
	});
}

#[test]
fn can_exit_pool_successfully() {
	let user: AccountId = create_account(1);
	let user_balance = 100;
	TestExt::default()
		.with_balances(&[(user, user_balance)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(Incentive::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			Incentive::on_initialize(start_block);
			Incentive::on_initialize(start_block + 1);

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			assert_ok!(Incentive::join_pool(Origin::signed(user), pool_id, amount));

			assert_ok!(Incentive::exit_pool(Origin::signed(user), pool_id));

			System::assert_last_event(MockEvent::Incentive(crate::Event::UserExited(
				user, pool_id, amount,
			)));

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

	let balances = vec![(Incentive::get_vault_account(0).unwrap(), user_balance * 100)];

	TestExt::default()
		.with_balances(&balances)
		.with_assets(&assets)
		.build()
		.execute_with(|| {
			let asset_id = TEST_ASSET_ID;
			let interest_rate = 1;
			let max_tokens = 100 * 50;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(Incentive::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			Incentive::on_initialize(start_block);
			Incentive::on_initialize(start_block + 1);

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			for account_id in 1..100 {
				let user: AccountId = create_account(account_id);
				assert_ok!(Incentive::join_pool(Origin::signed(user), pool_id, amount));
			}

			Incentive::on_initialize(end_block);
			System::set_block_number(end_block + 1);

			for account_id in 1..100 {
				let user: AccountId = create_account(account_id);
				assert_ok!(Incentive::claim_reward(Origin::signed(user), pool_id));

				System::assert_last_event(MockEvent::Incentive(crate::Event::RewardsClaimed(
					user, pool_id, amount,
				)));
				assert_ok!(Incentive::claim_reward(Origin::signed(user), pool_id));

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

	let balances = vec![(Incentive::get_vault_account(0).unwrap(), user_balance * 100)];

	TestExt::default()
		.with_balances(&balances)
		.with_assets(&assets)
		.build()
		.execute_with(|| {
			let asset_id = TEST_ASSET_ID;
			let interest_rate = 1;
			let max_tokens = 100 * 50;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(Incentive::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			Incentive::on_initialize(start_block);
			Incentive::on_initialize(start_block + 1);

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			for account_id in 1..100 {
				let user: AccountId = create_account(account_id);
				assert_ok!(Incentive::join_pool(Origin::signed(user), pool_id, amount));
				assert_ok!(Incentive::set_pool_rollover(Origin::signed(user), pool_id, false));
			}

			Incentive::on_initialize(end_block);
			System::set_block_number(end_block + 1);

			for account_id in 1..100 {
				let user: AccountId = create_account(account_id);
				assert_ok!(Incentive::claim_reward(Origin::signed(user), pool_id));

				System::assert_last_event(MockEvent::Incentive(crate::Event::RewardsClaimed(
					user, pool_id, amount,
				)));

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

	let balances = vec![(Incentive::get_vault_account(0).unwrap(), user_balance)];

	TestExt::default()
		.with_balances(&balances)
		.with_assets(&assets)
		.build()
		.execute_with(|| {
			let asset_id = TEST_ASSET_ID;
			let interest_rate = 1;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(Incentive::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let pool_id = NextPoolId::<Test>::get() - 1;

			Incentive::on_initialize(start_block);
			Incentive::on_initialize(start_block + 1);

			assert_noop!(
				Incentive::claim_reward(Origin::signed(user), pool_id),
				Error::<Test>::NoTokensStaked
			);
		});
}

#[test]
fn claim_reward_should_fail_if_pool_does_not_exist() {
	new_test_ext().execute_with(|| {
		let pool_id = 1;
		let user: AccountId = create_account(1);
		let amount = 10;

		assert_noop!(
			Incentive::join_pool(Origin::signed(user), pool_id, amount),
			Error::<Test>::PoolDoesNotExist
		);
	});
}

#[test]
fn should_update_user_info() {
	let user: AccountId = create_account(1);
	let user_balance = 100;
	TestExt::default()
		.with_balances(&[(user, user_balance)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(Incentive::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block,
			));

			Incentive::on_initialize(start_block);
			Incentive::on_initialize(start_block + 1);

			let pool_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			assert_ok!(Incentive::join_pool(Origin::signed(user), pool_id, amount));

			assert_ok!(Incentive::set_pool_rollover(Origin::signed(user), pool_id, false));

			System::assert_last_event(MockEvent::Incentive(crate::Event::UserInfoUpdated(
				pool_id, user, false,
			)));

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
	new_test_ext().execute_with(|| {
		let pool_id = 1;
		let user: AccountId = create_account(1);

		assert_noop!(
			Incentive::set_pool_rollover(Origin::signed(user), pool_id, false),
			Error::<Test>::PoolDoesNotExist
		);
	});
}

#[test]
fn should_not_update_when_pool_closed() {
	let user: AccountId = create_account(1);
	let user_balance = 100;
	TestExt::default()
		.with_balances(&[(user, user_balance)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(Incentive::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block,
			));

			let pool_id = NextPoolId::<Test>::get() - 1;

			assert_noop!(
				Incentive::set_pool_rollover(Origin::signed(user), pool_id, false),
				Error::<Test>::PoolNotActive
			);
		});
}

#[test]
fn should_not_update_for_user_without_tokens() {
	let user: AccountId = create_account(1);
	let user_balance = 100;
	TestExt::default()
		.with_balances(&[(user, user_balance)])
		.build()
		.execute_with(|| {
			let asset_id = 1;
			let interest_rate = 1;
			let max_tokens = 100;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(Incentive::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block,
			));

			Incentive::on_initialize(start_block);
			Incentive::on_initialize(start_block + 1);

			let pool_id = NextPoolId::<Test>::get() - 1;

			assert_noop!(
				Incentive::set_pool_rollover(Origin::signed(user), pool_id, false),
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

	let balances = vec![(Incentive::get_vault_account(0).unwrap(), user_balance * 100)];

	TestExt::default()
		.with_balances(&balances)
		.with_assets(&assets)
		.build()
		.execute_with(|| {
			let asset_id = TEST_ASSET_ID;
			let interest_rate = 1;
			let max_tokens = 100 * 50;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(Incentive::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			Incentive::on_initialize(start_block);
			Incentive::on_initialize(start_block + 1);

			let predecessor_id = NextPoolId::<Test>::get() - 1;
			let amount = 10;

			for account_id in 1..=user_amount {
				let user: AccountId = create_account(account_id);
				assert_ok!(Incentive::join_pool(Origin::signed(user), predecessor_id, amount));
			}
			// 10 user opt-out rollover should be left over when rollover
			for account_id in user_amount + 1..=user_amount + opt_out_rollover_amount {
				let user: AccountId = create_account(account_id);
				assert_ok!(Incentive::join_pool(Origin::signed(user), predecessor_id, amount));
				assert_ok!(Incentive::set_pool_rollover(
					Origin::signed(user),
					predecessor_id,
					false
				));
			}

			assert_ok!(Incentive::create_pool(
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

			assert_ok!(Incentive::set_incentive_pool_succession(
				Origin::root(),
				predecessor_id,
				successor_id
			));

			// Simulate rollover process
			System::set_block_number(reward_period);

			// Give some time for the rollover to be processed
			for _block_bump in 1..100 {
				Incentive::on_initialize(System::block_number());
				System::set_block_number(System::block_number() + 1);

				assert_ok!(Incentive::rollover_unsigned(
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

	let balances = vec![(Incentive::get_vault_account(0).unwrap(), user_balance * 100)];

	TestExt::default()
		.with_balances(&balances)
		.with_assets(&assets)
		.build()
		.execute_with(|| {
			let asset_id = TEST_ASSET_ID;
			let interest_rate = 1;
			let max_tokens = 100 * 50;
			let reward_period = 100;
			let start_block = System::block_number() + 1;
			let end_block = start_block + reward_period;

			assert_ok!(Incentive::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block
			));

			let predecessor_id = NextPoolId::<Test>::get() - 1;

			let end_block_2 = end_block + 100;
			assert_ok!(Incentive::create_pool(
				Origin::root(),
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block_2
			));

			let successor_id = NextPoolId::<Test>::get() - 1;

			Incentive::on_initialize(start_block);
			Incentive::on_initialize(start_block + 1);

			let amount = 10;

			for account_id in 1..=user_amount {
				let user: AccountId = create_account(account_id);
				assert_ok!(Incentive::join_pool(Origin::signed(user), predecessor_id, amount));
			}

			// Join the successor pool
			assert_ok!(Incentive::join_pool(
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

			assert_ok!(Incentive::set_incentive_pool_succession(
				Origin::root(),
				predecessor_id,
				successor_id
			));

			// Simulate rollover process
			System::set_block_number(reward_period);

			// Give some time for the rollover to be processed
			for _block_bump in 1..100 {
				Incentive::on_initialize(System::block_number());
				System::set_block_number(System::block_number() + 1);

				assert_ok!(Incentive::rollover_unsigned(
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
fn cannot_join_pool_when_not_provisioning() {
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		assert_ok!(Incentive::create_pool(
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
			Incentive::join_pool(Origin::signed(create_account(1)), pool_id, amount),
			Error::<Test>::PoolNotActive
		);
	});
}

#[test]
fn cannot_exit_pool_when_not_joined() {
	new_test_ext().execute_with(|| {
		let asset_id = 1;
		let interest_rate = 1;
		let max_tokens = 100;
		let reward_period = 100;
		let start_block = System::block_number() + 1;
		let end_block = start_block + reward_period;

		assert_ok!(Incentive::create_pool(
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
			Incentive::exit_pool(Origin::signed(create_account(1)), pool_id),
			Error::<Test>::NoTokensStaked
		);
	});
}
