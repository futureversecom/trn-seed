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

use crate::{traits::NFTCollectionInfo, *};
use frame_support::{
	ensure,
	traits::{tokens::Preservation, Get},
	weights::Weight,
};
use frame_system::RawOrigin;
use precompile_utils::constants::ERC721_PRECOMPILE_ADDRESS_PREFIX;
use seed_pallet_common::{
	log,
	utils::{next_asset_uuid, HasBurnAuthority, PublicMintInformation},
	Migrator, NFTExt, NFTMinter, OnNewAssetSubscriber, OnTransferSubscriber,
};
use seed_primitives::{
	CollectionUuid, MetadataScheme, OriginChain, RoyaltiesSchedule, SerialNumber, TokenCount,
	TokenId, WeightedDispatchResult,
};
use seed_primitives::{CrossChainCompatibility, MAX_COLLECTION_ENTITLEMENTS};
use sp_runtime::{
	traits::Zero, ArithmeticError, BoundedVec, DispatchError, DispatchResult, Permill,
	SaturatedConversion,
};

impl<T: Config> Pallet<T> {
	/// Returns the CollectionUuid unique across parachains
	pub fn next_collection_uuid() -> Result<CollectionUuid, DispatchError> {
		let collection_id = <NextCollectionId<T>>::get();
		match next_asset_uuid(collection_id, T::ParachainId::get()) {
			Some(next_collection_uuid) => Ok(next_collection_uuid),
			None => Err(Error::<T>::NoAvailableIds.into()),
		}
	}

	/// Return whether the collection exists or not
	pub fn collection_exists(collection_id: CollectionUuid) -> bool {
		if T::Migrator::ensure_migrated().is_err() {
			return false;
		}
		<CollectionInfo<T>>::contains_key(collection_id)
	}

	/// Returns number of tokens owned by an account in a collection
	/// Used by the ERC721 precompile for balance_of
	pub fn token_balance_of(who: &T::AccountId, collection_id: CollectionUuid) -> TokenCount {
		if T::Migrator::ensure_migrated().is_err() {
			return 0;
		}
		match <OwnedTokens<T>>::get(who, collection_id) {
			Some(owned_tokens) => owned_tokens.len() as TokenCount,
			None => TokenCount::zero(),
		}
	}

	/// Construct & return the full metadata URI for a given `token_id` (analogous to ERC721
	/// metadata token_uri)
	pub fn token_uri(token_id: TokenId) -> Vec<u8> {
		if T::Migrator::ensure_migrated().is_err() {
			return Default::default();
		}
		let Some(collection_info) = <CollectionInfo<T>>::get(token_id.0) else {
			// should not happen
			log!(warn, "🃏 Unexpected empty metadata scheme: {:?}", token_id);
			return Default::default();
		};
		collection_info.metadata_scheme.construct_token_uri(token_id.1)
	}

	/// Checks if all tokens in a list are unique.
	pub fn check_unique(serial_numbers: Vec<SerialNumber>) -> bool {
		let original_length = serial_numbers.len();
		let mut serial_numbers_trimmed = serial_numbers;
		serial_numbers_trimmed.sort_unstable();
		serial_numbers_trimmed.dedup();
		serial_numbers_trimmed.len() == original_length
	}

