#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;

use ethabi::{ParamType, Token, Uint};
// Pallet for managing NFTs bridged from *x* chain
use frame_support::{ensure, traits::Get, PalletId};
use pallet_nft::{OriginChain, TokenCount};
use scale_info::TypeInfo;
use sp_core::{H160, U256};
use sp_runtime::{
	traits::{AccountIdConversion, Saturating, Zero},
	DispatchError,
};

use codec::{Decode, Encode, MaxEncodedLen};
pub use pallet::*;
use seed_pallet_common::{log, EthereumEventSubscriber};
use seed_primitives::{AccountId20, Balance, CollectionUuid, EthAddress, SerialNumber};
use sp_std::{boxed::Box, vec::Vec};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use seed_primitives::EthAddress;
	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_nft::Config {
		type PalletId: Get<PalletId>;
		#[pallet::constant]
		type DelayLength: Get<Self::BlockNumber>;
		type MaxAddresses: Get<u32>;
		type MaxIdsPerCollection: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn contract_address)]
	pub type ContractAddress<T> = StorageValue<_, EthAddress, ValueQuery>;

	// Map Ethereum Collection ids to Root collection ids
	#[pallet::storage]
	#[pallet::getter(fn mapped_collections)]
	pub type CollectionsMapping<T: Config> = StorageMap<_, Twox64Concat, H160, u32, OptionQuery>;

	// Store nfts to be minted by the blocks they should be minted in
	#[pallet::storage]
	#[pallet::getter(fn delayed_mints)]
	pub type DelayedMints<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, (H160, Vec<H160>, Vec<Vec<U256>>, H160), OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// The abi data passed in could not be decoded
		InvalidAbiEncoding,
		/// The prefix uint in the abi encoded data was invalid
		InvalidAbiPrefix,
		/// The state sync decoding feature is not implemented
		StateSyncDisabled,
		/// Multiple tokens were passed from contract, but amounts were unqeual per each array
		UnequalTokenCount,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
	where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
	{
		fn on_initialize(block: T::BlockNumber) -> Weight {
			let mut weight = 0;

			if let Some((source, token_addresses, token_ids, destination)) = Self::delayed_mints(block) {
				Self::process_nfts_multiple(
					&source,
					token_addresses,
					token_ids,
					destination
				);
			}

			weight
		}
	}
}

/// A deposit event made by the ERC20 peg contract on Ethereum
#[derive(Debug, Default, Clone, PartialEq, Decode, Encode, TypeInfo, MaxEncodedLen)]
pub struct BridgedNftEvent {
	pub token_id: U256,
	/// The Seed beneficiary address
	pub beneficiary: H160,
}

pub struct GetEthAddress<T>(PhantomData<T>);

impl<T: Config> Get<H160> for GetEthAddress<T> {
	fn get() -> H160 {
		Pallet::<T>::contract_address()
	}
}

