/* Copyright 2019-2021 Centrality Investments Limited
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

use crate::*;
use frame_support::{ensure, traits::Get};
use precompile_utils::constants::ERC1155_PRECOMPILE_ADDRESS_PREFIX;
use seed_primitives::CollectionUuid;
use sp_runtime::{traits::Zero, DispatchError};

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

		// Validate MetadataScheme
		let metadata_scheme =
			metadata_scheme.sanitize().map_err(|_| Error::<T>::InvalidMetadataPath)?;

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
	#[transactional]
	pub fn do_mint(
		who: T::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: BoundedVec<SerialNumber, T::MaxSerialsPerMint>,
		quantities: BoundedVec<Balance, T::MaxSerialsPerMint>,
		token_owner: Option<T::AccountId>,
	) -> DispatchResult {
		// Validate serial_numbers and quantities length
		ensure!(serial_numbers.len() == quantities.len(), Error::<T>::InvalidMintInput);
		// Must be some serial numbers to mint
		ensure!(!serial_numbers.is_empty(), Error::<T>::NoToken);

		let sft_collection_info =
			SftCollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;

		// Caller must be collection_owner
		ensure!(sft_collection_info.is_collection_owner(&who), Error::<T>::NotCollectionOwner);

		let owner = token_owner.unwrap_or(who);

		for (serial_number, quantity) in serial_numbers
			.iter()
			.zip(quantities.iter())
			.collect::<Vec<(&SerialNumber, &Balance)>>()
		{
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

			// Mint the balance
			token_info.mint_balance(&owner, *quantity)?;
			token_info.token_issuance += quantity;
			TokenInfo::<T>::insert(token_id, token_info);
		}

		Self::deposit_event(Event::<T>::Mint { collection_id, serial_numbers, quantities, owner });

		Ok(())
	}
}
