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

use core::fmt::Write;
use ethabi::{ParamType, Token};
use frame_support::{ensure, traits::Get, weights::Weight, BoundedVec, PalletId};
pub use pallet::*;
use seed_pallet_common::{EthereumBridge, EthereumEventSubscriber, NFTMinter};
use seed_primitives::{
	CollectionUuid, MetadataScheme, OriginChain, SerialNumber, WeightedDispatchResult,
};
use sp_core::{H160, U256};
use sp_runtime::{traits::AccountIdConversion, DispatchError, SaturatedConversion};
use sp_std::{boxed::Box, vec, vec::Vec};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
mod types;
mod weights;

pub use types::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, transactional};
	use frame_system::{ensure_signed, pallet_prelude::*};
	use seed_primitives::EthAddress;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_nft::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type PalletId: Get<PalletId>;
		#[pallet::constant]
		type DelayLength: Get<BlockNumberFor<Self>>;
		type MaxAddresses: Get<u32>;
		type MaxTokensPerMint: Get<u32>;
		type EthBridge: EthereumBridge;
		// Defines the weight info trait.
		type NftPegWeightInfo: WeightInfo;
		type MaxCollectionsPerWithdraw: Get<u32>;
		type MaxSerialsPerWithdraw: Get<u32>;
		type NFTMinter: NFTMinter<AccountId = Self::AccountId>;
	}

	#[pallet::storage]
	pub type ContractAddress<T: Config> = StorageValue<_, EthAddress, ValueQuery>;

	// Map Ethereum contract address to Root collection id
	#[pallet::storage]
	pub type EthToRootNft<T: Config> =
		StorageMap<_, Twox64Concat, EthAddress, CollectionUuid, OptionQuery>;

	// Map Root collection id to Ethereum contract address
	#[pallet::storage]
	pub type RootNftToErc721<T: Config> =
		StorageMap<_, Twox64Concat, CollectionUuid, EthAddress, OptionQuery>;

	// Map BlockedMintId to tokens
	#[pallet::storage]
	pub type BlockedTokens<T: Config> =
		StorageMap<_, Twox64Concat, BlockedMintId, BlockedTokenInfo<T>, OptionQuery>;

	/// The next available BlockedMintId
	#[pallet::storage]
	pub type NextBlockedMintId<T> = StorageValue<_, BlockedMintId, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Send more addresses than are allowed
		ExceedsMaxAddresses,
		/// Sent more tokens than are allowed
		ExceedsMaxTokens,
		/// The abi data passed in could not be decoded
		InvalidAbiEncoding,
		/// The prefix uint in the abi encoded data was invalid
		InvalidAbiPrefix,
		/// No collection info exists
		NoCollectionFound,
		/// No mapped token was stored for bridging the token back to the bridged chain
		NoMappedTokenExists,
		/// Tried to bridge a token that originates from Root, which is not yet supported
		NoPermissionToBridge,
		/// The state sync decoding feature is not implemented
		StateSyncDisabled,
		/// Multiple tokens were passed from contract, but amounts were unqeual per each array
		TokenListLengthMismatch,
		/// The length of the given vec exceeds the maximal allowed length limit
		ExceedsMaxVecLength,
		/// No blocked tokens for the given id
		NoBlockedTokensFound,
		/// Blocked tokens can only be reclaimed by the destination address
		NotBlockedTokenDestination,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An ERC721 deposit was made
		Erc721Deposit { destination: T::AccountId },
		/// Bridged ERC721 tokens were minted
		Erc721Mint {
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerMint>,
			owner: T::AccountId,
		},
		/// Bridged ERC721 tokens were unable to be minted due to collection limit being reached
		ERC721Blocked {
			blocked_mint_id: BlockedMintId,
			destination_address: T::AccountId,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxSerialsPerWithdraw>,
		},
		/// An ERC721 withdraw was made
		Erc721Withdraw {
			origin: T::AccountId,
			collection_ids: BoundedVec<CollectionUuid, T::MaxCollectionsPerWithdraw>,
			serial_numbers: BoundedVec<
				BoundedVec<SerialNumber, T::MaxSerialsPerWithdraw>,
				T::MaxCollectionsPerWithdraw,
			>,
			destination: H160,
		},
		/// The NFT-peg contract address was set
		ContractAddressSet { contract: H160 },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight(T::NftPegWeightInfo::set_contract_address())]
		pub fn set_contract_address(origin: OriginFor<T>, contract: H160) -> DispatchResult {
			ensure_root(origin)?;
			ContractAddress::<T>::put(contract);
			Self::deposit_event(Event::<T>::ContractAddressSet { contract });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::NftPegWeightInfo::withdraw())]
		#[transactional]
		pub fn withdraw(
			origin: OriginFor<T>,
			collection_ids: BoundedVec<CollectionUuid, T::MaxCollectionsPerWithdraw>,
			serial_numbers: BoundedVec<
				BoundedVec<SerialNumber, T::MaxSerialsPerWithdraw>,
				T::MaxCollectionsPerWithdraw,
			>,
			destination: H160,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_withdrawal(who, collection_ids, serial_numbers, destination, None)?;
			Ok(())
		}

		/// Withdraw blocked tokens, must be called by the destination defined in `BlockedTokens`
		#[pallet::call_index(2)]
		#[pallet::weight(T::NftPegWeightInfo::reclaim_blocked_nfts())]
		#[transactional]
		pub fn reclaim_blocked_nfts(
			origin: OriginFor<T>,
			blocked_mint_id: BlockedMintId,
			destination: H160,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_reclaim_blocked_nfts(who, blocked_mint_id, destination)?;
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	fn decode_deposit_event(data: &[u8]) -> WeightedDispatchResult {
		let mut weight = Weight::zero();
		let abi_decoded = match ethabi::decode(
			&[
				// Bit to predetermine which function to route to; unused here
				ParamType::Uint(32),
				// Token addresses
				ParamType::Array(Box::new(ParamType::Address)),
				// Token ids
				ParamType::Array(Box::new(ParamType::Array(Box::new(ParamType::Uint(32))))),
				// Receiver of tokens
				ParamType::Address,
			],
			data,
		) {
			Ok(abi) => abi,
			Err(_) => return Err((weight, Error::<T>::InvalidAbiEncoding.into())),
		};

		if let [Token::Uint(_), Token::Array(token_addresses), Token::Array(token_ids), Token::Address(destination)] =
			abi_decoded.as_slice()
		{
			let token_addresses: Vec<H160> = token_addresses
				.into_iter()
				.filter_map(|k| {
					if let Token::Address(decoded) = k {
						Some(decoded.clone())
					} else {
						None
					}
				})
				.collect();

			let token_addresses: BoundedVec<H160, T::MaxAddresses> =
				BoundedVec::try_from(token_addresses)
					.map_err(|_| (weight, Error::<T>::InvalidAbiEncoding.into()))?;

			// Turn nested ethabi Tokens Vec into Nested BoundedVec of root types
			let mut new_token_ids: BoundedVec<
				BoundedVec<SerialNumber, T::MaxTokensPerMint>,
				T::MaxAddresses,
			> = BoundedVec::default();

			for token_id in token_ids.iter() {
				let Token::Array(token) = token_id else {
					return Err((weight, Error::<T>::ExceedsMaxTokens.into()));
				};

				let vec: Vec<SerialNumber> = token
					.iter()
					.filter_map(|j| {
						if let Token::Uint(token_id) = j {
							let token_id: SerialNumber = (*token_id).saturated_into();
							Some(token_id.clone())
						} else {
							None
						}
					})
					.collect();

				let vec = BoundedVec::try_from(vec)
					.map_err(|_| (weight, Error::<T>::ExceedsMaxTokens.into()))?;
				new_token_ids
					.try_push(vec)
					.map_err(|_| (weight, Error::<T>::ExceedsMaxAddresses.into()))?;
			}

			ensure!(
				token_addresses.len() == new_token_ids.len(),
				(weight, Error::<T>::TokenListLengthMismatch.into())
			);

			let token_information =
				GroupedTokenInfo::new(new_token_ids, token_addresses, destination.clone().into());

			let do_deposit_weight = Self::do_deposit(token_information, *destination)
				.map_err(|(deposit_weight, err)| (weight.saturating_add(deposit_weight), err))?;

			weight = T::DbWeight::get().writes(1).saturating_add(do_deposit_weight);

			Ok(weight)
		} else {
			// input data should be valid, we do not expect to fail here
			Err((weight, Error::<T>::InvalidAbiEncoding.into()))
		}
	}

	// TODO implement state sync feature for collection_owner, name and metadata
	fn decode_state_sync_event(_data: &[u8]) -> WeightedDispatchResult {
		Err((Weight::zero(), Error::<T>::StateSyncDisabled.into()))
	}

	// Accept some representation of one or more tokens from an outside source, and create a
	// Root-side representation of them Expects ERC721 tokens sent and verified through the existing
	// bridge
	fn do_deposit(token_info: GroupedTokenInfo<T>, destination: H160) -> WeightedDispatchResult {
		let mut weight = Weight::zero();

		let destination: T::AccountId = destination.into();
		let name = BoundedVec::truncate_from(b"bridged-collection".to_vec());

		for current_token in token_info.tokens.iter() {
			// Assign collection owner to pallet. User can claim it later
			let collection_owner_account =
				<T as pallet_nft::Config>::PalletId::get().into_account_truncating();
			// Check if incoming collection is in CollectionMapping, if not, create as
			// new collection along with its Eth > Root mapping
			let collection_id: CollectionUuid =
				match EthToRootNft::<T>::get(current_token.token_address) {
					Some(collection_id) => collection_id,
					None => {
						let mut h160_addr = sp_std::Writer::default();
						write!(&mut h160_addr, "ethereum://{:?}/", current_token.token_address)
							.expect("Not written");
						let metadata_scheme =
							MetadataScheme::try_from(h160_addr.inner().clone().as_slice())
								.map_err(|_| (weight, (Error::<T>::ExceedsMaxVecLength).into()))?;
						// Collection doesn't exist, create a new collection
						let new_collection_id = pallet_nft::Pallet::<T>::do_create_collection(
							collection_owner_account,
							name.clone(),
							0_u32,
							None,
							Some(destination.clone()),
							metadata_scheme.clone(),
							None,
							OriginChain::Ethereum,
							sp_std::default::Default::default(),
						)
						.map_err(|err| (weight, err))?;

						// Populate both mappings, building the relationship between the bridged
						// chain token, and this chain's token
						EthToRootNft::<T>::insert(current_token.token_address, new_collection_id);
						RootNftToErc721::<T>::insert(
							new_collection_id,
							current_token.token_address,
						);
						new_collection_id
					},
				};

			let serial_numbers = current_token.token_ids.clone().into_inner();

			// Mint the tokens
			let mint_result =
				T::NFTMinter::mint_bridged_nft(&destination, collection_id, serial_numbers.clone());

			match mint_result {
				Ok(mint_weight) => {
					weight = weight.saturating_add(mint_weight);
				},
				// If mint fails, add tokens to `BlockedTokens`
				Err((mint_weight, err)) => {
					weight = weight.saturating_add(mint_weight);

					let blocked_mint_id = NextBlockedMintId::<T>::get();

					// Rebound to `MaxSerialsPerWithdraw` - this shouldn't fail as
					// it is the same as `MaxTokensPerMint`
					let serial_numbers: BoundedVec<SerialNumber, T::MaxSerialsPerWithdraw> =
						BoundedVec::try_from(serial_numbers)
							.map_err(|_| (weight, Error::<T>::ExceedsMaxTokens.into()))?;

					<BlockedTokens<T>>::insert(
						blocked_mint_id,
						BlockedTokenInfo {
							collection_id,
							serial_numbers: serial_numbers.clone(),
							destination_address: destination.clone(),
						},
					);
					<NextBlockedMintId<T>>::mutate(|i| *i += 1);

					// Throw event with values necessary to reclaim tokens
					Self::deposit_event(Event::<T>::ERC721Blocked {
						blocked_mint_id,
						collection_id,
						serial_numbers,
						destination_address: destination,
					});

					weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 2));

					return Err((weight, err));
				},
			}

			// Throw event, listing all bridged tokens minted
			Self::deposit_event(Event::<T>::Erc721Mint {
				collection_id,
				serial_numbers: current_token.token_ids.clone(),
				owner: destination.clone(),
			});
			weight = weight.saturating_add(T::DbWeight::get().writes(2));
		}

		Self::deposit_event(Event::<T>::Erc721Deposit { destination });
		Ok(weight)
	}

	// Accepts one or more Ethereum originated ERC721 tokens to be sent back over the bridge
	pub fn do_withdrawal(
		who: T::AccountId,
		collection_ids: BoundedVec<CollectionUuid, T::MaxCollectionsPerWithdraw>,
		serial_numbers: BoundedVec<
			BoundedVec<SerialNumber, T::MaxSerialsPerWithdraw>,
			T::MaxCollectionsPerWithdraw,
		>,
		// Ethereum address to deposit the tokens into
		destination: H160,
		blocked_mint_id: Option<BlockedMintId>,
	) -> Result<u64, DispatchError> {
		ensure!(collection_ids.len() == serial_numbers.len(), Error::<T>::TokenListLengthMismatch);
		let mut source_collection_ids = Vec::with_capacity(collection_ids.len());
		let mut source_serial_numbers = Vec::with_capacity(collection_ids.len());

		for (idx, collection_id) in (&collection_ids).into_iter().enumerate() {
			let collection_info = <pallet_nft::CollectionInfo<T>>::get(collection_id)
				.ok_or(Error::<T>::NoCollectionFound)?;

			// At the time of writing, only Ethereum-originated NFTs can be bridged back.
			ensure!(
				collection_info.origin_chain == OriginChain::Ethereum,
				Error::<T>::NoPermissionToBridge
			);

			let mut current_serial_numbers = Vec::with_capacity(serial_numbers[idx].len());

			if let Some(blocked_mint_id) = blocked_mint_id {
				let blocked_tokens = BlockedTokens::<T>::get(blocked_mint_id)
					.ok_or(Error::<T>::NoBlockedTokensFound)?;

				for serial_number in &blocked_tokens.serial_numbers {
					current_serial_numbers.push(Token::Uint(U256::from(*serial_number)));
				}

				<BlockedTokens<T>>::remove(blocked_mint_id);
			} else {
				for serial_number in &serial_numbers[idx] {
					pallet_nft::Pallet::<T>::do_burn(&who, *collection_id, *serial_number)?;
					current_serial_numbers.push(Token::Uint(U256::from(*serial_number)));
				}
			}

			// Lookup the source chain token id for this token and remove it from the mapping
			let token_address =
				RootNftToErc721::<T>::get(collection_id).ok_or(Error::<T>::NoMappedTokenExists)?;
			source_collection_ids.push(Token::Address(token_address));
			source_serial_numbers.push(Token::Array(current_serial_numbers));
		}

		let source = <T as pallet::Config>::PalletId::get().into_account_truncating();

		let message = ethabi::encode(&[
			Token::Array(source_collection_ids),
			Token::Array(source_serial_numbers),
			Token::Address(destination),
		]);

		let event_proof_id =
			T::EthBridge::send_event(&source, &ContractAddress::<T>::get(), &message)?;

		Self::deposit_event(Event::<T>::Erc721Withdraw {
			origin: who,
			collection_ids,
			serial_numbers,
			destination,
		});
		Ok(event_proof_id)
	}

	fn do_reclaim_blocked_nfts(
		who: T::AccountId,
		blocked_mint_id: BlockedMintId,
		destination: H160,
	) -> Result<(), DispatchError> {
		let blocked_tokens =
			BlockedTokens::<T>::get(blocked_mint_id).ok_or(Error::<T>::NoBlockedTokensFound)?;

		ensure!(blocked_tokens.destination_address == who, Error::<T>::NotBlockedTokenDestination);

		Self::do_withdrawal(
			who,
			BoundedVec::truncate_from(vec![blocked_tokens.collection_id]),
			BoundedVec::truncate_from(vec![blocked_tokens.serial_numbers]),
			destination,
			Some(blocked_mint_id),
		)?;

		Ok(())
	}
}

impl<T: Config> EthereumEventSubscriber for Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<H160>,
{
	type Address = <T as Config>::PalletId;
	type SourceAddress = GetContractAddress<T>;

	fn on_event(_source: &H160, data: &[u8]) -> WeightedDispatchResult {
		let weight = Weight::zero();

		// Decode prefix from first 32 bytes of data
		let prefix_decoded = match ethabi::decode(&[ParamType::Uint(32)], &data[..32]) {
			Ok(abi) => abi,
			Err(_) => return Err((weight, Error::<T>::InvalidAbiPrefix.into())),
		};

		// match prefix and route to specific decoding path
		if let [Token::Uint(prefix)] = prefix_decoded.as_slice() {
			let prefix: u32 = (*prefix).saturated_into();
			// TODO: get the correct split of prefix versus rest of data to optimize decoding i.e.
			// let data = &data[~33..];

			match MessageDestination::from(prefix) {
				MessageDestination::Deposit => Self::decode_deposit_event(data),
				MessageDestination::StateSync => Self::decode_state_sync_event(data),
				MessageDestination::Other => Err((weight, Error::<T>::InvalidAbiPrefix.into())),
			}
		} else {
			return Err((weight, Error::<T>::InvalidAbiPrefix.into()));
		}
	}
}
