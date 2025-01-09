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
use crate::mock::{Nft, Test, TokenApprovals};
use crate::{ERC1155ApprovalsForAll, ERC20Approvals, ERC721Approvals, ERC721ApprovalsForAll};
use seed_pallet_common::test_prelude::*;
use seed_primitives::OriginChain;

pub fn create_account(seed: u64) -> AccountId {
	AccountId::from(H160::from_low_u64_be(seed))
}

#[allow(dead_code)]
pub struct TestData {
	pub coll_owner: AccountId,
	pub coll_id: CollectionUuid,
	pub coll_tokens: Vec<TokenId>,
	pub token_id: TokenId,
	pub token_owner: AccountId,
}

fn prepare_test() -> TestData {
	let alice = create_account(10);
	let coll_owner = alice;
	let collection_name = BoundedVec::truncate_from("Hello".into());
	let metadata_scheme = MetadataScheme::try_from(
		b"ethereum://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi/".as_slice(),
	)
	.unwrap();

	let coll_id = Nft::do_create_collection(
		coll_owner,
		collection_name,
		0,
		None,
		None,
		metadata_scheme,
		None,
		OriginChain::Root,
		Default::default(),
	)
	.unwrap();

	let origin = RawOrigin::Signed(alice).into();
	let count = 10u32;
	assert_ok!(Nft::mint(origin, coll_id, count + 1, Some(alice)));
	let coll_tokens: Vec<TokenId> = vec![(coll_id, count)];

	let token_id = coll_tokens[0];
	let token_owner = coll_owner;

	TestData { coll_owner, coll_id, coll_tokens, token_id, token_owner }
}

#[test]
fn set_erc721_approval() {
	TestExt::<Test>::default().build().execute_with(|| {
		let TestData { token_owner, token_id, .. } = prepare_test();
		let caller = token_owner;
		let operator = create_account(12);

		assert_ok!(TokenApprovals::erc721_approval(None.into(), caller, operator, token_id));
		assert_eq!(ERC721Approvals::<Test>::get(token_id).unwrap(), operator);
	});
}

#[test]
fn set_erc721_approval_approved_for_all() {
	TestExt::<Test>::default().build().execute_with(|| {
		let TestData { token_owner, token_id, coll_id, .. } = prepare_test();

		let caller = create_account(12);
		let operator = create_account(13);

		// Token owner approves caller for all
		assert_ok!(TokenApprovals::erc721_approval_for_all(
			None.into(),
			token_owner,
			caller,
			coll_id,
			true
		));

		// Caller is not token owner, but they are approved for all so this passes
		assert_ok!(TokenApprovals::erc721_approval(None.into(), caller, operator, token_id));
		assert_eq!(ERC721Approvals::<Test>::get(token_id).unwrap(), operator);
		// 000_001_500_000_000_000
	});
}

#[test]
fn set_erc721_approval_not_token_owner_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(10);
		let operator: AccountId = create_account(11);
		let token_id: TokenId = (0, 1);

		assert_noop!(
			TokenApprovals::erc721_approval(None.into(), caller, operator, token_id),
			Error::<Test>::NoToken,
		);
	});
}

#[test]
fn set_erc721_approval_caller_is_operator_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(10);
		let operator: AccountId = create_account(10);
		let token_id: TokenId = (0, 0);

		assert_noop!(
			TokenApprovals::erc721_approval(None.into(), caller, operator, token_id),
			Error::<Test>::CallerNotOperator,
		);
	});
}

#[test]
fn erc721_approval_removed_on_transfer() {
	TestExt::<Test>::default().build().execute_with(|| {
		let TestData { token_owner, token_id, .. } = prepare_test();
		let caller = token_owner;
		let operator = create_account(11);

		assert_ok!(TokenApprovals::erc721_approval(None.into(), caller, operator, token_id));
		assert_eq!(ERC721Approvals::<Test>::get(token_id).unwrap(), operator);
		TokenApprovals::on_nft_transfer(&token_id);
		assert!(!ERC721Approvals::<Test>::contains_key(token_id));
	});
}

#[test]
fn erc721_remove_approval() {
	TestExt::<Test>::default().build().execute_with(|| {
		let TestData { token_owner, token_id, .. } = prepare_test();
		let caller = token_owner;
		let operator = create_account(11);

		assert_ok!(TokenApprovals::erc721_approval(None.into(), caller, operator, token_id));

		// Remove approval
		assert_ok!(TokenApprovals::erc721_remove_approval(Some(caller).into(), token_id));
		assert!(!ERC721Approvals::<Test>::contains_key(token_id));
	});
}

#[test]
fn erc721_remove_approval_no_approval_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(10);
		let token_id: TokenId = (0, 0);

		// Try remove approval
		assert_noop!(
			TokenApprovals::erc721_remove_approval(Some(caller).into(), token_id),
			Error::<Test>::ApprovalDoesntExist
		);
	});
}

