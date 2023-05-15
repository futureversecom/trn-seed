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

use crate::*;
use frame_support::ensure;
use precompile_utils::constants::ERC1155_PRECOMPILE_ADDRESS_PREFIX;
use seed_primitives::CollectionUuid;
use sp_runtime::traits::Zero;

impl<T: Config> Pallet<T> {
	/// Perform the create collection operation and insert SftCollectionInfo into storage
	pub fn do_create_collection(
		origin: T::AccountId,
		collection_name: BoundedVec<u8, T::StringLimit>,
		collection_owner: Option<T::AccountId>,
		metadata_scheme: MetadataScheme,
		royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
		origin_chain: OriginChain,
	) -> DispatchResult {
		let collection_uuid = <T as Config>::NFTExt::next_collection_uuid()?;

		// Validate collection_name
		ensure!(!collection_name.is_empty(), Error::<T>::NameInvalid);
		ensure!(core::str::from_utf8(&collection_name).is_ok(), Error::<T>::NameInvalid);

		// Validate RoyaltiesSchedule
		if let Some(royalties_schedule) = royalties_schedule.clone() {
			ensure!(royalties_schedule.validate(), Error::<T>::RoyaltiesInvalid);
		}
		let owner = collection_owner.unwrap_or(origin);

		let sft_collection_info = SftCollectionInformation {
			collection_owner: owner.clone(),
			collection_name: collection_name.clone(),
			metadata_scheme: metadata_scheme.clone(),
			royalties_schedule: royalties_schedule.clone(),
			origin_chain: origin_chain.clone(),
			next_serial_number: 0,
		};

		<SftCollectionInfo<T>>::insert(collection_uuid, sft_collection_info);

		// Increment NextCollectionId in NFT pallet
		<T as Config>::NFTExt::increment_collection_id()?;

		// Add some code to the EVM
		T::OnNewAssetSubscription::on_asset_create(
			collection_uuid,
			ERC1155_PRECOMPILE_ADDRESS_PREFIX,
		);

		Self::deposit_event(Event::<T>::CollectionCreate {
			collection_id: collection_uuid,
			collection_owner: owner,
			metadata_scheme,
			name: collection_name,
			royalties_schedule,
			origin_chain,
		});

		Ok(())
	}

	pub fn do_create_token(
		who: T::AccountId,
		collection_id: CollectionUuid,
		token_name: BoundedVec<u8, T::StringLimit>,
		initial_issuance: Balance,
		max_issuance: Option<Balance>,
		token_owner: Option<T::AccountId>,
	) -> DispatchResult {
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

		let token_owner = token_owner.unwrap_or(who);
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

		TokenInfo::<T>::insert((collection_id, next_serial_number), new_sft);
		SftCollectionInfo::<T>::insert(collection_id, existing_collection);

		Self::deposit_event(Event::<T>::TokenCreated {
			collection_id,
			serial_number: next_serial_number,
			initial_issuance,
			max_issuance,
			token_owner,
			token_name,
		});

		Ok(())
	}

	/// Perform the mint operation and increase the quantity of the user
	/// Note there is one storage read and write per serial number minted
	pub fn do_mint(
		who: T::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
		token_owner: Option<T::AccountId>,
	) -> DispatchResult {
		// Must be some serial numbers to mint
		ensure!(!serial_numbers.is_empty(), Error::<T>::NoToken);

		let sft_collection_info =
			SftCollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;

		// Caller must be collection_owner
		ensure!(sft_collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);

		let owner = token_owner.unwrap_or(who);

		for (serial_number, quantity) in &serial_numbers {
			// Validate quantity
			ensure!(!quantity.is_zero(), Error::<T>::InvalidQuantity);

			let token_id: TokenId = (collection_id, *serial_number);
			let mut token_info = TokenInfo::<T>::get(token_id).ok_or(Error::<T>::NoToken)?;
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

			// Add the balance
			token_info.add_balance(&owner, *quantity)?;
			token_info.token_issuance += quantity;
			TokenInfo::<T>::insert(token_id, token_info);
		}

		let (serial_numbers, quantities) = Self::unzip_serial_numbers(serial_numbers);
		Self::deposit_event(Event::<T>::Mint { collection_id, serial_numbers, quantities, owner });

		Ok(())
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

		for (serial_number, quantity) in &serial_numbers {
			// Validate quantity
			ensure!(!quantity.is_zero(), Error::<T>::InvalidQuantity);

			let token_id: TokenId = (collection_id, *serial_number);
			let mut token_info = TokenInfo::<T>::get(token_id).ok_or(Error::<T>::NoToken)?;

			// Transfer the balance
			token_info.transfer_balance(&who, &new_owner, *quantity)?;
			TokenInfo::<T>::insert(token_id, token_info);
		}

		let (serial_numbers, quantities) = Self::unzip_serial_numbers(serial_numbers);
		Self::deposit_event(Event::<T>::Transfer {
			previous_owner: who,
			collection_id,
			serial_numbers,
			quantities,
			new_owner,
		});

		Ok(())
	}

	/// Perform the burn operation and decrease the quantity of the user
	/// Note there is one storage read and write per serial number burned
	#[transactional]
	pub fn do_burn(
		who: T::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
	) -> DispatchResult {
		// Must be some serial numbers to burn
		ensure!(!serial_numbers.is_empty(), Error::<T>::NoToken);

		for (serial_number, quantity) in &serial_numbers {
			// Validate quantity
			ensure!(!quantity.is_zero(), Error::<T>::InvalidQuantity);

			let token_id: TokenId = (collection_id, *serial_number);
			let mut token_info = TokenInfo::<T>::get(token_id).ok_or(Error::<T>::NoToken)?;

			// Burn the balance
			token_info.remove_balance(&who, *quantity)?;
			token_info.token_issuance = token_info.token_issuance.saturating_sub(*quantity);
			TokenInfo::<T>::insert(token_id, token_info);
		}

		let (serial_numbers, quantities) = Self::unzip_serial_numbers(serial_numbers);
		Self::deposit_event(Event::<T>::Burn {
			collection_id,
			serial_numbers,
			quantities,
			owner: who,
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
		ensure!(collection.is_collection_owner(&who), Error::<T>::NotCollectionOwner);

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
		ensure!(collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);

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
		ensure!(collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);

		collection_info.metadata_scheme = metadata_scheme.clone();
		SftCollectionInfo::<T>::insert(collection_id, collection_info);

		Self::deposit_event(Event::<T>::BaseUriSet { collection_id, metadata_scheme });
		Ok(())
	}

	/// Unzips the bounded vec of tuples (SerialNumber, Balance)
	/// into two bounded vecs of SerialNumber and Balance
	fn unzip_serial_numbers(
		serial_numbers: BoundedVec<(SerialNumber, Balance), T::MaxSerialsPerMint>,
	) -> (BoundedVec<SerialNumber, T::MaxSerialsPerMint>, BoundedVec<Balance, T::MaxSerialsPerMint>)
	{
		let (serial_numbers, quantities) = serial_numbers.into_iter().unzip();
		(BoundedVec::truncate_from(serial_numbers), BoundedVec::truncate_from(quantities))
	}
}