	/// Transfer the given token from `current_owner` to `new_owner`
	/// Does no verification
	pub fn do_transfer(
		collection_id: CollectionUuid,
		serial_numbers: BoundedVec<SerialNumber, T::TransferLimit>,
		current_owner: &T::AccountId,
		new_owner: &T::AccountId,
	) -> DispatchResult {
		T::Migrator::ensure_migrated()?;
		ensure!(current_owner != new_owner, Error::<T>::InvalidNewOwner);
		ensure!(
			<UtilityFlags<T>>::get(collection_id).transferable,
			Error::<T>::TransferUtilityBlocked
		);
		ensure!(
			Self::check_unique(serial_numbers.clone().into_inner()),
			Error::<T>::SerialNumbersNotUnique
		);
        ensure!(CollectionInfo::<T>::contains_key(collection_id), Error::<T>::NoCollectionFound);

		// Update `TokenOwner` mapping and check token level restrictions
		for &serial_number in &serial_numbers {
			TokenInfo::<T>::try_mutate(
				collection_id,
				serial_number,
				|token_info| -> DispatchResult {
					let token_info = token_info.as_mut().ok_or(Error::<T>::NoToken)?;
					ensure!(token_info.owner == current_owner.clone(), Error::<T>::NotTokenOwner);
					ensure!(token_info.lock_status.is_none(), Error::<T>::TokenLocked);
					ensure!(
						token_info.utility_flags.transferable,
						Error::<T>::TransferUtilityBlocked
					);
					// Check if soulbound
					ensure!(
						token_info.utility_flags.burn_authority.is_none(),
						Error::<T>::TransferUtilityBlocked
					);
					token_info.owner = new_owner.clone();
					Ok(())
				},
			)?;
		}

		// Update `OwnedTokens` for current owner
		OwnedTokens::<T>::try_mutate(
			current_owner,
			collection_id,
			|maybe_owned_serials| -> DispatchResult {
				if let Some(owned_serials) = maybe_owned_serials {
					owned_serials.retain(|serial| !serial_numbers.contains(serial));
					// If no tokens remain, remove the entry completely
					if owned_serials.is_empty() {
						*maybe_owned_serials = None;
					}
				} else {
					Err(Error::<T>::NotTokenOwner)?;
				}
				Ok(())
			},
		)?;

		// Update `OwnedTokens` for new owner
		OwnedTokens::<T>::try_mutate(
			new_owner,
			collection_id,
			|owned_serials| -> DispatchResult {
				match owned_serials.as_mut() {
					Some(owned_serials) => {
						for &serial_number in &serial_numbers {
							owned_serials
								.try_push(serial_number)
								.map_err(|_| Error::<T>::TokenLimitExceeded)?;
						}
					},
					None => {
						// convert bound to MaxTokensPerCollection (Which should be higher than TransferLimit)
						let bounded_serials: BoundedVec<SerialNumber, T::MaxTokensPerCollection> =
							BoundedVec::try_from(serial_numbers.clone().into_inner())
								.map_err(|_| Error::<T>::TokenLimitExceeded)?;
						*owned_serials = Some(bounded_serials);
					},
				}
				Ok(())
			},
		)?;

		for serial_number in &serial_numbers {
			T::OnTransferSubscription::on_nft_transfer(&(collection_id, *serial_number));
		}

		Self::deposit_event(Event::<T>::Transfer {
			previous_owner: current_owner.clone(),
			collection_id,
			serial_numbers: serial_numbers.into_inner(),
			new_owner: new_owner.clone(),
		});
		Ok(())
	}

	/// Mint additional tokens in a collection
	/// This is called by the nft-peg pallet and mints tokens based on the token ids bridged
	/// An extra check is made to ensure tokens have not already been minted, if this happens
	/// execution won't fail, however those tokens will not be minted twice.
	pub fn mint_bridged_token(
		owner: &T::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: Vec<SerialNumber>,
	) -> WeightedDispatchResult {
		T::Migrator::ensure_migrated().map_err(|e| (Weight::zero(), e))?;
		if serial_numbers.is_empty() {
			return Ok(Weight::zero());
		};

		let mut collection_info = match <CollectionInfo<T>>::get(collection_id) {
			Some(info) => info,
			None => return Ok(T::DbWeight::get().reads(1)),
		};

		// remove duplicates from serial_numbers
		let mut serial_numbers_trimmed = serial_numbers;
		serial_numbers_trimmed.sort_unstable();
		serial_numbers_trimmed.dedup();

		// Trim the new serial_numbers and remove any that have already been minted
		serial_numbers_trimmed = serial_numbers_trimmed
			.into_iter()
			.filter(|serial_number| {
				if TokenInfo::<T>::contains_key(collection_id, *serial_number) {
					// Since we don't want to error, throw a warning instead.
					// If we error, then some tokens may be lost
					log!(
						warn,
						"🃏 Token Couldn't be minted as this token_id already exists: ({:?},{:?})",
						collection_id,
						serial_number
					);
					false
				} else {
					true
				}
			})
			.collect::<Vec<SerialNumber>>();

		let serial_numbers: Result<BoundedVec<SerialNumber, T::MaxTokensPerCollection>, Vec<_>> =
			BoundedVec::try_from(serial_numbers_trimmed);
		match serial_numbers {
			Ok(serial_numbers) => {
                // TODO Fix this part
				let mint =
					Self::mint_tokens(collection_id, &mut collection_info, owner, &serial_numbers, TokenFlags::default());

				if mint.is_ok() {
					// throw event, listing all serial numbers minted from bridging
					// SerialNumbers will never exceed the limit denoted by
					// nft_peg::MaxTokensPerMint Which is set to 50 in the runtime, so this event is
					// safe to list all bridged serial_numbers
					Self::deposit_event(Event::<T>::BridgedMint {
						collection_id,
						serial_numbers,
						owner: owner.clone(),
					});

					Ok(T::DbWeight::get().reads_writes(1, 1))
				} else {
					Err((T::DbWeight::get().reads(1), Error::<T>::BlockedMint.into()))
				}
			},
			_ => Ok(T::DbWeight::get().reads(1)),
		}
	}

