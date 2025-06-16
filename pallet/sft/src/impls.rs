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

use crate::*;
use frame_support::{ensure, traits::tokens::Preservation};
use precompile_utils::constants::ERC1155_PRECOMPILE_ADDRESS_PREFIX;
use seed_pallet_common::{
	utils::{HasBurnAuthority, PublicMintInformation},
	NFIRequest, SFTExt,
};
use seed_primitives::{CollectionUuid, MAX_COLLECTION_ENTITLEMENTS};
use sp_runtime::{traits::Zero, DispatchError};

impl<T: Config> Pallet<T> {
	/// Perform the create collection operation and insert SftCollectionInfo into storage
	pub fn do_create_collection(
		collection_owner: T::AccountId,
		collection_name: BoundedVec<u8, T::StringLimit>,
		metadata_scheme: MetadataScheme,
		royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
		origin_chain: OriginChain,
	) -> Result<CollectionUuid, DispatchError> {
		let collection_uuid = <T as Config>::NFTExt::next_collection_uuid()?;

		// Validate collection_name
		ensure!(!collection_name.is_empty(), Error::<T>::NameInvalid);
		ensure!(core::str::from_utf8(&collection_name).is_ok(), Error::<T>::NameInvalid);

		// Validate RoyaltiesSchedule
		if let Some(royalties_schedule) = royalties_schedule.clone() {
			// Check that the entitlements are less than MAX_ENTITLEMENTS - 2
			// This is because when the token is listed, two more entitlements will be added
			// for the network fee and marketplace fee
			ensure!(
				royalties_schedule.entitlements.len() <= MAX_COLLECTION_ENTITLEMENTS as usize,
				Error::<T>::RoyaltiesInvalid
			);
			ensure!(royalties_schedule.validate(), Error::<T>::RoyaltiesInvalid);
		}

		let sft_collection_info = SftCollectionInformation {
			collection_owner: collection_owner.clone(),
			collection_name: collection_name.clone(),
			metadata_scheme: metadata_scheme.clone(),
			royalties_schedule: royalties_schedule.clone(),
			origin_chain: origin_chain.clone(),
			next_serial_number: 0,
		};

		<SftCollectionInfo<T>>::insert(collection_uuid, sft_collection_info);

		// Increment NextCollectionId in NFT pallet
		<T as Config>::NFTExt::increment_collection_uuid()?;

		// Add some code to the EVM
		T::OnNewAssetSubscription::on_asset_create(
			collection_uuid,
			ERC1155_PRECOMPILE_ADDRESS_PREFIX,
		);

		Self::deposit_event(Event::<T>::CollectionCreate {
			collection_id: collection_uuid,
			collection_owner,
			metadata_scheme,
			name: collection_name,
			royalties_schedule,
			origin_chain,
		});

		Ok(collection_uuid)
	}

	pub fn do_create_token(
		who: T::AccountId,
		collection_id: CollectionUuid,
		token_name: BoundedVec<u8, T::StringLimit>,
		initial_issuance: Balance,
		max_issuance: Option<Balance>,
		token_owner: Option<T::AccountId>,
	) -> Result<SerialNumber, DispatchError> {
		let mut existing_collection =
			SftCollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
		ensure!(who == existing_collection.collection_owner, Error::<T>::NotCollectionOwner);

		// Validate token_name
		ensure!(!token_name.is_empty(), Error::<T>::NameInvalid);
		ensure!(core::str::from_utf8(&token_name).is_ok(), Error::<T>::NameInvalid);

		// Validate max_issuance
		if let Some(max_issuance) = max_issuance {
			ensure!(max_issuance > Zero::zero(), Error::<T>::InvalidMaxIssuance);
			ensure!(initial_issuance <= max_issuance, Error::<T>::InvalidMaxIssuance);
		}

		let next_serial_number = existing_collection.next_serial_number;
		existing_collection.next_serial_number =
			next_serial_number.checked_add(1).ok_or(Error::<T>::Overflow)?;

		let token_owner = token_owner.unwrap_or(who.clone());
		let owned_tokens = if initial_issuance > Zero::zero() {
			let initial_balance: SftTokenBalance = SftTokenBalance::new(initial_issuance, 0);
			BoundedVec::truncate_from(vec![(token_owner.clone(), initial_balance)])
		} else {
			BoundedVec::truncate_from(vec![])
		};
		let new_sft = SftTokenInformation {
			token_name: token_name.clone(),
			max_issuance,
			token_issuance: initial_issuance,
			owned_tokens,
		};

		// Request NFI data for the minted tokens
		T::NFIRequest::request(&who, collection_id, vec![next_serial_number])?;

		TokenInfo::<T>::insert((collection_id, next_serial_number), new_sft);
		SftCollectionInfo::<T>::insert(collection_id, existing_collection);

		Self::deposit_event(Event::<T>::TokenCreate {
			token_id: (collection_id, next_serial_number),
			initial_issuance,
			max_issuance,
			token_owner,
			token_name,
		});

		Ok(next_serial_number)
	}

