/* Copyright 2019-2021 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */

use super::*;
use crate::mock::{AccountId, Test, TestExt, TokenApprovals};
use frame_support::{assert_noop, assert_ok};
use seed_primitives::TokenId;

#[test]
fn set_erc721_approval() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 10;
		let operator: AccountId = 11;
		let token_id: TokenId = (0, 0);

		assert_ok!(TokenApprovals::erc721_approval(None.into(), caller, operator, token_id));
		assert_eq!(TokenApprovals::erc721_approvals(token_id).unwrap(), operator);
	});
}

#[test]
fn set_erc721_approval_approved_for_all() {
	TestExt::default().build().execute_with(|| {
		let token_owner: AccountId = 10;
		let caller: AccountId = 12;
		let operator: AccountId = 13;
		let collection_id: CollectionUuid = 0;
		let token_id: TokenId = (collection_id, 0);

		// Token owner approves caller for all
		assert_ok!(TokenApprovals::erc721_approval_for_all(
			None.into(),
			token_owner,
			caller,
			collection_id,
			true
		));

		// Caller is not token owner, but they are approved for all so this passes
		assert_ok!(TokenApprovals::erc721_approval(None.into(), caller, operator, token_id));
		assert_eq!(TokenApprovals::erc721_approvals(token_id).unwrap(), operator);
	});
}

#[test]
fn set_erc721_approval_not_token_owner_should_fail() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 10;
		let operator: AccountId = 11;
		let token_id: TokenId = (0, 1);

		assert_noop!(
			TokenApprovals::erc721_approval(None.into(), caller, operator, token_id),
			Error::<Test>::NoToken,
		);
	});
}

#[test]
fn set_erc721_approval_caller_is_operator_should_fail() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 10;
		let operator: AccountId = 10;
		let token_id: TokenId = (0, 0);

		assert_noop!(
			TokenApprovals::erc721_approval(None.into(), caller, operator, token_id),
			Error::<Test>::CallerNotOperator,
		);
	});
}

#[test]
fn erc721_approval_removed_on_transfer() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 10;
		let operator: AccountId = 11;
		let token_id: TokenId = (0, 0);

		assert_ok!(TokenApprovals::erc721_approval(None.into(), caller, operator, token_id));
		assert_eq!(TokenApprovals::erc721_approvals(token_id).unwrap(), operator);
		TokenApprovals::on_nft_transfer(&token_id);
		assert!(!ERC721Approvals::<Test>::contains_key(token_id));
	});
}

#[test]
fn erc721_remove_approval() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 10;
		let operator: AccountId = 11;
		let token_id: TokenId = (0, 0);

		assert_ok!(TokenApprovals::erc721_approval(None.into(), caller, operator, token_id));

		// Remove approval
		assert_ok!(TokenApprovals::erc721_remove_approval(Some(caller).into(), token_id));
		assert!(!ERC721Approvals::<Test>::contains_key(token_id));
	});
}

#[test]
fn erc721_remove_approval_no_approval_should_fail() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 10;
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
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 10;
		let operator: AccountId = 11;
		let token_id: TokenId = (0, 0);

		assert_ok!(TokenApprovals::erc721_approval(None.into(), caller, operator, token_id));

		assert_noop!(
			TokenApprovals::erc721_remove_approval(Some(operator).into(), token_id),
			Error::<Test>::NotTokenOwner
		);
	});
}

#[test]
fn set_erc20_approval() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 12;
		let spender: AccountId = 11;
		let asset_id: AssetId = 0;
		let amount: Balance = 10;

		assert_ok!(TokenApprovals::erc20_approval(None.into(), caller, spender, asset_id, amount));
		assert_eq!(TokenApprovals::erc20_approvals((caller, asset_id), spender).unwrap(), amount);
	});
}

#[test]
fn set_erc20_approval_caller_is_operator_should_fail() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 12;
		let spender: AccountId = 12;
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
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 12;
		let spender: AccountId = 11;
		let asset_id: AssetId = 0;
		let amount: Balance = 10;

		assert_ok!(TokenApprovals::erc20_approval(None.into(), caller, spender, asset_id, amount));
		assert_eq!(TokenApprovals::erc20_approvals((caller, asset_id), spender).unwrap(), amount);

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
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 12;
		let spender: AccountId = 11;
		let asset_id: AssetId = 0;
		let amount: Balance = 10;

		assert_ok!(TokenApprovals::erc20_approval(None.into(), caller, spender, asset_id, amount));
		assert_eq!(TokenApprovals::erc20_approvals((caller, asset_id), spender).unwrap(), amount);

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
			TokenApprovals::erc20_approvals((caller, asset_id), spender).unwrap(),
			amount - removal_amount
		);
	});
}

#[test]
fn update_erc20_approval_amount_too_high_should_fail() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 12;
		let spender: AccountId = 11;
		let asset_id: AssetId = 0;
		let amount: Balance = 10;

		assert_ok!(TokenApprovals::erc20_approval(None.into(), caller, spender, asset_id, amount));
		assert_eq!(TokenApprovals::erc20_approvals((caller, asset_id), spender).unwrap(), amount);

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
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 12;
		let spender: AccountId = 11;
		let asset_id: AssetId = 0;
		let amount: Balance = 10;

		assert_ok!(TokenApprovals::erc20_approval(None.into(), caller, spender, asset_id, amount));
		assert_eq!(TokenApprovals::erc20_approvals((caller, asset_id), spender).unwrap(), amount);

		let malicious_spender = 13;
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
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 10;
		let operator: AccountId = 11;
		let collection_id: CollectionUuid = 1;

		// Set approval to true
		assert_ok!(TokenApprovals::erc721_approval_for_all(
			None.into(),
			caller,
			operator,
			collection_id,
			true
		));
		assert!(
			TokenApprovals::erc721_approvals_for_all(caller, (collection_id, operator)).unwrap()
		);

		// Remove approval
		assert_ok!(TokenApprovals::erc721_approval_for_all(
			None.into(),
			caller,
			operator,
			collection_id,
			false
		));
		assert!(
			TokenApprovals::erc721_approvals_for_all(caller, (collection_id, operator)).is_none()
		);
	});
}

#[test]
fn set_erc721_approval_for_all_multiple_approvals() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 10;
		let operator_1: AccountId = 11;
		let operator_2: AccountId = 12;
		let operator_3: AccountId = 13;
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
		assert!(
			TokenApprovals::erc721_approvals_for_all(caller, (collection_id, operator_1)).unwrap()
		);
		assert!(
			TokenApprovals::erc721_approvals_for_all(caller, (collection_id, operator_2)).unwrap()
		);
		assert!(
			TokenApprovals::erc721_approvals_for_all(caller, (collection_id, operator_3)).unwrap()
		);
	});
}

#[test]
fn set_erc721_approval_for_all_caller_is_operator_should_fail() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 10;
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
	TestExt::default().build().execute_with(|| {
		let token_owner: AccountId = 10;
		let approved_for_all_account: AccountId = 11;
		let approved_account: AccountId = 12;
		let collection_id: CollectionUuid = 0;
		let token_id: TokenId = (collection_id, 0);

		// Should return false for both as approvals have not been set up
		assert!(!TokenApprovals::is_approved_or_owner(token_id, approved_for_all_account));
		assert!(!TokenApprovals::is_approved_or_owner(token_id, approved_account));

		// set approve for all
		assert_ok!(TokenApprovals::erc721_approval_for_all(
			None.into(),
			token_owner,
			approved_for_all_account,
			collection_id,
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