	/// Called by mint extrinsics, performs pre mint checks, charges mint fee if applicable,
	/// mints the tokens and requests NFI storage where required
	pub fn do_mint(
		who: T::AccountId,
		collection_id: CollectionUuid,
		collection_info: &mut CollectionInformation<T::AccountId, T::StringLimit>,
		quantity: TokenCount,
		token_owner: &T::AccountId,
		public_mint_info: Option<PublicMintInformation>,
        utility_flags: TokenFlags,
	) -> Result<BoundedVec<SerialNumber, T::MaxTokensPerCollection>, DispatchError> {
		// Perform pre mint checks
		let serial_numbers = Self::pre_mint(collection_id, collection_info, quantity)?;
		let xls20_compatible = collection_info.cross_chain_compatibility.xrpl;
		let metadata_scheme = collection_info.metadata_scheme.clone();

		if let Some(public_mint_info) = public_mint_info {
			if &collection_info.owner != &who {
				// Charge the mint fee for the mint
				Self::charge_mint_fee(
					&who,
					collection_id,
					&collection_info.owner,
					public_mint_info,
					quantity,
				)?;
			}
		}
		// Perform the mint and update storage
		Self::mint_tokens(collection_id, collection_info, token_owner, &serial_numbers, utility_flags)?;
		// Pay XLS20 mint fee and send requests
		if xls20_compatible {
			T::Xls20MintRequest::request_xls20_mint(
				&who,
				collection_id,
				serial_numbers.clone().into_inner(),
				metadata_scheme,
			)?;
		}

		// Request NFI storage if enabled
		T::NFIRequest::request(&who, collection_id, serial_numbers.clone().into_inner())?;
		Ok(serial_numbers)
	}

	/// Perform validity checks on collection_info.
	/// Returns a bounded vec of serial numbers to mint.
	pub fn pre_mint(
		collection_id: CollectionUuid,
		collection_info: &mut CollectionInformation<T::AccountId, T::StringLimit>,
		quantity: TokenCount,
	) -> Result<BoundedVec<SerialNumber, T::MaxTokensPerCollection>, DispatchError> {
		ensure!(quantity <= T::MintLimit::get(), Error::<T>::MintLimitExceeded);
		// minting flag must be enabled on the collection
		ensure!(<UtilityFlags<T>>::get(collection_id).mintable, Error::<T>::MintUtilityBlocked);

		// Quantity must be some
		ensure!(quantity > Zero::zero(), Error::<T>::NoToken);
		// Check we don't exceed the token limit
		ensure!(
			collection_info.collection_issuance.saturating_add(quantity)
				< T::MaxTokensPerCollection::get(),
			Error::<T>::TokenLimitExceeded
		);
		// Cannot mint for a token that was bridged from Ethereum
		ensure!(
			collection_info.origin_chain == OriginChain::Root,
			Error::<T>::AttemptedMintOnBridgedToken
		);

		let previous_serial_number = collection_info.next_serial_number;
		let next_serial_number =
			previous_serial_number.checked_add(quantity).ok_or(Error::<T>::NoAvailableIds)?;

		// Check early that we won't exceed the BoundedVec limit
		ensure!(
			next_serial_number <= T::MaxTokensPerCollection::get(),
			Error::<T>::TokenLimitExceeded
		);

		// Can't mint more than specified max_issuance
		if let Some(max_issuance) = collection_info.max_issuance {
			ensure!(max_issuance >= next_serial_number, Error::<T>::MaxIssuanceReached);
		}

		let serial_numbers_unbounded: Vec<SerialNumber> =
			(previous_serial_number..next_serial_number).collect();
		let serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection> =
			BoundedVec::try_from(serial_numbers_unbounded)
				.map_err(|_| Error::<T>::TokenLimitExceeded)?;

		// Increment next serial number
		let next_serial_number = collection_info.next_serial_number;
		collection_info.next_serial_number =
			next_serial_number.checked_add(quantity).ok_or(Error::<T>::NoAvailableIds)?;

		Ok(serial_numbers)
	}

