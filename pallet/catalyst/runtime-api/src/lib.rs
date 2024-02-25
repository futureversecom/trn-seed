#![cfg_attr(not(feature = "std"), no_std)]
use sp_runtime::{codec::Codec, traits::MaybeDisplay};

sp_api::decl_runtime_apis! {
	pub trait CatalystApi<Balance, AssetId> where
		Balance: Codec + MaybeDisplay, AssetId: Codec + MaybeDisplay{
		fn query_total_cost(id: u32, amount: Balance, required_asset: AssetId) -> Balance;
	}
}
