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
use crate::mock::{AssetsExt, Nfi, Nft, RuntimeEvent as MockEvent, Sft, System, Test};
use core::ops::Mul;
use frame_support::traits::{fungibles::Inspect, OnInitialize};
use pallet_nft::{test_utils::NftBuilder, CrossChainCompatibility, TokenLocks};
use pallet_sft::{test_utils::SftBuilder, types::SftTokenBalance, TokenInfo};
use seed_pallet_common::test_prelude::*;
use seed_primitives::{MetadataScheme, RoyaltiesSchedule, TokenCount};
use sp_runtime::traits::{AccountIdConversion, Zero};

// Create an SFT collection
// Returns the created `collection_id`
fn create_sft_collection(owner: AccountId) -> CollectionUuid {
	let collection_id = Nft::next_collection_uuid().unwrap();
	let collection_name = bounded_string("test-sft-collection");
	let metadata_scheme = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
	assert_ok!(Sft::create_collection(
		Some(owner).into(),
		collection_name,
		None,
		metadata_scheme,
		None,
	));
	collection_id
}

/// Setup an SFT token, return collection id, token id, token owner
fn setup_sft_token(initial_issuance: Balance) -> (CollectionUuid, TokenId, AccountId) {
	let collection_owner = create_account(100);
	let collection_id = create_sft_collection(collection_owner);
	let token_name = bounded_string("test-sft-token");
	let token_owner = create_account(200);
	let token_id = (collection_id, 0);
	assert_ok!(Sft::create_token(
		Some(collection_owner).into(),
		collection_id,
		token_name,
		initial_issuance,
		None,
		Some(token_owner)
	));

	// Check free balance is correct
	let token_info = TokenInfo::<Test>::get(token_id).unwrap();
	assert_eq!(token_info.free_balance_of(&token_owner), initial_issuance);

	(collection_id, token_id, token_owner)
}

/// Setup an SFT token, return collection id, token id, token owner
fn setup_sft_token_with_royalties(
	initial_issuance: Balance,
	royalties: RoyaltiesSchedule<AccountId>,
) -> (CollectionUuid, TokenId, AccountId) {
	let collection_owner = create_account(100);
	let collection_id = Nft::next_collection_uuid().unwrap();
	let collection_name = bounded_string("test-sft-collection");
	let metadata_scheme = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
	assert_ok!(Sft::create_collection(
		Some(collection_owner).into(),
		collection_name,
		None,
		metadata_scheme,
		Some(royalties),
	));

	let token_name = bounded_string("test-sft-token");
	let token_owner = create_account(200);
	let token_id = (collection_id, 0);
	assert_ok!(Sft::create_token(
		Some(collection_owner).into(),
		collection_id,
		token_name,
		initial_issuance,
		None,
		Some(token_owner)
	));

	// Check free balance is correct
	let token_info = TokenInfo::<Test>::get(token_id).unwrap();
	assert_eq!(token_info.free_balance_of(&token_owner), initial_issuance);

	(collection_id, token_id, token_owner)
}

// Returns the SftTokenBalance of an account which includes free and reserved balance
fn sft_balance_of(token_id: TokenId, who: &AccountId) -> SftTokenBalance {
	let token_info = TokenInfo::<Test>::get(token_id).unwrap();
	token_info
		.owned_tokens
		.into_iter()
		.find(|(account, _)| account == who)
		.map(|(_, token_balance)| token_balance)
		.unwrap_or_default()
}

// Helper function for creating the collection name type
pub fn bounded_string(name: &str) -> BoundedVec<u8, <Test as pallet_nft::Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

#[test]
fn testy() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = NftBuilder::<Test>::new(collection_owner).initial_issuance(10).build();
		assert!(true);
	})
}

mod set_relayer {
	use super::*;

	#[test]
	fn set_relayer_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Not sudo should fail
			assert_noop!(Nfi::set_relayer(RawOrigin::Signed(alice()).into(), alice()), BadOrigin);
			assert_eq!(Relayer::<Test>::get(), None);

			// Set relayer to Alice
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), alice()));
			assert_eq!(Relayer::<Test>::get(), Some(alice()));

			// Check event
			System::assert_last_event(MockEvent::Nfi(crate::Event::RelayerSet {
				account: alice(),
			}));

			// Set relayer to Bob
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), bob()));
			assert_eq!(Relayer::<Test>::get(), Some(bob()));
		});
	}

	#[test]
	fn set_relayer_not_root_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Not sudo should fail
			assert_noop!(Nfi::set_relayer(RawOrigin::Signed(alice()).into(), alice()), BadOrigin);
		});
	}
}

