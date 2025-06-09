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
extern crate alloc;

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod weights;

use alloc::boxed::Box;
use frame_support::{
	dispatch::GetDispatchInfo,
	ensure,
	pallet_prelude::{DispatchError, DispatchResult, *},
	traits::{InstanceFilter, IsSubType, IsType},
	transactional,
};
use frame_system::pallet_prelude::*;
use precompile_utils::constants::FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX;
use seed_pallet_common::{AccountProxy, ExtrinsicChecker, FuturepassProvider};
use sp_core::H160;
use sp_io::hashing::keccak_256;
use sp_runtime::traits::Dispatchable;
use sp_std::{convert::TryInto, vec::Vec};
pub use weights::WeightInfo;

/// The logging target for this pallet
#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "futurepass";

pub trait ProxyProvider<T: Config>
where
	<T as frame_system::Config>::AccountId: From<H160>,
{
	fn exists(
		futurepass: &T::AccountId,
		delegate: &T::AccountId,
		proxy_type: Option<T::ProxyType>,
	) -> bool;
	fn owner(futurepass: &T::AccountId) -> Option<T::AccountId>;
	fn delegates(futurepass: &T::AccountId) -> Vec<(T::AccountId, T::ProxyType)>;
	fn add_delegate(
		funder: &T::AccountId,
		futurepass: &T::AccountId,
		delegate: &T::AccountId,
		proxy_type: &u8,
	) -> DispatchResult;
	fn remove_delegate(
		receiver: &T::AccountId,
		futurepass: &T::AccountId,
		delegate: &T::AccountId,
	) -> DispatchResult;
	fn remove_account(receiver: &T::AccountId, futurepass: &T::AccountId) -> DispatchResult;
	fn proxy_call(
		caller: OriginFor<T>,
		futurepass: T::AccountId,
		call: <T as Config>::RuntimeCall,
	) -> DispatchResult;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config
	where
		<Self as frame_system::Config>::AccountId: From<H160>,
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type Proxy: ProxyProvider<Self>;

		/// The overarching call type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>
			+ IsType<<Self as frame_system::Config>::RuntimeCall>;

		/// Futurepass proxy extrinsic inner call blacklist validator
		type BlacklistedCallValidator: ExtrinsicChecker<
			Call = <Self as pallet::Config>::RuntimeCall,
			Extra = (),
			Result = bool,
		>;

		/// A kind of proxy; specified with the proxy and passed in to the `IsProxyable` filter.
		/// The instance filter determines whether a given call may be proxied under this type.
		///
		/// IMPORTANT: `Default` must be provided and MUST BE the *most permissive* value.
		type ProxyType: Parameter
			+ Member
			+ Ord
			+ PartialOrd
			+ InstanceFilter<<Self as Config>::RuntimeCall>
			+ Default
			+ MaxEncodedLen
			+ Into<u8>;

		/// Interface to access weight values
		type WeightInfo: WeightInfo;

		#[cfg(feature = "runtime-benchmarks")]
		/// Handles a multi-currency fungible asset system for benchmarking.
		type MultiCurrency: frame_support::traits::fungibles::Inspect<
				Self::AccountId,
				AssetId = seed_primitives::AssetId,
			> + frame_support::traits::fungibles::Mutate<Self::AccountId>;
	}

	#[pallet::type_value]
	pub fn DefaultValue() -> u128 {
		1
	}

	/// The next available incrementing futurepass id
	#[pallet::storage]
	pub type NextFuturepassId<T> = StorageValue<_, u128, ValueQuery, DefaultValue>;

	/// Futurepass holders (account -> futurepass)
	#[pallet::storage]
	pub type Holders<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>;

	/// Accounts which have set futurepass as default proxied on-chain account (delegate ->
	/// futurepass)
	#[pallet::storage]
	pub type DefaultProxy<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config>
	where
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		/// Futurepass creation
		FuturepassCreated { futurepass: T::AccountId, delegate: T::AccountId },
		/// Delegate registration to Futurepass account
		DelegateRegistered {
			futurepass: T::AccountId,
			delegate: T::AccountId,
			proxy_type: T::ProxyType,
		},
		/// Delegate unregistration from Futurepass account
		DelegateUnregistered { futurepass: T::AccountId, delegate: T::AccountId },
		/// Futurepass transfer
		FuturepassTransferred {
			old_owner: T::AccountId,
			new_owner: Option<T::AccountId>,
			futurepass: T::AccountId,
		},
		/// A proxy call was executed with the given call
		ProxyExecuted { delegate: T::AccountId, result: DispatchResult },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Account is already futurepass holder
		AccountAlreadyRegistered,
		/// Account is not futurepass delegate
		DelegateNotRegistered,
		/// Account already exists as a delegate
		DelegateAlreadyExists,
		/// Blacklisted extrinsic
		BlacklistedExtrinsic,
		/// Account is not futurepass owner
		NotFuturepassOwner,
		/// Futurepass owner cannot remove themselves
		OwnerCannotUnregister,
		/// Account does not have permission to call this function
		PermissionDenied,
		/// Invalid proxy type
		InvalidProxyType,
		/// ExpiredDeadline
		ExpiredDeadline,
		/// Invalid signature
		InvalidSignature,
		/// AccountParsingFailure
		AccountParsingFailure,
		/// RegisterDelegateSignerMismatch
		RegisterDelegateSignerMismatch,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> where
		<T as frame_system::Config>::AccountId: From<H160>
	{
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		/// Create a futurepass account for the delegator that is able to make calls on behalf of
		/// futurepass.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// Parameters:
		/// - `account`: The delegated account for the futurepass.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create())]
		#[transactional]
		pub fn create(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_create_futurepass(who, account)?;
			Ok(())
		}

		/// Register a delegator to an existing futurepass account given message parameters for a
		/// respective signature. Note: Only futurepass owner account can add more delegates.
		/// Note: The signer is recovered from signature given the message parameters (which is used
		/// to reconstruct the message).
		/// - You can assume the message is constructed like so:
		/// ---
		/// ```solidity
		/// bytes32 message = keccak256(abi.encodePacked(futurepass, delegate, proxyType, deadline));
		/// ethSignedMessage = keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", message));
		/// ```
		/// ---
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// Parameters:
		/// - `futurepass`: Futurepass account to register the account as delegate; 20 bytes.
		/// - `delegate`: The delegated account for the futurepass; 20 bytes.
		/// - `proxy_type`: Delegate permission level; 1 byte.
		/// - `deadline`: Deadline for the signature; 4 bytes.
		/// - `signature`: Signature of the message parameters.
		///
		/// # <weight>
		/// Weight is a function of the number of proxies the user has.
		/// # </weight>
		#[pallet::call_index(1)]
		#[pallet::weight({
			let delegate_count = T::Proxy::delegates(futurepass).len() as u32;
			T::WeightInfo::register_delegate_with_signature(delegate_count)
		})]
		#[transactional]
		pub fn register_delegate_with_signature(
			origin: OriginFor<T>,
			futurepass: T::AccountId,
			delegate: T::AccountId,
			proxy_type: T::ProxyType,
			deadline: u32,
			signature: [u8; 65],
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let is_futurepass = caller == futurepass;

			// For V1 - caller must be futurepass holder or the futurepass
			ensure!(
				is_futurepass || Holders::<T>::get(caller.clone()) == Some(futurepass.clone()),
				Error::<T>::NotFuturepassOwner
			);

			ensure!(
				is_futurepass || T::Proxy::exists(&futurepass, &caller, None),
				Error::<T>::DelegateNotRegistered
			);
			// for V1, only T::ProxyType::default() is allowed.
			// TODO - update the restriction in V2 as required.
			ensure!(proxy_type == T::ProxyType::default(), Error::<T>::PermissionDenied);
			// delegate should not be an existing proxy of any T::ProxyType
			// This is required here coz pallet_proxy's duplicate check is only for the specific
			// proxy_type
			ensure!(
				!T::Proxy::exists(&futurepass, &delegate, None),
				Error::<T>::DelegateAlreadyExists
			);

			let deadline_block_number: BlockNumberFor<T> = deadline.into();
			ensure!(
				deadline_block_number >= frame_system::Pallet::<T>::block_number(),
				Error::<T>::ExpiredDeadline
			);

			let (_, eth_signed_msg) = Self::generate_add_delegate_eth_signed_message(
				&futurepass,
				&delegate,
				&proxy_type,
				&deadline,
			)?;
			let delegate_signer: T::AccountId =
				match sp_io::crypto::secp256k1_ecdsa_recover(&signature, &eth_signed_msg) {
					Ok(pubkey_bytes) => H160(
						keccak_256(&pubkey_bytes)[12..]
							.try_into()
							.map_err(|_| Error::<T>::AccountParsingFailure)?,
					)
					.into(),
					Err(_err) => Err(Error::<T>::InvalidSignature)?,
				};
			ensure!(delegate_signer == delegate, Error::<T>::RegisterDelegateSignerMismatch);

			T::Proxy::add_delegate(&caller, &futurepass, &delegate, &proxy_type.clone().into())?;
			Self::deposit_event(Event::<T>::DelegateRegistered {
				futurepass,
				delegate,
				proxy_type,
			});
			Ok(())
		}

		/// Unregister a delegate from a futurepass account.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// Parameters:
		/// - `futurepass`: Futurepass account to unregister the delegate from.
		/// - `delegate`: The delegated account for the futurepass. Note: if caller is futurepass
		///   holder onwer,
		/// they can remove any delegate (including themselves); otherwise the caller must be the
		/// delegate (can only remove themself).
		///
		/// # <weight>
		/// Weight is a function of the number of proxies the user has.
		/// # </weight>
		#[pallet::call_index(2)]
		#[pallet::weight({
			let delegate_count = T::Proxy::delegates(futurepass).len() as u32;
			T::WeightInfo::unregister_delegate(delegate_count)
		})]
		#[transactional]
		pub fn unregister_delegate(
			origin: OriginFor<T>,
			futurepass: T::AccountId,
			delegate: T::AccountId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			// Check if the caller is the owner of the futurepass
			let is_owner = Holders::<T>::get(&caller) == Some(futurepass.clone());
			let unreg_owner = Holders::<T>::get(&delegate) == Some(futurepass.clone());

			// The owner can not be unregistered
			ensure!(!unreg_owner, Error::<T>::OwnerCannotUnregister);

			// Check if the caller is the owner (can remove anyone) or the futurepass (can remove
			// anyone) or the delegate (can remove themsleves) from the futurepass
			ensure!(
				is_owner || caller == futurepass || caller == delegate,
				Error::<T>::PermissionDenied
			);

			// Check if the delegate is registered with the futurepass
			ensure!(
				T::Proxy::exists(&futurepass, &delegate, None),
				Error::<T>::DelegateNotRegistered
			);

			// Remove the delegate from the futurepass
			T::Proxy::remove_delegate(&caller, &futurepass, &delegate)?;

			Self::deposit_event(Event::<T>::DelegateUnregistered { futurepass, delegate });
			Ok(())
		}

		/// Transfer ownership of a futurepass to a new account.
		/// The new owner must not already own a futurepass.
		/// This removes all delegates from the futurepass.
		/// The new owner will be the only delegate; they can add more delegates.
		///
		/// The dispatch origin for this call must be _Signed_ and must be the current owner of the
		/// futurepass.
		///
		/// Parameters:
		/// - `current_owner`: The current owner of the futurepass.
		/// - `new_owner`: The new account that will become the owner of the futurepass.
		/// # <weight>
		/// Weight is a function of the number of proxies the user has.
		/// # </weight>
		#[pallet::call_index(3)]
		#[pallet::weight({
			match Holders::<T>::get(current_owner) {
				Some(futurepass) => {
					let delegate_count = T::Proxy::delegates(& futurepass).len() as u32;
					T::WeightInfo::transfer_futurepass(delegate_count)
				},
				None => T::WeightInfo::transfer_futurepass(0) // should have passed max value here
			}
		})]
		#[transactional]
		pub fn transfer_futurepass(
			origin: OriginFor<T>,
			current_owner: T::AccountId,
			new_owner: Option<T::AccountId>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			// only succeed if the current_owner has a futurepass account
			let futurepass =
				Holders::<T>::take(&current_owner).ok_or(Error::<T>::NotFuturepassOwner)?;
			if caller != current_owner {
				// if current owner is not the caller; then the caller must be futurepass itself
				ensure!(futurepass == caller.clone(), Error::<T>::NotFuturepassOwner);
			}

			if let Some(ref new_owner) = new_owner {
				// Ensure that the new owner does not already own a futurepass
				ensure!(
					!Holders::<T>::contains_key(new_owner),
					Error::<T>::AccountAlreadyRegistered
				);

				// Add the new owner as a proxy delegate with the most permissive type, i.e.,
				T::Proxy::add_delegate(&caller, &futurepass, new_owner, &255)?; // owner is maxu8

				// Iterate through the list of delegates and remove them, except for the new_owner
				let delegates = T::Proxy::delegates(&futurepass);
				for delegate in delegates.iter() {
					if delegate.0 != *new_owner {
						T::Proxy::remove_delegate(&caller, &futurepass, &delegate.0)?;
					}
				}

				// Set the new owner as the owner of the futurepass
				Holders::<T>::insert(new_owner, futurepass.clone());
			} else {
				// remove the account - which should remove all delegates
				T::Proxy::remove_account(&caller, &futurepass)?;
			}

			Self::deposit_event(Event::<T>::FuturepassTransferred {
				old_owner: current_owner,
				new_owner,
				futurepass,
			});
			Ok(())
		}

		/// Dispatch the given call through Futurepass account. Transaction fees will be paid by the
		/// Futurepass. The dispatch origin for this call must be _Signed_
		///
		/// Parameters:
		/// - `futurepass`: The Futurepass account though which the call is dispatched
		/// - `call`: The Call that needs to be dispatched through the Futurepass account
		///
		/// # <weight>
		/// Weight is a function of the number of proxies the user has.
		/// # </weight>
		#[pallet::call_index(4)]
		#[pallet::weight({
			let di = call.get_dispatch_info();
			let delegate_count = T::Proxy::delegates(futurepass).len() as u32;
			(T::WeightInfo::proxy_extrinsic(delegate_count)
				.saturating_add(di.weight)
				 // AccountData for inner call origin accountdata.
				.saturating_add(T::DbWeight::get().reads_writes(1, 1)),
			di.class)
		})]
		pub fn proxy_extrinsic(
			origin: OriginFor<T>,
			futurepass: T::AccountId,
			call: Box<<T as Config>::RuntimeCall>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			// disallow blacklisted extrinsics
			ensure!(
				!<T as pallet::Config>::BlacklistedCallValidator::check_extrinsic(&call, &()),
				Error::<T>::BlacklistedExtrinsic,
			);

			// restrict delegate access to whitelist
			match call.is_sub_type() {
				Some(Call::register_delegate_with_signature { .. })
				| Some(Call::unregister_delegate { .. })
				| Some(Call::transfer_futurepass { .. }) => {
					ensure!(
						Holders::<T>::get(who.clone()) == Some(futurepass.clone()),
						Error::<T>::NotFuturepassOwner
					);
				},
				_ => {},
			}

			let result = T::Proxy::proxy_call(origin, futurepass, *call);
			Self::deposit_event(Event::ProxyExecuted { delegate: who, result: result.map(|_| ()) });
			result
		}
	}
}

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<H160>,
{
	/// Generate the next Ethereum address (H160) with a custom prefix.
	///
	/// The Ethereum address will have a prefix of "FFFFFFFF" (8 hex digits) followed by the current
	/// value of `NextFuturepassId` (32 hex digits) in hexadecimal representation, resulting in a
	/// 40-hex-digit Ethereum address.
	///
	/// `NextFuturepassId` is a 128-bit unsigned integer - which converts to 32 digit hexadecimal
	/// (16 bytes) ensuring sufficient address space for unique addresses.
	///
	/// This function also increments the `NextFuturepassId` storage value for future use.
	///
	/// # Returns
	/// - `T::AccountId`: A generated Ethereum address (H160) with the desired custom prefix.
	fn generate_futurepass_account() -> T::AccountId {
		// Convert the futurepass_id to a byte array and increment the value
		let futurepass_id_bytes = NextFuturepassId::<T>::mutate(|futurepass_id| {
			let bytes = futurepass_id.to_be_bytes();
			*futurepass_id += 1;
			bytes
		});

		let prefix = FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX;

		// Create a new byte array with the combined length of the prefix and the futurepass_id
		// (bytes)
		let mut address_bytes = [0u8; 20];
		address_bytes[..4].copy_from_slice(prefix);
		address_bytes[4..].copy_from_slice(&futurepass_id_bytes);

		let address = H160::from_slice(&address_bytes);

		T::AccountId::from(address)
	}

	pub fn do_create_futurepass(
		funder: T::AccountId,
		account: T::AccountId,
	) -> Result<T::AccountId, DispatchError> {
		ensure!(!Holders::<T>::contains_key(&account), Error::<T>::AccountAlreadyRegistered);
		let futurepass = Self::generate_futurepass_account();
		Holders::<T>::set(&account, Some(futurepass.clone()));
		T::Proxy::add_delegate(&funder, &futurepass, &account, &255)?; // owner is maxu8

		Self::deposit_event(Event::<T>::FuturepassCreated {
			futurepass: futurepass.clone(),
			delegate: account,
		});
		Ok(futurepass)
	}

	fn generate_add_delegate_eth_signed_message(
		futurepass: &T::AccountId,
		delegate: &T::AccountId,
		proxy_type: &T::ProxyType,
		deadline: &u32,
	) -> Result<([u8; 32], [u8; 32]), DispatchError> {
		let mut buffer = Vec::new(); // re-use buffer for encoding (performance)

		futurepass.encode_to(&mut buffer);
		let futurepass: [u8; 20] = H160::from_slice(&buffer[..]).into();
		buffer.clear();

		delegate.encode_to(&mut buffer);
		let delegate: [u8; 20] = H160::from_slice(&buffer[..]).into();
		buffer.clear();

		proxy_type.encode_to(&mut buffer);
		let proxy_type: [u8; 1] =
			buffer[..].try_into().map_err(|_| Error::<T>::InvalidProxyType)?;
		buffer.clear();

		let deadline: [u8; 4] = deadline.to_be_bytes();

		// create packed message - anologous to Solidity's abi.encodePacked
		let mut packed_msg = Vec::new();
		packed_msg.extend(&futurepass);
		packed_msg.extend(&delegate);
		packed_msg.extend(&proxy_type);
		packed_msg.extend(&deadline);

		#[cfg(test)]
		println!("packed_msg: {:?}", hex::encode(&packed_msg));

		let hashed_msg: [u8; 32] = keccak_256(&packed_msg);

		#[cfg(test)]
		println!("hashed_msg: {:?}", hex::encode(hashed_msg));

		let eth_signed_msg: [u8; 32] = keccak_256(
			seed_primitives::ethereum_signed_message(hex::encode(hashed_msg).as_bytes()).as_ref(),
		);
		#[cfg(test)]
		println!("ethereum_signed_message: {:?}", hex::encode(eth_signed_msg));

		Ok((hashed_msg, eth_signed_msg))
	}
}

impl<T: Config> AccountProxy<T::AccountId> for Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<H160>,
{
	fn primary_proxy(who: &T::AccountId) -> Option<T::AccountId> {
		<DefaultProxy<T>>::get(who)
	}
}

impl<T: Config> FuturepassProvider for Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<H160>,
{
	type AccountId = T::AccountId;

	fn create_futurepass(
		funder: Self::AccountId,
		owner: Self::AccountId,
	) -> Result<Self::AccountId, DispatchError> {
		Self::do_create_futurepass(funder, owner)
	}
}
