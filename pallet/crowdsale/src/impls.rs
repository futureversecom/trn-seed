// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use crate::*;
use scale_info::prelude::format;

impl<T: Config> Pallet<T> {
	/// Creates a unique voucher asset for a sale. Returns the AssetId of the created asset
	pub(crate) fn create_voucher_asset(sale_id: SaleId) -> Result<AssetId, DispatchError> {
		let voucher_owner = T::PalletId::get().into_account_truncating();
		let voucher_asset_id = T::MultiCurrency::create_with_metadata(
			&voucher_owner,
			format!("CrowdSale Voucher-{}", sale_id).as_bytes().to_vec(),
			format!("CSV-{}", sale_id).as_bytes().to_vec(),
			VOUCHER_DECIMALS,
			None,
		)
		.map_err(|_| Error::<T>::CreateAssetFailed)?;
		Ok(voucher_asset_id)
	}

	/// Calculate how many vouchers an account should receive based on their contribution at the
	/// end of the sale
	/// 'soft_cap_price' - What was the initial soft cap price?
	/// 'total_funds_raised' - How many funds were raised in total for the sale
	/// 'account_contribution' - How much has the user contributed to this round?
	/// 'voucher_total_supply' - Also NFT max_issuance
	pub(crate) fn calculate_voucher_rewards(
		soft_cap_price: Balance,
		total_funds_raised: Balance,
		account_contribution: Balance,
		voucher_total_supply: Balance,
	) -> Balance {
		// Calculate the price of the soft cap across the total supply. This is our baseline
		let soft_cap_total = soft_cap_price * voucher_total_supply;

		// Check if we are over or under committed
		let voucher_price: Balance = if total_funds_raised > soft_cap_total {
			// We are over committed! Calculate the voucher price based on the total
			total_funds_raised / voucher_total_supply
		} else {
			// We are under committed so we will pay out the soft cap
			soft_cap_price
		};

		let total_user_vouchers = account_contribution / voucher_price;

		const VOUCHER_DECIMALS: u32 = 6;
		// Return total vouchers converted to the correct decimals
		return total_user_vouchers * 10u128.pow(VOUCHER_DECIMALS)
	}

	/// Close all crowdsales that are scheduled to end this block
	pub(crate) fn close_sales_at(now: T::BlockNumber) -> u32 {
		let mut removed = 0_u32;
		let Some(sales_to_close) = SaleEndBlocks::<T>::take(now) else {
			return removed
		};

		for sale_id in sales_to_close.into_iter() {
			// TODO log error, can't error here
			// Neither of the errors should happen
			let _ = SaleInfo::<T>::try_mutate(sale_id, |sale_info| -> DispatchResult {
				removed += 1;
				let Some(sale_info) = sale_info else {
					return Err(Error::<T>::CrowdsaleNotFound.into());
				};
				ensure!(sale_info.status == SaleStatus::Enabled, Error::<T>::SaleNotEnabled);
				// Close the sale
				sale_info.status = SaleStatus::Closed;
				Ok(())
			});
		}
		removed
	}
}
