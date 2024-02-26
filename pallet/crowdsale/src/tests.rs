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
use crate::{
	mock::{AssetsExt, Crowdsale, Nft, System, Test},
	Pallet,
};
use pallet_nft::CrossChainCompatibility;
use seed_pallet_common::test_prelude::{BlockNumber, *};
use seed_primitives::TokenCount;

// Create an NFT collection
// Returns the created `collection_id`
fn create_nft_collection(owner: AccountId, max_issuance: TokenCount) -> CollectionUuid {
	let collection_id = Nft::next_collection_uuid().unwrap();
	let collection_name = bounded_string("test-collection");
	let metadata_scheme = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
	assert_ok!(Nft::create_collection(
		Some(owner).into(),
		collection_name,
		0,
		Some(max_issuance),
		None,
		metadata_scheme,
		None,
		CrossChainCompatibility::default(),
	));
	collection_id
}

// Helper function for creating the collection name type
pub fn bounded_string(name: &str) -> BoundedVec<u8, <Test as pallet_nft::Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

fn add_decimals(balance: Balance, decimals: u8) -> Balance {
	balance * 10u128.pow(decimals as u32)
}

mod calculate_voucher_rewards {
	use super::*;
	use crate::mock::MaxTokensPerCollection;

	#[test]
	fn calculate_voucher_rewards_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 50;
			let funds_raised = 5000;
			let contribution = 100;
			let voucher_total_supply = 100;

			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
				soft_cap_price,
				funds_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			);

			let expected_vouchers = 2_000_000;
			assert_eq!(user_vouchers, expected_vouchers);
		});
	}

	#[test]
	fn calculate_voucher_rewards_over_committed_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 50;
			let funds_raised = 10000; // twice as much as the soft cap
			let voucher_total_supply = 100;
			let contribution = 100;

			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
				soft_cap_price,
				funds_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			);

			let expected_vouchers = 1_000_000;
			assert_eq!(user_vouchers, expected_vouchers);
		});
	}

	#[test]
	fn calculate_voucher_rewards_under_committed_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 50;
			let funds_raised = 100; // Not nearly enough was raised :(
			let voucher_total_supply = 100;
			let contribution = 100;

			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
				soft_cap_price,
				funds_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			);

			// We still get 2 vouchers because we are paying out the soft cap price
			let expected_vouchers = 2_000_000;
			assert_eq!(user_vouchers, expected_vouchers);
		});
	}

	#[test]
	fn calculate_voucher_rewards_different_decimals_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 50_000_000_000_000_000_000; // Simulate 18 Decimal Places
			let funds_raised = 5_000_000_000_000_000_000_000; // Just enoughw as raised for 1<>1
			let voucher_total_supply = 100_000_000; // 6 DP Voucher issuance
			let contribution: Balance = 100_000_000_000_000_000_000; // Contribution in 18 DP asset

			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
				soft_cap_price,
				funds_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			);

			// We should get 2_000_000 vouchers (at 6DP)
			let expected_vouchers = 2_000_000;
			assert_eq!(user_vouchers, expected_vouchers);
		});
	}

	#[test]
	fn calculate_voucher_rewards_partial_rewards() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 50_000_000;
			let funds_raised = 10_000_000_000_000;
			let voucher_total_supply = 135_000; // 135000 * 50 = 6_750_000_000_000
			let contribution = 50_000_000;

			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
				soft_cap_price,
				funds_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			);

			let expected_vouchers = 675000;
			assert_eq!(user_vouchers, expected_vouchers);
		});
	}

	#[test]
	fn calculate_voucher_rewards_partial_rewards_2() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 10_000_000;
			let funds_raised = 20_000_000_000;
			let voucher_total_supply = 1000;
			let contribution = 500_000_000;

			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
				soft_cap_price,
				funds_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			);

			let expected_vouchers = 25_000_000;
			assert_eq!(user_vouchers, expected_vouchers);
		});
	}

	#[test]
	fn calculate_voucher_rewards_3() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 10_000_000_000_000_000_000;
			let funds_raised = 20_000_000_000_000_000_000_000;
			let voucher_total_supply = 1000;
			let contribution: Balance = 500_000_000_000_000_000_000;

			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
				soft_cap_price,
				funds_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			);

			let expected_vouchers = 25_000_000;
			assert_eq!(user_vouchers, expected_vouchers);
		});
	}

	#[test]
	fn calculate_voucher_rewards_rounding_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 6;
			let total_contributors: Balance = 123456;
			let soft_cap_price = add_decimals(1, decimals);
			let voucher_total_supply = 1234;

			let mut funds_raised = 0;
			let mut contributions: Vec<Balance> = Vec::new();
			for i in 0..total_contributors {
				let contribution = soft_cap_price * (i + 1);
				funds_raised += contribution;
				contributions.push(contribution);
			}

			let mut total_vouchers = 0;
			let mut total_paid_contributions = 0;
			for i in 0..total_contributors {
				let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
					soft_cap_price,
					funds_raised,
					contributions[i as usize].into(),
					voucher_total_supply,
					total_vouchers,
					total_paid_contributions.into(),
				);
				total_vouchers += user_vouchers;
				total_paid_contributions += contributions[i as usize];
			}

			assert_eq!(total_vouchers, add_decimals(voucher_total_supply, VOUCHER_DECIMALS));
		});
	}

	#[test]
	fn calculate_voucher_rewards_rounding_smallest_issue_with_adjustments() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 6;
			let total_contributors: Balance = 53; //53;
			let soft_cap_price = add_decimals(1, decimals);
			let voucher_total_supply = 1;

			let mut funds_raised = 0;
			let mut contributions: Vec<Balance> = Vec::new();
			for i in 0..total_contributors {
				let contribution = soft_cap_price * (i + 1);
				funds_raised += contribution;
				contributions.push(contribution);
			}

			let mut total_vouchers = 0;
			let mut total_paid_contributions = 0;
			for i in 0..total_contributors {
				let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
					soft_cap_price,
					funds_raised,
					contributions[i as usize].into(),
					voucher_total_supply,
					total_vouchers,
					total_paid_contributions.into(),
				);
				total_vouchers += user_vouchers;
				total_paid_contributions += contributions[i as usize];
			}

			assert_eq!(total_vouchers, add_decimals(voucher_total_supply, VOUCHER_DECIMALS));
		});
	}

	#[test]
	fn calculate_voucher_rewards_rounding_many_contributors() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 6;
			let iterations = 1000;
			// Test that total vouchers is always correct even with varying contributors
			for n in 0..iterations {
				let total_contributors: Balance = 20 + n;
				let soft_cap_price = add_decimals(1, decimals);
				let voucher_total_supply = 14;

				let mut funds_raised = 0;
				let mut contributions: Vec<Balance> = Vec::new();
				for i in 0..total_contributors {
					let contribution = soft_cap_price * (i + 1);
					funds_raised += contribution;
					contributions.push(contribution);
				}

				let mut total_vouchers = 0;
				let mut total_paid_contributions = 0;
				for i in 0..total_contributors {
					let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
						soft_cap_price,
						funds_raised,
						contributions[i as usize].into(),
						voucher_total_supply,
						total_vouchers,
						total_paid_contributions.into(),
					);
					total_vouchers += user_vouchers;
					total_paid_contributions += contributions[i as usize];
				}

				assert_eq!(total_vouchers, add_decimals(voucher_total_supply, VOUCHER_DECIMALS));
			}
		});
	}

	#[test]
	fn calculate_voucher_rewards_rounding_many_total_supplies() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 6;
			let iterations = 1000;
			// Test that total vouchers is always correct even with varying total supplies
			for n in 0..iterations {
				let total_contributors: Balance = iterations;
				let soft_cap_price = add_decimals(1, decimals);
				let voucher_total_supply = n;

				let mut funds_raised = 0;
				let mut contributions: Vec<Balance> = Vec::new();
				for i in 0..total_contributors {
					let contribution = soft_cap_price + i;
					funds_raised += contribution;
					contributions.push(contribution);
				}

				let mut total_vouchers = 0;
				let mut total_paid_contributions = 0;
				for i in 0..total_contributors {
					let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
						soft_cap_price,
						funds_raised,
						contributions[i as usize].into(),
						voucher_total_supply,
						total_vouchers,
						total_paid_contributions.into(),
					);
					total_vouchers += user_vouchers;
					total_paid_contributions += contributions[i as usize];
				}

				assert_eq!(total_vouchers, add_decimals(voucher_total_supply, VOUCHER_DECIMALS));
			}
		});
	}

	#[test]
	fn calculate_voucher_rewards_many_single_payments() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 1;
			// all contributing 1 each.
			// If not accounted for, our total supply would be 0
			let total_contributors: Balance = 10_000_000;
			let soft_cap_price = add_decimals(1, decimals);
			let voucher_total_supply = 1;

			let mut funds_raised = 0;
			let mut contributions: Vec<Balance> = Vec::new();
			for i in 0..total_contributors {
				let contribution = 1;
				funds_raised += contribution;
				contributions.push(contribution);
			}

			let mut total_vouchers = 0;
			let mut total_paid_contributions = 0;
			for i in 0..total_contributors {
				let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
					soft_cap_price,
					funds_raised,
					contributions[i as usize].into(),
					voucher_total_supply,
					total_vouchers,
					total_paid_contributions.into(),
				);
				total_vouchers += user_vouchers;
				total_paid_contributions += contributions[i as usize];
			}

			assert_eq!(total_vouchers, add_decimals(voucher_total_supply, VOUCHER_DECIMALS));
		});
	}

	#[test]
	fn calculate_voucher_rewards_over_max_decimals() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 34;
			// all contributing 1 each.
			// If not accounted for, our total supply would be 0
			let soft_cap_price = add_decimals(1, decimals);
			let voucher_total_supply = MaxTokensPerCollection::get() as u128;
			let total_raised = voucher_total_supply * soft_cap_price;
			let contribution = total_raised;
			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
				soft_cap_price,
				total_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			);

			assert_eq!(user_vouchers, add_decimals(voucher_total_supply, VOUCHER_DECIMALS));
		});
	}

	#[test]
	fn calculate_voucher_rewards_zero_total_funds() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 18;
			let soft_cap_price = add_decimals(10, decimals);
			let voucher_total_supply = 12;
			let total_raised = 0;
			let contribution = 0;

			// Although this should never happen, in the case where total raised is zero
			// we should expect 0 to be paid out
			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
				soft_cap_price,
				total_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			);

			assert_eq!(user_vouchers, 0);
		});
	}

	#[test]
	fn calculate_voucher_rewards_zero_total_supply() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 6;
			let soft_cap_price = add_decimals(1, decimals);
			let voucher_total_supply = 0;
			let total_raised = soft_cap_price;
			let contribution = soft_cap_price;

			// Although this should never happen, in the case where total supply is zero
			// we should expect 0 to be paid out
			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
				soft_cap_price,
				total_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			);

			assert_eq!(user_vouchers, 0);
		});
	}

	#[test]
	#[ignore]
	// TODO Remove this test. It is purely for demonstrating the difference between the old and new
	// methods
	fn calculate_voucher_rewards_old_vs_new_test_for_demonstration() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 6;
			let total_contributors: Balance = 53; //53;
			let soft_cap_price = add_decimals(1, decimals);
			let voucher_total_supply = 1;

			let mut funds_raised = 0;
			let mut contributions: Vec<Balance> = Vec::new();
			for i in 0..total_contributors {
				let contribution = soft_cap_price * (i + 1);
				funds_raised += contribution;
				contributions.push(contribution);
			}

			let mut total_vouchers = 0;
			let mut total_paid_contributions = 0;
			for i in 0..total_contributors {
				println!("\n===== User {:?} contributed: {:?}", i, contributions[i as usize]);
				let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
					soft_cap_price,
					funds_raised,
					contributions[i as usize].into(),
					voucher_total_supply,
					total_vouchers,
					total_paid_contributions.into(),
				);

				let vouchers_old_method = Pallet::<Test>::calculate_voucher_rewards_old(
					soft_cap_price,
					funds_raised,
					contributions[i as usize],
					voucher_total_supply,
				);
				println!("New Method: {:?} | Old Method: {:?}", user_vouchers, vouchers_old_method);
				total_vouchers += user_vouchers;
				total_paid_contributions += contributions[i as usize];
			}

			println!("\n===== SUMMARY =====");
			println!("Total Contributors: {:?}", total_contributors);
			println!("Total Voucher Supply: {:?}", voucher_total_supply);
			println!("Soft Cap Price: {:?}", soft_cap_price);
			println!("Funds Raised          : {:?}", funds_raised);
			println!("Expected funds Raised : {:?}", voucher_total_supply * soft_cap_price);
			println!("\nTotal Vouchers          : {:?}", total_vouchers);
			println!(
				"Expected total Vouchers : {:?}\n",
				add_decimals(voucher_total_supply, VOUCHER_DECIMALS)
			);

			assert_eq!(total_vouchers, add_decimals(voucher_total_supply, VOUCHER_DECIMALS));
		});
	}

	#[test]
	fn calculate_voucher_rewards_doesnt_exceed_max_supply() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 6;
			let soft_cap_price = add_decimals(1, decimals);
			let voucher_total_supply = 1;
			let contribution = 11111111111111111;
			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards_new(
				soft_cap_price,
				contribution,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			);

			// Even if one user over commits, we still only mint the max_supply
			assert_eq!(add_decimals(voucher_total_supply, VOUCHER_DECIMALS), user_vouchers);
		});
	}
}

