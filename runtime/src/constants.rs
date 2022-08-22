// Copyright (C) 2021-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use frame_support::parameter_types;

pub use constants::*;
pub use currency::*;

parameter_types! {
	/// Getter for the MYCL asset Id
	pub const MyclAssetId: seed_primitives::AssetId = currency::MYCL_ASSET_ID;
	/// Getter for the XRP asset Id
	pub const XrpAssetId: seed_primitives::AssetId = currency::XRP_ASSET_ID;
}

pub mod currency {
	//! root network currency constants
	use seed_primitives::{AssetId, Balance};
	/// Decimal places of MYCL
	pub const MYCL_DECIMALS: u8 = 6;
	/// The MYCL asset id within the root network
	pub const MYCL_ASSET_ID: AssetId = 1;
	/// The minimal MYCL asset balance before account storage is reaped
	pub const MYCL_MINIMUM_BALANCE: Balance = 1;
	/// One whole MYCL token in units
	pub const ONE_MYCL: Balance = (10 as Balance).pow(MYCL_DECIMALS as u32);

	/// Decimal places of XRP
	pub const XRP_DECIMALS: u8 = 6;
	/// XRP asset Id within the root network
	pub const XRP_ASSET_ID: AssetId = 2;
	/// The minimal XRP asset balance before account storage is reaped
	pub const XRP_MINIMUM_BALANCE: Balance = 1;
	pub const ONE_XRP: Balance = (10 as Balance).pow(XRP_DECIMALS as u32);
}

/// Common constants of parachains.
mod constants {
	use frame_support::weights::{constants::WEIGHT_PER_SECOND, Weight};
	use seed_primitives::BlockNumber;
	use sp_runtime::Perbill;
	/// This determines the average expected block time that we are targeting. Blocks will be
	/// produced at a minimum duration defined by `SLOT_DURATION`. `SLOT_DURATION` is picked up by
	/// `pallet_timestamp` which is in turn picked up by `pallet_aura` to implement `fn
	/// slot_duration()`.
	///
	/// Change this to adjust the block time.
	pub const MILLISECS_PER_BLOCK: u64 = 4_000;
	pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

	// Time is measured by number of blocks.
	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;

	/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
	/// used to limit the maximal weight of a single extrinsic.
	pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);
	/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
	/// Operational  extrinsics.
	pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

	/// We allow for 0.5 seconds of compute with a 6 second average block time.
	pub const MAXIMUM_BLOCK_WEIGHT: Weight = WEIGHT_PER_SECOND / 2;
}