	pub(crate) fn charge_mint_fee(
		who: &T::AccountId,
		collection_id: CollectionUuid,
		collection_owner: &T::AccountId,
		public_mint_info: PublicMintInformation,
		token_count: TokenCount,
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
				collection_id,
				payment_asset: asset,
				payment_amount: total_fee,
				token_count,
			});
		}

		Ok(())
	}

	/// Perform the mint operation and update storage accordingly.
	pub(crate) fn mint_tokens(
		collection_id: CollectionUuid,
		collection_info: &mut CollectionInformation<T::AccountId, T::StringLimit>,
		token_owner: &T::AccountId,
		serial_numbers: &BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
		utility_flags: TokenFlags,
	) -> DispatchResult {
        T::Migrator::ensure_migrated()?;
		// Update collection issuance
		collection_info.collection_issuance = collection_info
			.collection_issuance
			.checked_add(serial_numbers.len().saturated_into())
			.ok_or(ArithmeticError::Overflow)?;

		ensure!(
			collection_info.collection_issuance <= T::MaxTokensPerCollection::get(),
			Error::<T>::TokenLimitExceeded
		);

		// Update `TokenInfo` mapping
		for serial_number in serial_numbers {
			let token_info = TokenInformation::new(token_owner.clone(), utility_flags);
			TokenInfo::<T>::insert(collection_id, serial_number, token_info);
		}

		// Update `OwnedTokens`
		OwnedTokens::<T>::try_mutate(
			token_owner,
			collection_id,
			|owned_serials| -> DispatchResult {
				match owned_serials.as_mut() {
					Some(owned_serials) => {
						for serial_number in serial_numbers {
							owned_serials
								.try_push(*serial_number)
								.map_err(|_| Error::<T>::TokenLimitExceeded)?;
						}
					},
					None => {
						*owned_serials = Some(serial_numbers.clone());
					},
				}
				Ok(())
			},
		)?;

		// Update CollectionInfo storage
		<CollectionInfo<T>>::insert(collection_id, collection_info);
		Ok(())
	}

	/// Find the tokens owned by an `address` in the given collection
	/// limit return tokens that are larger than the cursor
	/// Returns list of tokens, the sum of all tokens owned by the user
	/// and the new cursor for the next owned SerialNumber not included in the returned list
	pub fn owned_tokens(
		collection_id: CollectionUuid,
		who: &T::AccountId,
		cursor: SerialNumber,
		limit: u16,
	) -> (SerialNumber, TokenCount, Vec<SerialNumber>) {
		if T::Migrator::ensure_migrated().is_err() {
			return (Default::default(), Default::default(), Default::default());
		}
		let mut owned_tokens = match <OwnedTokens<T>>::get(who, collection_id) {
			Some(tokens) => tokens,
			None => return (Default::default(), Default::default(), Default::default()),
		};

		// Sort the vec to ensure no tokens are missed
		owned_tokens.sort();
		// Store the last owned token by this account
		let last_id: SerialNumber = owned_tokens.last().copied().unwrap_or_default();
		// Get the sum of all tokens owned by this account
		let total_owned: TokenCount = owned_tokens.len().saturated_into();

		// Shorten list to any tokens above the cursor and return the limit
		// Note max limit is restricted by MAX_OWNED_TOKENS_LIMIT const
		let response: Vec<SerialNumber> = owned_tokens
			.into_iter()
			.filter(|serial_number| serial_number >= &cursor)
			.take(sp_std::cmp::min(limit, MAX_OWNED_TOKENS_LIMIT).into())
			.collect();

		let new_cursor: SerialNumber = match response.last().copied() {
			Some(highest) => {
				if highest != last_id {
					// There are still tokens remaining that aren't being returned in this call,
					// return the next cursor
					highest.saturating_add(1)
				} else {
					// 0 indicates that this is the end of the owned tokens
					0
				}
			},
			None => 0,
		};

		(new_cursor, total_owned, response)
	}

	/// Find the tokens details for the given collection id
	/// Returns collection owner, name, metadata schema, max issuance,
	/// next available serial number, collection issuance, is_cross_chain_compatible
	pub fn collection_details(
		collection_id: CollectionUuid,
	) -> Result<CollectionDetail<T::AccountId>, DispatchError>
	where
		<T as frame_system::Config>::AccountId: core::default::Default,
	{
		T::Migrator::ensure_migrated()?;
		let collection_info =
			<CollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
		let collection_info = collection_info;
		let owner = collection_info.owner;
		let name = collection_info.name.into();
		let metadata_scheme = collection_info.metadata_scheme.0.into_inner();
		let royalties_schedule: Option<Vec<(T::AccountId, Permill)>> =
			match collection_info.royalties_schedule {
				Some(royalties) => Some(royalties.entitlements.into_inner()),
				None => None,
			};
		let max_issuance = collection_info.max_issuance;
		let next_serial_number = collection_info.next_serial_number;
		let collection_issuance = collection_info.collection_issuance;
		let cross_chain_compatibility = collection_info.cross_chain_compatibility;
		let origin_chain = collection_info.origin_chain;

		Ok(CollectionDetail {
			owner,
			name,
			metadata_scheme,
			royalties_schedule,
			max_issuance,
			next_serial_number,
			collection_issuance,
			cross_chain_compatibility,
			origin_chain,
		})
	}

	/// Create the collection
	pub fn do_create_collection(
		owner: T::AccountId,
		name: BoundedVec<u8, T::StringLimit>,
		initial_issuance: TokenCount,
		max_issuance: Option<TokenCount>,
		token_owner: Option<T::AccountId>,
		metadata_scheme: MetadataScheme,
		royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
		origin_chain: OriginChain,
		cross_chain_compatibility: CrossChainCompatibility,
	) -> Result<u32, DispatchError> {
		T::Migrator::ensure_migrated()?;
		// Check we can issue the new tokens
		let collection_uuid = Self::next_collection_uuid()?;

		// Check max issuance is valid
		if let Some(max_issuance) = max_issuance {
			ensure!(max_issuance > Zero::zero(), Error::<T>::InvalidMaxIssuance);
			ensure!(initial_issuance <= max_issuance, Error::<T>::InvalidMaxIssuance);
			ensure!(
				max_issuance <= T::MaxTokensPerCollection::get(),
				Error::<T>::InvalidMaxIssuance
			);
		}

		// Validate collection attributes
		ensure!(!name.is_empty(), Error::<T>::CollectionNameInvalid);
		ensure!(core::str::from_utf8(&name).is_ok(), Error::<T>::CollectionNameInvalid);
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

		// Mint the collection tokens
		if initial_issuance > Zero::zero() {
			ensure!(initial_issuance <= T::MintLimit::get(), Error::<T>::MintLimitExceeded);
			// XLS-20 compatible collections cannot have an initial issuance
			// This is to prevent the fees from being bypassed in the mint function.
			// Instead the user should specify 0 initial_issuance and use the mint function to
			// mint tokens
			ensure!(!cross_chain_compatibility.xrpl, Error::<T>::InitialIssuanceNotZero);

			// mint initial tokens to token_owner or owner
			let token_owner = token_owner.unwrap_or(owner.clone());
			let serial_numbers_unbounded: Vec<SerialNumber> = (0..initial_issuance).collect();
			let serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection> =
				BoundedVec::try_from(serial_numbers_unbounded)
					.map_err(|_| Error::<T>::TokenLimitExceeded)?;

			// Update `TokenInfo` mapping
			for &serial_number in &serial_numbers {
				let token_info = TokenInformation::new(token_owner.clone(), TokenFlags::default());
				TokenInfo::<T>::insert(collection_uuid, serial_number, token_info);
			}

			// Update `OwnedTokens`
			OwnedTokens::<T>::try_mutate(
				token_owner,
				collection_uuid,
				|owned_serials| -> DispatchResult {
					match owned_serials.as_mut() {
						Some(owned_serials) => {
							for serial_number in serial_numbers {
								owned_serials
									.try_push(serial_number)
									.map_err(|_| Error::<T>::TokenLimitExceeded)?;
							}
						},
						None => {
							*owned_serials = Some(serial_numbers.clone());
						},
					}
					Ok(())
				},
			)?;
		}

		let collection_info = CollectionInformation {
			owner: owner.clone(),
			name: name.clone(),
			metadata_scheme: metadata_scheme.clone(),
			royalties_schedule: royalties_schedule.clone(),
			max_issuance,
			origin_chain: origin_chain.clone(),
			next_serial_number: initial_issuance,
			collection_issuance: initial_issuance,
			cross_chain_compatibility,
		};
		<CollectionInfo<T>>::insert(collection_uuid, collection_info);

		// will not overflow, asserted prior qed.
		<NextCollectionId<T>>::mutate(|i| *i += u32::one());

		// Add some code to the EVM
		T::OnNewAssetSubscription::on_asset_create(
			collection_uuid,
			ERC721_PRECOMPILE_ADDRESS_PREFIX,
		);

		Self::deposit_event(Event::<T>::CollectionCreate {
			collection_uuid,
			initial_issuance,
			max_issuance,
			collection_owner: owner,
			metadata_scheme,
			name: name.into_inner(),
			royalties_schedule,
			origin_chain,
			compatibility: cross_chain_compatibility,
		});
		Ok(collection_uuid)
	}

	/// Check token locks and burn tokens
	pub fn do_burn(
		who: &T::AccountId,
		collection_id: CollectionUuid,
		serial_number: SerialNumber,
	) -> DispatchResult {
		T::Migrator::ensure_migrated()?;
		ensure!(<UtilityFlags<T>>::get(collection_id).burnable, Error::<T>::BurnUtilityBlocked);

		CollectionInfo::<T>::try_mutate(collection_id, |maybe_collection_info| -> DispatchResult {
			let collection_info =
				maybe_collection_info.as_mut().ok_or(Error::<T>::NoCollectionFound)?;

			collection_info.collection_issuance =
				collection_info.collection_issuance.saturating_sub(1);

			TokenInfo::<T>::try_mutate(
				collection_id,
				serial_number,
				|maybe_token_info| -> DispatchResult {
					let token_info = maybe_token_info.as_mut().ok_or(Error::<T>::NoToken)?;
					let token_owner = &token_info.owner;
					ensure!(token_info.lock_status.is_none(), Error::<T>::TokenLocked);
					if let Some(burn_authority) = token_info.utility_flags.burn_authority {
						ensure!(
							burn_authority.has_burn_authority(
								&collection_info.owner,
								token_owner,
								who,
							),
							Error::<T>::InvalidBurnAuthority
						);
					} else {
						ensure!(token_owner == who, Error::<T>::NotTokenOwner);
					}

					*maybe_token_info = None;
					Ok(())
				},
			)?;

			OwnedTokens::<T>::try_mutate(
				who,
				collection_id,
				|maybe_owned_serials| -> DispatchResult {
					if let Some(owned_serials) = maybe_owned_serials {
						owned_serials.retain(|serial| serial != &serial_number);
						// If no tokens remain, remove the entry completely
						if owned_serials.is_empty() {
							*maybe_owned_serials = None;
						}
					}
					Ok(())
				},
			)?;

			// Remove approvals for this token
			T::OnTransferSubscription::on_nft_transfer(&(collection_id, serial_number));

			// Remove any NFI data associated with this token
			T::NFIRequest::on_burn((collection_id, serial_number));
			Ok(())
		})
	}

	/// Enables XLS-20 compatibility for a collection with 0 issuance
	pub fn enable_xls20_compatibility(
		who: T::AccountId,
		collection_id: CollectionUuid,
	) -> DispatchResult {
		T::Migrator::ensure_migrated()?;
		let mut collection_info =
			CollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;

		// Caller must be collection owner
		ensure!(collection_info.owner == who, Error::<T>::NotCollectionOwner);
		// Collection issuance must be 0 (i.e. no tokens minted)
		ensure!(
			collection_info.collection_issuance.is_zero(),
			Error::<T>::CollectionIssuanceNotZero
		);

		collection_info.cross_chain_compatibility.xrpl = true;
		CollectionInfo::<T>::insert(collection_id, collection_info);
		Ok(())
	}

	/// Sets the owner of a collection to a new account
	pub fn do_set_owner(
		previous_owner: T::AccountId,
		collection_id: CollectionUuid,
		new_owner: T::AccountId,
	) -> DispatchResult {
		T::Migrator::ensure_migrated()?;
		let mut collection_info =
			<CollectionInfo<T>>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
		ensure!(collection_info.owner == previous_owner, Error::<T>::NotCollectionOwner);
		collection_info.owner = new_owner.clone();
		<CollectionInfo<T>>::insert(collection_id, collection_info);
		Self::deposit_event(Event::<T>::OwnerSet { collection_id, new_owner });
		Ok(())
	}

	/// The account ID of the NFT pallet.
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	/// Sets the additional data for a token.
	/// If `additional_data` is `None`, it removes the existing data.
	pub fn do_set_additional_data(
		token_id: TokenId,
		additional_data: Option<BoundedVec<u8, T::MaxDataLength>>,
	) -> DispatchResult {
		match &additional_data {
			None => AdditionalTokenData::<T>::remove(token_id),
			Some(data) => {
				ensure!(!data.is_empty(), Error::<T>::InvalidAdditionalData);
				AdditionalTokenData::<T>::insert(token_id, data);
			},
		}
		Self::deposit_event(Event::<T>::AdditionalDataSet { token_id, additional_data });
		Ok(())
	}
}

