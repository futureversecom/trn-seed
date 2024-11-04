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

use frame_support::parameter_types;

pub use durations::*;
pub use currency::*;

parameter_types! {
	/// Getter for the ROOT asset Id
	pub const RootAssetId: seed_primitives::AssetId = currency::ROOT_ASSET_ID;
	/// Getter for the XRP asset Id
	pub const XrpAssetId: seed_primitives::AssetId = currency::XRP_ASSET_ID;
}

pub mod currency {
	//! root network currency constants
	use seed_primitives::{AssetId, Balance};

	/// The ROOT token name
	pub const ROOT_NAME: &str = "Root";
	/// The ROOT token symbol
	pub const ROOT_SYMBOL: &str = "ROOT";
	/// Decimal places of ROOT
	pub const ROOT_DECIMALS: u8 = 6;
	/// The ROOT asset id within the root network
	pub const ROOT_ASSET_ID: AssetId = 1;
	/// The minimal ROOT asset balance before account storage is reaped
	pub const ROOT_MINIMUM_BALANCE: Balance = 1;
	/// One whole ROOT token in units
	pub const ONE_ROOT: Balance = (10 as Balance).pow(ROOT_DECIMALS as u32);

	/// The XRP token name
	pub const XRP_NAME: &str = "XRP";
	/// The XRP token symbol
	pub const XRP_SYMBOL: &str = "XRP";
	/// Decimal places of XRP
	pub const XRP_DECIMALS: u8 = 6;
	/// XRP asset Id within the root network
	pub const XRP_ASSET_ID: AssetId = 2;
	/// The minimal XRP asset balance before account storage is reaped
	pub const XRP_MINIMUM_BALANCE: Balance = 1;
	pub const ONE_XRP: Balance = (10 as Balance).pow(XRP_DECIMALS as u32); // 1_000_000 drops

	/// The VTX token name
	pub const VTX_NAME: &str = "Vortex";
	/// The VTX token symbol
	pub const VTX_SYMBOL: &str = "VTX";
	/// Decimal places of VTX
	pub const VTX_DECIMALS: u8 = 6;
	/// VTX asset Id within the root network
	pub const VTX_ASSET_ID: AssetId = 3;
	/// The minimal VTX asset balance before account storage is reaped
	pub const VTX_MINIMUM_BALANCE: Balance = 1;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		// TODO: figure out a better way to calculate this
		items as Balance * 100 * XRP_MINIMUM_BALANCE + (bytes as Balance) * 6 * XRP_MINIMUM_BALANCE
	}
}

/// Common constants of parachains.
mod durations {
	use seed_primitives::BlockNumber;

	/// This determines the average expected block time that we are targeting. Blocks will be
	/// produced at a minimum duration defined by `SLOT_DURATION`. `SLOT_DURATION` is picked up by
	/// `pallet_timestamp` which is in turn picked up by `pallet_aura` to implement `fn
	/// slot_duration()`.
	///
	/// Change this to adjust the block time.
	pub const MILLISECS_PER_BLOCK: u64 = 4_000;
	pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

	/// Aka blocks per session
	#[cfg(not(test))]
	pub const EPOCH_DURATION_IN_SLOTS: BlockNumber = 4 * HOURS;
	#[cfg(test)]
	pub const EPOCH_DURATION_IN_SLOTS: BlockNumber = MINUTES / 3; // 5 blocks per session in tests

	/// 24 hour eras `EPOCH_DURATION_IN_SLOTS * SESSIONS_PER_ERA`
	#[cfg(not(test))]
	pub const SESSIONS_PER_ERA: sp_staking::SessionIndex = 24 * HOURS / EPOCH_DURATION_IN_SLOTS;
	#[cfg(test)]
	pub const SESSIONS_PER_ERA: sp_staking::SessionIndex = 1 * MINUTES / EPOCH_DURATION_IN_SLOTS;

	// 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
	pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

	// Time is measured by number of blocks.
	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;
}
