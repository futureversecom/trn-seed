// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Decode;
use ethabi::{ParamType, Token};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure, log,
	pallet_prelude::*,
	traits::{fungibles, fungibles::Mutate, Get, IsType},
	transactional,
	weights::constants::RocksDbWeight as DbWeight,
	PalletId,
};
use frame_system::pallet_prelude::*;
use sp_core::{H160, U256};
use sp_runtime::{
	traits::{AccountIdConversion, One, Saturating},
	SaturatedConversion,
};
use sp_std::prelude::*;

use seed_pallet_common::{CreateExt, EthereumBridge, EthereumEventSubscriber, OnEventResult};
use seed_primitives::{AccountId, AssetId, Balance, EthAddress};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod types;
use types::*;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod weights;

pub use weights::WeightInfo;

pub trait Config: frame_system::Config<AccountId = AccountId> {
	/// An onchain address for this pallet
	type PegPalletId: Get<PalletId>;
	/// Submits event messages to Ethereum
	type EthBridge: EthereumBridge;
	/// Currency functions
	type MultiCurrency: CreateExt<AccountId = Self::AccountId>
		+ fungibles::Inspect<Self::AccountId, AssetId = AssetId>
		+ fungibles::Transfer<Self::AccountId, AssetId = AssetId, Balance = Balance>
		+ fungibles::Mutate<Self::AccountId>;
	/// The overarching event type.
	type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	/// Interface to generate weights
	type WeightInfo: WeightInfo;
}

decl_storage! {
	trait Store for Module<T: Config> as Erc20Peg {
		/// Whether deposit are active
		DepositsActive get(fn deposits_active): bool;
		/// Whether withdrawals are active
		WithdrawalsActive get(fn withdrawals_active): bool;
		/// Map ERC20 address to GA asset Id
		Erc20ToAssetId get(fn erc20_to_asset): map hasher(twox_64_concat) EthAddress => Option<AssetId>;
		/// Map GA asset Id to ERC20 address
		pub AssetIdToErc20 get(fn asset_to_erc20): map hasher(twox_64_concat) AssetId => Option<EthAddress>;
		/// Metadata for well-known erc20 tokens (symbol, decimals)
		Erc20Meta get(fn erc20_meta): map hasher(twox_64_concat) EthAddress => Option<(Vec<u8>, u8)>;
		/// Map from asset_id to minimum amount and delay
		PaymentDelay get(fn payment_delay): map hasher(twox_64_concat) AssetId => Option<(Balance, T::BlockNumber)>;
		/// Map from DelayedPaymentId to PendingPayment
		DelayedPayments get(fn delayed_payments): map hasher(twox_64_concat) DelayedPaymentId => Option<PendingPayment>;
		/// Map from block number to DelayedPaymentIds scheduled for that block
		DelayedPaymentSchedule get(fn delayed_payment_schedule): map hasher(twox_64_concat) T::BlockNumber => Vec<DelayedPaymentId>;
		/// The blocks with payments that are ready to be processed
		ReadyBlocks get(fn ready_blocks): Vec<T::BlockNumber>;
		/// The next available payment id for withdrawals and deposits
		NextDelayedPaymentId get(fn next_delayed_payment_id): DelayedPaymentId;
		/// The peg contract address on Ethereum
		pub ContractAddress get(fn contract_address): EthAddress;
	}
	add_extra_genesis {
		config(erc20s): Vec<(EthAddress, Vec<u8>, u8)>;
		build(|config: &GenesisConfig| {
			for (address, symbol, decimals) in config.erc20s.iter() {
				Erc20Meta::insert(address, (symbol, decimals));
			}
		});
	}
}

