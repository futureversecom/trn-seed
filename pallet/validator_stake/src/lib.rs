#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	pallet_prelude::*,
	traits::{Currency, LockIdentifier, LockableCurrency, UnixTime, WithdrawReasons},
};
use frame_system::{ensure_signed, pallet_prelude::*};
pub use pallet::*;
use pallet_validator_set::ValidatorSet;
use seed_primitives::{Balance, ValidatorId};
use sp_runtime::{traits::UniqueSaturatedInto, ArithmeticError};
use sp_std::prelude::*;

mod helpers;
pub mod weights;

type BalanceOf<T> =
	<<T as Config>::Balances as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub const VALIDATOR_STAKING_LOCK_ID: LockIdentifier = *b"vstaking";

pub use weights::WeightInfo;

pub use crate::helpers::Depository;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The balances will be locked on staking
		type Balances: LockableCurrency<
			Self::AccountId,
			Balance = Balance,
			Moment = Self::BlockNumber,
		>;
		type ValidatorSet: ValidatorSet<ValidatorId, AccountIdOf<Self>>;
		/// Unix time
		type UnixTime: UnixTime;
		/// Weight information
		type WeightInfo: WeightInfo;
		/// Minimum account balance (for future use)
		#[pallet::constant]
		type MinAccountBalance: Get<BalanceOf<Self>>;
		/// Minimum stake amount (for future use)
		#[pallet::constant]
		type MinStake: Get<BalanceOf<Self>>;
		/// Minimum unstake amount (for future use)
		#[pallet::constant]
		type MinUnstake: Get<BalanceOf<Self>>;
		/// Minimum hold time (for future use)
		#[pallet::constant]
		type MinHoldTime: Get<BalanceOf<Self>>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Amount Is Less Than Min Stake
		AmountIsLessThanMinStake,
		/// Low Balance
		AmountIsLessThanAvailableBalance,
		/// Amount Is Less Than Min UnStake
		AmountIsLessThanMinUnStake,
		/// Staking Account not found
		StakingAccountNotFound,
		/// The given validator is not active
		InactiveValidator,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The amount is staked successfully
		Staked(BalanceOf<T>),
		/// The amount is unstaked successfully
		Unstaked(BalanceOf<T>),
	}

	#[pallet::storage]
	#[pallet::getter(fn total_staked)]
	/// Total staked (locked) amount (not associated with any given validator)
	pub type TotalStaked<T: Config> = StorageMap<_, Blake2_128Concat, ValidatorId, Balance>;

	#[pallet::storage]
	#[pallet::getter(fn staking_validator_info)]
	/// staking information for a given account (total staked, rewards and timestamp)
	pub type StakingValidatorInfo<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ValidatorId,
		Blake2_128Concat,
		T::AccountId,
		Depository,
	>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Stake the given amount to the given pool.
		/// Parameters:
		/// - `validator_id`: The validator id to which amount will be staked.
		/// - `validator_acc`: The validator account to verify its a registered validator.
		/// - `amount`: The amount to stake.
		#[pallet::weight((<T as Config>::WeightInfo::stake(), DispatchClass::Normal, Pays::No))]
		pub fn stake(
			origin: OriginFor<T>,
			validator_id: ValidatorId,
			validator_acc: AccountIdOf<T>,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			ensure!(
				T::ValidatorSet::is_validator(validator_id, &validator_acc),
				Error::<T>::InactiveValidator
			);

			let sender = ensure_signed(origin)?;

			// Check for the minimum amount to stake
			ensure!(amount >= T::MinStake::get(), Error::<T>::AmountIsLessThanMinStake);

			let available_balance = Self::available_balance(sender.clone(), validator_id)?;
			ensure!(amount <= available_balance, Error::<T>::AmountIsLessThanAvailableBalance);

			Self::stake_amount(sender, validator_id, amount.clone())
				.expect("Error updating Stake Amount for Provider's Account");

			Self::deposit_event(Event::Staked(amount));
			Ok(().into())
		}

		/// Unstake from the given pool.
		/// Parameters:
		/// - `validator_id`: The validator id to which amount will be staked.
		/// - `validator_acc`: The validator account to verify its a registered validator.
		/// - `amount`: The amount to stake.
		#[pallet::weight((<T as Config>::WeightInfo::unstake(), DispatchClass::Normal, Pays::No))]
		pub fn unstake(
			origin: OriginFor<T>,
			validator_id: ValidatorId,
			validator_acc: AccountIdOf<T>,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			ensure!(
				T::ValidatorSet::is_validator(validator_id, &validator_acc),
				Error::<T>::InactiveValidator
			);

			let receiver = ensure_signed(origin)?;

			ensure!(amount > T::MinUnstake::get(), Error::<T>::AmountIsLessThanMinUnStake);

			ensure!(
				StakingValidatorInfo::<T>::contains_key(validator_id, receiver.clone()),
				Error::<T>::StakingAccountNotFound
			);
			Self::deposit_event(Event::Unstaked(amount));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Update Stake Amount
	/// - sender: AccountId from which balances will be locked
	/// - validator: AccountId of the validator
	/// - validator_id: Validator's Id
	/// - amount: Amount of funds to be staked from sender
	pub fn stake_amount(
		sender: AccountIdOf<T>,
		validator_id: ValidatorId,
		amount: BalanceOf<T>,
	) -> DispatchResultWithPostInfo {
		Self::add_to_total_stake(validator_id, amount)?;
		if StakingValidatorInfo::<T>::contains_key(validator_id, sender.clone()) {
			StakingValidatorInfo::<T>::mutate(
				validator_id,
				sender.clone(),
				|info| -> DispatchResultWithPostInfo {
					let info = info.as_mut().ok_or(Error::<T>::StakingAccountNotFound)?;
					info.total = info
						.total
						.checked_add(amount.unique_saturated_into())
						.ok_or(ArithmeticError::Underflow)?;
					info.timestamp = T::UnixTime::now().as_secs();

					// Lock funds
					Self::lock_balances(sender, amount.unique_saturated_into());
					Ok(().into())
				},
			)
		} else {
			let mut info =
				Self::staking_validator_info(validator_id, sender.clone()).unwrap_or_default();

			info.total = amount.unique_saturated_into();
			info.timestamp = T::UnixTime::now().as_secs();
			StakingValidatorInfo::<T>::insert(validator_id, &sender, info);
			Self::lock_balances(sender, amount.unique_saturated_into());
			Ok(().into())
		}
	}
	/// Balances for given account
	pub fn available_balance(
		account: AccountIdOf<T>,
		validator_id: ValidatorId,
	) -> Result<u128, DispatchError> {
		let mut available_balance: u128 =
			T::Balances::free_balance(&account).unique_saturated_into();
		if StakingValidatorInfo::<T>::contains_key(validator_id, account.clone()) {
			let total_staked = StakingValidatorInfo::<T>::get(validator_id, account).unwrap().total;
			available_balance =
				available_balance.checked_sub(total_staked).ok_or(ArithmeticError::Overflow)?;
		}
		available_balance = available_balance
			.checked_sub(T::MinAccountBalance::get().unique_saturated_into())
			.ok_or(ArithmeticError::Overflow)?;
		Ok(available_balance)
	}

	/// Adds to total stake amount for all validators
	pub fn add_to_total_stake(
		validator_id: ValidatorId,
		amount: BalanceOf<T>,
	) -> DispatchResultWithPostInfo {
		if TotalStaked::<T>::get(validator_id) == None {
			let amount_tmp: u128 = amount;
			TotalStaked::<T>::insert(validator_id, amount_tmp);
		} else {
			TotalStaked::<T>::mutate(validator_id, |balance| -> DispatchResultWithPostInfo {
				if balance.is_none() {
					let amount_tmp: u128 = amount.unique_saturated_into();
					TotalStaked::<T>::insert(validator_id, amount_tmp);
				} else {
					let val = balance
						.unwrap()
						.checked_add(amount.unique_saturated_into())
						.ok_or(ArithmeticError::Underflow)?;
					*balance = Option::from(val);
				}
				Ok(().into())
			})?;
		}

		Ok(().into())
	}

	/// Lock stakers funds
	/// - sender: AccountId from which balance will be locked
	/// - amount: Amount of funds to be staked (locked)
	pub fn lock_balances(sender: AccountIdOf<T>, amount: BalanceOf<T>) {
		T::Balances::set_lock(VALIDATOR_STAKING_LOCK_ID, &sender, amount, WithdrawReasons::all());
	}
}
