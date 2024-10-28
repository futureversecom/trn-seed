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

use crate as pallet_xrpl_bridge;
use crate::Config;
use frame_support::{storage_alias, Twox64Concat};
use seed_pallet_common::test_prelude::*;
use seed_primitives::ethy::{crypto::AuthorityId, EventProofId};
use seed_primitives::xrpl::Xls20TokenId;
use seed_primitives::WeightedDispatchResult;
use sp_core::ByteArray;
use sp_runtime::Percent;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		XRPLBridge: pallet_xrpl_bridge,
		AssetsExt: pallet_assets_ext,
		TimestampPallet: pallet_timestamp,
		Nft: pallet_nft,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_timestamp_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_nft_config!(Test);

// Time is measured by number of blocks.
pub const MILLISECS_PER_BLOCK: u64 = 4_000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);

#[storage_alias]
pub type Xls20TokenTest<Test: Config> =
	StorageMap<crate::Pallet<Test>, Twox64Concat, AccountId, Xls20TokenId>;

// Mock Xls20Ext implementation by just storing the token in the mock storage to ensure
// the correct path is triggered.
pub struct MockXls20Ext;
impl Xls20Ext for MockXls20Ext {
	type AccountId = AccountId;

	fn deposit_xls20_token(
		receiver: &Self::AccountId,
		xls20_token_id: Xls20TokenId,
	) -> WeightedDispatchResult {
		Xls20TokenTest::<Test>::insert(receiver, xls20_token_id);
		Ok(Weight::zero())
	}

	fn get_xls20_token_id(_token_id: TokenId) -> Option<Xls20TokenId> {
		None
	}
}

parameter_types! {
	pub const XrpTxChallengePeriod: u32 = 10 * MINUTES as u32;
	pub const TicketSequenceThreshold: Percent = Percent::from_percent(66_u8);
	pub const XRPTransactionLimit: u32 = 10;
	pub const XRPLTransactionLimitPerLedger: u32 = 10;
	pub const MaxPrunedTransactionsPerBlock: u32 = 5000;
	pub const MaxDelayedPaymentsPerBlock: u32 = 1000;
	pub const DelayedPaymentBlockLimit: BlockNumber = 1000;
	pub const XrpAssetId: u32 = XRP_ASSET_ID;
	pub const SourceTag: u32 = 723456_u32;
	pub const XrplPalletId: PalletId = PalletId(*b"xrpl-peg");
}

impl pallet_xrpl_bridge::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type EthyAdapter = MockEthyAdapter;
	type MultiCurrency = AssetsExt;
	type ApproveOrigin = EnsureRoot<Self::AccountId>;
	type WeightInfo = ();
	type XrpAssetId = XrpAssetId;
	type NativeAssetId = NativeAssetId;
	type PalletId = XrplPalletId;
	type ChallengePeriod = XrpTxChallengePeriod;
	type MaxPrunedTransactionsPerBlock = MaxPrunedTransactionsPerBlock;
	type MaxDelayedPaymentsPerBlock = MaxDelayedPaymentsPerBlock;
	type DelayedPaymentBlockLimit = DelayedPaymentBlockLimit;
	type UnixTime = TimestampPallet;
	type TicketSequenceThreshold = TicketSequenceThreshold;
	type XRPTransactionLimit = XRPTransactionLimit;
	type XRPLTransactionLimitPerLedger = XRPLTransactionLimitPerLedger;
	type NFTExt = Nft;
	type Xls20Ext = MockXls20Ext;
}

pub struct MockEthyAdapter;

impl XrplBridgeToEthyAdapter<AuthorityId> for MockEthyAdapter {
	/// Mock implementation of XrplBridgeToEthyAdapter
	fn sign_xrpl_transaction(_tx_data: &[u8]) -> Result<EventProofId, DispatchError> {
		Ok(1)
	}
	fn validators() -> Vec<AuthorityId> {
		// some hard coded validators
		vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
		]
	}
	fn xrp_validators() -> Vec<AuthorityId> {
		// some hard coded validators
		vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
		]
	}
}

// Mock implementation for Xls20Ext
pub struct MockXls20Ext;

impl Xls20Ext for MockXls20Ext {
	fn get_xls20_token_id(_token_id: TokenId) -> Option<Xls20TokenId> {
		Some([1_u8; 32])
	}
}