impl<T: Config> NFTExt for Pallet<T> {
	type AccountId = T::AccountId;
	type StringLimit = T::StringLimit;

	fn do_mint(
		origin: Self::AccountId,
		collection_id: CollectionUuid,
		quantity: TokenCount,
		token_owner: Option<Self::AccountId>,
	) -> DispatchResult {
		T::Migrator::ensure_migrated()?;
		Self::mint(RawOrigin::Signed(origin).into(), collection_id, quantity, token_owner)
	}

	fn do_transfer(
		origin: &Self::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: Vec<SerialNumber>,
		new_owner: &Self::AccountId,
	) -> DispatchResult {
		T::Migrator::ensure_migrated()?;
		let bounded_serials =
			BoundedVec::try_from(serial_numbers).map_err(|_| Error::<T>::TokenLimitExceeded)?;
		Self::do_transfer(collection_id, bounded_serials, origin, new_owner)
	}

	fn do_create_collection(
		owner: Self::AccountId,
		name: BoundedVec<u8, Self::StringLimit>,
		initial_issuance: TokenCount,
		max_issuance: Option<TokenCount>,
		token_owner: Option<Self::AccountId>,
		metadata_scheme: MetadataScheme,
		royalties_schedule: Option<RoyaltiesSchedule<Self::AccountId>>,
		origin_chain: OriginChain,
		cross_chain_compatibility: CrossChainCompatibility,
	) -> Result<CollectionUuid, DispatchError> {
		T::Migrator::ensure_migrated()?;
		Self::do_create_collection(
			owner,
			name,
			initial_issuance,
			max_issuance,
			token_owner,
			metadata_scheme,
			royalties_schedule,
			origin_chain,
			cross_chain_compatibility,
		)
	}

