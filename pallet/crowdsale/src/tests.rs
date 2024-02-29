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
	mock::{
		AssetsExt, Crowdsale, MaxConsecutiveSales, MaxSaleDuration, MaxSalesPerBlock,
		MaxTokensPerCollection, Nft, System, Test,
	},
	Pallet,
};
use frame_support::traits::fungibles::Inspect;
use pallet_nft::{traits::NFTCollectionInfo, CrossChainCompatibility};
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

// Helper function ton initialize a crowdsale with default values
fn initialize_crowdsale(
	max_issuance: Balance,
) -> (SaleId, SaleInformation<AccountId, BlockNumber>) {
	let reward_collection_id = create_nft_collection(alice(), max_issuance.saturated_into());
	let payment_asset_id = ROOT_ASSET_ID;
	let soft_cap_price = 10;
	let duration = 100;

	// Get sale_id
	let sale_id = NextSaleId::<Test>::get();
	// Get next asset id
	let next_asset_id = AssetsExt::next_asset_uuid().unwrap();

	// Initialize the crowdsale
	assert_ok!(Crowdsale::initialize(
		Some(alice()).into(),
		payment_asset_id,
		reward_collection_id,
		soft_cap_price,
		duration
	));

	let vault = Pallet::<Test>::vault_account(sale_id);
	let sale_info = SaleInformation::<AccountId, BlockNumber> {
		status: SaleStatus::Pending(System::block_number()),
		admin: alice(),
		vault,
		payment_asset_id,
		reward_collection_id,
		soft_cap_price,
		funds_raised: 0,
		voucher_asset_id: next_asset_id,
		duration,
	};
	return (sale_id, sale_info)
}

// Helper function for creating the collection name type
pub fn bounded_string(name: &str) -> BoundedVec<u8, <Test as pallet_nft::Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

fn add_decimals(balance: Balance, decimals: u8) -> Balance {
	balance * 10u128.pow(decimals as u32)
}

mod vault_account {
	use super::*;

	#[test]
	fn is_deterministic() {
		TestExt::<Test>::default().build().execute_with(|| {
			let vault_1 = Pallet::<Test>::vault_account(0);
			let vault_2 = Pallet::<Test>::vault_account(0);
			assert_eq!(vault_1, vault_2);

			let vault_3 = Pallet::<Test>::vault_account(1);
			let vault_4 = Pallet::<Test>::vault_account(1);
			assert_eq!(vault_3, vault_4);

			// Different seeds produce different vault addresses
			assert_ne!(vault_1, vault_3);

			// Check with u64::MAX
			let vault_5 = Pallet::<Test>::vault_account(u64::MAX);
			let vault_6 = Pallet::<Test>::vault_account(u64::MAX);
			assert_eq!(vault_5, vault_6);

			let vault_7 = Pallet::<Test>::vault_account(u64::MAX - 1);
			assert_ne!(vault_5, vault_7);
		});
	}
}

mod calculate_voucher_rewards {
	use super::*;