decl_event! {
	pub enum Event<T> where
		AccountId = <T as frame_system::Config>::AccountId,
		BlockNumber = <T as frame_system::Config>::BlockNumber,
	{
		/// An erc20 deposit has been delayed.(payment_id, scheduled block, amount, beneficiary)
		Erc20DepositDelayed(DelayedPaymentId, BlockNumber, Balance, AccountId),
		/// A withdrawal has been delayed.(payment_id, scheduled block, amount, beneficiary)
		Erc20WithdrawalDelayed(DelayedPaymentId, BlockNumber, Balance, EthAddress),
		/// A delayed erc20 deposit has failed (payment_id, beneficiary)
		DelayedErc20DepositFailed(DelayedPaymentId, AccountId),
		/// A delayed erc20 withdrawal has failed (asset_id, beneficiary)
		DelayedErc20WithdrawalFailed(AssetId, EthAddress),
		/// A bridged erc20 deposit succeeded. (asset, amount, beneficiary)
		Erc20Deposit(AssetId, Balance, AccountId),
		/// Tokens were burnt for withdrawal on Ethereum as ERC20s (asset, amount, beneficiary)
		Erc20Withdraw(AssetId, Balance, EthAddress),
		/// A bridged erc20 deposit failed. (source address, abi data)
		Erc20DepositFail(H160, Vec<u8>),
		/// The peg contract address has been set
		SetContractAddress(EthAddress),
		/// A delay was added for an asset_id (asset_id, min_balance, delay)
		PaymentDelaySet(AssetId, Balance, BlockNumber),
		/// There are no more payment ids available, they've been exhausted
		NoAvailableDelayedPaymentIds,
	}
}

decl_error! {
	pub enum Error for Module<T: Config> {
		/// Could not create the bridged asset
		CreateAssetFailed,
		/// Deposit has bad amount
		InvalidAmount,
		/// Could not convert pallet id to account
		InvalidPalletId,
		/// Deposits are inactive
		DepositsPaused,
		/// Withdrawals are inactive
		WithdrawalsPaused,
		/// Withdrawals of this asset are not supported
		UnsupportedAsset,
		/// Withdrawals over the set payment delay for EVM calls are disabled
		EvmWithdrawalFailed,
		/// The abi received does not match the encoding scheme
		InvalidAbiEncoding,
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Check and process outstanding payments
		fn on_initialize(now: T::BlockNumber) -> Weight {
			let mut weight: Weight = DbWeight::get().reads(1 as Weight);
			if DelayedPaymentSchedule::<T>::contains_key(now) {
				ReadyBlocks::<T>::append(now);
				weight = weight.saturating_add(DbWeight::get().writes(1 as Weight));
			}
			weight as Weight
		}

		/// Check and process outstanding payments
		fn on_idle(_now: T::BlockNumber, remaining_weight: Weight) -> Weight {
			let initial_read_cost = DbWeight::get().reads(1 as Weight);
			// Ensure we have enough weight to perform the initial read
			if remaining_weight <= initial_read_cost {
				return 0;
			}
			// Check that there are blocks in ready_blocks
			let ready_blocks_length = ReadyBlocks::<T>::decode_len();
			if ready_blocks_length.is_none() || ready_blocks_length == Some(0) {
				return 0;
			}

			// Process as many payments as we can
			let weight_each: Weight = DbWeight::get().reads(8 as Weight).saturating_add(DbWeight::get().writes(10 as Weight));
			let max_payments = ((remaining_weight - initial_read_cost) / weight_each).saturated_into::<u8>();
			let ready_blocks: Vec<T::BlockNumber> = Self::ready_blocks();
			// Total payments processed in this block
			let mut processed_payment_count: u8 = 0;
			// Count of blocks where all payments have been processed
			let mut processed_block_count: u8 = 0;

			for block in ready_blocks.iter() {
				let mut payment_ids = DelayedPaymentSchedule::<T>::take(block);
				let remaining_payments = (max_payments - processed_payment_count) as usize;
				if payment_ids.len() > remaining_payments {
					// Update storage with unprocessed payments
					DelayedPaymentSchedule::<T>::insert(block, payment_ids.split_off(remaining_payments));
				} else {
					processed_block_count += 1;
				}
				processed_payment_count += payment_ids.len() as u8;
				// Process remaining payments from block
				for payment_id in payment_ids {
					Self::process_delayed_payment(payment_id);
				}
				if processed_payment_count >= max_payments {
					break;
				}
			}

			ReadyBlocks::<T>::put(&ready_blocks[processed_block_count as usize..]);
			initial_read_cost + weight_each * processed_payment_count as Weight
		}

		/// Activate/deactivate deposits (root only)
		#[weight = T::WeightInfo::activate_deposits()]
		pub fn activate_deposits(origin, activate: bool) {
			ensure_root(origin)?;
			DepositsActive::put(activate);
		}

		/// Activate/deactivate withdrawals (root only)
		#[weight = T::WeightInfo::activate_withdrawals()]
		pub fn activate_withdrawals(origin, activate: bool) {
			ensure_root(origin)?;
			WithdrawalsActive::put(activate);
		}

		#[weight = T::WeightInfo::withdraw()]
		/// Tokens will be transferred to peg account and a proof generated to allow redemption of tokens on Ethereum
		#[transactional]
		pub fn withdraw(origin, asset_id: AssetId, amount: Balance, beneficiary: EthAddress) {
			let origin = ensure_signed(origin)?;
			Self::do_withdrawal(origin, asset_id, amount, beneficiary, WithdrawCallOrigin::Runtime)?;
		}

		#[weight = T::WeightInfo::set_contract_address()]
		/// Set the peg contract address on Ethereum (requires governance)
		pub fn set_contract_address(origin, eth_address: EthAddress) {
			ensure_root(origin)?;
			ContractAddress::put(eth_address);
			Self::deposit_event(<Event<T>>::SetContractAddress(eth_address));
		}

		#[weight = T::WeightInfo::set_erc20_meta()]
		/// Set the metadata details for a given ERC20 address (requires governance)
		/// details: `[(contract address, symbol, decimals)]`
		pub fn set_erc20_meta(origin, details: Vec<(EthAddress, Vec<u8>, u8)>) {
			ensure_root(origin)?;
			for (address, symbol, decimals) in details {
				Erc20Meta::insert(address, (symbol, decimals));
			}
		}

		#[weight = T::WeightInfo::set_payment_delay()]
		/// Sets the payment delay for a given AssetId
		pub fn set_payment_delay(origin, asset_id: AssetId, min_balance: Balance, delay: T::BlockNumber) {
			ensure_root(origin)?;
			PaymentDelay::<T>::insert(asset_id, (min_balance, delay));
			Self::deposit_event(<Event<T>>::PaymentDelaySet(asset_id, min_balance, delay));
		}
	}
}