	fn get_token_owner(token_id: &TokenId) -> Option<Self::AccountId> {
		let token_info = TokenInfo::<T>::get(token_id.0, token_id.1)?;
		Some(token_info.owner)
	}

	fn token_exists(token_id: &TokenId) -> bool {
		TokenInfo::<T>::contains_key(token_id.0, token_id.1)
	}

	fn get_collection_issuance(
		collection_id: CollectionUuid,
	) -> Result<(TokenCount, Option<TokenCount>), DispatchError> {
		T::Migrator::ensure_migrated()?;
		let collection_info =
			CollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
		Ok((collection_info.collection_issuance, collection_info.max_issuance))
	}

	fn get_public_mint_info(
		collection_id: CollectionUuid,
	) -> Result<PublicMintInformation, DispatchError> {
		let mint_info =
			<PublicMintInfo<T>>::get(collection_id).ok_or(Error::<T>::PublicMintDisabled)?;
		Ok(mint_info)
	}

	fn transfer_collection_ownership(
		who: Self::AccountId,
		collection_id: CollectionUuid,
		new_owner: Self::AccountId,
	) -> DispatchResult {
		T::Migrator::ensure_migrated()?;
		Self::do_set_owner(who, collection_id, new_owner)
	}

	fn get_royalties_schedule(
		collection_id: CollectionUuid,
	) -> Result<Option<RoyaltiesSchedule<Self::AccountId>>, DispatchError> {
		T::Migrator::ensure_migrated()?;
		let collection_info =
			CollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
		Ok(collection_info.royalties_schedule)
	}