	#[test]
	fn over_committed_works_1() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 50;
			let funds_raised = 5000;
			let contribution = 100;
			let voucher_total_supply = 100;

			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
				soft_cap_price,
				funds_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			)
			.unwrap();

			let expected_vouchers = 2_000_000;
			assert_eq!(user_vouchers, expected_vouchers);
		});
	}

	#[test]
	fn over_committed_works_2() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 50;
			let funds_raised = 10000; // twice as much as the soft cap
			let voucher_total_supply = 100;
			let contribution = 100;

			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
				soft_cap_price,
				funds_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			)
			.unwrap();

			let expected_vouchers = 1_000_000;
			assert_eq!(user_vouchers, expected_vouchers);
		});
	}

	#[test]
	fn under_committed_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 50;
			let funds_raised = 100; // Not nearly enough was raised :(
			let voucher_total_supply = 100;
			let contribution = 100;

			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
				soft_cap_price,
				funds_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			)
			.unwrap();

			// We still get 2 vouchers because we are paying out the soft cap price
			let expected_vouchers = 2_000_000;
			assert_eq!(user_vouchers, expected_vouchers);
		});
	}

	#[test]
	fn different_decimals_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 50_000_000_000_000_000_000; // Simulate 18 Decimal Places
			let funds_raised = 5_000_000_000_000_000_000_000; // Just enough as raised for 1<>1
			let voucher_total_supply = 100_000_000; // 6 DP Voucher issuance
			let contribution: Balance = 100_000_000_000_000_000_000; // Contribution in 18 DP asset

			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
				soft_cap_price,
				funds_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			)
			.unwrap();

			// We should get 2_000_000 vouchers (at 6DP)
			let expected_vouchers = 2_000_000;
			assert_eq!(user_vouchers, expected_vouchers);
		});
	}

	#[test]
	fn partial_rewards() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 50_000_000;
			let funds_raised = 10_000_000_000_000;
			let voucher_total_supply = 135_000; // 135000 * 50 = 6_750_000_000_000
			let contribution = 50_000_000;

			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
				soft_cap_price,
				funds_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			)
			.unwrap();

			let expected_vouchers = 675000;
			assert_eq!(user_vouchers, expected_vouchers);
		});
	}

	#[test]
	fn partial_rewards_2() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 10_000_000;
			let funds_raised = 20_000_000_000;
			let voucher_total_supply = 1000;
			let contribution = 500_000_000;

			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
				soft_cap_price,
				funds_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			)
			.unwrap();

			let expected_vouchers = 25_000_000;
			assert_eq!(user_vouchers, expected_vouchers);
		});
	}

	#[test]
	fn partial_rewards_3() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 10_000_000_000_000_000_000;
			let funds_raised = 20_000_000_000_000_000_000_000;
			let voucher_total_supply = 1000;
			let contribution: Balance = 500_000_000_000_000_000_000;

			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
				soft_cap_price,
				funds_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			)
			.unwrap();

			let expected_vouchers = 25_000_000;
			assert_eq!(user_vouchers, expected_vouchers);
		});
	}

	#[test]
	fn rounding_works() {
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
				let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
					soft_cap_price,
					funds_raised,
					contributions[i as usize].into(),
					voucher_total_supply,
					total_vouchers,
					total_paid_contributions.into(),
				)
				.unwrap();
				total_vouchers += user_vouchers;
				total_paid_contributions += contributions[i as usize];
			}

			assert_eq!(total_vouchers, add_decimals(voucher_total_supply, VOUCHER_DECIMALS));
		});
	}

	#[test]
	fn rounding_smallest_issue_with_adjustments() {
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
				let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
					soft_cap_price,
					funds_raised,
					contributions[i as usize].into(),
					voucher_total_supply,
					total_vouchers,
					total_paid_contributions.into(),
				)
				.unwrap();
				total_vouchers += user_vouchers;
				total_paid_contributions += contributions[i as usize];
			}

			assert_eq!(total_vouchers, add_decimals(voucher_total_supply, VOUCHER_DECIMALS));
		});
	}

	#[test]
	fn rounding_many_contributors() {
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
					let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
						soft_cap_price,
						funds_raised,
						contributions[i as usize].into(),
						voucher_total_supply,
						total_vouchers,
						total_paid_contributions.into(),
					)
					.unwrap();
					total_vouchers += user_vouchers;
					total_paid_contributions += contributions[i as usize];
				}

				assert_eq!(total_vouchers, add_decimals(voucher_total_supply, VOUCHER_DECIMALS));
			}
		});
	}

	#[test]
	fn rounding_many_total_supplies() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 6;
			let iterations = 1000;
			// Test that total vouchers is always correct even with varying total supplies
			for n in 1..iterations {
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
					let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
						soft_cap_price,
						funds_raised,
						contributions[i as usize].into(),
						voucher_total_supply,
						total_vouchers,
						total_paid_contributions.into(),
					)
					.unwrap();
					total_vouchers += user_vouchers;
					total_paid_contributions += contributions[i as usize];
				}

				assert_eq!(total_vouchers, add_decimals(voucher_total_supply, VOUCHER_DECIMALS));
			}
		});
	}

	#[test]
	fn many_single_payments() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 1;
			// all contributing 1 each.
			// If not accounted for, our total supply would be 0
			let total_contributors: Balance = 10_000_000;
			let soft_cap_price = add_decimals(1, decimals);
			let voucher_total_supply = 1;

			let mut funds_raised = 0;
			let mut contributions: Vec<Balance> = Vec::new();
			for _ in 0..total_contributors {
				let contribution = 1;
				funds_raised += contribution;
				contributions.push(contribution);
			}

			let mut total_vouchers = 0;
			let mut total_paid_contributions = 0;
			for i in 0..total_contributors {
				let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
					soft_cap_price,
					funds_raised,
					contributions[i as usize].into(),
					voucher_total_supply,
					total_vouchers,
					total_paid_contributions.into(),
				)
				.unwrap();
				total_vouchers += user_vouchers;
				total_paid_contributions += contributions[i as usize];
			}

			assert_eq!(total_vouchers, add_decimals(voucher_total_supply, VOUCHER_DECIMALS));
		});
	}

	#[test]
	fn over_max_decimals() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 32;
			// all contributing 1 each.
			// If not accounted for, our total supply would be 0
			let soft_cap_price = add_decimals(1, decimals);
			let voucher_total_supply = MaxTokensPerCollection::get() as u128;
			let total_raised = voucher_total_supply * soft_cap_price;
			let contribution = total_raised;
			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
				soft_cap_price,
				total_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			)
			.unwrap();

			assert_eq!(user_vouchers, add_decimals(voucher_total_supply, VOUCHER_DECIMALS));
		});
	}

	#[test]
	fn zero_total_funds() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 18;
			let soft_cap_price = add_decimals(10, decimals);
			let voucher_total_supply = 12;
			let total_raised = 0;
			let contribution = 0;

			// Although this should never happen, in the case where total raised is zero
			// we should expect 0 to be paid out
			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
				soft_cap_price,
				total_raised,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			)
			.unwrap();

			assert_eq!(user_vouchers, 0);
		});
	}

	#[test]
	fn zero_total_funds_and_soft_cap() {
		TestExt::<Test>::default().build().execute_with(|| {
			let soft_cap_price = 0;
			let voucher_total_supply = 12;
			let total_raised = 0;
			let contribution = 0;

			// Where soft cap and total funds raised are both 0, an error should be returned
			assert_err!(
				Pallet::<Test>::calculate_voucher_rewards(
					soft_cap_price,
					total_raised,
					contribution.into(),
					voucher_total_supply,
					0,
					0.into(),
				),
				"Voucher price must be greater than 0"
			);
		});
	}

	#[test]
	fn zero_total_supply() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 6;
			let soft_cap_price = add_decimals(1, decimals);
			let voucher_total_supply = 0;
			let total_raised = soft_cap_price;
			let contribution = soft_cap_price;

			// Although this should never happen, in the case where total supply is zero
			// we should expect 0 to be paid out
			assert_err!(
				Pallet::<Test>::calculate_voucher_rewards(
					soft_cap_price,
					total_raised,
					contribution.into(),
					voucher_total_supply,
					0,
					0.into(),
				),
				"Voucher max supply must be greater than 0"
			);
		});
	}

	#[test]
	#[ignore]
	// TODO Remove this test. It is purely for demonstrating the difference between the old and new
	// methods
	fn old_vs_new_test_for_demonstration() {
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
				let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
					soft_cap_price,
					funds_raised,
					contributions[i as usize].into(),
					voucher_total_supply,
					total_vouchers,
					total_paid_contributions.into(),
				)
				.unwrap();

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
	fn doesnt_exceed_max_supply() {
		TestExt::<Test>::default().build().execute_with(|| {
			let decimals = 6;
			let soft_cap_price = add_decimals(1, decimals);
			let voucher_total_supply = 1;
			let contribution = 11111111111111111;
			let user_vouchers = Pallet::<Test>::calculate_voucher_rewards(
				soft_cap_price,
				contribution,
				contribution.into(),
				voucher_total_supply,
				0,
				0.into(),
			)
			.unwrap();

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
			let max_issuance = 10_000;
			let reward_collection_id = create_nft_collection(alice(), max_issuance);
			let payment_asset_id = 1;
			let soft_cap_price = 10;
			let duration = 100;

			// Get sale_id
			let sale_id = NextSaleId::<Test>::get();
			// Get next asset id
			let next_asset_id = AssetsExt::next_asset_uuid().unwrap();

			// Initialize the crowdsale
			assert_ok!(Crowdsale::initialize(
				Some(alice()).into(),
				payment_asset_id,
				reward_collection_id,
				soft_cap_price,
				duration
			));

			let vault = Pallet::<Test>::vault_account(sale_id);
			let sale_info = SaleInformation::<AccountId, BlockNumber> {
				status: SaleStatus::Pending(System::block_number()),
				admin: alice(),
				vault,
				payment_asset_id,
				reward_collection_id,
				soft_cap_price,
				funds_raised: 0,
				voucher_asset_id: next_asset_id,
				duration,
			};
			// Check storage
			assert_eq!(SaleInfo::<Test>::get(sale_id).unwrap(), sale_info);
			assert_eq!(NextSaleId::<Test>::get(), sale_id + 1);

			// Check NFT collection ownership
			let collection_info = Nft::get_collection_info(reward_collection_id).unwrap();
			assert_eq!(collection_info.owner, vault);

			// Check voucher asset max issuance
			let token_issuance = AssetsExt::total_issuance(next_asset_id);
			assert_eq!(token_issuance, add_decimals(max_issuance.into(), VOUCHER_DECIMALS));

			// Check voucher balance
			let vault_balance =
				AssetsExt::reducible_balance(sale_info.voucher_asset_id, &sale_info.vault, false);
			assert_eq!(vault_balance, token_issuance);

			// Check all relevant events thrown
			System::assert_has_event(
				pallet_assets_ext::Event::CreateAsset {
					asset_id: next_asset_id,
					creator: vault,
					initial_balance: 1,
				}
				.into(),
			);
			System::assert_has_event(
				pallet_nft::Event::OwnerSet {
					collection_id: reward_collection_id,
					new_owner: vault,
				}
				.into(),
			);
			System::assert_last_event(Event::CrowdsaleCreated { sale_id, info: sale_info }.into());
		});
	}

	#[test]
	fn no_ids_fails() {
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

	#[test]
	fn invalid_asset_fails() {
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

	#[test]
	fn invalid_soft_cap_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let collection_id = create_nft_collection(alice(), 10);
			let payment_asset = 1;
			let soft_cap_price = 0;
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
				Error::<Test>::InvalidSoftCapPrice
			);
		});
	}

	#[test]
	fn invalid_sale_duration_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let collection_id = create_nft_collection(alice(), 10);
			let payment_asset = 1;
			let soft_cap_price = 10;
			let duration = MaxSaleDuration::get() + 1;

			// Initialize the crowdsale
			assert_noop!(
				Crowdsale::initialize(
					Some(alice()).into(),
					payment_asset,
					collection_id,
					soft_cap_price,
					duration
				),
				Error::<Test>::SaleDurationTooLong
			);
		});
	}

	#[test]
	fn no_collection_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let collection_id = 1;
			let payment_asset = 1;
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
				pallet_nft::Error::<Test>::NoCollectionFound
			);
		});
	}

	#[test]
	fn invalid_collection_max_issuance_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let collection_id = Nft::next_collection_uuid().unwrap();
			let max_issuance = None;
			assert_ok!(Nft::create_collection(
				Some(alice()).into(),
				bounded_string("test-collection"),
				0,
				max_issuance,
				None,
				MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));
			let payment_asset = 1;
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
				Error::<Test>::MaxIssuanceNotSet
			);
		});
	}

	#[test]
	fn invalid_collection_total_issuance_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let collection_id = Nft::next_collection_uuid().unwrap();
			let initial_issuance = 1;
			assert_ok!(Nft::create_collection(
				Some(alice()).into(),
				bounded_string("test-collection"),
				initial_issuance,
				Some(1000),
				None,
				MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));
			let payment_asset = 1;
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
				Error::<Test>::CollectionIssuanceNotZero
			);
		});
	}

	#[test]
	fn not_collection_owner_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let collection_id = Nft::next_collection_uuid().unwrap();
			let collection_owner = alice();
			assert_ok!(Nft::create_collection(
				Some(collection_owner).into(),
				bounded_string("test-collection"),
				0,
				Some(1000),
				None,
				MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));
			let payment_asset = 1;
			let soft_cap_price = 10;
			let duration = 100;

			// Initialize the crowdsale
			assert_noop!(
				Crowdsale::initialize(
					Some(bob()).into(), // Not collection owner
					payment_asset,
					collection_id,
					soft_cap_price,
					duration
				),
				pallet_nft::Error::<Test>::NotCollectionOwner
			);
		});
	}
}

mod enable {
	use super::*;

	#[test]
	fn enable_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (sale_id, mut sale_info) = initialize_crowdsale(100);

			assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

			// Sale info status updated
			let block_number = System::block_number();
			let new_sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
			assert_eq!(new_sale_info.status, SaleStatus::Enabled(block_number));

			// SaleEndBlocks updated
			let end_block = block_number + sale_info.duration;
			assert_eq!(
				SaleEndBlocks::<Test>::get(end_block).unwrap(),
				BoundedVec::<SaleId, MaxSalesPerBlock>::truncate_from(vec![sale_id])
			);

			// Event emitted
			let end_block = block_number + sale_info.duration;
			sale_info.status = SaleStatus::Enabled(block_number);
			System::assert_last_event(
				Event::CrowdsaleEnabled { sale_id, info: sale_info, end_block }.into(),
			);
		});
	}

	#[test]
	fn not_admin_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (sale_id, _) = initialize_crowdsale(100);

			// Bob fails
			assert_noop!(
				Crowdsale::enable(Some(bob()).into(), sale_id),
				Error::<Test>::AccessDenied
			);

			// Alice ok
			assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));
		});
	}

	#[test]
	fn no_sale_failes() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Sale not set up
			assert_noop!(
				Crowdsale::enable(Some(alice()).into(), 2),
				Error::<Test>::CrowdsaleNotFound
			);
		});
	}

	#[test]
	fn too_many_sales_at_end_block_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (sale_id, sale_info) = initialize_crowdsale(100);

			// Insert 5 sales at the same end block
			let end_block = System::block_number() + sale_info.duration;
			let sale_ids = vec![1, 2, 3, 4, 5];
			SaleEndBlocks::<Test>::insert(
				end_block,
				BoundedVec::<SaleId, MaxSalesPerBlock>::truncate_from(sale_ids),
			);

			// Any more should fail
			assert_noop!(
				Crowdsale::enable(Some(alice()).into(), sale_id),
				Error::<Test>::TooManySales
			);

			// Moving forward one block should allow the sale to be enabled
			System::set_block_number(System::block_number() + 1);
			assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));
		});
	}

	#[test]
	fn invalid_sale_status_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (sale_id, mut sale_info) = initialize_crowdsale(100);

			sale_info.status = SaleStatus::Enabled(0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_noop!(
				Crowdsale::enable(Some(alice()).into(), sale_id),
				Error::<Test>::InvalidCrowdsaleStatus
			);

			sale_info.status = SaleStatus::Distributing(0, 0, 0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_noop!(
				Crowdsale::enable(Some(alice()).into(), sale_id),
				Error::<Test>::InvalidCrowdsaleStatus
			);

			sale_info.status = SaleStatus::Ended(0, 0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_noop!(
				Crowdsale::enable(Some(alice()).into(), sale_id),
				Error::<Test>::InvalidCrowdsaleStatus
			);

			sale_info.status = SaleStatus::DistributionFailed(0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_noop!(
				Crowdsale::enable(Some(alice()).into(), sale_id),
				Error::<Test>::InvalidCrowdsaleStatus
			);

			// Sanity check
			sale_info.status = SaleStatus::Pending(0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));
		});
	}
}

mod participate {
	use super::*;

	#[test]
	fn participate_works() {
		let initial_balance = 1_000_000;

		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let (sale_id, sale_info) = initialize_crowdsale(100);

				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				let amount = 10_000;
				assert_ok!(Crowdsale::participate(Some(bob()).into(), sale_id, amount));

				// Check storage
				let vault = sale_info.vault;
				let asset_id = sale_info.payment_asset_id;

				// Vault account should have the contributed amount
				let vault_balance = AssetsExt::reducible_balance(asset_id, &vault, false);
				assert_eq!(vault_balance, amount);

				// Bobs balance should be decreased
				let bob_balance = AssetsExt::reducible_balance(asset_id, &bob(), false);
				assert_eq!(bob_balance, initial_balance - amount);

				// Contribution should be stored
				assert_eq!(SaleParticipation::<Test>::get(sale_id, bob()).unwrap(), amount);
				assert_eq!(SaleInfo::<Test>::get(sale_id).unwrap().funds_raised, amount);

				// Event thrown
				System::assert_last_event(
					Event::CrowdsaleParticipated { sale_id, who: bob(), asset: asset_id, amount }
						.into(),
				);
			});
	}

	#[test]
	fn multiple_participations_adds_funds_correctly() {
		let initial_balance = 1_000_000;
		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance), (charlie(), initial_balance)])
			.build()
			.execute_with(|| {
				let (sale_id, sale_info) = initialize_crowdsale(100);

				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Bob's participations
				let b_amount_1 = 10_000;
				let b_amount_2 = 20_000;
				let b_amount_3 = 30_000;
				assert_ok!(Crowdsale::participate(Some(bob()).into(), sale_id, b_amount_1));
				assert_ok!(Crowdsale::participate(Some(bob()).into(), sale_id, b_amount_2));
				assert_ok!(Crowdsale::participate(Some(bob()).into(), sale_id, b_amount_3));

				// Charlie's participation
				let c_amount_1 = 40_000;
				let c_amount_2 = 50_000;
				let c_amount_3 = 60_000;
				assert_ok!(Crowdsale::participate(Some(charlie()).into(), sale_id, c_amount_1));
				assert_ok!(Crowdsale::participate(Some(charlie()).into(), sale_id, c_amount_2));
				assert_ok!(Crowdsale::participate(Some(charlie()).into(), sale_id, c_amount_3));

				// Check storage
				let vault = sale_info.vault;
				let asset_id = sale_info.payment_asset_id;

				// Vault account should have the contributed amount
				let vault_balance = AssetsExt::reducible_balance(asset_id, &vault, false);
				let expected_vault_balance =
					b_amount_1 + b_amount_2 + b_amount_3 + c_amount_1 + c_amount_2 + c_amount_3;
				assert_eq!(vault_balance, expected_vault_balance);

				// Bobs balance should be decreased
				let bob_balance = AssetsExt::reducible_balance(asset_id, &bob(), false);
				let expected_bob_balance = initial_balance - b_amount_1 - b_amount_2 - b_amount_3;
				assert_eq!(bob_balance, expected_bob_balance);

				// Contribution should be stored
				assert_eq!(
					SaleParticipation::<Test>::get(sale_id, bob()).unwrap(),
					b_amount_1 + b_amount_2 + b_amount_3
				);
				assert_eq!(
					SaleParticipation::<Test>::get(sale_id, charlie()).unwrap(),
					c_amount_1 + c_amount_2 + c_amount_3
				);
				assert_eq!(
					SaleInfo::<Test>::get(sale_id).unwrap().funds_raised,
					expected_vault_balance
				);
			});
	}

	#[test]
	fn no_sale_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_noop!(
				Crowdsale::participate(Some(alice()).into(), 1, 2),
				Error::<Test>::CrowdsaleNotFound
			);
		});
	}

	#[test]
	fn invalid_sale_status_fails() {
		let initial_balance = 1_000_000;
		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let (sale_id, mut sale_info) = initialize_crowdsale(100);
				let amount = 2;

				sale_info.status = SaleStatus::Pending(0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_noop!(
					Crowdsale::participate(Some(bob()).into(), sale_id, amount),
					Error::<Test>::CrowdsaleNotEnabled
				);

				sale_info.status = SaleStatus::Distributing(0, 0, 0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_noop!(
					Crowdsale::participate(Some(bob()).into(), sale_id, amount),
					Error::<Test>::CrowdsaleNotEnabled
				);

				sale_info.status = SaleStatus::Ended(0, 0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_noop!(
					Crowdsale::participate(Some(bob()).into(), sale_id, amount),
					Error::<Test>::CrowdsaleNotEnabled
				);

				sale_info.status = SaleStatus::DistributionFailed(0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_noop!(
					Crowdsale::participate(Some(bob()).into(), sale_id, amount),
					Error::<Test>::CrowdsaleNotEnabled
				);

				// Sanity check
				sale_info.status = SaleStatus::Enabled(0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_ok!(Crowdsale::participate(Some(bob()).into(), sale_id, amount));
			});
	}

	#[test]
	fn zero_participation_fails() {
		let initial_balance = 1_000_000;

		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let (sale_id, _) = initialize_crowdsale(100);

				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				let amount = 0;
				assert_noop!(
					Crowdsale::participate(Some(bob()).into(), sale_id, amount),
					Error::<Test>::InvalidAmount
				);
			});
	}

	#[test]
	fn insufficient_balance_fails() {
		let initial_balance = 1_000_000;

		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let (sale_id, sale_info) = initialize_crowdsale(100);

				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				let amount = 10_000;
				assert_ok!(Crowdsale::participate(Some(bob()).into(), sale_id, amount));

				// Check storage
				let vault = sale_info.vault;
				let asset_id = sale_info.payment_asset_id;

				// Vault account should have the contributed amount
				let vault_balance = AssetsExt::reducible_balance(asset_id, &vault, false);
				assert_eq!(vault_balance, amount);

				// Bobs balance should be decreased
				let bob_balance = AssetsExt::reducible_balance(asset_id, &bob(), false);
				assert_eq!(bob_balance, initial_balance - amount);

				// Contribution should be stored
				assert_eq!(SaleParticipation::<Test>::get(sale_id, bob()).unwrap(), amount);
				assert_eq!(SaleInfo::<Test>::get(sale_id).unwrap().funds_raised, amount);

				// Event thrown
				System::assert_last_event(
					Event::CrowdsaleParticipated { sale_id, who: bob(), asset: asset_id, amount }
						.into(),
				);
			});
	}
}