impl<T: Config> Module<T> {
	/// Initiate the withdrawal
	/// Can be called by the runtime or erc20-peg precompile
	/// If a payment delay is in place for the asset, this will be handled when called from the
	/// runtime The runtime doesn't use the returned value so 0 is returned in this case
	/// Delays from the EVM will return an error
	pub fn do_withdrawal(
		origin: T::AccountId,
		asset_id: AssetId,
		amount: Balance,
		beneficiary: EthAddress,
		call_origin: WithdrawCallOrigin,
	) -> Result<u64, DispatchError> {
		ensure!(Self::withdrawals_active(), Error::<T>::WithdrawalsPaused);

		// there should be a known ERC20 address mapped for this asset
		// otherwise there may be no liquidity on the Ethereum side of the peg
		let token_address = Self::asset_to_erc20(asset_id);
		ensure!(token_address.is_some(), Error::<T>::UnsupportedAsset);
		let token_address = token_address.unwrap();

		let message = WithdrawMessage { token_address, amount: amount.into(), beneficiary };

		// Check if there is a delay on the asset
		let payment_delay: Option<(Balance, T::BlockNumber)> = Self::payment_delay(asset_id);
		if let Some((min_amount, delay)) = payment_delay {
			if min_amount <= amount {
				return match call_origin {
					WithdrawCallOrigin::Runtime => {
						// Delay the payment
						let _imbalance = T::MultiCurrency::burn_from(asset_id, &origin, amount)?;
						Self::delay_payment(delay, PendingPayment::Withdrawal(message));
						Ok(0)
					},
					WithdrawCallOrigin::Evm => {
						// EVM payment delays are not supported
						Err(Error::<T>::EvmWithdrawalFailed.into())
					},
				}
			}
		};

		// Process transfer or withdrawal of payment asset
		let _imbalance = T::MultiCurrency::burn_from(asset_id, &origin, amount)?;
		Self::process_withdrawal(message, asset_id)
	}