mod initialize {
	use super::*;

	#[test]
	fn initialize_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let reward_collection_id = create_nft_collection(alice(), 10);
			let payment_asset = 1;
			let soft_cap_price = 10;
			let duration = 100;

			// Get sale_id
			let sale_id = NextSaleId::<Test>::get();
			// Get next asset id
			let next_asset_id = AssetsExt::next_asset_uuid().unwrap();

			// Initialize the crowdsale
			assert_ok!(Crowdsale::initialize(
				Some(alice()).into(),
				payment_asset,
				reward_collection_id,
				soft_cap_price,
				duration
			));

			let sale_info = SaleInformation::<AccountId, BlockNumber> {
				status: SaleStatus::Disabled,
				admin: alice(),
				payment_asset,
				reward_collection_id,
				soft_cap_price,
				funds_raised: 0,
				voucher: next_asset_id,
				sale_duration: duration,
			};
			// Check storage
			assert_eq!(SaleInfo::<Test>::get(sale_id).unwrap(), sale_info);
			assert_eq!(NextSaleId::<Test>::get(), sale_id + 1);

			// Check event thrown
			System::assert_last_event(
				Event::CrowdsaleCreated { id: sale_id, info: sale_info }.into(),
			);
		});
	}

	#[test]
	fn initialize_no_ids_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let collection_id = create_nft_collection(alice(), 10);
			let payment_asset = 1;
			let soft_cap_price = 10;
			let duration = 100;

			// Put max sale_id
			NextSaleId::<Test>::put(SaleId::MAX);

			// Initialize the crowdsale
			assert_noop!(
				Crowdsale::initialize(
					Some(alice()).into(),
					payment_asset,
					collection_id,
					soft_cap_price,
					duration
				),
				Error::<Test>::NoAvailableIds
			);
		});
	}
	//
	// #[test]
	// fn initialize_invalid_block_range_fails() {
	// 	TestExt::<Test>::default().build().execute_with(|| {
	// 		let collection_id = create_nft_collection(alice(), 10);
	// 		let payment_asset = 1;
	// 		let soft_cap_price = 10;
	// 		let duration = 100;
	//
	// 		// Initialize the crowdsale
	// 		assert_noop!(
	// 			Crowdsale::initialize(
	// 				Some(alice()).into(),
	// 				payment_asset,
	// 				collection_id,
	// 				soft_cap_price,
	// 				duration
	// 			),
	// 			Error::<Test>::InvalidBlockRange
	// 		);
	// 	});
	// }
	//
	// #[test]
	// fn initialize_invalid_start_block_fails() {
	// 	TestExt::<Test>::default().build().execute_with(|| {
	// 		let collection_id = create_nft_collection(alice(), 10);
	// 		let payment_asset = 1;
	// 		let soft_cap_price = 10;
	// 		let start_block = System::block_number(); // Start block as current block is invalid
	// 		let end_block = 100;
	//
	// 		// Initialize the crowdsale
	// 		assert_noop!(
	// 			Crowdsale::initialize(
	// 				Some(alice()).into(),
	// 				payment_asset,
	// 				collection_id,
	// 				soft_cap_price,
	// 				duration
	// 			),
	// 			Error::<Test>::SaleStartBlockInPast
	// 		);
	// 	});
	// }

	#[test]
	fn initialize_invalid_asset_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let collection_id = create_nft_collection(alice(), 10);
			let payment_asset = 100; // Payment asset doesn't exist
			let soft_cap_price = 10;
			let duration = 100;

			// Initialize the crowdsale
			assert_noop!(
				Crowdsale::initialize(
					Some(alice()).into(),
					payment_asset,
					collection_id,
					soft_cap_price,
					duration
				),
				Error::<Test>::InvalidAsset
			);
		});
	}
}