mod on_initialize {
	use super::*;

	#[test]
	fn on_initialize_works() {
		let initial_balance = 1_000_000;

		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let max_issuance = 100;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
				let voucher_asset_id = sale_info.voucher_asset_id;

				let vault_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &sale_info.vault, false);
				assert_eq!(vault_balance, add_decimals(max_issuance, VOUCHER_DECIMALS));

				// Enable crowdsale
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount
				let participation_amount = 10;
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Check storage
				assert_eq!(SaleEndBlocks::<Test>::get(end_block), None);
				let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
				assert_eq!(sale_info.status, SaleStatus::Distributing(end_block, 0, 0));
				assert_eq!(SaleDistribution::<Test>::get().into_inner(), vec![sale_id]);

				// Check vouchers are refunded to admin
				let voucher_asset_id = sale_info.voucher_asset_id;
				let vault_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &sale_info.vault, false);
				let admin_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &sale_info.admin, false);

				// Vault account should have the vouchers that are to be paid out
				let vault_expected =
					add_decimals(participation_amount, VOUCHER_DECIMALS) / sale_info.soft_cap_price;
				assert_eq!(vault_balance, vault_expected);
				// Admin account should have refunded vouchers
				let admin_expected = add_decimals(max_issuance, VOUCHER_DECIMALS) - vault_expected;
				assert_eq!(admin_balance, admin_expected);

				// Event thrown
				System::assert_last_event(
					Event::CrowdsaleClosed { sale_id, info: sale_info }.into(),
				);
			});
	}

	#[test]
	fn over_committed_doesnt_pay_admin() {
		let initial_balance = 1_000_000;

		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let max_issuance = 100;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
				let voucher_asset_id = sale_info.voucher_asset_id;

				let vault_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &sale_info.vault, false);
				assert_eq!(vault_balance, add_decimals(max_issuance, VOUCHER_DECIMALS));

				// Enable crowdsale
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount == target amount
				let participation_amount = 1000;
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Check storage
				assert_eq!(SaleEndBlocks::<Test>::get(end_block), None);
				let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
				assert_eq!(sale_info.status, SaleStatus::Distributing(end_block, 0, 0));
				assert_eq!(SaleDistribution::<Test>::get().into_inner(), vec![sale_id]);

				// Check no vouchers are refunded to admin
				let voucher_asset_id = sale_info.voucher_asset_id;
				let vault_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &sale_info.vault, false);
				let admin_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &sale_info.admin, false);

				// Vault account has the entire voucher supply
				assert_eq!(vault_balance, add_decimals(max_issuance, VOUCHER_DECIMALS));
				// Admin gets no refund
				assert_eq!(admin_balance, 0);

				// Event thrown
				System::assert_last_event(
					Event::CrowdsaleClosed { sale_id, info: sale_info }.into(),
				);
			});
	}

	#[test]
	fn under_committed_pays_admin() {
		let initial_balance = 1_000_000;

		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let max_issuance = 100;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);

				// Enable crowdsale
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount just under target amount
				let participation_amount = 999;
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Check storage
				assert_eq!(SaleEndBlocks::<Test>::get(end_block), None);
				let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
				assert_eq!(sale_info.status, SaleStatus::Distributing(end_block, 0, 0));
				assert_eq!(SaleDistribution::<Test>::get().into_inner(), vec![sale_id]);

				// Check no vouchers are refunded to admin
				let voucher_asset_id = sale_info.voucher_asset_id;
				let vault_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &sale_info.vault, false);
				let admin_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &sale_info.admin, false);

				// Vault account should have the vouchers that are to be paid out
				let vault_expected =
					add_decimals(participation_amount, VOUCHER_DECIMALS) / sale_info.soft_cap_price;
				assert_eq!(vault_balance, vault_expected);
				// Admin account should have refunded vouchers
				let admin_expected = add_decimals(max_issuance, VOUCHER_DECIMALS) - vault_expected;
				assert_eq!(admin_balance, admin_expected);

				// Event thrown
				System::assert_last_event(
					Event::CrowdsaleClosed { sale_id, info: sale_info }.into(),
				);
			});
	}

	#[test]
	fn zero_balance_skips_distribution() {
		let initial_balance = 1_000_000;

		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let (sale_id, sale_info) = initialize_crowdsale(100);

				// Enable crowdsale
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Call on_initialize at sale close with no participation
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Check storage
				assert_eq!(SaleEndBlocks::<Test>::get(end_block), None);
				let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
				// Status should be ended with 0 funds raised
				assert_eq!(sale_info.status, SaleStatus::Ended(end_block, 0));
				assert!(SaleDistribution::<Test>::get().is_empty());

				// Event thrown
				System::assert_last_event(
					Event::CrowdsaleClosed { sale_id, info: sale_info }.into(),
				);
			});
	}

	#[test]
	fn distribution_failed_works() {
		let initial_balance = 1_000_000;

		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let max_issuance = 100;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);

				// Enable crowdsale
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount
				let participation_amount = 10;
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));

				// Fill SaleDistribution with random sale_ids
				let sale_ids = vec![3; MaxConsecutiveSales::get() as usize];
				SaleDistribution::<Test>::put(BoundedVec::truncate_from(sale_ids.clone()));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Check storage
				assert_eq!(SaleEndBlocks::<Test>::get(end_block), None);
				let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
				// Status should be DistributionFailed
				assert_eq!(sale_info.status, SaleStatus::DistributionFailed(end_block));
				assert_eq!(SaleDistribution::<Test>::get().into_inner(), sale_ids);

				// Check vouchers are refunded to admin
				let voucher_asset_id = sale_info.voucher_asset_id;
				let vault_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &sale_info.vault, false);
				let admin_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &sale_info.admin, false);

				// Vault account should have the vouchers that are to be paid out
				let vault_expected =
					add_decimals(participation_amount, VOUCHER_DECIMALS) / sale_info.soft_cap_price;
				assert_eq!(vault_balance, vault_expected);
				// Admin account should have refunded vouchers
				let admin_expected = add_decimals(max_issuance, VOUCHER_DECIMALS) - vault_expected;
				assert_eq!(admin_balance, admin_expected);
			});
	}
}

mod claim_voucher {
	use super::*;

	#[test]
	fn claim_voucher_works() {
		let initial_balance = 1_000_000;
		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let max_issuance = 1000;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
				let voucher_asset_id = sale_info.voucher_asset_id;
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount
				let participation_amount = 100;
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Sanity check, should have no vouchers
				let voucher_balance = AssetsExt::reducible_balance(voucher_asset_id, &bob(), false);
				assert_eq!(voucher_balance, 0);
				assert_eq!(
					SaleParticipation::<Test>::get(sale_id, bob()),
					Some(participation_amount)
				);

				// Manual claim
				assert_ok!(Crowdsale::claim_voucher(Some(bob()).into(), sale_id));

				// Check storage updated
				assert_eq!(SaleParticipation::<Test>::get(sale_id, bob()), None);

				// Check balance
				let voucher_balance = AssetsExt::reducible_balance(voucher_asset_id, &bob(), false);
				let expected_balance =
					add_decimals(participation_amount, VOUCHER_DECIMALS) / sale_info.soft_cap_price;
				assert_eq!(voucher_balance, expected_balance);

				// Check vault balance is 0 (All vouchers redeemed)
				let vault_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &sale_info.vault, false);
				assert_eq!(vault_balance, 0);