	/// Process withdrawal and send
	fn process_withdrawal(
		withdrawal_message: WithdrawMessage,
		asset_id: AssetId,
	) -> Result<u64, DispatchError> {
		let source: T::AccountId = T::PegPalletId::get().into_account_truncating();
		let message = ethabi::encode(&[
			Token::Address(withdrawal_message.token_address),
			Token::Uint(withdrawal_message.amount.into()),
			Token::Address(withdrawal_message.beneficiary),
		]);

		// Call whatever handler loosely coupled from ethy
		let event_proof_id =
			T::EthBridge::send_event(&source.into(), &Self::contract_address(), &message)?;
		Self::deposit_event(Event::<T>::Erc20Withdraw(
			asset_id,
			withdrawal_message.amount.saturated_into(),
			withdrawal_message.beneficiary,
		));
		Ok(event_proof_id)
	}

	/// Process payments at a block after a delay
	fn process_delayed_payment(payment_id: DelayedPaymentId) {
		if let Some(pending_payment) = DelayedPayments::take(payment_id) {
			match pending_payment {
				PendingPayment::Deposit(deposit) => {
					if Self::process_deposit(deposit.clone()).is_err() {
						Self::deposit_event(Event::<T>::DelayedErc20DepositFailed(
							payment_id,
							deposit.beneficiary.into(),
						));
					}
				},
				PendingPayment::Withdrawal(withdrawal_message) => {
					// At this stage it is assumed that a mapping between erc20 to asset id exists
					// for this token
					let asset_id = Self::erc20_to_asset(withdrawal_message.token_address);
					if let Some(asset_id) = asset_id {
						// Process transfer or withdrawal of payment asset
						if Self::process_withdrawal(withdrawal_message.clone(), asset_id).is_err() {
							Self::deposit_event(Event::<T>::DelayedErc20WithdrawalFailed(
								asset_id,
								withdrawal_message.beneficiary.into(),
							));
						}
					} else {
						log::error!(
							"📌 ERC20 withdrawal failed unexpectedly: {:?}",
							withdrawal_message
						);
					}
				},
			}
		}
	}

	/// Delay a withdrawal or deposit until a later block
	pub fn delay_payment(delay: T::BlockNumber, pending_payment: PendingPayment) {
		let payment_id = NextDelayedPaymentId::get();
		if !payment_id.checked_add(One::one()).is_some() {
			Self::deposit_event(Event::<T>::NoAvailableDelayedPaymentIds);
			return
		}
		let payment_block = <frame_system::Pallet<T>>::block_number().saturating_add(delay);
		DelayedPayments::insert(payment_id, &pending_payment);
		// Modify DelayedPaymentSchedule with new payment_id
		DelayedPaymentSchedule::<T>::append(payment_block, payment_id);
		NextDelayedPaymentId::put(payment_id + 1);

		// Throw event for delayed payment
		match pending_payment {
			PendingPayment::Withdrawal(withdrawal) => {
				Self::deposit_event(Event::<T>::Erc20WithdrawalDelayed(
					payment_id,
					payment_block,
					withdrawal.amount.as_u128(),
					withdrawal.beneficiary,
				));
			},
			PendingPayment::Deposit(deposit) => {
				let beneficiary: T::AccountId =
					T::AccountId::decode(&mut &deposit.beneficiary.0[..]).unwrap();
				Self::deposit_event(Event::<T>::Erc20DepositDelayed(
					payment_id,
					payment_block,
					deposit.amount.as_u128(),
					beneficiary,
				));
			},
		}
	}

