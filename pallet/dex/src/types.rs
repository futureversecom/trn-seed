#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode, MaxEncodedLen};
use root_primitives::AssetId;
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(
	Encode,
	Decode,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	Ord,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TradingPair(pub AssetId, pub AssetId);

impl From<(AssetId, AssetId)> for TradingPair {
	fn from(asset_ids: (AssetId, AssetId)) -> Self {
		if asset_ids.0 > asset_ids.1 {
			TradingPair(asset_ids.0, asset_ids.1)
		} else {
			TradingPair(asset_ids.1, asset_ids.0)
		}
	}
}

impl TradingPair {
	pub fn new(asset_id_a: AssetId, asset_id_b: AssetId) -> Self {
		TradingPair::from((asset_id_a, asset_id_b))
	}
}