	pub(crate) fn charge_mint_fee(
		who: &T::AccountId,
		token_id: TokenId,
		collection_owner: &T::AccountId,
		public_mint_info: PublicMintInformation,
		token_count: Balance,
	) -> DispatchResult {
		// Calculate the total fee
		let total_fee = public_mint_info
			.pricing_details
			.map(|(asset, price)| (asset, price.saturating_mul(token_count as Balance)));
		// Charge the fee if there is a fee set
		if let Some((asset, total_fee)) = total_fee {
			T::MultiCurrency::transfer(
				asset,
				who,
				collection_owner,
				total_fee,
				Preservation::Expendable,
			)?;
			// Deposit event
			Self::deposit_event(Event::<T>::MintFeePaid {
				who: who.clone(),
				token_id,
				payment_asset: asset,
				payment_amount: total_fee,
				token_count,
			});
		}

		Ok(())
	}

	/// Perform some validation checks to ensure minting of the specified
	/// tokens is allowed.
	pub fn pre_mint(
		who: T::AccountId,
		collection_id: CollectionUuid,
		collection_info: SftCollectionInformation<T::AccountId, T::StringLimit>,
		serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
	) -> DispatchResult {
		// Must be some serial numbers to mint
		ensure!(!serial_numbers.is_empty(), Error::<T>::NoToken);

		// minting flag must be enabled on the collection
		ensure!(<UtilityFlags<T>>::get(collection_id).mintable, Error::<T>::MintUtilityBlocked);

		for (serial_number, quantity) in &serial_numbers {
			// Validate quantity
			ensure!(!quantity.is_zero(), Error::<T>::InvalidQuantity);

			let token_id: TokenId = (collection_id, *serial_number);

			let public_mint_info = <PublicMintInfo<T>>::get(token_id).unwrap_or_default();

			// Caller must be collection_owner if public mint is disabled
			ensure!(
				collection_info.collection_owner == who || public_mint_info.enabled,
				Error::<T>::PublicMintDisabled
			);

			let token_info = TokenInfo::<T>::get(token_id).ok_or(Error::<T>::NoToken)?;
			// Check for overflow
			ensure!(
				token_info.token_issuance.checked_add(*quantity).is_some(),
				Error::<T>::Overflow
			);
			// Check that the max issuance will not be reached
			// Can't mint more than specified max_issuance
			if let Some(max_issuance) = token_info.max_issuance {
				ensure!(
					token_info.token_issuance + quantity <= max_issuance,
					Error::<T>::MaxIssuanceReached
				);
			}
		}

		Ok(())
	}

	/// Perform the mint operation and increase the quantity of the user
	/// Note there is one storage read and write per serial number minted
	pub fn do_mint(
		who: T::AccountId,
		collection_id: CollectionUuid,
		collection_info: SftCollectionInformation<T::AccountId, T::StringLimit>,
		serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
		token_owner: Option<T::AccountId>,
	) -> DispatchResult {
		let owner = token_owner.unwrap_or(who.clone());

		for (serial_number, quantity) in &serial_numbers {
			let token_id: TokenId = (collection_id, *serial_number);

			let public_mint_info = <PublicMintInfo<T>>::get(token_id).unwrap_or_default();

			// Only charge mint fee if public mint enabled and caller is not collection owner
			if public_mint_info.enabled && collection_info.collection_owner != who {
				// Charge the mint fee for the mint
				Self::charge_mint_fee(
					&who,
					token_id,
					&collection_info.collection_owner,
					public_mint_info,
					*quantity,
				)?;
			}

			let mut token_info = TokenInfo::<T>::get(token_id).ok_or(Error::<T>::NoToken)?;

			// Add the balance
			token_info.add_balance(&owner, *quantity).map_err(Error::<T>::from)?;
			token_info.token_issuance += quantity;
			TokenInfo::<T>::insert(token_id, token_info);
		}

		Ok(())
	}