	/// Deposit received from bridge, do pre flight checks
	/// If the token has a delay and the amount is above the delay amount, add this deposit to
	/// pending
	pub fn do_deposit(deposit_event: Erc20DepositEvent) -> DispatchResult {
		ensure!(Self::deposits_active(), Error::<T>::DepositsPaused);
		// fail a deposit early for an amount that is too large
		ensure!(deposit_event.amount < U256::from(Balance::max_value()), Error::<T>::InvalidAmount);

		let asset_id = Self::erc20_to_asset(deposit_event.token_address);
		if asset_id.is_some() {
			// Asset exists, check if there are delays on this deposit
			let payment_delay: Option<(Balance, T::BlockNumber)> =
				Self::payment_delay(asset_id.unwrap());
			if let Some((min_amount, delay)) = payment_delay {
				if U256::from(min_amount) <= deposit_event.amount {
					Self::delay_payment(delay, PendingPayment::Deposit(deposit_event.clone()));
					return Ok(())
				}
			};
		}
		// process deposit immediately
		Self::process_deposit(deposit_event)
	}

	/// fulfill a deposit for the given event
	/// Handles mint and asset creation
	pub fn process_deposit(verified_event: Erc20DepositEvent) -> DispatchResult {
		let asset_id = match Self::erc20_to_asset(verified_event.token_address) {
			None => {
				// create asset with known values from `Erc20Meta`
				// asset will be created with `18` decimal places and "" for symbol if the asset is
				// unknown dapps can also use `AssetToERC20` to retrieve the appropriate decimal
				// places from ethereum
				let (symbol, decimals) = Erc20Meta::get(verified_event.token_address)
					.unwrap_or((Default::default(), 18));

				let pallet_id = T::PegPalletId::get().into_account_truncating();
				let asset_id = T::MultiCurrency::create_with_metadata(
					&pallet_id,
					// TODO: We may want to accept a name as input as well later. For now, we will
					// use the symbol for both symbol and name
					symbol.clone(),
					symbol,
					decimals,
					None,
				)
				.map_err(|_| Error::<T>::CreateAssetFailed)?;

				Erc20ToAssetId::insert(verified_event.token_address, asset_id);
				AssetIdToErc20::insert(asset_id, verified_event.token_address);
				asset_id
			},
			Some(asset_id) => asset_id,
		};

		// checked at the time of initiating the verified_event that beneficiary value is valid and
		// this op will not fail qed.
		let beneficiary: T::AccountId = verified_event.beneficiary.into();
		// Asserted prior
		let amount = verified_event.amount.as_u128();
		// mint tokens to user
		T::MultiCurrency::mint_into(asset_id, &beneficiary, amount)?;

		Self::deposit_event(Event::<T>::Erc20Deposit(asset_id, amount, beneficiary));
		Ok(())
	}
}

impl Get<H160> for ContractAddress {
	fn get() -> H160 {
		<ContractAddress as storage::StorageValue<_>>::get()
	}
}

impl<T: Config> EthereumEventSubscriber for Module<T> {
	type Address = T::PegPalletId;

	type SourceAddress = ContractAddress;

	fn on_event(source: &H160, data: &[u8]) -> OnEventResult {
		let abi_decoded = match ethabi::decode(
			&[ParamType::Address, ParamType::Uint(128), ParamType::Address],
			data,
		) {
			Ok(abi) => abi,
			Err(_) => return Err((0, Error::<T>::InvalidAbiEncoding.into())),
		};

		if let &[Token::Address(token_address), Token::Uint(amount), Token::Address(beneficiary)] =
			abi_decoded.as_slice()
		{
			let token_address: H160 = token_address.into();
			let amount: U256 = amount.into();
			let beneficiary: H160 = beneficiary.into();
			// The total weight of do_deposit assuming it reaches every path
			let deposit_weight =
				DbWeight::get().reads(6 as Weight) + DbWeight::get().writes(4 as Weight);
			match Self::do_deposit(Erc20DepositEvent { token_address, amount, beneficiary }) {
				Ok(_) => Ok(deposit_weight),
				Err(e) => {
					Self::deposit_event(Event::<T>::Erc20DepositFail(*source, data.to_vec()));
					Err((deposit_weight, e.into()))
				},
			}
		} else {
			// input data should be valid, we do not expect to fail here
			Err((0, Error::<T>::InvalidAbiEncoding.into()))
		}
	}
}
