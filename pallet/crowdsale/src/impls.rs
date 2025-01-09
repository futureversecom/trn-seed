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
use alloc::format;
use frame_support::{
	sp_runtime::traits::{BlakeTwo256, Hash},
	traits::{
		fungibles::Inspect,
		tokens::{Fortitude, Preservation},
	},
};
use sp_core::U256;

impl<T: Config> Pallet<T> {
	/// Create a unique account (vault) to hold funds (payment_asset) for a crowdsale.
	pub(crate) fn vault_account(nonce: SaleId) -> T::AccountId {
		let seed: T::AccountId = T::PalletId::get().into_account_truncating();
		let entropy = (seed, nonce).using_encoded(BlakeTwo256::hash);
		T::AccountId::decode(&mut &entropy[..]).expect("Created account ID is always valid; qed")
	}

	/// Creates a unique voucher asset for a sale with 6 decimals
	/// Returns the AssetId of the created asset.
	pub(crate) fn create_voucher_asset(
		vault: &T::AccountId,
		sale_id: SaleId,
		collection_max_issuance: TokenCount,
		voucher_name: Option<Vec<u8>>,
		voucher_symbol: Option<Vec<u8>>,
	) -> Result<AssetId, DispatchError> {
		let voucher_name = voucher_name
			.unwrap_or_else(|| format!("CrowdSale Voucher-{}", sale_id).as_bytes().to_vec());
		let voucher_symbol =
			voucher_symbol.unwrap_or_else(|| format!("CSV-{}", sale_id).as_bytes().to_vec());
		let voucher_asset_id = T::MultiCurrency::create_with_metadata(
			vault,
			voucher_name,
			voucher_symbol,
			VOUCHER_DECIMALS,
			None,
		)?;

		// Calculate total supply and mint into the vault
		let total_supply = Balance::from(collection_max_issuance)
			.checked_mul(10u128.pow(VOUCHER_DECIMALS as u32))
			.ok_or(Error::<T>::InvalidMaxIssuance)?;
		T::MultiCurrency::mint_into(voucher_asset_id, vault, total_supply)?;

		Ok(voucher_asset_id)
	}

	/// Calculate how many vouchers an account should receive based on their contribution at the
	/// end of the sale.
	/// 'soft_cap_price' - What was the initial soft cap price?
	/// 'total_funds_raised' - How many funds were raised in total for the sale
	/// 'account_contribution' - How much has the user contributed to this round?
	/// 'voucher_total_supply' - Also NFT max_issuance
	pub(crate) fn calculate_voucher_rewards(
		soft_cap_price: Balance,
		total_funds_raised: Balance,
		account_contribution: U256,
		voucher_total_supply: Balance,
	) -> Result<Balance, &'static str> {
		// Calculate the price of the soft cap across the total supply. This is our baseline
		let crowd_sale_target = soft_cap_price.saturating_mul(voucher_total_supply);

		// Add 6 zeros to the account contribution to match the voucher price decimals.
		// If we add this later, we will lose precision
		let contribution =
			account_contribution.saturating_mul(U256::from(10_u128.pow(VOUCHER_DECIMALS as u32)));

		// Check if we are over or under committed
		let voucher_quantity: U256 = if total_funds_raised > crowd_sale_target {
			// We are over committed. Calculate the voucher price based on the total
			contribution
				.saturating_mul(U256::from(voucher_total_supply))
				.checked_div(U256::from(total_funds_raised))
				.ok_or("Total funds raised must be greater than 0")?
		} else {
			// We are under committed so we will pay out the soft cap
			contribution
				.checked_div(U256::from(soft_cap_price))
				.ok_or("Voucher price must be greater than 0")?
		};

		let voucher_quantity: Balance = voucher_quantity.saturated_into();

