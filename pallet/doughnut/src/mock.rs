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
use crate::{self as pallet_doughnut};
use frame_support::{traits::InstanceFilter, weights::WeightToFee};
use seed_pallet_common::test_prelude::*;
use seed_primitives::{Address, Signature};
use sp_runtime::{generic, traits::LookupError};

pub type SignedExtra = DoughnutSenderValidations<Test>;
pub type UncheckedExtrinsicT =
	fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
pub type BlockT = generic::Block<Header, UncheckedExtrinsicT>;

construct_runtime!(
	pub enum Test where
		Block = BlockT,
		NodeBlock = BlockT,
		UncheckedExtrinsic = UncheckedExtrinsicT,
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		TransactionPayment: pallet_transaction_payment,
		FeeControl: pallet_fee_control,
		Doughnut: pallet_doughnut,
		Futurepass: pallet_futurepass,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_fee_control_config!(Test);
impl_pallet_futurepass_config!(Test);

pub struct FeeControlWeightToFee;
impl WeightToFee for FeeControlWeightToFee {
	type Balance = Balance;
	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		FeeControl::weight_to_fee(weight)
	}
}
pub struct FeeControlLengthToFee;
impl WeightToFee for FeeControlLengthToFee {
	type Balance = Balance;
	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		FeeControl::length_to_fee(weight)
	}
}
parameter_types! {
	pub const OperationalFeeMultiplier: u8 = 1;
	pub const XrpAssetId: AssetId = XRP_ASSET_ID;
}
pub type XrpCurrency = pallet_assets_ext::AssetCurrency<Test, XrpAssetId>;
impl pallet_transaction_payment::Config for Test {
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<XrpCurrency, ()>;
	type RuntimeEvent = RuntimeEvent;
	type WeightToFee = FeeControlWeightToFee;
	type LengthToFee = FeeControlLengthToFee;
	type FeeMultiplierUpdate = ();
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

parameter_types! {
	pub const GetExchangeFee: (u32, u32) = (3, 1000); // 0.3% fee
	pub const TradingPathLimit: u32 = 3;
	pub const DEXBurnPalletId: PalletId = PalletId(*b"burnaddr");
	pub const LPTokenDecimals: u8 = 6;
	pub const TxFeePotId: PalletId = PalletId(*b"txfeepot");
	pub const DefaultFeeTo: Option<PalletId> = Some(TxFeePotId::get());
}
impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type CallValidator = MockDoughnutCallValidator;
	type FuturepassLookup = FuturepassIdentityLookup;
	type WeightInfo = ();
}

impl fp_self_contained::SelfContainedCall for RuntimeCall {
	type SignedInfo = H160;

	fn is_self_contained(&self) -> bool {
		match self {
			RuntimeCall::Doughnut(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			RuntimeCall::Doughnut(ref call) => call.check_self_contained(),
			_ => None,
		}
	}

	fn validate_self_contained(
		&self,
		signed_info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Self>,
		len: usize,
	) -> Option<TransactionValidity> {
		match self {
			RuntimeCall::Doughnut(ref call) =>
				call.validate_self_contained(signed_info, dispatch_info, len),
			_ => None,
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		signed_info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Self>,
		len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			RuntimeCall::Doughnut(ref call) =>
				call.pre_dispatch_self_contained(signed_info, dispatch_info, len),
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Self>,
		len: usize,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			RuntimeCall::Doughnut(call) => pallet_doughnut::Call::<Test>::apply_self_contained(
				call.into(),
				&info,
				&dispatch_info,
				len,
			),
			_ => None,
		}
	}
}

pub type Executive = frame_executive::Executive<
	Test,
	BlockT,
	frame_system::ChainContext<Test>,
	Test,
	AllPalletsWithSystem,
>;

pub struct MockDoughnutCallValidator;

impl ExtrinsicChecker for MockDoughnutCallValidator {
	type Call = RuntimeCall;
	type PermissionObject = TRNNutV0;
	fn check_extrinsic(_call: &Self::Call, _trnnut: &Self::PermissionObject) -> DispatchResult {
		Ok(())
	}
}

pub struct FuturepassIdentityLookup;
impl StaticLookup for FuturepassIdentityLookup {
	type Source = H160;
	type Target = H160;
	fn lookup(s: Self::Source) -> Result<Self::Target, LookupError> {
		Ok(s)
	}
	fn unlookup(t: Self::Target) -> Self::Source {
		t
	}
}
impl ExtrinsicChecker for FuturepassIdentityLookup {
	type Call = RuntimeCall;
	type PermissionObject = ();
	fn check_extrinsic(
		_call: &Self::Call,
		_permissioned_object: &Self::PermissionObject,
	) -> DispatchResult {
		Ok(())
	}
}