	fn enable_xls20_compatibility(
		who: Self::AccountId,
		collection_id: CollectionUuid,
	) -> DispatchResult {
		T::Migrator::ensure_migrated()?;
		Self::enable_xls20_compatibility(who, collection_id)
	}

	fn next_collection_uuid() -> Result<CollectionUuid, DispatchError> {
		Self::next_collection_uuid()
	}

	fn increment_collection_uuid() -> DispatchResult {
		ensure!(<NextCollectionId<T>>::get().checked_add(1).is_some(), Error::<T>::NoAvailableIds);
		<NextCollectionId<T>>::mutate(|i| *i += u32::one());
		Ok(())
	}

	fn get_token_lock(token_id: TokenId) -> Option<TokenLockReason> {
		if T::Migrator::ensure_migrated().is_err() {
			return None;
		}
		<TokenInfo<T>>::get(token_id.0, token_id.1)?.lock_status
	}

	fn set_token_lock(
		token_id: TokenId,
		lock_reason: TokenLockReason,
		who: Self::AccountId,
	) -> DispatchResult {
		T::Migrator::ensure_migrated()?;
		TokenInfo::<T>::try_mutate(token_id.0, token_id.1, |maybe_token_info| -> DispatchResult {
			let token_info = maybe_token_info.as_mut().ok_or(Error::<T>::NoToken)?;
			ensure!(token_info.lock_status.is_none(), Error::<T>::TokenLocked);
			ensure!(token_info.owner == who, Error::<T>::NotTokenOwner);
			token_info.lock_status = Some(lock_reason);
			Ok(())
		})
	}