// pub struct BridgedNftInfo {
// 	source: H160,
// 	token_addresses: Vec<>,
// 	token_ids: token_ids.clone(),
// 	contract_owners: contract_owners.clone(),
// 	destination: destination.clone(),
// }

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	fn decode_deposit_event(source: &sp_core::H160, data: &[u8]) -> Result<(u64), (u64, DispatchError)> {
		let mut weight = 0;

		let abi_decoded = match ethabi::decode(
			&[
				ParamType::Array(Box::new(ParamType::Address)),
				ParamType::Array(Box::new(ParamType::Array(Box::new(ParamType::Uint(32))))),
				ParamType::Address,
			],
			data,
		) {
			Ok(abi) => abi,
			Err(_) => return Err((weight, Error::<T>::InvalidAbiEncoding.into())),
		};

		if let [
			Token::Array(token_addresses),
			Token::Array(token_ids),
			Token::Address(destination)
		] =
		abi_decoded.as_slice()
		{
			let token_addresses: Vec<&H160> = token_addresses.into_iter().filter_map(|k| {
				if let Token::Address(decoded) = k {
					Some(decoded)
				} else {
					None
				}
			}).collect();

			let token_ids: Vec<Vec<&U256>> = token_ids.iter().filter_map(|k| {
				if let Token::Array(token_ids) = k {
					let new: Vec<&U256> = token_ids.iter().filter_map(|j| {
						if let Token::Uint(token_id) = j {
							Some(token_id)
						} else {
							None
						}
					})
					.collect();
					Some(new)
				} else {
					None
				}
			}).collect();


			// let process_mint_at_block = <frame_system::Pallet<T>>::block_number() + T::DelayLength::get();
			let process_mint_at_block = <frame_system::Pallet<T>>::block_number().saturating_add(
				T::DelayLength::get()
			);
			DelayedMints::insert(process_mint_at_block, (
				source,
				token_addresses,
				token_ids,
				destination
			)
			);

			weight = T::DbWeight::get().writes(1);

			Ok(weight)
		} else {
			// input data should be valid, we do not expect to fail here
			Err((weight, Error::<T>::InvalidAbiEncoding.into()))
		}
	}

	// TODO implement state sync feature for collection_owner, name and metadata
	fn decode_state_sync_event(data: &[u8]) -> Result<(), DispatchError> {
		Err(Error::<T>::StateSyncDisabled.into())
	}

	// Non functional atm. This needs to accept and process multiple tokens
	fn process_nfts_multiple(
		source: &H160,
		token_addresses: Vec<H160>,
		token_ids: Vec<Vec<U256>>,
		destination: H160,
	) -> Result<(), DispatchError> {
		// Assumed values for each. We may need to change this later
		let initial_issuance = token_addresses.len();
		let max_issuance = None;
		let royalties_schedule = None;
		let destination: T::AccountId = destination.into();
		let source_chain = OriginChain::Ethereum;
		let metadata_scheme = pallet_nft::MetadataScheme::Ethereum(Self::contract_address());

		let name = "".encode();

		ensure!(token_addresses.len() == token_ids.len(), Error::<T>::UnequalTokenCount);

		token_addresses.iter().enumerate().for_each(|(((collection_idx, address)))| {

				// Get the list of token ids corresponding to the current collection
				let current_collections_tokens = token_ids[collection_idx];

				// Assign collection owner to pallet. User can claim it later
				let collection_owner_account =
					<T as pallet_nft::Config>::PalletId::get().into_account_truncating();

				// Check if incoming collection is in CollectionMapping, if not, create a
				// new collection along with its Eth > Root mapping
				if let Some(root_collection_id) = Self::mapped_collections(address) {
					pallet_nft::Pallet::<T>::do_mint_multiple_with_ids(&destination, root_collection_id, current_collections_tokens)?;
				} else {
					let new_collection_id = pallet_nft::Pallet::<T>::do_create_collection(
						collection_owner_account,
						name.clone(),
						initial_issuance as u32,
						max_issuance,
						// Token owner
						Some(contract_owner.clone().into()),
						metadata_scheme.clone(),
						royalties_schedule.clone(),
						// Some(source_collection_id),
						source_chain.clone(), // TODO: remove:
					)
					.unwrap();

					CollectionsMapping::<T>::insert(source, new_collection_id);
					pallet_nft::Pallet::<T>::do_mint_multiple_with_ids(&destination, new_collection_id, current_collections_tokens)?;
				}
		});

		Ok(())
	}
}

impl<T: Config> EthereumEventSubscriber for Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<H160>,
{
	type Address = <T as pallet::Config>::PalletId;
	type SourceAddress = GetEthAddress<T>;

	fn on_event(source: &sp_core::H160, data: &[u8]) -> seed_pallet_common::OnEventResult {
		// TODO: Count weight
		let mut weight = 10000;

		// Decode prefix from first 32 bytes of data
		let prefix_decoded = match ethabi::decode(&[ParamType::Uint(32)], &data[..32]) {
			Ok(abi) => abi,
			Err(_) => return Err((weight, Error::<T>::InvalidAbiPrefix.into())),
		};

		// match prefix and route to specific decoding path
		if let [Token::Uint(prefix)] = abi_decoded.as_slice() {
			let prefix: u32 = (*prefix).saturated_into();
			let data = &data[33..];

			let _ = match prefix {
				1_u32 => Self::decode_deposit_event(source, data),
				2_u32 => Self::decode_state_sync_event(data),
				_ => Err(Error::<T>::InvalidAbiPrefix.into()),
			}
			.map_err(|e| Err((weight, e)))?;
		} else {
			Err((weight, Error::<T>::InvalidAbiPrefix.into()))
		}

		Ok(weight)
	}
}
