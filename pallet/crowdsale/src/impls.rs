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
	pub(crate) fn calculate_voucher_rewards_old(
		soft_cap_price: Balance,
		total_funds_raised: Balance,
		account_contribution: Balance,
		voucher_total_supply: Balance,
	) -> Balance {
		// Calculate the price of the soft cap across the total supply. This is our baseline
		let crowd_sale_target = soft_cap_price * voucher_total_supply;

		// Check if we are over or under committed
		let voucher_price: Balance = if total_funds_raised > crowd_sale_target {
			// We are over committed. Calculate the voucher price based on the total
			total_funds_raised / voucher_total_supply
		// Self::divide_rounding(total_funds_raised, voucher_total_supply)
		} else {
			// We are under committed so we will pay out the soft cap
			soft_cap_price
		};

		// Add 6 zeros to the account contribution to match the voucher price decimals.
		// If we add this later, we will lose precision
		let contribution = account_contribution * 10u128.pow(VOUCHER_DECIMALS as u32);
		// divide account_contribution by voucher_price and round up or down
		let voucher_quantity = Self::divide_rounding(contribution, voucher_price);

		voucher_quantity
	}

	// Divide two numbers and round up if the remainder is greater than half the divisor
	fn divide_rounding(numerator: Balance, denominator: Balance) -> Balance {
		let quotient = numerator / denominator;
		let remainder = numerator % denominator;

		if remainder * 2 >= denominator {
			quotient //+ 1
		} else {
			quotient
		}
	}

	/// Calculate how many vouchers an account should receive based on their contribution at the
	/// end of the sale
	/// 'soft_cap_price' - What was the initial soft cap price?
	/// 'total_funds_raised' - How many funds were raised in total for the sale
	/// 'account_contribution' - How much has the user contributed to this round?
	/// 'voucher_max_supply' - The max amount of vouchers to be minted.
	///                        Also NFT max_issuance
	/// 'voucher_current_supply' - The current amount of vouchers minted
	/// 'total_paid_contributions' - The total amount of contributions paid so far
	///
	/// Note. The standard calculation involves dividing the users contribution by
	/// the voucher_price. This works, however we end up accumulating inaccuracies due to the
	/// precision of using 6 decimal places.
	/// We can counter this by calculating the total supply of vouchers after this payment is made
	/// and subtracting the amount of vouchers that were minted before this payment was made.
	/// That way we spread the inaccuracies across multiple accounts and end up with a more accurate
	/// total supply.
	/// As a last precaution, we limit the total supply to the max supply to avoid minting more than
	/// the max supply.
	pub(crate) fn calculate_voucher_rewards(
		soft_cap_price: Balance,
		total_funds_raised: Balance,
		account_contribution: U256,
		voucher_max_supply: Balance,
		voucher_current_supply: Balance,
		total_paid_contributions: U256,
	) -> Result<Balance, &'static str> {
		// Calculate the price of the soft cap across the total supply. This is our baseline
		let crowd_sale_target = soft_cap_price * voucher_max_supply;

		// Check if we are over or under committed
		let voucher_price: Balance = if total_funds_raised > crowd_sale_target {
			// We are over committed. Calculate the voucher price based on the total
			total_funds_raised
				.checked_div(voucher_max_supply)
				.ok_or("Voucher max supply must be greater than 0")?
		} else {
			// We are under committed so we will pay out the soft cap
			soft_cap_price
		};

		// Total contributions of all payments prior to this payment + the contributions
		// from this account
		// Converted to U256 to avoid overflow during calculations
		let contribution_after: U256 =
			account_contribution.saturating_add(total_paid_contributions);

		// Add voucher decimals due to the fact that voucher_total_supply is excluding decimals
		// (As we need a whole number of NFTs)
		// Note. We add decimals here to avoid losing precision
		let contribution_after =
			contribution_after.saturating_mul(U256::from(10_u128.pow(VOUCHER_DECIMALS as u32)));

		// The total supply of vouchers after this payment is made
		// Use checked div, if voucher_price is 0, return 0
		let voucher_supply_after = contribution_after
			.checked_div(U256::from(voucher_price))
			.ok_or("Voucher price must be greater than 0")?;
		let voucher_supply_after: u128 = voucher_supply_after.saturated_into();

		// Limit the voucher supply to the total supply in the case where voucher_price
		// is inaccurate. This ensures we will never payout more than the total supply
		let voucher_supply_after = u128::min(
			voucher_supply_after,
			voucher_max_supply * 10_u128.pow(VOUCHER_DECIMALS as u32),
		);

		// Return the number of vouchers to be paid out, which is the difference between
		// the total supply after this payment and the total supply before this payment
		return Ok(voucher_supply_after.saturating_sub(voucher_current_supply))
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

				// close the sale
				sale_info.status = SaleStatus::Closed;

				// TODO: use NFTExt to get the collection max issuance
				let collection_max_issuance = 1000;
				let crowd_sale_target = sale_info.soft_cap_price * collection_max_issuance;

				// example:
				// soft_cap_price = 10_000_000 ROOT (10 root)
				// max_issuance = 1000
				// = crowd_sale_target = 10_000_000 * 1000 = 10_000_000_000 (10_000 root)
				// funds_raised = 20_000_000_000 (20_000 root)

				// voucher_price = 20_000_000_000 / 1000 = 20_000_000 (20 root)
				let mut voucher_price = sale_info.soft_cap_price;
				if sale_info.funds_raised > crowd_sale_target {
					// We are over committed! Calculate the voucher price based on the total
					voucher_price = sale_info.funds_raised / collection_max_issuance;
				}

				let refunded_vouchers = sale_info.funds_raised.saturating_sub(crowd_sale_target);
				if refunded_vouchers > 0 {
					T::MultiCurrency::mint_into(
						sale_info.payment_asset,
						&sale_info.admin,
						refunded_vouchers,
					)?;
				}

				// TODO: get contributers list from storage map based on sale ID
				// TODO: figure out an optimized way to do that; example below is with 1 contributor
				// let contributor = T::PalletId::get().into_account_truncating();
				// let contribution = 500_000_000; // 500 root
				// 				// let vouchers_quantity_redeemed = contribution / voucher_price; // 500_000_000
				// / 				// 20_000_000 = 25 let voucher_decimals =
				// 				// T::MultiCurrency::decimals(&sale_info.payment_asset); let voucher_amount =
				// 				// vouchers_quantity_redeemed 	.saturating_mul(10u32.pow(voucher_decimals as
				// 				// u32).into());
				//
				// let allocated_vouchers = Self::calculate_voucher_rewards(
				// 	sale_info.soft_cap_price,
				// 	sale_info.funds_raised,
				// 	contribution,
				// 	collection_max_issuance,
				// );
				// T::MultiCurrency::mint_into(sale_info.voucher, &contributor,
				// allocated_vouchers)?;

				// TODO: emit an event for each contributor redeeming their vouchers

				// TODO: emit event for sale closing with:
				// - voucher price
				// - soft cap target
				// - total funds raised
				// - admin vouchers refunded

				Ok(())
			});
		}
		removed
	}
}
