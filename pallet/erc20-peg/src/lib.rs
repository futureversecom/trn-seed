/* Copyright 2021 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure, log,
	pallet_prelude::*,
	traits::{ExistenceRequirement, Get, IsType, Len, WithdrawReasons, fungibles},
	transactional,
	weights::constants::RocksDbWeight as DbWeight,
	PalletId,
	inherent::Vec
};
use frame_support::traits::fungibles::{Mutate, Transfer};
use frame_system::pallet_prelude::*;
use seed_pallet_common::{IsTokenOwner, OnTransferSubscriber, EventClaimVerifier, EventClaimSubscriber, EthAbiCodec};
use seed_primitives::{AssetId, Balance, TokenId, EthAddress};
use sp_runtime::{
	traits::{AccountIdConversion, Hash, One, Saturating, Zero},
	DispatchError, DispatchResult, SaturatedConversion
};
use sp_core::{H256, U256, H160};

mod types;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	// use frame_support::traits::tokens::AssetId;
	use seed_primitives::{AssetId, Balance};

	use seed_pallet_common::CreateExt;
	use seed_primitives::EventId;
	// use sp_core::{H256, U256};
	use crate::types::{ClaimId, EthAddress, PendingClaim};
	use super::*;
	use types::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: frame_system::Config {
		/// An onchain address for this pallet
		type PegPalletId: Get<PalletId>;
		/// The EVM event signature of a deposit
		type DepositEventSignature: Get<[u8; 32]>;
		/// Submits event claims for Ethereum
		type EthBridge: EventClaimVerifier;
		/// Currency functions
		// type MultiCurrency: MultiCurrency<AccountId = Self::AccountId, Balance = Balance, CurrencyId = AssetId>;
		type MultiCurrency: CreateExt<AccountId = Self::AccountId>
			+ fungibles::Transfer<Self::AccountId, Balance = Balance>
			+ fungibles::Inspect<Self::AccountId, AssetId = AssetId>
			+ fungibles::Transfer<Self::AccountId, AssetId = AssetId, Balance = Balance>
			+ fungibles::Mutate<Self::AccountId>; 

			// type MultiCurrency: CreateExt<AccountId = Self::AccountId>
			// + fungibles::Transfer<Self::AccountId, Balance = Balance>
			// + fungibles::Inspect<Self::AccountId, AssetId = AssetId>
			// + fungibles::Mutate<Self::AccountId>;

		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Max length of metadata for erc20 tokens
		type MaxLengthErc20Meta: Get<u32>;
		type MaxClaimsPerBlock: Get<u32>;
		/// Max amount of blocks with claims
		type MaxReadyBlocks: Get<u32>;
		type MaxInitialErcMetas: Get<u8>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An erc20 deposit claim has started. (deposit Id, sender)
		Erc20Claim(u64, T::AccountId),
		/// An erc20 claim has been delayed.(claim_id, scheduled block, amount, beneficiary)
		Erc20DepositDelayed(ClaimId, T::BlockNumber, Balance, T::AccountId),
		/// A withdrawal has been delayed.(claim_id, scheduled block, amount, beneficiary)
		Erc20WithdrawalDelayed(ClaimId, T::BlockNumber, Balance, EthAddress),
		/// A delayed erc20 deposit claim has failed (tx_hash, beneficiary)
		DelayedErc20DepositFailed(H256, T::AccountId),
		/// A delayed erc20 withdrawal has failed (asset_id, beneficiary)
		DelayedErc20WithdrawalFailed(AssetId, EthAddress),
		/// A bridged erc20 deposit succeeded.(deposit Id, asset, amount, beneficiary)
		Erc20Deposit(u64, AssetId, Balance, T::AccountId),
		/// Tokens were burnt for withdrawal on Ethereum as ERC20s (withdrawal Id, asset, amount, beneficiary)
		Erc20Withdraw(u64, AssetId, Balance, EthAddress),
		/// A bridged erc20 deposit failed.(deposit Id)
		Erc20DepositFail(u64),
		/// The peg contract address has been set
		SetContractAddress(EthAddress),
		/// A delay was added for an asset_id (asset_id, min_balance, delay)
		ClaimDelaySet(AssetId, Balance, T::BlockNumber),
		/// There are no more claim ids available, they've been exhausted
		NoAvailableClaimIds,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Could not create the bridged asset
		CreateAssetFailed,
		/// Claim has bad account
		InvalidAddress,
		/// Claim has bad amount
		InvalidAmount,
		/// Deposits are inactive
		DepositsPaused,
		/// Withdrawals are inactive
		WithdrawalsPaused,
		/// Withdrawals of this asset are not supported
		UnsupportedAsset,
		/// Withdrawals over the set claim delay for EVM calls are disabled
		EvmWithdrawalFailed,
		/// Could not convert pallet id to account
		InvalidPalletId
	}

	/// Whether deposit are active
	#[pallet::storage]
	#[pallet::getter(fn deposits_active)]
	pub(super) type DepositsActive<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Whether withdrawals are active
	#[pallet::storage]
	#[pallet::getter(fn withdrawals_active)]
	pub(super) type WithdrawalsActive<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Map ERC20 address to GA asset Id
	#[pallet::storage]
	#[pallet::getter(fn erc20_to_asset)]
	pub(super) type Erc20ToAssetId<T: Config> = StorageMap<_, Twox64Concat, EthAddress, AssetId, OptionQuery>;

	/// Map GA asset Id to ERC20 address
	#[pallet::storage]
	#[pallet::getter(fn asset_to_erc20)]
	pub type AssetIdToErc20<T: Config> = StorageMap<_, Twox64Concat, AssetId, EthAddress, OptionQuery>;

	/// Metadata for well-known erc20 tokens (symbol, decimals)
	#[pallet::storage]
	#[pallet::getter(fn erc20_meta)]
	pub(super) type Erc20Meta<T: Config> = StorageMap<_, Twox64Concat, EthAddress, (BoundedVec<u8, T::MaxLengthErc20Meta>, u8), OptionQuery>;

	/// Map from asset_id to minimum amount and delay
	#[pallet::storage]
	#[pallet::getter(fn claim_delay)]
	pub(super) type ClaimDelay<T: Config> = StorageMap<_, Twox64Concat, AssetId, (Balance, T::BlockNumber), OptionQuery>;

	/// Map from claim id to claim
	#[pallet::storage]
	#[pallet::getter(fn delayed_claims)]
	pub(super) type DelayedClaims<T: Config> = StorageMap<_, Twox64Concat, ClaimId, PendingClaim, OptionQuery>;

	/// Map from block number to claims scheduled for that block
	#[pallet::storage]
	#[pallet::getter(fn delayed_claim_schedule)]
	pub(super) type DelayedClaimSchedule<T: Config> = StorageMap<_, Twox64Concat, T::BlockNumber, BoundedVec<ClaimId, T::MaxClaimsPerBlock>>;

	/// The blocks with claims that are ready to be processed
	#[pallet::storage]
	#[pallet::getter(fn ready_blocks)]
	pub(super) type ReadyBlocks<T: Config> = StorageValue<_, BoundedVec<T::BlockNumber, T::MaxReadyBlocks>, ValueQuery>;

	/// The next available claim id for withdrawals and deposit claims
	#[pallet::storage]
	#[pallet::getter(fn next_delayed_claim_id)]
	pub type NextDelayedClaimId<T: Config> = StorageValue<_, ClaimId, ValueQuery>;

	/// Hash of withdrawal information
	#[pallet::storage]
	#[pallet::getter(fn withdrawal_digests)]
	pub type WithdrawalDigests<T> = StorageMap<_, Twox64Concat, EventId, <T as frame_system::Config>::Hash>;

	/// The peg contract address on Ethereum
	#[pallet::storage]
	#[pallet::getter(fn contract_address)]
	pub type ContractAddress<T> = StorageValue<_, EthAddress, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub erc20s: Vec<(EthAddress, BoundedVec<u8, T::MaxLengthErc20Meta>, u8)>
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for (address, symbol, decimals) in self.erc20s.iter() {
				Erc20Meta::<T>::insert::<&H160, (BoundedVec<u8, T::MaxLengthErc20Meta>, &u8)>(address, (symbol.clone(), decimals));
			}
		}
	}

	#[cfg(feature = "std")]
	impl<T:Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { erc20s: Vec::new() }
		}
	}



#[pallet::hooks]
impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
	fn on_initialize(now: T::BlockNumber) -> Weight {
		let mut weight: Weight = DbWeight::get().reads(1 as Weight);
		if DelayedClaimSchedule::<T>::contains_key(now) {
			ReadyBlocks::<T>::try_mutate(|v| {
				v.try_push(now)
			});
			weight = weight.saturating_add(DbWeight::get().writes(1 as Weight));
		}
		weight as Weight
	}

	/// Check and process outstanding claims
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

		// Process as many claims as we can
		let weight_each: Weight = DbWeight::get().reads(8 as Weight).saturating_add(DbWeight::get().writes(10 as Weight));
		let max_claims = ((remaining_weight - initial_read_cost) / weight_each).saturated_into::<u8>();
		let ready_blocks: Vec<T::BlockNumber> = Self::ready_blocks().into_inner();
		// Total claims processed in this block
		let mut processed_claim_count: u8 = 0;
		// Count of blocks where all claims have been processed
		let mut processed_block_count: u8 = 0;

		for block in ready_blocks.iter() {
			if let Some(claim_ids) = DelayedClaimSchedule::<T>::take(block) {
				let remaining_claims = (max_claims - processed_claim_count) as usize;

				if claim_ids.clone().len() > remaining_claims {
					// Update storage with unprocessed claims

					// TODO: Handle unwrap
					let new: BoundedVec<ClaimId, T::MaxClaimsPerBlock> = claim_ids.clone().into_inner().split_off(remaining_claims).try_into().unwrap();

					DelayedClaimSchedule::<T>::insert::<&<T as frame_system::Config>::BlockNumber, _>(block, new);
				} else {
					processed_block_count += 1;
				}
				processed_claim_count += claim_ids.len() as u8;
				// Process remaining claims from block
				for claim_id in claim_ids {
					Self::process_claim(claim_id);
				}
				if processed_claim_count >= max_claims {
					break;
				}
			}
		}

		let placement: BoundedVec<T::BlockNumber, T::MaxReadyBlocks> = ready_blocks[processed_block_count as usize..].to_vec().try_into().unwrap();
		ReadyBlocks::<T>::put(placement);
		initial_read_cost + weight_each * processed_claim_count as Weight
	}	
}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Activate/deactivate deposits (root only)
		#[pallet::weight(10_000_000)]
		pub fn activate_deposits(origin: OriginFor<T>, activate: bool) -> DispatchResult {
			ensure_root(origin)?;
			DepositsActive::<T>::put(activate);
			Ok(())
		}
	
		/// Activate/deactivate withdrawals (root only)
		#[pallet::weight(10_000_000)]
		pub fn activate_withdrawals(origin: OriginFor<T>, activate: bool)  -> DispatchResult {
			ensure_root(origin)?;
			WithdrawalsActive::<T>::put(activate);
			Ok(())
		}
	
		#[pallet::weight(60_000_000)]
		/// Submit deposit claim for an ethereum tx hash
		/// The deposit details must be provided for cross-checking by notaries
		/// Any caller may initiate a claim while only the intended beneficiary will be paid.
		#[transactional]
		pub fn deposit_claim(origin: OriginFor<T>, tx_hash: H256, claim: Erc20DepositEvent) -> DispatchResult  {
			// Note: require caller to provide the `claim` so we don't need to handle the-
			// complexities of notaries reporting differing deposit events
			let _origin = ensure_signed(origin)?;
			ensure!(Self::deposits_active(), Error::<T>::DepositsPaused);
			// fail a claim early for an amount that is too large
			ensure!(claim.amount < U256::from(u128::max_value()), Error::<T>::InvalidAmount);
			// fail a claim if beneficiary is not a valid CENNZnet address
			ensure!(T::AccountId::decode(&mut &claim.beneficiary.0[..]).is_ok(), Error::<T>::InvalidAddress);

			if let Some(asset_id) = Self::erc20_to_asset(claim.token_address) {
				let claim_delay: Option<(Balance, T::BlockNumber)> = Self::claim_delay(asset_id);

				if let Some((min_amount, delay)) = claim_delay {
					if U256::from(min_amount) <= claim.amount {
						Self::delay_claim(delay, PendingClaim::Deposit((claim.clone(), tx_hash)));
						return Ok(());
					}
				};
			}
			// process deposit immediately
			Self::process_deposit_claim(claim, tx_hash);
			Ok(())
		}
	
		#[pallet::weight(60_000_000)]
		/// Withdraw generic assets from CENNZnet in exchange for ERC20s
		/// Tokens will be transferred to peg account and a proof generated to allow redemption of tokens on Ethereum
		#[transactional]
		pub fn withdraw(origin: OriginFor<T>, asset_id: AssetId, amount: Balance, beneficiary: EthAddress) -> DispatchResult  {
			let origin = ensure_signed(origin)?;
			Self::do_withdrawal(origin, asset_id, amount, beneficiary, WithdrawCallOrigin::Runtime)?;
			Ok(())
		}
	
		#[pallet::weight(1_000_000)]
		#[transactional]
		/// Set the peg contract address on Ethereum (requires governance)
		pub fn set_contract_address(origin: OriginFor<T>, eth_address: EthAddress) -> DispatchResult {
			ensure_root(origin)?;
			ContractAddress::<T>::put(eth_address);
			Self::deposit_event(<Event<T>>::SetContractAddress(eth_address));
			Ok(())
		}
	
		#[pallet::weight({
			1_000_000 * details.len() as u64
		})]
		/// Set the metadata details for a given ERC20 address (requires governance)
		/// details: `[(contract address, symbol, decimals)]`
		pub fn set_erc20_meta(origin: OriginFor<T>, details: Vec<(EthAddress, BoundedVec<u8, T::MaxLengthErc20Meta>, u8)>)-> DispatchResult{
			ensure_root(origin)?;
			for (address, symbol, decimals) in details {
				Erc20Meta::<T>::insert(address, (symbol, decimals));
			}
			Ok(())
		}
	
		#[pallet::weight(1_000_000)]
		/// Sets the claim delay for a given AssetId
		pub fn set_claim_delay(origin: OriginFor<T>, asset_id: AssetId, min_balance: Balance, delay: T::BlockNumber) -> DispatchResult{
			ensure_root(origin)?;
			ClaimDelay::<T>::insert(asset_id, (min_balance, delay));
			Self::deposit_event(<Event<T>>::ClaimDelaySet(asset_id, min_balance, delay));
			Ok(())
		}
	}
	

	impl<T: Config> Pallet<T> {
		/// Process the withdrawal, returning the event_proof_id
		/// Can be called by the runtime or erc20-peg precompile
		/// If a claim delay is in place for the asset, this will be handled when called from the runtime
		/// The runtime doesn't use the returned value so 0 is returned in this case
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
	
			let message = WithdrawMessage {
				token_address,
				amount: amount.into(),
				beneficiary,
			};
	
			// Check if there is a delay on the asset
			let claim_delay: Option<(Balance, T::BlockNumber)> = Self::claim_delay(asset_id);
			if let Some((min_amount, delay)) = claim_delay {
				if min_amount <= amount {
					return match call_origin {
						WithdrawCallOrigin::Runtime => {
							// Process transfer or withdrawal of payment asset
							Self::process_withdrawal_payment(origin, asset_id, amount)?;
							// Delay the claim
							Self::delay_claim(delay, PendingClaim::Withdrawal(message));
							Ok(0)
						}
						WithdrawCallOrigin::Evm => {
							// EVM claim delays are not supported
							Err(Error::<T>::EvmWithdrawalFailed.into())
						}
					};
				}
			};
	
			// Process transfer or withdrawal of payment asset
			Self::process_withdrawal_payment(origin, asset_id, amount)?;
			// process withdrawal immediately
			Self::process_withdrawal(message, asset_id)
		}
	
		fn process_withdrawal_payment(
			origin: T::AccountId,
			asset_id: AssetId,
			amount: Balance,
		) -> Result<(), DispatchError> {
			use frame_support::traits::fungibles::Transfer;
			let pallet_id = T::PegPalletId::get()
				.try_into_account()
				.ok_or(Error::<T>::InvalidPalletId)?;

			// if asset_id == T::MultiCurrency::staking_currency() {
			// 	let _result = T::MultiCurrency::transfer(
			// 		&origin,
			// 		&pallet_id,
			// 		asset_id,
			// 		amount, // checked amount < u128 in `deposit_claim` qed.
			// 		ExistenceRequirement::KeepAlive,
			// 	)?;
			// } else {
			let _imbalance = T::MultiCurrency::burn_from(
				// &origin,
				asset_id,
				&origin,
				amount,
				// WithdrawReasons::TRANSFER,
				// frame_support::traits::ExistenceRequirement::KeepAlive,
			)?;
			// }
			Ok(())
		}
	
		/// Process claims at a block after a delay
		fn process_claim(claim_id: ClaimId) {
			if let Some(pending_claim) = DelayedClaims::<T>::take(claim_id) {
				match pending_claim {
					PendingClaim::Deposit((deposit_claim, tx_hash)) => {
						Self::process_deposit_claim(deposit_claim, tx_hash);
					}
					PendingClaim::Withdrawal(withdrawal_message) => {
						// At this stage it is assumed that a mapping between erc20 to asset id exists for this token
						let asset_id = Self::erc20_to_asset(withdrawal_message.token_address);
						if let Some(asset_id) = asset_id {
							let _ = Self::process_withdrawal(withdrawal_message, asset_id);
						} else {
							log::error!(
								"📌 ERC20 withdrawal claim failed unexpectedly: {:?}",
								withdrawal_message
							);
						}
					}
				}
			}
		}
	
		fn process_deposit_claim(claim: Erc20DepositEvent, tx_hash: H256) {
			let event_claim_id = T::EthBridge::submit_event_claim(
				&Self::contract_address().into(),
				&T::DepositEventSignature::get().into(),
				&tx_hash,
				&EthAbiCodec::encode(&claim),
			);
			let beneficiary: T::AccountId = T::AccountId::decode(&mut &claim.beneficiary.0[..]).unwrap();
			match event_claim_id {
				Ok(claim_id) => Self::deposit_event(<Event<T>>::Erc20Claim(claim_id, beneficiary)),
				Err(_) => Self::deposit_event(<Event<T>>::DelayedErc20DepositFailed(tx_hash, beneficiary)),
			}
		}
	
		fn process_withdrawal(message: WithdrawMessage, asset_id: AssetId) -> Result<u64, DispatchError> {
			let amount: Balance = message.amount.as_u128();
			let event_proof_id = T::EthBridge::generate_event_proof(&message);
	
			match event_proof_id {
				Ok(proof_id) => {
					// Create a hash of withdrawAmount, tokenAddress, receiver, eventId
					let proof_id: EventId = proof_id;
					let withdrawal_hash: T::Hash = T::Hashing::hash(&mut (message.clone(), proof_id).encode());
					WithdrawalDigests::<T>::insert(proof_id, withdrawal_hash);
					Self::deposit_event(<Event<T>>::Erc20Withdraw(
						proof_id,
						asset_id,
						amount,
						message.beneficiary,
					));
				}
				Err(_) => Self::deposit_event(<Event<T>>::DelayedErc20WithdrawalFailed(asset_id, message.beneficiary)),
			}
			event_proof_id
		}
	
		/// Delay a withdrawal or deposit claim until a later block
		pub fn delay_claim(delay: T::BlockNumber, pending_claim: PendingClaim) -> Result<(), ()> {
			let claim_id = Self::next_delayed_claim_id();
			if !claim_id.checked_add(One::one()).is_some() {
				Self::deposit_event(<Event<T>>::NoAvailableClaimIds);
				return Ok(())
			}
			let claim_block = <frame_system::Pallet<T>>::block_number().saturating_add(delay);
			DelayedClaims::<T>::insert(claim_id, &pending_claim);
			// Modify DelayedClaimSchedule with new claim_id
			// DelayedClaimSchedule::<T>::append(claim_block, claim_id);

			DelayedClaimSchedule::<T>::try_mutate(claim_block, |maybe_stored_for_block| {
				if let Some(current) = maybe_stored_for_block {
					current.try_push(claim_id)
				} else {
					// Do nothing
					Ok(())
				}
			})?;

			NextDelayedClaimId::<T>::put(claim_id + 1);
	
			// Throw event for delayed claim
			match pending_claim {
				PendingClaim::Withdrawal(withdrawal) => {
					Self::deposit_event(<Event<T>>::Erc20WithdrawalDelayed(
						claim_id,
						claim_block,
						withdrawal.amount.as_u128(),
						withdrawal.beneficiary,
					));
				}
				PendingClaim::Deposit(deposit) => {
					let beneficiary: T::AccountId = T::AccountId::decode(&mut &deposit.0.beneficiary.0[..]).unwrap();
					Self::deposit_event(<Event<T>>::Erc20DepositDelayed(
						claim_id,
						claim_block,
						deposit.0.amount.as_u128(),
						beneficiary,
					));
				}
			};
			Ok(())
		}
	
		/// fulfil a deposit claim for the given event
		pub fn do_deposit(verified_event: Erc20DepositEvent) -> Result<(AssetId, Balance, T::AccountId), DispatchError> {
			let asset_id = match Self::erc20_to_asset(verified_event.token_address) {
				None => {
					// create asset with known values from `Erc20Meta`
					// asset will be created with `18` decimal places and "" for symbol if the asset is unknown
					// dapps can also use `AssetToERC20` to retrieve the appropriate decimal places from ethereum
					let (symbol, decimals) =
						Erc20Meta::<T>::get(verified_event.token_address).unwrap_or((Default::default(), 18));

					let pallet_id = T::PegPalletId::get()
						.try_into_account()
						.ok_or(Error::<T>::InvalidPalletId)?;

					let asset_id = T::MultiCurrency::create_with_metadata(
						&pallet_id,
						symbol.clone().into_inner(),
						symbol.into_inner(),
						decimals
					)
					.map_err(|_| Error::<T>::CreateAssetFailed)?;
					Erc20ToAssetId::<T>::insert(verified_event.token_address, asset_id);
					AssetIdToErc20::<T>::insert(asset_id, verified_event.token_address);
	
					asset_id
				}
				Some(asset_id) => asset_id,
			};
	
			// checked at the time of initiating the verified_event that beneficiary value is valid and this op will not fail qed.
			let beneficiary: T::AccountId = T::AccountId::decode(&mut &verified_event.beneficiary.0[..]).unwrap();	
			let amount = verified_event.amount.as_u128();

			T::MultiCurrency::mint_into(
				asset_id,
				&beneficiary,
				amount
			);
	
			Ok((asset_id, amount, beneficiary))
		}
	}
}


impl<T: Config> EventClaimSubscriber for Pallet<T> {
	fn on_success(event_claim_id: u64, contract_address: &EthAddress, event_type: &H256, event_data: &[u8]) {
		if *contract_address == EthAddress::from(Self::contract_address())
			&& *event_type == H256::from(T::DepositEventSignature::get())
		{
			if let Some(deposit_event) = EthAbiCodec::decode(event_data) {
				match Self::do_deposit(deposit_event) {
					Ok((asset_id, amount, beneficiary)) => {
						Self::deposit_event(<Event<T>>::Erc20Deposit(event_claim_id, asset_id, amount, beneficiary))
					}
					Err(_err) => Self::deposit_event(<Event<T>>::Erc20DepositFail(event_claim_id)),
				}
			} else {
				// input data should be valid, we do not expect to fail here
				log::error!("📌 ERC20 deposit claim failed unexpectedly: {:?}", event_data);
			}
		}
	}
	fn on_failure(event_claim_id: u64, contract_address: &H160, event_type: &H256, _event_data: &[u8]) {
		if *contract_address == EthAddress::from(Self::contract_address())
			&& *event_type == H256::from(T::DepositEventSignature::get())
		{
			Self::deposit_event(<Event<T>>::Erc20DepositFail(event_claim_id));
		}
	}
}
