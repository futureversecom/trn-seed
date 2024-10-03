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

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Decode;
use ethabi::{ParamType, Token};
use frame_support::{
	ensure, log,
	pallet_prelude::*,
	traits::{
		fungibles,
		fungibles::Mutate,
		tokens::{Fortitude, Precision, Preservation},
		Get, IsType,
	},
	transactional,
	weights::constants::RocksDbWeight as DbWeight,
	PalletId,
};
use frame_system::pallet_prelude::*;
use seed_pallet_common::{CreateExt, EthereumBridge, EthereumEventSubscriber, OnEventResult};
use seed_primitives::{AccountId, AssetId, Balance, EthAddress};
use sp_core::{H160, U256};
use sp_runtime::{
	traits::{AccountIdConversion, One, Saturating},
	SaturatedConversion,
};
use sp_std::prelude::*;

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

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::{DispatchResult, *};

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		erc20s: Vec<(EthAddress, BoundedVec<u8, T::StringLimit>, u8)>,
		_phantom: sp_std::marker::PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { erc20s: vec![], _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			for (address, symbol, decimals) in self.erc20s.iter() {
				Erc20Meta::<T>::insert(address, (symbol, decimals));
			}
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		/// An onchain address for this pallet
		type PegPalletId: Get<PalletId>;
		/// Submits event messages to Ethereum
		type EthBridge: EthereumBridge;
		/// Currency functions
		type MultiCurrency: CreateExt<AccountId = Self::AccountId>
			+ fungibles::Inspect<Self::AccountId, AssetId = AssetId>
			+ fungibles::Mutate<Self::AccountId, AssetId = AssetId, Balance = Balance>;
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Interface to generate weights
		type WeightInfo: WeightInfo;
		/// The native token asset Id (managed by pallet-balances)
		#[pallet::constant]
		type NativeAssetId: Get<AssetId>;
		/// The maximum length of an ERC20 token symbol
		#[pallet::constant]
		type StringLimit: Get<u32>;
		/// The maximum number of delayed payments allowed per block
		#[pallet::constant]
		type MaxDelaysPerBlock: Get<u32>;
		/// The maximum number of ready blocks allowed per storage
		#[pallet::constant]
		type MaxReadyBlocks: Get<u32>;
	}

	/// Whether deposit are active
	#[pallet::storage]
	pub type DepositsActive<T> = StorageValue<_, bool, ValueQuery>;

	/// Whether withdrawals are active
	#[pallet::storage]
	pub type WithdrawalsActive<T> = StorageValue<_, bool, ValueQuery>;

	/// Default value for deposit and withdrawal delays
	#[pallet::type_value]
	pub fn DefaultTrue() -> bool {
		true
	}

	/// Whether deposit delays are active, default is set to true
	#[pallet::storage]
	pub type DepositsDelayActive<T> = StorageValue<_, bool, ValueQuery, DefaultTrue>;

	/// Whether withdrawals delays are active, default is set to true
	#[pallet::storage]
	pub type WithdrawalsDelayActive<T> = StorageValue<_, bool, ValueQuery, DefaultTrue>;

	/// Map ERC20 address to GA asset Id
	#[pallet::storage]
	pub type Erc20ToAssetId<T: Config> = StorageMap<_, Twox64Concat, EthAddress, AssetId>;

	/// Map GA asset Id to ERC20 address
	#[pallet::storage]
	pub type AssetIdToErc20<T: Config> = StorageMap<_, Twox64Concat, AssetId, EthAddress>;

	/// Metadata for well-known erc20 tokens (symbol, decimals)
	#[pallet::storage]
	pub type Erc20Meta<T: Config> =
		StorageMap<_, Twox64Concat, EthAddress, (BoundedVec<u8, T::StringLimit>, u8)>;

	/// Map from asset_id to minimum amount and delay
	#[pallet::storage]
	pub type PaymentDelay<T: Config> =
		StorageMap<_, Twox64Concat, AssetId, (Balance, BlockNumberFor<T>)>;

	/// Map from DelayedPaymentId to PendingPayment
	#[pallet::storage]
	pub type DelayedPayments<T: Config> =
		StorageMap<_, Twox64Concat, DelayedPaymentId, PendingPayment>;

	/// Map from block number to DelayedPaymentIds scheduled for that block
	#[pallet::storage]
	pub type DelayedPaymentSchedule<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<DelayedPaymentId, T::MaxDelaysPerBlock>,
		ValueQuery,
	>;

	/// The blocks with payments that are ready to be processed
	#[pallet::storage]
	pub type ReadyBlocks<T: Config> =
		StorageValue<_, BoundedVec<BlockNumberFor<T>, T::MaxReadyBlocks>, ValueQuery>;

	/// The next available payment id for withdrawals and deposits
	#[pallet::storage]
	pub type NextDelayedPaymentId<T> = StorageValue<_, DelayedPaymentId, ValueQuery>;

	/// The peg contract address on Ethereum
	#[pallet::storage]
	pub type ContractAddress<T> = StorageValue<_, EthAddress, ValueQuery>;

	/// The ROOT peg contract address on Ethereum
	#[pallet::storage]
	pub type RootPegContractAddress<T> = StorageValue<_, EthAddress, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An erc20 deposit has been delayed.
		Erc20DepositDelayed {
			payment_id: DelayedPaymentId,
			scheduled_block: BlockNumberFor<T>,
			amount: Balance,
			beneficiary: T::AccountId,
			asset_id: AssetId,
		},
		/// A withdrawal has been delayed.
		Erc20WithdrawalDelayed {
			payment_id: DelayedPaymentId,
			scheduled_block: BlockNumberFor<T>,
			amount: Balance,
			beneficiary: EthAddress,
			asset_id: AssetId,
			source: T::AccountId,
		},
		/// An ERC20 delay has failed due to storage bounds.
		Erc20DelayFailed {
			payment_id: DelayedPaymentId,
			scheduled_block: BlockNumberFor<T>,
			asset_id: AssetId,
			source: T::AccountId,
		},
		/// A delayed erc20 deposit has failed.
		DelayedErc20DepositFailed { payment_id: DelayedPaymentId, beneficiary: T::AccountId },
		/// A delayed erc20 withdrawal has failed.
		DelayedErc20WithdrawalFailed { asset_id: AssetId, beneficiary: EthAddress },
		/// A bridged erc20 deposit succeeded.
		Erc20Deposit { asset_id: AssetId, amount: Balance, beneficiary: T::AccountId },
		/// Tokens were burnt for withdrawal on Ethereum as ERC20s
		Erc20Withdraw {
			asset_id: AssetId,
			amount: Balance,
			beneficiary: EthAddress,
			source: T::AccountId,
		},
		/// A bridged erc20 deposit failed.
		Erc20DepositFail { source: H160, abi_data: Vec<u8> },
		/// The peg contract address has been set.
		SetContractAddress { address: EthAddress },
		/// The ROOT peg contract address has been set.
		SetRootPegContract { address: EthAddress },
		/// A delay was added for an asset_id.
		PaymentDelaySet { asset_id: AssetId, min_balance: Balance, delay: BlockNumberFor<T> },
		/// There are no more payment ids available, they've been exhausted.
		NoAvailableDelayedPaymentIds,
		/// Toggle deposit delay.
		ActivateDepositDelay { active: bool },
		/// Toggle withdrawal delay.
		ActivateWithdrawalDelay { active: bool },
		/// Toggle deposits.
		ActivateDeposits { active: bool },
		/// Toggle withdrawals.
		ActivateWithdrawals { active: bool },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Could not create the bridged asset
		CreateAssetFailed,
		/// Deposit has bad amount
		InvalidAmount,
		/// Could not convert pallet id to account
		InvalidPalletId,
		/// The peg source address is incorrect for the token being bridged
		InvalidSourceAddress,
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

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Check and process outstanding payments
		fn on_idle(_now: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			let initial_read_cost = DbWeight::get().reads(1u64);
			// Ensure we have enough weight to perform the initial read
			if remaining_weight.ref_time() <= initial_read_cost.ref_time() {
				return Weight::zero();
			}
			// Check that there are blocks in ready_blocks
			let ready_blocks_length = ReadyBlocks::<T>::decode_len();
			if ready_blocks_length.is_none() || ready_blocks_length == Some(0) {
				return Weight::zero();
			}

			// Process as many payments as we can
			let weight_each: Weight =
				DbWeight::get().reads(8u64).saturating_add(DbWeight::get().writes(10u64));
			let max_payments = remaining_weight
				.sub_ref_time(initial_read_cost.ref_time())
				.div(weight_each.ref_time())
				.ref_time()
				.saturated_into::<u8>();
			let ready_blocks: Vec<BlockNumberFor<T>> = ReadyBlocks::<T>::get().into_inner();
			// Total payments processed in this block
			let mut processed_payment_count: u8 = 0;
			// Count of blocks where all payments have been processed
			let mut processed_block_count: u8 = 0;

			for block in ready_blocks.iter() {
				let mut payment_ids = DelayedPaymentSchedule::<T>::take(block).into_inner();
				let remaining_payments = (max_payments - processed_payment_count) as usize;
				if payment_ids.len() > remaining_payments {
					// Update storage with unprocessed payments
					DelayedPaymentSchedule::<T>::insert(
						block,
						BoundedVec::truncate_from(payment_ids.split_off(remaining_payments)),
					);
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

			let ready_blocks =
				BoundedVec::truncate_from(ready_blocks[processed_block_count as usize..].to_vec());
			ReadyBlocks::<T>::put(ready_blocks);
			initial_read_cost
				.add_ref_time(weight_each.mul(processed_payment_count as u64).ref_time())
		}

		/// Check and process outstanding payments
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			let mut weight: Weight = DbWeight::get().reads(1u64);
			if DelayedPaymentSchedule::<T>::contains_key(now) {
				let _ = ReadyBlocks::<T>::try_append(now).map_err(|_| {
					log::error!("ERC20-Peg: 📌 Failed to append ready block: {:?}", now);
				});
				weight = weight.saturating_add(DbWeight::get().writes(1u64));
			}
			weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Activate/deactivate deposits (root only)
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::activate_deposits())]
		pub fn activate_deposits(origin: OriginFor<T>, activate: bool) -> DispatchResult {
			ensure_root(origin)?;
			DepositsActive::<T>::put(activate);
			Self::deposit_event(Event::<T>::ActivateDeposits { active: activate });
			Ok(())
		}

		/// Activate/deactivate withdrawals (root only)
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::activate_withdrawals())]
		pub fn activate_withdrawals(origin: OriginFor<T>, activate: bool) -> DispatchResult {
			ensure_root(origin)?;
			WithdrawalsActive::<T>::put(activate);
			Self::deposit_event(Event::<T>::ActivateWithdrawals { active: activate });
			Ok(())
		}

		/// Activate/deactivate delay deposits (root only)
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::activate_deposits_delay())]
		pub fn activate_deposits_delay(origin: OriginFor<T>, activate: bool) -> DispatchResult {
			ensure_root(origin)?;
			DepositsDelayActive::<T>::put(activate);
			Self::deposit_event(Event::<T>::ActivateDepositDelay { active: activate });
			Ok(())
		}

		/// Activate/deactivate withdrawals (root only)
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::activate_withdrawals_delay())]
		pub fn activate_withdrawals_delay(origin: OriginFor<T>, activate: bool) -> DispatchResult {
			ensure_root(origin)?;
			WithdrawalsDelayActive::<T>::put(activate);
			Self::deposit_event(Event::<T>::ActivateWithdrawalDelay { active: activate });
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::withdraw())]
		/// Tokens will be transferred to peg account and a proof generated to allow redemption of
		/// tokens on Ethereum
		#[transactional]
		pub fn withdraw(
			origin: OriginFor<T>,
			asset_id: AssetId,
			amount: Balance,
			beneficiary: EthAddress,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			Self::do_withdrawal(
				origin,
				asset_id,
				amount,
				beneficiary,
				WithdrawCallOrigin::Runtime,
			)?;
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::set_erc20_peg_address())]
		/// Set the ERC20 peg contract address on Ethereum (requires governance)
		pub fn set_erc20_peg_address(
			origin: OriginFor<T>,
			eth_address: EthAddress,
		) -> DispatchResult {
			ensure_root(origin)?;
			ContractAddress::<T>::put(eth_address);
			Self::deposit_event(Event::<T>::SetContractAddress { address: eth_address });
			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::set_root_peg_address())]
		/// Set the ROOT peg contract address on Ethereum (requires governance)
		pub fn set_root_peg_address(
			origin: OriginFor<T>,
			eth_address: EthAddress,
		) -> DispatchResult {
			ensure_root(origin)?;
			RootPegContractAddress::<T>::put(eth_address);
			Self::deposit_event(Event::<T>::SetRootPegContract { address: eth_address });
			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::set_erc20_meta())]
		/// Set the metadata details for a given ERC20 address (requires governance)
		/// details: `[(contract address, symbol, decimals)]`
		pub fn set_erc20_meta(
			origin: OriginFor<T>,
			details: Vec<(EthAddress, BoundedVec<u8, T::StringLimit>, u8)>,
		) -> DispatchResult {
			ensure_root(origin)?;
			for (address, symbol, decimals) in details {
				Erc20Meta::<T>::insert(address, (symbol, decimals));
			}
			Ok(())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::set_erc20_asset_map())]
		/// Sets the mapping for an asset to an ERC20 address (requires governance)
		/// Sets both Erc20ToAssetId and AssetIdToErc20
		pub fn set_erc20_asset_map(
			origin: OriginFor<T>,
			asset_id: AssetId,
			eth_address: EthAddress,
		) -> DispatchResult {
			ensure_root(origin)?;
			Erc20ToAssetId::<T>::insert(eth_address, asset_id);
			AssetIdToErc20::<T>::insert(asset_id, eth_address);
			Ok(())
		}

		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::set_payment_delay())]
		/// Sets the payment delay for a given AssetId
		pub fn set_payment_delay(
			origin: OriginFor<T>,
			asset_id: AssetId,
			min_balance: Balance,
			delay: BlockNumberFor<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			PaymentDelay::<T>::insert(asset_id, (min_balance, delay));
			Self::deposit_event(Event::<T>::PaymentDelaySet { asset_id, min_balance, delay });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
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
	) -> Result<Option<u64>, DispatchError> {
		ensure!(WithdrawalsActive::<T>::get(), Error::<T>::WithdrawalsPaused);

		// there should be a known ERC20 address mapped for this asset
		// otherwise there may be no liquidity on the Ethereum side of the peg
		let token_address = AssetIdToErc20::<T>::get(asset_id);
		ensure!(token_address.is_some(), Error::<T>::UnsupportedAsset);
		let token_address = token_address.unwrap();

		let message = WithdrawMessage { token_address, amount: amount.into(), beneficiary };

		// Check if there is a delay on the asset
		let payment_delay: Option<(Balance, BlockNumberFor<T>)> = PaymentDelay::<T>::get(asset_id);
		if WithdrawalsDelayActive::<T>::get() {
			if let Some((min_amount, delay)) = payment_delay {
				if min_amount <= amount {
					return match call_origin {
						WithdrawCallOrigin::Runtime => {
							// Delay the payment
							let _imbalance = Self::burn_or_transfer(asset_id, &origin, amount)?;
							Self::delay_payment(
								delay,
								PendingPayment::Withdrawal((origin.clone(), message)),
								asset_id,
								origin,
							);
							Ok(None)
						},
						WithdrawCallOrigin::Evm => {
							// EVM payment delays are not supported
							Err(Error::<T>::EvmWithdrawalFailed.into())
						},
					};
				}
			};
		}

		// Process transfer or withdrawal of payment asset
		let _imbalance = Self::burn_or_transfer(asset_id, &origin, amount)?;
		Self::process_withdrawal(origin.clone(), message, asset_id)
	}

	/// For a withdrawal, either transfer ROOT tokens to Peg address or burn all other tokens
	fn burn_or_transfer(
		asset_id: AssetId,
		origin: &T::AccountId,
		amount: Balance,
	) -> DispatchResult {
		if asset_id == T::NativeAssetId::get() {
			// transfer all ROOT tokens to the peg address
			let pallet_address: T::AccountId = T::PegPalletId::get().into_account_truncating();
			T::MultiCurrency::transfer(
				asset_id,
				origin,
				&pallet_address,
				amount,
				Preservation::Expendable,
			)?;
		} else {
			// burn all other tokens
			T::MultiCurrency::burn_from(
				asset_id,
				origin,
				amount,
				Precision::Exact,
				Fortitude::Polite,
			)?;
		}
		Ok(())
	}

	/// Process withdrawal and send
	fn process_withdrawal(
		sender: T::AccountId,
		withdrawal_message: WithdrawMessage,
		asset_id: AssetId,
	) -> Result<Option<u64>, DispatchError> {
		let source: T::AccountId = T::PegPalletId::get().into_account_truncating();
		let message = ethabi::encode(&[
			Token::Address(withdrawal_message.token_address),
			Token::Uint(withdrawal_message.amount.into()),
			Token::Address(withdrawal_message.beneficiary),
		]);

		// Call whatever handler loosely coupled from ethy
		let event_proof_id = if asset_id == T::NativeAssetId::get() {
			// Call with ROOT contract address
			T::EthBridge::send_event(&source.into(), &RootPegContractAddress::<T>::get(), &message)?
		} else {
			// Call with ERC20Peg contract address
			T::EthBridge::send_event(&source.into(), &ContractAddress::<T>::get(), &message)?
		};

		Self::deposit_event(Event::<T>::Erc20Withdraw {
			asset_id,
			amount: withdrawal_message.amount.saturated_into(),
			beneficiary: withdrawal_message.beneficiary,
			source: sender,
		});
		Ok(Some(event_proof_id))
	}

	/// Process payments at a block after a delay
	fn process_delayed_payment(payment_id: DelayedPaymentId) {
		if let Some(pending_payment) = DelayedPayments::<T>::take(payment_id) {
			match pending_payment {
				PendingPayment::Deposit(deposit) => {
					if Self::process_deposit(deposit.clone()).is_err() {
						Self::deposit_event(Event::<T>::DelayedErc20DepositFailed {
							payment_id,
							beneficiary: deposit.beneficiary.into(),
						});
					}
				},
				PendingPayment::Withdrawal((source, withdrawal_message)) => {
					// At this stage it is assumed that a mapping between erc20 to asset id exists
					// for this token
					let asset_id = Erc20ToAssetId::<T>::get(withdrawal_message.token_address);
					if let Some(asset_id) = asset_id {
						// Process transfer or withdrawal of payment asset
						if Self::process_withdrawal(source, withdrawal_message.clone(), asset_id)
							.is_err()
						{
							Self::deposit_event(Event::<T>::DelayedErc20WithdrawalFailed {
								asset_id,
								beneficiary: withdrawal_message.beneficiary.into(),
							});
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
	pub fn delay_payment(
		delay: BlockNumberFor<T>,
		pending_payment: PendingPayment,
		asset_id: AssetId,
		source: T::AccountId,
	) {
		let payment_id = NextDelayedPaymentId::<T>::get();
		if !payment_id.checked_add(One::one()).is_some() {
			Self::deposit_event(Event::<T>::NoAvailableDelayedPaymentIds);
			return;
		}
		let payment_block = <frame_system::Pallet<T>>::block_number().saturating_add(delay);
		DelayedPayments::<T>::insert(payment_id, &pending_payment);
		NextDelayedPaymentId::<T>::put(payment_id + 1);
		let _ = DelayedPaymentSchedule::<T>::try_append(payment_block, payment_id).map_err(|_| {
			// If we fail to append the payment_id to the schedule, log the error and throw an event
			log::error!(
				"ERC20-Peg: 📌 Failed to add delayed payment to DelayedPaymentSchedule: {:?}",
				payment_id
			);
			Self::deposit_event(Event::<T>::Erc20DelayFailed {
				payment_id,
				scheduled_block: payment_block,
				asset_id,
				source,
			});
		});

		// Throw event for delayed payment
		match pending_payment {
			PendingPayment::Withdrawal((_sender, withdrawal)) => {
				Self::deposit_event(Event::<T>::Erc20WithdrawalDelayed {
					payment_id,
					scheduled_block: payment_block,
					amount: withdrawal.amount.as_u128(),
					beneficiary: withdrawal.beneficiary,
					asset_id,
					source,
				});
			},
			PendingPayment::Deposit(deposit) => {
				let beneficiary: T::AccountId =
					T::AccountId::decode(&mut &deposit.beneficiary.0[..]).unwrap();
				Self::deposit_event(Event::<T>::Erc20DepositDelayed {
					payment_id,
					scheduled_block: payment_block,
					amount: deposit.amount.as_u128(),
					beneficiary,
					asset_id,
				});
			},
		}
	}

	/// Deposit received from bridge, do pre flight checks
	/// If the token has a delay and the amount is above the delay amount, add this deposit to
	/// pending
	pub fn do_deposit(source: &H160, deposit_event: Erc20DepositEvent) -> DispatchResult {
		ensure!(DepositsActive::<T>::get(), Error::<T>::DepositsPaused);
		// fail a deposit early for an amount that is too large
		ensure!(deposit_event.amount < U256::from(Balance::max_value()), Error::<T>::InvalidAmount);

		let asset_id = Erc20ToAssetId::<T>::get(deposit_event.token_address);
		if asset_id.is_some() {
			let asset_id = asset_id.unwrap();
			if asset_id == T::NativeAssetId::get() {
				// If this is the root token, check it comes from the root peg contract address
				ensure!(
					source == &RootPegContractAddress::<T>::get(),
					Error::<T>::InvalidSourceAddress
				);
			} else {
				// If this is not a root token, check it comes from the erc20peg contract address
				ensure!(source == &ContractAddress::<T>::get(), Error::<T>::InvalidSourceAddress);
			}
			// Asset exists, check if there are delays on this deposit
			let payment_delay: Option<(Balance, BlockNumberFor<T>)> =
				PaymentDelay::<T>::get(asset_id);
			if DepositsDelayActive::<T>::get() {
				if let Some((min_amount, delay)) = payment_delay {
					if U256::from(min_amount) <= deposit_event.amount {
						Self::delay_payment(
							delay,
							PendingPayment::Deposit(deposit_event.clone()),
							asset_id,
							(*source).into(),
						);
						return Ok(());
					}
				};
			}
		}
		// process deposit immediately
		Self::process_deposit(deposit_event)
	}

	/// fulfill a deposit for the given event
	/// Handles mint and asset creation
	pub fn process_deposit(verified_event: Erc20DepositEvent) -> DispatchResult {
		let asset_id = match Erc20ToAssetId::<T>::get(verified_event.token_address) {
			None => {
				// create asset with known values from `Erc20Meta`
				// asset will be created with `18` decimal places and "" for symbol if the asset is
				// unknown dapps can also use `AssetToERC20` to retrieve the appropriate decimal
				// places from ethereum
				let (symbol, decimals) = Erc20Meta::<T>::get(verified_event.token_address)
					.unwrap_or((Default::default(), 18));

				let pallet_id = T::PegPalletId::get().into_account_truncating();
				let asset_id = T::MultiCurrency::create_with_metadata(
					&pallet_id,
					// TODO: We may want to accept a name as input as well later. For now, we will
					// use the symbol for both symbol and name
					symbol.clone().into_inner(),
					symbol.into_inner(),
					decimals,
					None,
				)
				.map_err(|_| Error::<T>::CreateAssetFailed)?;

				Erc20ToAssetId::<T>::insert(verified_event.token_address, asset_id);
				AssetIdToErc20::<T>::insert(asset_id, verified_event.token_address);
				asset_id
			},
			Some(asset_id) => asset_id,
		};

		// checked at the time of initiating the verified_event that beneficiary value is valid and
		// this op will not fail qed.
		let beneficiary: T::AccountId = verified_event.beneficiary.into();
		// Asserted prior
		let amount = verified_event.amount.as_u128();
		// Give tokens to user
		Self::mint_or_transfer(asset_id, &beneficiary, amount)?;

		Self::deposit_event(Event::<T>::Erc20Deposit { asset_id, amount, beneficiary });
		Ok(())
	}

	/// For a deposit, either transfer ROOT tokens from Peg address or mint all other tokens
	fn mint_or_transfer(
		asset_id: AssetId,
		beneficiary: &T::AccountId,
		amount: Balance,
	) -> DispatchResult {
		if asset_id == T::NativeAssetId::get() {
			// Transfer all ROOT tokens from the peg address
			let pallet_address: T::AccountId = T::PegPalletId::get().into_account_truncating();
			T::MultiCurrency::transfer(
				asset_id,
				&pallet_address,
				beneficiary,
				amount,
				Preservation::Expendable,
			)?;
		} else {
			// Mint all other tokens
			T::MultiCurrency::mint_into(asset_id, beneficiary, amount)?;
		}
		Ok(())
	}
}

pub struct GetContractAddress<T>(PhantomData<T>);

impl<T: Config> Get<H160> for GetContractAddress<T> {
	fn get() -> H160 {
		ContractAddress::<T>::get()
	}
}

impl<T: Config> EthereumEventSubscriber for Pallet<T> {
	type Address = T::PegPalletId;
	type SourceAddress = GetContractAddress<T>;

	/// Verifies the source address with either the erc20Peg contract address
	/// Or the RootPeg contract address
	fn verify_source(source: &H160) -> OnEventResult {
		let erc20_peg_contract_address: H160 = Self::SourceAddress::get();
		let root_peg_contract_address: H160 = RootPegContractAddress::<T>::get();
		if source == &erc20_peg_contract_address || source == &root_peg_contract_address {
			Ok(DbWeight::get().reads(2u64))
		} else {
			Err((
				DbWeight::get().reads(2u64),
				DispatchError::Other("Invalid source address").into(),
			))
		}
	}

	fn on_event(source: &H160, data: &[u8]) -> OnEventResult {
		let abi_decoded = match ethabi::decode(
			&[ParamType::Address, ParamType::Uint(128), ParamType::Address],
			data,
		) {
			Ok(abi) => abi,
			Err(_) => return Err((Weight::zero(), Error::<T>::InvalidAbiEncoding.into())),
		};

		if let &[Token::Address(token_address), Token::Uint(amount), Token::Address(beneficiary)] =
			abi_decoded.as_slice()
		{
			let token_address: H160 = token_address.into();
			let amount: U256 = amount.into();
			let beneficiary: H160 = beneficiary.into();
			// The total weight of do_deposit assuming it reaches every path
			let deposit_weight = DbWeight::get().reads(6u64) + DbWeight::get().writes(4u64);
			match Self::do_deposit(source, Erc20DepositEvent { token_address, amount, beneficiary })
			{
				Ok(_) => Ok(deposit_weight),
				Err(e) => {
					Self::deposit_event(Event::<T>::Erc20DepositFail {
						source: *source,
						abi_data: data.to_vec(),
					});
					Err((deposit_weight, e.into()))
				},
			}
		} else {
			// input data should be valid, we do not expect to fail here
			Err((Weight::zero(), Error::<T>::InvalidAbiEncoding.into()))
		}
	}
}