#[test]
fn erc721_remove_approval_not_owner_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let TestData { token_owner, token_id, .. } = prepare_test();

		let caller = token_owner;
		let operator = create_account(11);

		assert_ok!(TokenApprovals::erc721_approval(None.into(), caller, operator, token_id));

		assert_noop!(
			TokenApprovals::erc721_remove_approval(Some(operator).into(), token_id),
			Error::<Test>::NotTokenOwner
		);
	});
}

#[test]
fn set_erc20_approval() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(12);
		let spender: AccountId = create_account(11);
		let asset_id: AssetId = 0;
		let amount: Balance = 10;

		assert_ok!(TokenApprovals::erc20_approval(None.into(), caller, spender, asset_id, amount));
		assert_eq!(ERC20Approvals::<Test>::get((caller, asset_id), spender).unwrap(), amount);
	});
}

#[test]
fn set_erc20_approval_caller_is_operator_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(11);
		let spender: AccountId = create_account(11);
		let asset_id: AssetId = 0;
		let amount: Balance = 10;

		assert_noop!(
			TokenApprovals::erc20_approval(None.into(), caller, spender, asset_id, amount),
			Error::<Test>::CallerNotOperator,
		);
	});
}

#[test]
fn update_erc20_approval_full_amount() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(12);
		let spender: AccountId = create_account(11);
		let asset_id: AssetId = 0;
		let amount: Balance = 10;

		assert_ok!(TokenApprovals::erc20_approval(None.into(), caller, spender, asset_id, amount));
		assert_eq!(ERC20Approvals::<Test>::get((caller, asset_id), spender).unwrap(), amount);

		// Remove approval
		assert_ok!(TokenApprovals::erc20_update_approval(
			None.into(),
			caller,
			spender,
			asset_id,
			amount
		));
		assert!(!ERC20Approvals::<Test>::contains_key((caller, asset_id), spender));
	});
}

#[test]
fn update_erc20_approval_some_amount() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(12);
		let spender: AccountId = create_account(11);
		let asset_id: AssetId = 0;
		let amount: Balance = 10;

		assert_ok!(TokenApprovals::erc20_approval(None.into(), caller, spender, asset_id, amount));
		assert_eq!(ERC20Approvals::<Test>::get((caller, asset_id), spender).unwrap(), amount);

		let removal_amount: Balance = 9;
		// Remove approval
		assert_ok!(TokenApprovals::erc20_update_approval(
			None.into(),
			caller,
			spender,
			asset_id,
			removal_amount
		));
		assert_eq!(
			ERC20Approvals::<Test>::get((caller, asset_id), spender).unwrap(),
			amount - removal_amount
		);
	});
}

#[test]
fn update_erc20_approval_amount_too_high_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(12);
		let spender: AccountId = create_account(11);
		let asset_id: AssetId = 0;
		let amount: Balance = 10;

		assert_ok!(TokenApprovals::erc20_approval(None.into(), caller, spender, asset_id, amount));
		assert_eq!(ERC20Approvals::<Test>::get((caller, asset_id), spender).unwrap(), amount);

		let removal_amount: Balance = 11;
		// Attempt to remove approval
		assert_noop!(
			TokenApprovals::erc20_update_approval(
				None.into(),
				caller,
				spender,
				asset_id,
				removal_amount
			),
			Error::<Test>::ApprovedAmountTooLow
		);
	});
}

#[test]
fn update_erc20_approval_not_approved_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(12);
		let spender: AccountId = create_account(11);
		let asset_id: AssetId = 0;
		let amount: Balance = 10;

		assert_ok!(TokenApprovals::erc20_approval(None.into(), caller, spender, asset_id, amount));
		assert_eq!(ERC20Approvals::<Test>::get((caller, asset_id), spender).unwrap(), amount);

		let malicious_spender = create_account(13);
		// Attempt to remove approval
		assert_noop!(
			TokenApprovals::erc20_update_approval(
				None.into(),
				caller,
				malicious_spender,
				asset_id,
				amount
			),
			Error::<Test>::CallerNotApproved
		);
	});
}

#[test]
fn set_erc721_approval_for_all() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(10);
		let operator: AccountId = create_account(11);
		let collection_id: CollectionUuid = 1;

		// Set approval to true
		assert_ok!(TokenApprovals::erc721_approval_for_all(
			None.into(),
			caller,
			operator,
			collection_id,
			true
		));
		assert!(ERC721ApprovalsForAll::<Test>::get(caller, (collection_id, operator)).unwrap());

		// Remove approval
		assert_ok!(TokenApprovals::erc721_approval_for_all(
			None.into(),
			caller,
			operator,
			collection_id,
			false
		));
		assert!(ERC721ApprovalsForAll::<Test>::get(caller, (collection_id, operator)).is_none());
	});
}

