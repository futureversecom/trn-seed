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

use crate as pallet_sft;
use crate::Config;
use seed_pallet_common::test_prelude::*;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		Nft: pallet_nft,
		Sft: pallet_sft,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_nft_config!(Test);

parameter_types! {
	pub const SftPalletId: PalletId = PalletId(*b"sftokens");
	pub const MaxTokensPerSftCollection: u32 = 10_000;
	pub const MaxSerialsPerMint: u32 = 10;
	pub const MaxOwnersPerSftToken: u32 = 100;
	pub const MaxSftPendingIssuances: u32 = 10_000;
	pub const MaxSftDataLength: u32 = 32;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = AssetsExt;
	type NFTExt = Nft;
	type OnTransferSubscription = ();
	type OnNewAssetSubscription = ();
	type PalletId = SftPalletId;
	type ParachainId = TestParachainId;
	type StringLimit = StringLimit;
	type MaxDataLength = MaxSftDataLength;
	type WeightInfo = ();
	type MaxTokensPerSftCollection = MaxTokensPerSftCollection;
	type MaxSerialsPerMint = MaxSerialsPerMint;
	type MaxOwnersPerSftToken = MaxOwnersPerSftToken;
	type NFIRequest = ();
	type MaxSftPendingIssuances = MaxSftPendingIssuances;
}