mod set_fee_to {
	use super::*;

	#[test]
	fn set_fee_to_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Change fee_to account
			let new_fee_to = create_account(10);
			assert_ok!(Nfi::set_fee_to(RawOrigin::Root.into(), Some(new_fee_to.clone())));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::FeeToSet {
				account: Some(new_fee_to),
			}));
			// Storage updated
			assert_eq!(FeeTo::<Test>::get().unwrap(), new_fee_to);
		});
	}

	#[test]
	fn set_fee_to_not_root_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Change fee_to account from not sudo should fail
			let new_fee_to = create_account(10);
			assert_noop!(
				Nfi::set_fee_to(Some(create_account(11)).into(), Some(new_fee_to)),
				BadOrigin
			);
		});
	}
}

mod set_fee_details {
	use super::*;

	#[test]
	fn set_fee_details_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let new_fee = FeeDetails { asset_id: 123, amount: 400, receiver: bob() };
			let sub_type = NFISubType::NFI;
			assert_ok!(Nfi::set_fee_details(
				RawOrigin::Root.into(),
				sub_type,
				Some(new_fee.clone())
			));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::FeeDetailsSet {
				sub_type: NFISubType::NFI,
				fee_details: Some(new_fee.clone()),
			}));

			// Storage updated
			assert_eq!(MintFee::<Test>::get(sub_type).unwrap(), new_fee);
		});
	}

	#[test]
	fn set_fee_details_removes_storage_if_none() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Set initial value
			let new_fee = FeeDetails { asset_id: 123, amount: 400, receiver: bob() };
			let sub_type = NFISubType::NFI;
			assert_ok!(Nfi::set_fee_details(
				RawOrigin::Root.into(),
				sub_type,
				Some(new_fee.clone())
			));
			// Sanity check
			assert_eq!(MintFee::<Test>::get(sub_type).unwrap(), new_fee);

			// Set to none
			assert_ok!(Nfi::set_fee_details(RawOrigin::Root.into(), sub_type, None));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::FeeDetailsSet {
				sub_type: NFISubType::NFI,
				fee_details: None,
			}));

			// Storage updated
			assert_eq!(MintFee::<Test>::get(sub_type), None);
		});
	}

	#[test]
	fn set_fee_details_zero_amount_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let new_fee = FeeDetails { asset_id: 123, amount: 0, receiver: bob() };
			let sub_type = NFISubType::NFI;
			assert_noop!(
				Nfi::set_fee_details(RawOrigin::Root.into(), sub_type, Some(new_fee.clone())),
				Error::<Test>::InvalidMintFee
			);
		});
	}

	#[test]
	fn set_fee_details_not_root_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let new_fee = FeeDetails { asset_id: 123, amount: 400, receiver: bob() };
			let sub_type = NFISubType::NFI;
			assert_noop!(
				Nfi::set_fee_details(
					RawOrigin::Signed(alice()).into(),
					sub_type,
					Some(new_fee.clone())
				),
				BadOrigin
			);
		});
	}
}

mod enable_nfi {
	use super::*;

	#[test]
	fn enable_nfi_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let collection_id = NftBuilder::<Test>::new(collection_owner).build();

			// Sanity check
			assert!(!NfiEnabled::<Test>::get(collection_id, sub_type));

			// Enable NFI
			assert_ok!(Nfi::enable_nfi(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::NfiEnabled {
				sub_type,
				collection_id,
			}));

			// Storage updated
			assert!(NfiEnabled::<Test>::get(collection_id, sub_type));
		});
	}

	#[test]
	fn enable_nfi_not_collection_owner_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let collection_id = NftBuilder::<Test>::new(collection_owner).build();

			// Enable NFI should fail
			assert_noop!(
				Nfi::enable_nfi(RawOrigin::Signed(bob()).into(), collection_id, sub_type),
				Error::<Test>::NotCollectionOwner
			);

			// Still disabled
			assert!(!NfiEnabled::<Test>::get(collection_id, sub_type));
		});
	}
}

mod manual_data_request {
	use super::*;
	use crate::mock::NativeAssetId;

	#[test]
	fn manual_data_request_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let token_owner = bob();
			let collection_id = NftBuilder::<Test>::new(collection_owner)
				.token_owner(token_owner)
				.initial_issuance(1)
				.build();
			let token_id = (collection_id, 0);