	/// Checks if all tokens in a list are unique.
	pub fn check_unique(serial_numbers: Vec<(SerialNumber, Balance)>) -> bool {
		let serial_numbers: Vec<SerialNumber> = serial_numbers.iter().map(|(sn, _)| *sn).collect();
		let original_length = serial_numbers.len();
		let mut serial_numbers_trimmed = serial_numbers;
		serial_numbers_trimmed.sort_unstable();
		serial_numbers_trimmed.dedup();
		serial_numbers_trimmed.len() == original_length
	}

	/// Perform the transfer operation and move quantities from one user to another
	/// Note there is one storage read and write per serial number transferred
	pub fn do_transfer(
		who: T::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
		new_owner: T::AccountId,
	) -> DispatchResult {
		// Must be some serial numbers to transfer
		ensure!(!serial_numbers.is_empty(), Error::<T>::NoToken);
		// Caller must not be new owner
		ensure!(who != new_owner, Error::<T>::InvalidNewOwner);
		// transferable flag must be enabled on the collection
		ensure!(
			<UtilityFlags<T>>::get(collection_id).transferable,
			Error::<T>::TransferUtilityBlocked
		);
		// Check that all serial numbers are unique
		ensure!(Self::check_unique(serial_numbers.to_vec()), Error::<T>::SerialNumbersNotUnique);

		for (serial_number, quantity) in &serial_numbers {
			// Validate quantity
			ensure!(!quantity.is_zero(), Error::<T>::InvalidQuantity);

			let token_utility_flags = <TokenUtilityFlags<T>>::get((collection_id, serial_number));
			ensure!(token_utility_flags.transferable, Error::<T>::TransferUtilityBlocked);
			ensure!(
				token_utility_flags.burn_authority.is_none(),
				Error::<T>::TransferUtilityBlocked
			);

			let token_id: TokenId = (collection_id, *serial_number);
			let mut token_info = TokenInfo::<T>::get(token_id).ok_or(Error::<T>::NoToken)?;

			// Transfer the balance
			token_info
				.transfer_balance(&who, &new_owner, *quantity)
				.map_err(Error::<T>::from)?;
			TokenInfo::<T>::insert(token_id, token_info);
		}

		let (serial_numbers, balances) = Self::unzip_serial_numbers(serial_numbers);
		Self::deposit_event(Event::<T>::Transfer {
			previous_owner: who,
			collection_id,
			serial_numbers,
			balances,
			new_owner,
		});

		Ok(())
	}

	/// Perform the burn operation and decrease the quantity of the user
	/// Note there is one storage read and write per serial number burned
	#[transactional]
	pub fn do_burn(
		who: &T::AccountId,
		token_owner: &T::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
	) -> DispatchResult {
		// Must be some serial numbers to burn
		ensure!(!serial_numbers.is_empty(), Error::<T>::NoToken);
		ensure!(<UtilityFlags<T>>::get(collection_id).burnable, Error::<T>::BurnUtilityBlocked);

		let collection_info =
			SftCollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;

		for (serial_number, quantity) in &serial_numbers {
			// Validate quantity
			ensure!(!quantity.is_zero(), Error::<T>::InvalidQuantity);

			if let Some(burn_authority) =
				TokenUtilityFlags::<T>::get((collection_id, serial_number)).burn_authority
			{
				ensure!(
					burn_authority.has_burn_authority(
						&collection_info.collection_owner,
						token_owner,
						who
					),
					Error::<T>::InvalidBurnAuthority
				);
			} else {
				ensure!(who == token_owner, Error::<T>::InvalidBurnAuthority);
			}

			let token_id: TokenId = (collection_id, *serial_number);
			let mut token_info = TokenInfo::<T>::get(token_id).ok_or(Error::<T>::NoToken)?;

			// Burn the balance
			token_info.remove_balance(token_owner, *quantity).map_err(Error::<T>::from)?;
			token_info.token_issuance = token_info.token_issuance.saturating_sub(*quantity);
			TokenInfo::<T>::insert(token_id, token_info);
		}

		let (serial_numbers, balances) = Self::unzip_serial_numbers(serial_numbers);
		Self::deposit_event(Event::<T>::Burn {
			collection_id,
			serial_numbers,
			balances,
			owner: token_owner.clone(),
		});

		Ok(())
	}

	pub fn do_set_owner(
		who: T::AccountId,
		collection_id: CollectionUuid,
		new_owner: T::AccountId,
	) -> DispatchResult {
		let mut collection =
			SftCollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
		ensure!(collection.collection_owner == who, Error::<T>::NotCollectionOwner);

		collection.collection_owner = new_owner.clone();
		SftCollectionInfo::<T>::insert(collection_id, collection);
		Self::deposit_event(Event::<T>::OwnerSet { new_owner, collection_id });

		Ok(())
	}

