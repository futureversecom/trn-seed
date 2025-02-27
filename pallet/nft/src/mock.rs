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

use crate as pallet_nft;
use frame_support::traits::FindAuthor;
use pallet_evm::{AddressMapping, BlockHashMapping, EnsureAddressNever, GasWeightMapping};
use seed_pallet_common::test_prelude::*;
use sp_runtime::ConsensusEngineId;
use std::marker::PhantomData;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		Nft: pallet_nft,
		EVM: pallet_evm,
		Timestamp: pallet_timestamp,
		FeeControl: pallet_fee_control,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_timestamp_config!(Test);
impl_pallet_evm_config!(Test);
impl_pallet_fee_control_config!(Test);

pub struct MockTransferSubscriber;

impl OnTransferSubscriber for MockTransferSubscriber {
	fn on_nft_transfer(_token_id: &TokenId) {}
}

pub struct MockNewAssetSubscription;

impl<RuntimeId> OnNewAssetSubscriber<RuntimeId> for MockNewAssetSubscription
where
	RuntimeId: From<u32> + Into<u32>,
{
	fn on_asset_create(runtime_id: RuntimeId, _precompile_address_prefix: &[u8; 4]) {
		// Mock address without conversion
		let address = H160::from_low_u64_be(runtime_id.into().into());
		pallet_evm::Pallet::<Test>::create_account(address, b"TRN Asset Precompile".to_vec());
	}
}

pub struct MockXls20MintRequest;

impl Xls20MintRequest for MockXls20MintRequest {
	type AccountId = AccountId;

	fn request_xls20_mint(
		_who: &Self::AccountId,
		_collection_id: CollectionUuid,
		_serial_numbers: Vec<SerialNumber>,
		_metadata_scheme: MetadataScheme,
	) -> DispatchResult {
		Ok(())
	}
}

parameter_types! {
	pub const NftPalletId: PalletId = PalletId(*b"nftokens");
	pub const MaxOffers: u32 = 10;
	pub const MaxTokensPerCollection: u32 = 10_000;
	pub const MintLimit: u32 = 5000;
	pub const Xls20PaymentAsset: AssetId = XRP_ASSET_ID;
	pub const StringLimit: u32 = 50;
	pub const MaxPendingIssuances: u32 = 10_000;
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type MaxTokensPerCollection = MaxTokensPerCollection;
	type MintLimit = MintLimit;
	type OnTransferSubscription = MockTransferSubscriber;
	type OnNewAssetSubscription = MockNewAssetSubscription;
	type MultiCurrency = AssetsExt;
	type PalletId = NftPalletId;
	type ParachainId = TestParachainId;
	type StringLimit = StringLimit;
	type WeightInfo = ();
	type Xls20MintRequest = MockXls20MintRequest;
	type NFIRequest = ();
	type MaxPendingIssuances = MaxPendingIssuances;
	type Migrator = ();
}