			// Enable NFI
			assert_ok!(Nfi::enable_nfi(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			// Request data
			assert_ok!(Nfi::manual_data_request(
				RawOrigin::Signed(token_owner).into(),
				token_id,
				sub_type
			));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataRequest {
				sub_type,
				collection_id,
				serial_numbers: vec![0],
			}));
		});
	}

	#[test]
	fn manual_data_request_works_for_sft() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let token_id = SftBuilder::<Test>::new(collection_owner).build();

			// Enable NFI
			assert_ok!(Nfi::enable_nfi(
				RawOrigin::Signed(collection_owner).into(),
				token_id.0,
				sub_type
			));

			// Request data
			assert_ok!(Nfi::manual_data_request(
				RawOrigin::Signed(collection_owner).into(),
				token_id,
				sub_type
			));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataRequest {
				sub_type,
				collection_id: token_id.0,
				serial_numbers: vec![0],
			}));
		});
	}

	#[test]
	fn manual_data_request_not_token_owner_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let token_owner = bob();
			let collection_id = NftBuilder::<Test>::new(collection_owner)
				.token_owner(token_owner)
				.initial_issuance(1)
				.build();
			let token_id = (collection_id, 0);

			// Enable NFI
			assert_ok!(Nfi::enable_nfi(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			// Request data should fail
			assert_noop!(
				Nfi::manual_data_request(
					RawOrigin::Signed(create_account(123)).into(),
					token_id,
					sub_type
				),
				Error::<Test>::NotTokenOwner
			);
		});
	}

	#[test]
	fn manual_data_request_not_token_owner_fails_for_sft() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let token_id = SftBuilder::<Test>::new(collection_owner).build();

			// Enable NFI
			assert_ok!(Nfi::enable_nfi(
				RawOrigin::Signed(collection_owner).into(),
				token_id.0,
				sub_type
			));

			// Request data should fail
			assert_noop!(
				Nfi::manual_data_request(
					RawOrigin::Signed(create_account(123)).into(),
					token_id,
					sub_type
				),
				Error::<Test>::NotTokenOwner
			);
		});
	}

	#[test]
	fn manual_data_request_pays_fees() {
		let collection_owner = alice();
		let mint_fee = 400;

		TestExt::<Test>::default()
			.with_balances(&[(collection_owner, mint_fee)])
			.build()
			.execute_with(|| {
				let sub_type = NFISubType::NFI;
				let collection_id =
					NftBuilder::<Test>::new(collection_owner).initial_issuance(1).build();
				let token_id = (collection_id, 0);

				// Enable NFI
				assert_ok!(Nfi::enable_nfi(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					sub_type
				));

				// Set fee details to 400 ROOT
				let fee = FeeDetails { asset_id: 1, amount: mint_fee, receiver: bob() };
				assert_ok!(Nfi::set_fee_details(
					RawOrigin::Root.into(),
					sub_type,
					Some(fee.clone())
				));

				// Request data
				assert_ok!(Nfi::manual_data_request(
					RawOrigin::Signed(collection_owner).into(),
					token_id,
					sub_type
				));

				// Check fees paid
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &bob()), mint_fee);
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &collection_owner), 0);
			});
	}

	#[test]
	fn manual_data_request_low_balance_fails() {
		let collection_owner = alice();
		let mint_fee = 400;

		TestExt::<Test>::default()
			.with_balances(&[(collection_owner, mint_fee - 1)])
			.build()
			.execute_with(|| {
				let sub_type = NFISubType::NFI;
				let collection_id =
					NftBuilder::<Test>::new(collection_owner).initial_issuance(1).build();
				let token_id = (collection_id, 0);

				// Enable NFI
				assert_ok!(Nfi::enable_nfi(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					sub_type
				));

				// Set fee details to 400 ROOT
				let fee = FeeDetails { asset_id: 1, amount: mint_fee, receiver: bob() };
				assert_ok!(Nfi::set_fee_details(
					RawOrigin::Root.into(),
					sub_type,
					Some(fee.clone())
				));

				// Request data fails due to low balance
				assert_noop!(
					Nfi::manual_data_request(
						RawOrigin::Signed(collection_owner).into(),
						token_id,
						sub_type
					),
					ArithmeticError::Underflow
				);
			});
	}
}

// TODO Test minting through NFT pallet pays mint fee