	/// Perfrom the set max issuance operation
	/// Caller must be the collection owner
	/// Max issuance can only be set once
	pub fn do_set_max_issuance(
		who: T::AccountId,
		token_id: TokenId,
		max_issuance: Balance,
	) -> DispatchResult {
		ensure!(!max_issuance.is_zero(), Error::<T>::InvalidMaxIssuance);

		let collection_info =
			SftCollectionInfo::<T>::get(token_id.0).ok_or(Error::<T>::NoCollectionFound)?;
		// Caller must be collection_owner
		ensure!(collection_info.collection_owner == who, Error::<T>::NotCollectionOwner);

		let mut token_info = TokenInfo::<T>::get(token_id).ok_or(Error::<T>::NoToken)?;
		// Max issuance can only be set once
		ensure!(token_info.max_issuance.is_none(), Error::<T>::MaxIssuanceAlreadySet);
		// Max issuance cannot exceed token issuance
		ensure!(token_info.token_issuance <= max_issuance, Error::<T>::InvalidMaxIssuance);

		token_info.max_issuance = Some(max_issuance);
		TokenInfo::<T>::insert(token_id, token_info);

		Self::deposit_event(Event::<T>::MaxIssuanceSet { token_id, max_issuance });

		Ok(())
	}

	/// Perform the set base uri operation
	/// Caller must be collection owner
	pub fn do_set_base_uri(
		who: T::AccountId,
		collection_id: CollectionUuid,
		metadata_scheme: MetadataScheme,
	) -> DispatchResult {
		let mut collection_info =
			SftCollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
		// Caller must be collection_owner
		ensure!(collection_info.collection_owner == who, Error::<T>::NotCollectionOwner);

		collection_info.metadata_scheme = metadata_scheme.clone();
		SftCollectionInfo::<T>::insert(collection_id, collection_info);

		Self::deposit_event(Event::<T>::BaseUriSet { collection_id, metadata_scheme });
		Ok(())
	}

	/// Perform the set name operation
	/// Caller must be collection owner
	pub fn do_set_name(
		who: T::AccountId,
		collection_id: CollectionUuid,
		collection_name: BoundedVec<u8, T::StringLimit>,
	) -> DispatchResult {
		let mut collection_info =
			SftCollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
		// Caller must be collection_owner
		ensure!(collection_info.collection_owner == who, Error::<T>::NotCollectionOwner);

		// Validate collection_name
		ensure!(!collection_name.is_empty(), Error::<T>::NameInvalid);
		ensure!(core::str::from_utf8(&collection_name).is_ok(), Error::<T>::NameInvalid);
		collection_info.collection_name = collection_name.clone();

		SftCollectionInfo::<T>::insert(collection_id, collection_info);

		Self::deposit_event(Event::<T>::NameSet { collection_id, collection_name });
		Ok(())
	}

	/// Perform the set name operation on the token
	/// Caller must be collection owner
	pub fn do_set_token_name(
		who: T::AccountId,
		token_id: TokenId,
		token_name: BoundedVec<u8, T::StringLimit>,
	) -> DispatchResult {
		let collection_info =
			SftCollectionInfo::<T>::get(token_id.0).ok_or(Error::<T>::NoCollectionFound)?;
		// Caller must be collection_owner
		ensure!(collection_info.collection_owner == who, Error::<T>::NotCollectionOwner);

		// Validate token_name
		ensure!(!token_name.is_empty(), Error::<T>::NameInvalid);
		ensure!(core::str::from_utf8(&token_name).is_ok(), Error::<T>::NameInvalid);

		TokenInfo::<T>::try_mutate(token_id, |maybe_token_info| -> DispatchResult {
			let token_info = maybe_token_info.as_mut().ok_or(Error::<T>::NoToken)?;
			token_info.token_name = token_name.clone();
			Ok(())
		})?;

		Self::deposit_event(Event::<T>::TokenNameSet { token_id, token_name });
		Ok(())
	}