		Ok(voucher_quantity)
	}

	// Transfers vouchers from the vault account into a users wallet and returns the amount minted.
	pub fn transfer_user_vouchers(
		who: T::AccountId,
		sale_info: &SaleInformation<T::AccountId, BlockNumberFor<T>>,
		contribution: Balance,
		voucher_max_supply: Balance,
	) -> Result<Balance, DispatchError> {
		// calculate the claimable vouchers
		let claimable_vouchers = Self::calculate_voucher_rewards(
			sale_info.soft_cap_price,
			sale_info.funds_raised,
			contribution.into(),
			voucher_max_supply,
		)
		.map_err(|_| Error::<T>::VoucherClaimFailed)?;

		let vault_balance = T::MultiCurrency::reducible_balance(
			sale_info.voucher_asset_id,
			&sale_info.vault,
			Preservation::Expendable,
			Fortitude::Polite,
		);
		let vouchers = u128::min(vault_balance, claimable_vouchers);

		// transfer claimable vouchers from vault account to the user
		T::MultiCurrency::transfer(
			sale_info.voucher_asset_id,
			&sale_info.vault,
			&who,
			vouchers,
			Preservation::Expendable,
		)?;

		Ok(claimable_vouchers)
	}

	/// Close all crowdsales that are scheduled to end this block.
	pub(crate) fn close_sales_at(now: BlockNumberFor<T>) -> Result<u32, &'static str> {
		let mut removed = 0_u32;

		let Some(sales_to_close) = SaleEndBlocks::<T>::take(now) else {
			return Ok(removed);
		};

		for sale_id in sales_to_close.into_iter() {
			let _ = SaleInfo::<T>::try_mutate(sale_id, |sale_info| -> DispatchResult {
				removed += 1;
				let sale_info = sale_info.as_mut().ok_or(Error::<T>::CrowdsaleNotFound)?;

				ensure!(
					matches!(sale_info.status, SaleStatus::Enabled(_)),
					Error::<T>::CrowdsaleNotEnabled
				);

				// transfer all payment_asset from the sale vault to the admin
				T::MultiCurrency::transfer(
					sale_info.payment_asset_id,
					&sale_info.vault,
					&sale_info.admin,
					sale_info.funds_raised,
					Preservation::Expendable,
				)?;

				let collection_max_issuance =
					T::NFTExt::get_collection_issuance(sale_info.reward_collection_id)?
						.1
						.ok_or(Error::<T>::MaxIssuanceNotSet)?;
				let collection_max_issuance: Balance = collection_max_issuance.into();

				// Should find the voucher price
				let crowd_sale_target =
					sale_info.soft_cap_price.saturating_mul(collection_max_issuance);

				if sale_info.funds_raised < crowd_sale_target {
					// Refunded amount is equal to the total issuance minus the total vouchers paid
					// out. Total vouchers paid out is the total funds raised divided by the voucher
					// price
					let voucher_total_issuance =
						collection_max_issuance.saturating_mul(10u128.pow(VOUCHER_DECIMALS as u32));
					let voucher_price = sale_info.soft_cap_price;
					let total_vouchers = sale_info
						.funds_raised
						.saturating_mul(10u128.pow(VOUCHER_DECIMALS as u32))
						.saturating_div(voucher_price);
					let refunded_vouchers = voucher_total_issuance.saturating_sub(total_vouchers);

					T::MultiCurrency::transfer(
						sale_info.voucher_asset_id,
						&sale_info.vault,
						&sale_info.admin,
						refunded_vouchers,
						Preservation::Expendable,
					)?;
				}

				if sale_info.funds_raised.is_zero() {
					// No funds raised, end the sale now and skip distribution step
					sale_info.status = SaleStatus::Ended(now);
				} else {
					// Mark the sale for distribution
					// Try append to distributingSales, if this fails due to upper vec bounds
					// Set status to DistributionFailed and log the error
					if SaleDistribution::<T>::try_append(sale_id).is_err() {
						sale_info.status = SaleStatus::DistributionFailed(now);
						log!(error, "⛔️ failed to mark sale {:?} for distribution", sale_id);
						return Ok(());
					}
					sale_info.status = SaleStatus::Distributing(now, Balance::default());
				}

				// Emit event to mark end of crowdsale
				Self::deposit_event(Event::CrowdsaleClosed { sale_id, info: sale_info.clone() });
				Ok(())
			});
		}
		Ok(removed)
	}
}