				// Check sale_info.status updated to Ended
				let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
				assert_eq!(sale_info.status, SaleStatus::Ended(end_block, expected_balance));

				// Event thrown
				System::assert_last_event(
					Event::CrowdsaleVouchersClaimed {
						sale_id,
						who: bob(),
						amount: expected_balance,
					}
					.into(),
				);
			});
	}

	#[test]
	fn multiple_claims_updates_status() {
		let initial_balance = 1_000_000;
		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance), (charlie(), initial_balance)])
			.build()
			.execute_with(|| {
				let max_issuance = 1000;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
				let voucher_asset_id = sale_info.voucher_asset_id;
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount
				let b_amount = 10;
				assert_ok!(Crowdsale::participate(Some(bob()).into(), sale_id, b_amount));
				let c_amount = 35;
				assert_ok!(Crowdsale::participate(Some(charlie()).into(), sale_id, c_amount));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Manual claim Bob,
				assert_ok!(Crowdsale::claim_voucher(Some(bob()).into(), sale_id));
				assert_eq!(SaleParticipation::<Test>::get(sale_id, bob()), None);

				// Check Bob balance
				let bob_balance = AssetsExt::reducible_balance(voucher_asset_id, &bob(), false);
				let bob_expected =
					add_decimals(b_amount, VOUCHER_DECIMALS) / sale_info.soft_cap_price;
				assert_eq!(bob_balance, bob_expected);

				// Check sale_info.status still distributing
				let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
				assert_eq!(
					sale_info.status,
					SaleStatus::Distributing(end_block, b_amount, bob_balance)
				);

				// Manual claim Charlie
				assert_ok!(Crowdsale::claim_voucher(Some(charlie()).into(), sale_id));
				assert_eq!(SaleParticipation::<Test>::get(sale_id, charlie()), None);

				// Check Charlie balance
				let charlie_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &charlie(), false);
				let charlie_expected =
					add_decimals(c_amount, VOUCHER_DECIMALS) / sale_info.soft_cap_price;
				assert_eq!(charlie_balance, charlie_expected);

				// Check sale_info.status updated to Ended
				let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
				let vouchers_issued = charlie_balance + bob_balance;
				assert_eq!(sale_info.status, SaleStatus::Ended(end_block, vouchers_issued));
			});
	}

	#[test]
	fn multiple_claims() {
		// Participate 10 accounts
		let participations = vec![
			(create_account(1), 100),
			(create_account(2), 200),
			(create_account(3), 300),
			(create_account(4), 400),
			(create_account(5), 500),
			(create_account(6), 600),
			(create_account(7), 700),
			(create_account(8), 800),
			(create_account(9), 900),
			(create_account(10), 1000),
		];
		TestExt::<Test>::default()
			.with_balances(&participations)
			.build()
			.execute_with(|| {
				let max_issuance = 1000;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
				let voucher_asset_id = sale_info.voucher_asset_id;
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Each account participates their amount
				for (account, amount) in participations.clone() {
					assert_ok!(Crowdsale::participate(Some(account).into(), sale_id, amount));
				}

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Sanity check, all accounts should have no vouchers
				for (account, _) in participations.clone() {
					let voucher_balance =
						AssetsExt::reducible_balance(voucher_asset_id, &account, false);
					assert_eq!(voucher_balance, 0);
				}

				// Manual claim for each
				let mut total_vouchers_issued = 0;
				for (account, amount) in participations {
					assert_ok!(Crowdsale::claim_voucher(Some(account).into(), sale_id));
					let voucher_balance =
						AssetsExt::reducible_balance(voucher_asset_id, &account, false);
					let expected_balance =
						add_decimals(amount, VOUCHER_DECIMALS) / sale_info.soft_cap_price;
					assert_eq!(voucher_balance, expected_balance);
					assert_eq!(SaleParticipation::<Test>::get(sale_id, account), None);
					total_vouchers_issued += voucher_balance;

					// Event thrown
					System::assert_has_event(
						Event::CrowdsaleVouchersClaimed {
							sale_id,
							who: account,
							amount: expected_balance,
						}
						.into(),
					);
				}

				// Check ended status
				let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
				assert_eq!(sale_info.status, SaleStatus::Ended(end_block, total_vouchers_issued));

				// Check vault balance is 0 (All vouchers redeemed)
				let vault_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &sale_info.vault, false);
				assert_eq!(vault_balance, 0);
			});
	}

	#[test]
	fn no_sale_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_noop!(
				Crowdsale::claim_voucher(Some(bob()).into(), 2),
				Error::<Test>::CrowdsaleNotFound
			);
		});
	}

	#[test]
	fn claim_twice_fails() {
		let initial_balance = 1_000_000;
		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance), (charlie(), initial_balance)])
			.build()
			.execute_with(|| {
				let max_issuance = 1000;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount
				let participation_amount = 100;
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));
				assert_ok!(Crowdsale::participate(
					Some(charlie()).into(),
					sale_id,
					participation_amount
				));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Manual claim
				assert_ok!(Crowdsale::claim_voucher(Some(bob()).into(), sale_id));
				// Check storage updated
				assert_eq!(SaleParticipation::<Test>::get(sale_id, bob()), None);

				// Manual claim again fails
				assert_noop!(
					Crowdsale::claim_voucher(Some(bob()).into(), sale_id),
					Error::<Test>::VouchersAlreadyClaimed
				);
			});
	}

	#[test]
	fn invalid_status_fails() {
		let initial_balance = 1_000_000;
		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let (sale_id, mut sale_info) = initialize_crowdsale(1000);
				let participation_amount = 100;
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));

				sale_info.status = SaleStatus::Enabled(0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_noop!(
					Crowdsale::claim_voucher(Some(bob()).into(), sale_id),
					Error::<Test>::InvalidCrowdsaleStatus
				);

				sale_info.status = SaleStatus::Pending(0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_noop!(
					Crowdsale::claim_voucher(Some(bob()).into(), sale_id),
					Error::<Test>::InvalidCrowdsaleStatus
				);

				sale_info.status = SaleStatus::Ended(0, 0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_noop!(
					Crowdsale::claim_voucher(Some(bob()).into(), sale_id),
					Error::<Test>::InvalidCrowdsaleStatus
				);

				sale_info.status = SaleStatus::DistributionFailed(0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_noop!(
					Crowdsale::claim_voucher(Some(bob()).into(), sale_id),
					Error::<Test>::InvalidCrowdsaleStatus
				);

				// Sanity check
				sale_info.status = SaleStatus::Distributing(0, 0, 0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_ok!(Crowdsale::claim_voucher(Some(bob()).into(), sale_id));
			});
	}
}

mod redeem_voucher {
	use super::*;

	#[test]
	fn redeem_voucher_works() {
		let initial_balance = 1_000_000;
		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let max_issuance = 1000;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
				let voucher_asset_id = sale_info.voucher_asset_id;
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount
				let participation_amount = 100;
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Manual claim
				assert_ok!(Crowdsale::claim_voucher(Some(bob()).into(), sale_id));
				let voucher_balance = AssetsExt::reducible_balance(voucher_asset_id, &bob(), false);
				let quantity = voucher_balance / 10u128.pow(VOUCHER_DECIMALS as u32);

				// Redeem voucher
				assert_ok!(Crowdsale::redeem_voucher(Some(bob()).into(), sale_id, quantity as u32));

				// Check voucher total_issuance reduced
				let total_issuance = AssetsExt::total_issuance(voucher_asset_id);
				assert_eq!(
					total_issuance,
					add_decimals(max_issuance, VOUCHER_DECIMALS) - voucher_balance as u128
				);

				// Check voucher burned
				let voucher_balance = AssetsExt::reducible_balance(voucher_asset_id, &bob(), false);
				assert_eq!(voucher_balance, 0);

				// Check NFT ownership
				assert_eq!(
					Nft::token_balance_of(&bob(), sale_info.reward_collection_id),
					quantity as u32
				);

				// Event thrown
				System::assert_last_event(
					Event::CrowdsaleNFTRedeemed {
						sale_id,
						who: bob(),
						collection_id: sale_info.reward_collection_id,
						quantity: quantity as u32,
					}
					.into(),
				);
			});
	}

	#[test]
	fn fractional_voucher() {
		let initial_balance = 1_000_000;
		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let max_issuance = 1000;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
				let voucher_asset_id = sale_info.voucher_asset_id;
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount that is not a multiple of softcap price (10)
				let participation_amount = 105;
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Manual claim
				assert_ok!(Crowdsale::claim_voucher(Some(bob()).into(), sale_id));
				let voucher_balance = AssetsExt::reducible_balance(voucher_asset_id, &bob(), false);
				// Quantity = 105 / 10 = 10.5, rounded to 10
				let quantity = voucher_balance / 10u128.pow(VOUCHER_DECIMALS as u32);

				// Redeem voucher
				assert_ok!(Crowdsale::redeem_voucher(Some(bob()).into(), sale_id, quantity as u32));

				// Check voucher burned and no remainder
				let voucher_balance = AssetsExt::reducible_balance(voucher_asset_id, &bob(), false);
				assert_eq!(voucher_balance, 500_000);

				// Check NFT ownership
				assert_eq!(
					Nft::token_balance_of(&bob(), sale_info.reward_collection_id),
					quantity as u32
				);
			});
	}

	#[test]
	fn partial_redemption() {
		let initial_balance = 1_000_000;
		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let max_issuance = 1000;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
				let voucher_asset_id = sale_info.voucher_asset_id;
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount
				let participation_amount = 100;
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Manual claim
				assert_ok!(Crowdsale::claim_voucher(Some(bob()).into(), sale_id));
				let voucher_balance = AssetsExt::reducible_balance(voucher_asset_id, &bob(), false);
				// Quantity = 105 / 10 = 10.5, rounded to 10
				let quantity = voucher_balance / 10u128.pow(VOUCHER_DECIMALS as u32);
				let redeem_quantity = quantity as u32 - 1;

				// Redeem vouchers - 1
				assert_ok!(Crowdsale::redeem_voucher(Some(bob()).into(), sale_id, redeem_quantity));

				// Check voucher burned but remainder is kept
				let voucher_balance = AssetsExt::reducible_balance(voucher_asset_id, &bob(), false);
				assert_eq!(voucher_balance, 1_000_000);

				// Check NFT ownership
				assert_eq!(
					Nft::token_balance_of(&bob(), sale_info.reward_collection_id),
					redeem_quantity as u32
				);

				// Redeem last voucher
				assert_ok!(Crowdsale::redeem_voucher(Some(bob()).into(), sale_id, 1));

				// Check voucher burned but remainder is kept
				let voucher_balance = AssetsExt::reducible_balance(voucher_asset_id, &bob(), false);
				assert_eq!(voucher_balance, 0);

				// Check NFT ownership
				assert_eq!(
					Nft::token_balance_of(&bob(), sale_info.reward_collection_id),
					quantity as u32
				);
			});
	}

	#[test]
	fn zero_quantity_fails() {
		let initial_balance = 1_000_000;
		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let max_issuance = 1000;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount
				let participation_amount = 100;
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Manual claim
				assert_ok!(Crowdsale::claim_voucher(Some(bob()).into(), sale_id));
				// Redeem more vouchers than allocated
				let redeem_quantity = 0;
				assert_noop!(
					Crowdsale::redeem_voucher(Some(bob()).into(), sale_id, redeem_quantity),
					Error::<Test>::InvalidQuantity
				);
			});
	}

	#[test]
	fn no_sale_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_noop!(
				Crowdsale::redeem_voucher(Some(bob()).into(), 2, 1),
				Error::<Test>::CrowdsaleNotFound
			);
		});
	}

	#[test]
	fn insufficient_balance_fails() {
		let initial_balance = 1_000_000;
		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let max_issuance = 1000;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
				let voucher_asset_id = sale_info.voucher_asset_id;
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount
				let participation_amount = 100;
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Manual claim
				assert_ok!(Crowdsale::claim_voucher(Some(bob()).into(), sale_id));
				let voucher_balance = AssetsExt::reducible_balance(voucher_asset_id, &bob(), false);
				let quantity = voucher_balance / 10u128.pow(VOUCHER_DECIMALS as u32);

				// Redeem more vouchers than allocated
				let redeem_quantity = quantity as u32 + 1;
				assert_noop!(
					Crowdsale::redeem_voucher(Some(bob()).into(), sale_id, redeem_quantity),
					pallet_assets::Error::<Test>::BalanceLow
				);
			});
	}

	#[test]
	fn invalid_sale_status_fails() {
		let initial_balance = 1_000_000;
		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let (sale_id, mut sale_info) = initialize_crowdsale(100);
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount
				let participation_amount = 100;
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Manual claim
				assert_ok!(Crowdsale::claim_voucher(Some(bob()).into(), sale_id));

				// Check redeem_voucher against invalid statuses
				sale_info.status = SaleStatus::Enabled(0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_noop!(
					Crowdsale::redeem_voucher(Some(alice()).into(), sale_id, 1),
					Error::<Test>::InvalidCrowdsaleStatus
				);

				sale_info.status = SaleStatus::Pending(0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_noop!(
					Crowdsale::redeem_voucher(Some(alice()).into(), sale_id, 1),
					Error::<Test>::InvalidCrowdsaleStatus
				);

				sale_info.status = SaleStatus::DistributionFailed(0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_noop!(
					Crowdsale::redeem_voucher(Some(alice()).into(), sale_id, 1),
					Error::<Test>::InvalidCrowdsaleStatus
				);

				// Sanity check, these two should both work
				sale_info.status = SaleStatus::Distributing(0, 0, 0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_ok!(Crowdsale::redeem_voucher(Some(alice()).into(), sale_id, 1));

				sale_info.status = SaleStatus::Ended(0, 0);
				SaleInfo::<Test>::insert(sale_id, sale_info);
				assert_ok!(Crowdsale::redeem_voucher(Some(alice()).into(), sale_id, 1),);
			});
	}
}

mod try_force_distribution {
	use super::*;

	#[test]
	fn try_force_distribution_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let max_issuance = 1000;
			let (sale_id, mut sale_info) = initialize_crowdsale(max_issuance);

			// Manually set status to DistributionFailed
			sale_info.status = SaleStatus::DistributionFailed(0);
			SaleInfo::<Test>::insert(sale_id, sale_info);

			// Try force distribution
			assert_ok!(Crowdsale::try_force_distribution(Some(alice()).into(), sale_id));

			// Sale status should be set to Ended as nobody participated
			let block_number = System::block_number();
			sale_info.status = SaleStatus::Ended(block_number, 0);
			assert_eq!(SaleInfo::<Test>::get(sale_id).unwrap(), sale_info);

			// Event thrown
			System::assert_last_event(
				Event::CrowdsaleManualDistribution { sale_id, info: sale_info, who: alice() }
					.into(),
			);
		});
	}

	#[test]
	fn not_admin_can_call() {
		TestExt::<Test>::default().build().execute_with(|| {
			let max_issuance = 1000;
			let (sale_id, mut sale_info) = initialize_crowdsale(max_issuance);

			// Manually set status to DistributionFailed
			sale_info.status = SaleStatus::DistributionFailed(0);
			SaleInfo::<Test>::insert(sale_id, sale_info);

			// Try force distribution as not sale admin
			assert_ok!(Crowdsale::try_force_distribution(Some(bob()).into(), sale_id));
		});
	}

	#[test]
	fn no_sale_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_noop!(
				Crowdsale::try_force_distribution(Some(alice()).into(), 3),
				Error::<Test>::CrowdsaleNotFound
			);
		});
	}

	#[test]
	fn distribution_table_full_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let max_issuance = 1000;
			let (sale_id, mut sale_info) = initialize_crowdsale(max_issuance);

			// Manually set status to DistributionFailed
			sale_info.status = SaleStatus::DistributionFailed(0);
			SaleInfo::<Test>::insert(sale_id, sale_info);

			// Fill SaleDistribution with random sale_ids
			let sale_ids = vec![3; MaxConsecutiveSales::get() as usize];
			SaleDistribution::<Test>::put(BoundedVec::truncate_from(sale_ids.clone()));

			// Try force distribution fails as we are still full
			assert_noop!(
				Crowdsale::try_force_distribution(Some(alice()).into(), sale_id),
				Error::<Test>::SaleDistributionFailed
			);
		});
	}

	#[test]
	fn invalid_status_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let max_issuance = 1000;
			let (sale_id, mut sale_info) = initialize_crowdsale(max_issuance);

			sale_info.status = SaleStatus::Enabled(0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_noop!(
				Crowdsale::try_force_distribution(Some(bob()).into(), sale_id),
				Error::<Test>::InvalidCrowdsaleStatus
			);

			sale_info.status = SaleStatus::Pending(0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_noop!(
				Crowdsale::try_force_distribution(Some(bob()).into(), sale_id),
				Error::<Test>::InvalidCrowdsaleStatus
			);

			sale_info.status = SaleStatus::Ended(0, 0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_noop!(
				Crowdsale::try_force_distribution(Some(bob()).into(), sale_id),
				Error::<Test>::InvalidCrowdsaleStatus
			);

			sale_info.status = SaleStatus::Distributing(0, 0, 0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_noop!(
				Crowdsale::try_force_distribution(Some(bob()).into(), sale_id),
				Error::<Test>::InvalidCrowdsaleStatus
			);

			// Sanity check
			sale_info.status = SaleStatus::DistributionFailed(0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_ok!(Crowdsale::try_force_distribution(Some(bob()).into(), sale_id));
		});
	}

	#[test]
	fn triggers_distribution() {
		let initial_balance = 1_000_000;

		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let max_issuance = 100;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);

				// Enable crowdsale
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount
				let participation_amount = 10;
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));

				// Fill SaleDistribution with random sale_ids
				let sale_ids = vec![3; MaxConsecutiveSales::get() as usize];
				SaleDistribution::<Test>::put(BoundedVec::truncate_from(sale_ids.clone()));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Remove one sale_id from SaleDistribution
				let sale_ids = vec![3; MaxConsecutiveSales::get() as usize - 1];
				SaleDistribution::<Test>::put(BoundedVec::truncate_from(sale_ids.clone()));

				// Try force distribution
				assert_ok!(Crowdsale::try_force_distribution(Some(alice()).into(), sale_id));

				let block_number = System::block_number();
				let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
				assert_eq!(sale_info.status, SaleStatus::Distributing(block_number, 0, 0));
			});
	}
}