	/// Perform the set name operation
	/// Caller must be collection owner
	pub fn do_set_royalties_schedule(
		who: T::AccountId,
		collection_id: CollectionUuid,
		royalties_schedule: RoyaltiesSchedule<T::AccountId>,
	) -> DispatchResult {
		let mut collection_info =
			SftCollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
		ensure!(collection_info.collection_owner == who, Error::<T>::NotCollectionOwner);

		// Check that the entitlements are less than MAX_ENTITLEMENTS - 2
		// This is because when the token is listed, two more entitlements will be added
		// for the network fee and marketplace fee
		ensure!(
			royalties_schedule.entitlements.len() <= MAX_COLLECTION_ENTITLEMENTS as usize,
			Error::<T>::RoyaltiesInvalid
		);
		ensure!(royalties_schedule.validate(), Error::<T>::RoyaltiesInvalid);

		collection_info.royalties_schedule = Some(royalties_schedule.clone());

		SftCollectionInfo::<T>::insert(collection_id, collection_info);
		Self::deposit_event(Event::<T>::RoyaltiesScheduleSet { collection_id, royalties_schedule });
		Ok(())
	}

	/// Unzips the bounded vec of tuples (SerialNumber, Balance)
	/// into two bounded vecs of SerialNumber and Balance
	pub fn unzip_serial_numbers(
		serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
	) -> (BoundedVec<SerialNumber, T::MaxSerialsPerMint>, BoundedVec<Balance, T::MaxSerialsPerMint>)
	{
		let (serial_numbers, quantities) = serial_numbers.into_iter().unzip();
		(BoundedVec::truncate_from(serial_numbers), BoundedVec::truncate_from(quantities))
	}

	/// Returns true if an SFT collection exists for this collectionId
	pub fn collection_exists(collection_id: CollectionUuid) -> bool {
		SftCollectionInfo::<T>::contains_key(collection_id)
	}

	/// Returns the owner of a collection
	pub fn get_collection_owner(collection_id: CollectionUuid) -> Option<T::AccountId> {
		SftCollectionInfo::<T>::get(collection_id).map(|info| info.collection_owner)
	}

	// Returns the balance of who of a token_id
	pub fn balance_of(who: &T::AccountId, token_id: TokenId) -> Balance {
		let Some(token_info) = TokenInfo::<T>::get(token_id) else { return Balance::zero() };
		token_info.free_balance_of(who)
	}

	/// Returns the total supply of a specified token_id
	pub fn total_supply(token_id: TokenId) -> Balance {
		let Some(token_info) = TokenInfo::<T>::get(token_id) else { return Balance::zero() };
		token_info.token_issuance
	}

	/// Indicates whether a token with a given id exists or not
	pub fn token_exists(token_id: TokenId) -> bool {
		TokenInfo::<T>::contains_key(token_id)
	}

	/// Returns the metadatascheme or None if no collection exists
	pub fn token_uri(token_id: TokenId) -> Vec<u8> {
		let Some(collection_info) = SftCollectionInfo::<T>::get(token_id.0) else {
			return Default::default();
		};
		collection_info.metadata_scheme.construct_token_uri(token_id.1)
	}
}

impl<T: Config> SFTExt for Pallet<T> {
	type AccountId = T::AccountId;

	fn do_transfer(
		origin: Self::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: Vec<(SerialNumber, Balance)>,
		new_owner: Self::AccountId,
	) -> DispatchResult {
		let bounded_serials =
			BoundedVec::try_from(serial_numbers).map_err(|_| Error::<T>::TokenLimitExceeded)?;
		Self::do_transfer(origin, collection_id, bounded_serials, new_owner)
	}

	fn reserve_balance(
		token_id: TokenId,
		amount: Balance,
		who: &Self::AccountId,
	) -> DispatchResult {
		let mut token_info = TokenInfo::<T>::get(token_id).ok_or(Error::<T>::NoToken)?;
		token_info.reserve_balance(who, amount).map_err(Error::<T>::from)?;
		TokenInfo::<T>::insert(token_id, token_info);
		Ok(())
	}

	fn free_reserved_balance(
		token_id: TokenId,
		amount: Balance,
		who: &Self::AccountId,
	) -> DispatchResult {
		let mut token_info = TokenInfo::<T>::get(token_id).ok_or(Error::<T>::NoToken)?;
		token_info.free_reserved_balance(who, amount).map_err(Error::<T>::from)?;
		TokenInfo::<T>::insert(token_id, token_info);
		Ok(())
	}

	fn get_royalties_schedule(
		collection_id: CollectionUuid,
	) -> Result<Option<RoyaltiesSchedule<Self::AccountId>>, DispatchError> {
		let collection_info =
			SftCollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
		Ok(collection_info.royalties_schedule)
	}

	fn get_collection_owner(
		collection_id: CollectionUuid,
	) -> Result<Self::AccountId, DispatchError> {
		let collection_info =
			SftCollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
		Ok(collection_info.collection_owner)
	}

	fn token_exists(token_id: TokenId) -> bool {
		Self::token_exists(token_id)
	}
}