#[test]
fn set_erc721_approval_for_all_multiple_approvals() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(10);
		let operator_1: AccountId = create_account(11);
		let operator_2: AccountId = create_account(12);
		let operator_3: AccountId = create_account(13);
		let collection_id: CollectionUuid = 1;

		// Set approval to true for all three accounts
		assert_ok!(TokenApprovals::erc721_approval_for_all(
			None.into(),
			caller,
			operator_1,
			collection_id,
			true
		));
		assert_ok!(TokenApprovals::erc721_approval_for_all(
			None.into(),
			caller,
			operator_2,
			collection_id,
			true
		));
		assert_ok!(TokenApprovals::erc721_approval_for_all(
			None.into(),
			caller,
			operator_3,
			collection_id,
			true
		));

		// Check storage
		assert!(ERC721ApprovalsForAll::<Test>::get(caller, (collection_id, operator_1)).unwrap());
		assert!(ERC721ApprovalsForAll::<Test>::get(caller, (collection_id, operator_2)).unwrap());
		assert!(ERC721ApprovalsForAll::<Test>::get(caller, (collection_id, operator_3)).unwrap());
	});
}

#[test]
fn set_erc721_approval_for_all_caller_is_operator_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(10);
		let collection_id: CollectionUuid = 1;

		// Set approval to true
		assert_noop!(
			TokenApprovals::erc721_approval_for_all(
				None.into(),
				caller,
				caller,
				collection_id,
				true
			),
			Error::<Test>::CallerNotOperator
		);
	});
}

#[test]
fn is_approved_or_owner_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let TestData { token_owner, coll_id, token_id, .. } = prepare_test();

		let approved_for_all_account: AccountId = create_account(11);
		let approved_account: AccountId = create_account(12);

		// Should return false for both as approvals have not been set up
		assert!(!TokenApprovals::is_approved_or_owner(token_id, approved_for_all_account));
		assert!(!TokenApprovals::is_approved_or_owner(token_id, approved_account));

		// set approve for all
		assert_ok!(TokenApprovals::erc721_approval_for_all(
			None.into(),
			token_owner,
			approved_for_all_account,
			coll_id,
			true
		));

		// set approve
		assert_ok!(TokenApprovals::erc721_approval(
			None.into(),
			token_owner,
			approved_account,
			token_id
		));

		// Should return true for all three
		assert!(TokenApprovals::is_approved_or_owner(token_id, token_owner));
		assert!(TokenApprovals::is_approved_or_owner(token_id, approved_for_all_account));
		assert!(TokenApprovals::is_approved_or_owner(token_id, approved_account));
	});
}

#[test]
fn set_erc1155_approval_for_all() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(10);
		let operator: AccountId = create_account(11);
		let collection_id: CollectionUuid = 1;

		// Set approval to true
		assert_ok!(TokenApprovals::erc1155_approval_for_all(
			None.into(),
			caller,
			operator,
			collection_id,
			true
		));
		assert!(ERC1155ApprovalsForAll::<Test>::get(caller, (collection_id, operator)).unwrap());

		// Remove approval
		assert_ok!(TokenApprovals::erc1155_approval_for_all(
			None.into(),
			caller,
			operator,
			collection_id,
			false
		));
		assert!(ERC1155ApprovalsForAll::<Test>::get(caller, (collection_id, operator)).is_none());
	});
}

#[test]
fn set_erc1155_approval_for_all_multiple_approvals() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(10);
		let operator_1: AccountId = create_account(11);
		let operator_2: AccountId = create_account(12);
		let operator_3: AccountId = create_account(13);
		let collection_id: CollectionUuid = 1;

		// Set approval to true for all three accounts
		assert_ok!(TokenApprovals::erc1155_approval_for_all(
			None.into(),
			caller,
			operator_1,
			collection_id,
			true
		));
		assert_ok!(TokenApprovals::erc1155_approval_for_all(
			None.into(),
			caller,
			operator_2,
			collection_id,
			true
		));
		assert_ok!(TokenApprovals::erc1155_approval_for_all(
			None.into(),
			caller,
			operator_3,
			collection_id,
			true
		));

		// Check storage
		assert!(ERC1155ApprovalsForAll::<Test>::get(caller, (collection_id, operator_1)).unwrap());
		assert!(ERC1155ApprovalsForAll::<Test>::get(caller, (collection_id, operator_2)).unwrap());
		assert!(ERC1155ApprovalsForAll::<Test>::get(caller, (collection_id, operator_3)).unwrap());
	});
}

#[test]
fn set_erc1155_approval_for_all_caller_is_operator_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller: AccountId = create_account(10);
		let collection_id: CollectionUuid = 1;

		// Set approval to true
		assert_noop!(
			TokenApprovals::erc1155_approval_for_all(
				None.into(),
				caller,
				caller,
				collection_id,
				true
			),
			Error::<Test>::CallerNotOperator
		);
	});
}