mod automatic_distribution {
	use super::*;
	use crate::mock::{MaxPaymentsPerBlock, UnsignedInterval};

	#[test]
	fn automatic_distribution_works() {
		let initial_balance = 1_000_000;
		TestExt::<Test>::default()
			.with_balances(&[(bob(), initial_balance)])
			.build()
			.execute_with(|| {
				let max_issuance = 1000;
				let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
				let voucher_asset_id = sale_info.voucher_asset_id;
				assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

				// Participate some amount
				let participation_amount = 100;
				assert_ok!(Crowdsale::participate(
					Some(bob()).into(),
					sale_id,
					participation_amount
				));

				// Call on_initialize at sale close
				let end_block = System::block_number() + sale_info.duration;
				System::set_block_number(end_block);
				Crowdsale::on_initialize(end_block);

				// Calling auto distribution works
				assert_ok!(Crowdsale::distribute_crowdsale_rewards(None.into()));

				// Check storage updated
				let voucher_balance = AssetsExt::reducible_balance(voucher_asset_id, &bob(), false);
				let expected_balance =
					add_decimals(participation_amount, VOUCHER_DECIMALS) / sale_info.soft_cap_price;
				assert_eq!(voucher_balance, expected_balance);

				// Check vault balance is 0 (All vouchers redeemed)
				let vault_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &sale_info.vault, false);
				assert_eq!(vault_balance, 0);

				// Check other storage values
				let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
				assert_eq!(sale_info.status, SaleStatus::Ended(end_block, expected_balance));
				let block_number = System::block_number();
				let next_unsigned_at = block_number + UnsignedInterval::get() as u64;
				assert_eq!(NextUnsignedAt::<Test>::get(), next_unsigned_at);

				// Check Events are thrown
				System::assert_has_event(
					Event::CrowdsaleVouchersClaimed {
						sale_id,
						who: bob(),
						amount: expected_balance,
					}
					.into(),
				);
				System::assert_has_event(
					Event::CrowdsaleDistributionComplete {
						sale_id,
						vouchers_distributed: expected_balance,
					}
					.into(),
				);
			});
	}

	#[test]
	fn distribution_across_multiple_blocks() {
		let total_contributors = MaxPaymentsPerBlock::get() as u32 * 2;
		let mut accounts = vec![];
		for i in 0..total_contributors {
			let i = i + 1;
			accounts.push((create_account(i as u64), i as u128 * 100u128));
		}

		TestExt::<Test>::default().with_balances(&accounts).build().execute_with(|| {
			let max_issuance = 1000;
			let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
			let voucher_asset_id = sale_info.voucher_asset_id;
			assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

			// Participate for each account
			for (account, amount) in accounts.clone() {
				assert_ok!(Crowdsale::participate(Some(account).into(), sale_id, amount));
				assert_eq!(SaleParticipation::<Test>::get(sale_id, account), Some(amount));
			}

			// Call on_initialize at sale close
			let end_block = System::block_number() + sale_info.duration;
			System::set_block_number(end_block);
			Crowdsale::on_initialize(end_block);

			// Calling auto distribution works
			assert_ok!(Crowdsale::distribute_crowdsale_rewards(None.into()));

			let block_number = System::block_number();
			let next_unsigned_at = block_number + UnsignedInterval::get() as u64;
			assert_eq!(NextUnsignedAt::<Test>::get(), next_unsigned_at);

			// SaleParticipation should be a half the size
			assert_eq!(
				SaleParticipation::<Test>::iter_prefix(sale_id).count(),
				total_contributors as usize / 2
			);

			// Sales status should be ended as we have paid out all accounts
			let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
			assert!(matches!(sale_info.status, SaleStatus::Distributing(_, _, _)));

			// Distribute the next half
			assert_ok!(Crowdsale::distribute_crowdsale_rewards(None.into()));

			// SaleParticipation should be empty
			assert_eq!(SaleParticipation::<Test>::iter_prefix(sale_id).count(), 0);

			let mut total_distributed = 0;
			// Check status of each individual account
			for (account, amount) in accounts.into_iter() {
				let voucher_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &account, false);
				let expected_balance =
					add_decimals(amount, VOUCHER_DECIMALS) / sale_info.soft_cap_price;
				total_distributed += expected_balance;
				assert_eq!(voucher_balance, expected_balance);
				assert!(SaleParticipation::<Test>::get(sale_id, account).is_none());
			}

			// Check vault balance is 0 (All vouchers redeemed)
			let vault_balance =
				AssetsExt::reducible_balance(voucher_asset_id, &sale_info.vault, false);
			assert_eq!(vault_balance, 0);

			// total supply remains the max issuance
			assert_eq!(
				AssetsExt::total_issuance(voucher_asset_id),
				add_decimals(max_issuance, VOUCHER_DECIMALS)
			);

			// Sales status should be ended as we have paid out all accounts
			let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
			assert_eq!(sale_info.status, SaleStatus::Ended(end_block, total_distributed));
			let block_number = System::block_number();
			let next_unsigned_at = block_number + UnsignedInterval::get() as u64;
			assert_eq!(NextUnsignedAt::<Test>::get(), next_unsigned_at);
		});
	}

	#[test]
	// This test checks that payment is only made once if a user manually redeems tokens
	// while auto distribution is occurring
	fn distribution_across_multiple_blocks_with_manual_redemption() {
		let total_contributors = MaxPaymentsPerBlock::get() as u32 * 2;
		let mut accounts = vec![];
		for i in 0..total_contributors {
			let i = i + 1;
			accounts.push((create_account(i as u64), i as u128 * 100u128));
		}

		TestExt::<Test>::default().with_balances(&accounts).build().execute_with(|| {
			let max_issuance = 1000;
			let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
			let voucher_asset_id = sale_info.voucher_asset_id;
			assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

			// Participate for each account
			for (account, amount) in accounts.clone() {
				assert_ok!(Crowdsale::participate(Some(account).into(), sale_id, amount));
				assert_eq!(SaleParticipation::<Test>::get(sale_id, account), Some(amount));
			}

			// Call on_initialize at sale close
			let end_block = System::block_number() + sale_info.duration;
			System::set_block_number(end_block);
			Crowdsale::on_initialize(end_block);

			// Calling auto distribution works
			assert_ok!(Crowdsale::distribute_crowdsale_rewards(None.into()));

			let block_number = System::block_number();
			let next_unsigned_at = block_number + UnsignedInterval::get() as u64;
			assert_eq!(NextUnsignedAt::<Test>::get(), next_unsigned_at);

			// SaleParticipation should be a half the size
			assert_eq!(
				SaleParticipation::<Test>::iter_prefix(sale_id).count(),
				total_contributors as usize / 2
			);

			// Manually redeem an account while auto distribution is occurring
			let (account, contribution) =
				SaleParticipation::<Test>::iter_prefix(sale_id).next().unwrap();
			assert_ok!(Crowdsale::claim_voucher(Some(account).into(), sale_id));
			let voucher_balance = AssetsExt::reducible_balance(voucher_asset_id, &account, false);
			let expected_balance =
				add_decimals(contribution, VOUCHER_DECIMALS) / sale_info.soft_cap_price;
			assert_eq!(voucher_balance, expected_balance);

			// SaleParticipation should be a half the size - 1
			assert_eq!(
				SaleParticipation::<Test>::iter_prefix(sale_id).count(),
				(total_contributors as usize / 2) - 1
			);

			// Distribute the next half (Which should not include the account that manually
			// redeemed)
			assert_ok!(Crowdsale::distribute_crowdsale_rewards(None.into()));

			let mut total_distributed = 0;
			// Check status of each individual account
			for (account, amount) in accounts.into_iter() {
				let voucher_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &account, false);
				let expected_balance =
					add_decimals(amount, VOUCHER_DECIMALS) / sale_info.soft_cap_price;
				total_distributed += expected_balance;
				assert_eq!(voucher_balance, expected_balance);
				assert!(SaleParticipation::<Test>::get(sale_id, account).is_none());
			}

			// Check vault balance is 0 (All vouchers redeemed)
			let vault_balance =
				AssetsExt::reducible_balance(voucher_asset_id, &sale_info.vault, false);
			assert_eq!(vault_balance, 0);

			// total supply remains the max issuance
			assert_eq!(
				AssetsExt::total_issuance(voucher_asset_id),
				add_decimals(max_issuance, VOUCHER_DECIMALS)
			);

			// SaleParticipation should be empty
			assert_eq!(SaleParticipation::<Test>::iter_prefix(sale_id).count(), 0);

			// Sales status should be ended as we have paid out all accounts
			let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
			assert_eq!(sale_info.status, SaleStatus::Ended(end_block, total_distributed));
			let block_number = System::block_number();
			let next_unsigned_at = block_number + UnsignedInterval::get() as u64;
			assert_eq!(NextUnsignedAt::<Test>::get(), next_unsigned_at);
		});
	}

	#[test]
	fn distribution_across_many_blocks_under_committed() {
		let payout_iterations = 50;
		let total_contributors = MaxPaymentsPerBlock::get() as u32 * payout_iterations;
		let mut accounts = vec![];
		for i in 0..total_contributors {
			accounts.push((create_account(i as u64 + 1), 100u128));
		}

		TestExt::<Test>::default().with_balances(&accounts).build().execute_with(|| {
			let max_issuance = 10000;
			let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
			let voucher_asset_id = sale_info.voucher_asset_id;
			assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

			// Participate for each account
			for (account, amount) in accounts.clone() {
				assert_ok!(Crowdsale::participate(Some(account).into(), sale_id, amount));
				assert_eq!(SaleParticipation::<Test>::get(sale_id, account), Some(amount));
			}

			// Call on_initialize at sale close
			let end_block = System::block_number() + sale_info.duration;
			System::set_block_number(end_block);
			Crowdsale::on_initialize(end_block);

			// Call auto distribute many times
			for _ in 0..payout_iterations {
				assert_ok!(Crowdsale::distribute_crowdsale_rewards(None.into()));
			}

			let mut total_distributed = 0;
			// Check status of each individual account
			for (account, amount) in accounts.into_iter() {
				let voucher_balance =
					AssetsExt::reducible_balance(voucher_asset_id, &account, false);
				let expected_balance =
					add_decimals(amount, VOUCHER_DECIMALS) / sale_info.soft_cap_price;
				total_distributed += expected_balance;
				assert_eq!(voucher_balance, expected_balance);
				assert!(SaleParticipation::<Test>::get(sale_id, account).is_none());
			}

			// Check vault balance is 0 (All vouchers redeemed)
			let vault_balance =
				AssetsExt::reducible_balance(voucher_asset_id, &sale_info.vault, false);
			assert_eq!(vault_balance, 0);

			// total supply remains the max issuance
			assert_eq!(
				AssetsExt::total_issuance(voucher_asset_id),
				add_decimals(max_issuance, VOUCHER_DECIMALS)
			);

			// SaleParticipation should be empty
			assert_eq!(SaleParticipation::<Test>::iter_prefix(sale_id).count(), 0);

			// Sales status should be ended as we have paid out all accounts
			let sale_info = SaleInfo::<Test>::get(sale_id).unwrap();
			assert_eq!(sale_info.status, SaleStatus::Ended(end_block, total_distributed));
		});
	}

	#[test]
	fn distribution_across_many_blocks_over_committed() {
		let payout_iterations = 50;
		let total_contributors = MaxPaymentsPerBlock::get() as u32 * payout_iterations;
		let mut accounts = vec![];
		for i in 0..total_contributors {
			let i = i + 1;
			// Add some random value to give non round results
			accounts.push((create_account(i as u64), i as u128 * 1237u128 + i as u128));
		}

		TestExt::<Test>::default().with_balances(&accounts).build().execute_with(|| {
			let max_issuance = 5000;
			let (sale_id, sale_info) = initialize_crowdsale(max_issuance);
			let voucher_asset_id = sale_info.voucher_asset_id;
			assert_ok!(Crowdsale::enable(Some(alice()).into(), sale_id));

			// Participate for each account
			for (account, amount) in accounts.clone() {
				assert_ok!(Crowdsale::participate(Some(account).into(), sale_id, amount));
				assert_eq!(SaleParticipation::<Test>::get(sale_id, account), Some(amount));
			}

			// Call on_initialize at sale close
			let end_block = System::block_number() + sale_info.duration;
			System::set_block_number(end_block);
			Crowdsale::on_initialize(end_block);

			// Call auto distribute many times
			for _ in 0..payout_iterations {
				assert_ok!(Crowdsale::distribute_crowdsale_rewards(None.into()));
			}

			// Check status of each individual account
			for (account, _) in accounts.into_iter() {
				assert!(SaleParticipation::<Test>::get(sale_id, account).is_none());
			}

			// Check vault balance is 0 (All vouchers redeemed)
			let vault_balance =
				AssetsExt::reducible_balance(voucher_asset_id, &sale_info.vault, false);
			assert_eq!(vault_balance, 0);

			// total supply remains the max issuance
			assert_eq!(
				AssetsExt::total_issuance(voucher_asset_id),
				add_decimals(max_issuance, VOUCHER_DECIMALS)
			);

			// SaleParticipation should be empty
			assert_eq!(SaleParticipation::<Test>::iter_prefix(sale_id).count(), 0);
		});
	}

	#[test]
	fn empty_sales_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Calling auto distribution fails when there are no sales
			assert_noop!(
				Crowdsale::distribute_crowdsale_rewards(None.into()),
				Error::<Test>::CrowdsaleNotFound
			);
		});
	}

	#[test]
	fn no_sale_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Put random sales id in SaleDistribution
			SaleDistribution::<Test>::put(BoundedVec::truncate_from(vec![
				3;
				MaxConsecutiveSales::get()
					as usize
			]));
			// Calling auto distribution fails when there are no sales
			assert_noop!(
				Crowdsale::distribute_crowdsale_rewards(None.into()),
				Error::<Test>::CrowdsaleNotFound
			);
		});
	}

	#[test]
	fn invalid_sale_status_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (sale_id, mut sale_info) = initialize_crowdsale(100);

			// Put sales id in SaleDistribution
			SaleDistribution::<Test>::put(BoundedVec::truncate_from(vec![sale_id]));

			// Check redeem_voucher against invalid statuses
			sale_info.status = SaleStatus::Enabled(0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_noop!(
				Crowdsale::distribute_crowdsale_rewards(None.into()),
				Error::<Test>::InvalidCrowdsaleStatus
			);

			sale_info.status = SaleStatus::Pending(0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_noop!(
				Crowdsale::distribute_crowdsale_rewards(None.into()),
				Error::<Test>::InvalidCrowdsaleStatus
			);

			sale_info.status = SaleStatus::DistributionFailed(0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_noop!(
				Crowdsale::distribute_crowdsale_rewards(None.into()),
				Error::<Test>::InvalidCrowdsaleStatus
			);

			sale_info.status = SaleStatus::Ended(0, 0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_noop!(
				Crowdsale::distribute_crowdsale_rewards(None.into()),
				Error::<Test>::InvalidCrowdsaleStatus
			);

			// Sanity check
			sale_info.status = SaleStatus::Distributing(0, 0, 0);
			SaleInfo::<Test>::insert(sale_id, sale_info);
			assert_ok!(Crowdsale::distribute_crowdsale_rewards(None.into()));
		});
	}
}