	fn remove_token_lock(token_id: TokenId) -> DispatchResult {
		TokenInfo::<T>::try_mutate(token_id.0, token_id.1, |maybe_token_info| -> DispatchResult {
			let token_info = maybe_token_info.as_mut().ok_or(Error::<T>::NoToken)?;
			token_info.lock_status = None;
			Ok(())
		})
	}

	fn get_collection_owner(
		collection_id: CollectionUuid,
	) -> Result<Self::AccountId, DispatchError> {
		T::Migrator::ensure_migrated()?;
		let collection_info =
			CollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
		Ok(collection_info.owner)
	}

	fn do_burn(
		who: Self::AccountId,
		collection_id: CollectionUuid,
		serial_number: SerialNumber,
	) -> DispatchResult {
		T::Migrator::ensure_migrated()?;
		Self::do_burn(&who, collection_id, serial_number)
	}

	fn get_cross_chain_compatibility(
		collection_id: CollectionUuid,
	) -> Result<CrossChainCompatibility, DispatchError> {
		T::Migrator::ensure_migrated()?;
		let collection_info =
			CollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
		Ok(collection_info.cross_chain_compatibility)
	}
}

impl<T: Config> NFTCollectionInfo for Pallet<T> {
	type AccountId = T::AccountId;
	type StringLimit = T::StringLimit;

	fn get_collection_info(
		collection_id: CollectionUuid,
	) -> Result<CollectionInformation<Self::AccountId, Self::StringLimit>, DispatchError> {
		T::Migrator::ensure_migrated()?;
		CollectionInfo::<T>::get(collection_id).ok_or(Error::<T>::NoCollectionFound.into())
	}
}

impl<T: Config> NFTMinter for Pallet<T> {
	type AccountId = T::AccountId;

	/// Mint bridged tokens from Ethereum or XRPL
	/// Note that in an attempt to match the serial numbers between chains, we will mint
	/// the serial numbers as they are provided. If a serial number already exists, we will not mint
	fn mint_bridged_nft(
		owner: &Self::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: Vec<SerialNumber>,
	) -> WeightedDispatchResult {
		T::Migrator::ensure_migrated().map_err(|e| (Weight::zero(), e))?;
		Self::mint_bridged_token(owner, collection_id, serial_numbers)
	}
}
