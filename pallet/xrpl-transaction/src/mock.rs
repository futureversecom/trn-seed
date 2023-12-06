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

use crate::{self as pallet_xrpl_transaction, *};
use frame_support::{parameter_types, PalletId};
use frame_system::EnsureRoot;
use seed_pallet_common::test_prelude::*;
use seed_primitives::{AccountId, AssetId, Balance};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block<Test>,
		NodeBlock = Block<Test>,
		UncheckedExtrinsic = UncheckedExtrinsic<Test>,
	{
		System: frame_system,
	Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
	XRPLTransaction: pallet_xrpl_transaction,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);

parameter_types! {
	pub const MaxMessageLength: u32 = 2048;
	pub const MaxSignatureLength: u32 = 80;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type MaxMessageLength = MaxMessageLength;
	type MaxSignatureLength = MaxSignatureLength;
}

// impl frame_system::offchain::SigningTypes for Test {
// 	type Public = <Signature as Verify>::Signer;
// 	type Signature = Signature;
// }

// impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
// where
// 	RuntimeCall: From<C>,
// {
// 	type OverarchingCall = RuntimeCall;
// 	type Extrinsic = Extrinsic;
// }

// impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
// where
// 	RuntimeCall: From<LocalCall>,
// {
// 	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
// 		call: RuntimeCall,
// 		_public: <Signature as Verify>::Signer,
// 		_account: AccountId,
// 		nonce: u64,
// 	) -> Option<(RuntimeCall, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
// 		Some((call, (nonce, ())))
// 	}
// }
