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
fn set_erc721_approval_not_token_owner_should_fail() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 10;
		let operator: AccountId = 11;
		let token_id: TokenId = (0, 1);

		assert_noop!(
			TokenApprovals::erc721_approval(None.into(), caller, operator, token_id),
			Error::<Test>::NotTokenOwner,
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
