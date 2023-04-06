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
		ensure!(!collection_name.is_empty(), Error::<T>::CollectionNameInvalid);
		ensure!(core::str::from_utf8(&collection_name).is_ok(), Error::<T>::CollectionNameInvalid);

		// Validate MetadataScheme
		let metadata_scheme =
			metadata_scheme.sanitize().map_err(|_| Error::<T>::InvalidMetadataPath)?;

		// Validate RoyaltiesSchedule
		if let Some(royalties_schedule) = royalties_schedule.clone() {
			ensure!(royalties_schedule.validate(), Error::<T>::RoyaltiesInvalid);
		}
		let owner = collection_owner.unwrap_or(origin);

		let sft_collection_info = SftCollectionInformation {
			collection_owner: owner,
			name: collection_name.clone(),
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
			collection_uuid,
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
		token_name: CollectionNameType,
		initial_issuance: Balance,
		max_issuance: Option<Balance>,
		token_owner: Option<T::AccountId>,
	) -> DispatchResult {
		let mut existing_collection =
			SftCollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
		ensure!(who == existing_collection.collection_owner, Error::<T>::NotCollectionOwner);

		if let Some(max_issuance) = max_issuance {
			ensure!(max_issuance > Zero::zero(), Error::<T>::InvalidMaxIssuance);
			ensure!(initial_issuance <= max_issuance, Error::<T>::InvalidMaxIssuance);
			ensure!(
				max_issuance <= T::MaxTokensPerSftCollection::get().into(),
				Error::<T>::InvalidMaxIssuance
			);
		}

		let next_serial_number = existing_collection.next_serial_number;

		existing_collection.next_serial_number =
			next_serial_number.checked_add(1).ok_or(Error::<T>::OverFlow)?;

		let token_owner = token_owner.unwrap_or(who);

		let initial_balance = SftTokenBalance::new(initial_issuance, 0);

		let new_sft = SftTokenInformation {
			name: token_name.clone(),
			max_issuance,
			token_issuance: initial_issuance,
			owned_tokens: BoundedVec::truncate_from(vec![(token_owner, initial_balance)]),
		};

		TokenInfo::<T>::insert((collection_id, next_serial_number), new_sft);
		SftCollectionInfo::<T>::insert(collection_id, existing_collection);

		Self::deposit_event(Event::<T>::TokenCreated {
			collection_id,
			serial_number: next_serial_number,
			initial_issuance,
			max_issuance,
			owner: token_owner,
			token_name,
		});

		Ok(())
	}
}
