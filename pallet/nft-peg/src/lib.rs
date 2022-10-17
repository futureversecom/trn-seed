#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;

use ethabi::{ParamType, Token};
// Pallet for managing NFTs bridged from *x* chain
use frame_support::{ensure, fail, traits::Get, BoundedVec, PalletId};
use pallet_nft::OriginChain;
use scale_info::TypeInfo;
use seed_primitives::{CollectionUuid, SerialNumber};
use sp_core::{H160, U256};
use sp_runtime::{traits::AccountIdConversion, DispatchError, SaturatedConversion};

use codec::{Decode, Encode, MaxEncodedLen};
pub use pallet::*;
use seed_pallet_common::{EthereumBridge, EthereumEventSubscriber};
use sp_std::{boxed::Box, vec, vec::Vec};

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, transactional};
	use frame_system::{ensure_signed, pallet_prelude::*};
	use seed_primitives::EthAddress;
	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_nft::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type PalletId: Get<PalletId>;
		#[pallet::constant]
		type DelayLength: Get<Self::BlockNumber>;
		type MaxAddresses: Get<u32>;
		type MaxTokensPerCollection: Get<u32>;
		type EthBridge: EthereumBridge;
	}

	#[pallet::storage]
	#[pallet::getter(fn contract_address)]
	pub type ContractAddress<T> = StorageValue<_, EthAddress, ValueQuery>;

	// Map Ethereum Collection ids to Root collection ids
	#[pallet::storage]
	#[pallet::getter(fn eth_to_root_nft)]
	pub type EthToRootNft<T: Config> = StorageMap<_, Twox64Concat, H160, u32, OptionQuery>;

	// Map Ethereum Collection ids to Root collection ids
	#[pallet::storage]
	#[pallet::getter(fn root_to_eth_nft)]
	pub type RootNftToErc721<T: Config> = StorageMap<_, Twox64Concat, u32, H160, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn delayed_mints)]
	pub type DelayedMints<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, PeggedNftInfo<T>, OptionQuery>;

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
		NoCollectionInfo,
		/// No mapped token was stored for bridging the token back to the bridged chain
		/// chain(Should not happen)
		NoMappedTokenExists,
		/// Tried to bridge a token that originates from Root, which is not yet supported
		NoPermissionToBridge,
		/// The state sync decoding feature is not implemented
		StateSyncDisabled,
		/// Multiple tokens were passed from contract, but amounts were unqeual per each array
		UnequalTokenCount,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Erc721DepositFailed(DispatchError),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160>,
	{
		#[pallet::weight(10000)]
		#[transactional]
		pub fn withdraw(
			origin: OriginFor<T>,
			collection_ids: Vec<CollectionUuid>,
			token_ids: Vec<Vec<SerialNumber>>,
			destination: H160,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_withdraw(H160::from(who.into()), collection_ids, token_ids, destination)?;
			Ok(().into())
		}
	}
}

pub struct GetEthAddress<T>(PhantomData<T>);

impl<T: Config> Get<H160> for GetEthAddress<T> {
	fn get() -> H160 {
		Pallet::<T>::contract_address()
	}
}

#[derive(Debug, Default, Clone, PartialEq, Decode, Encode, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct PeggedNftInfo<T: Config> {
	/// Sender contract
	source: H160,
	/// NFT token addresses
	token_addresses: BoundedVec<H160, T::MaxAddresses>,
	/// List of token ids. For a given address `n` from `token_addresses`, its corresponding token
	/// ids exist at `token_ids[n]`.
	token_ids: BoundedVec<BoundedVec<U256, T::MaxTokensPerCollection>, T::MaxAddresses>,
	/// The address to send the tokens to
	destination: H160,
}

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	fn decode_deposit_event(
		source: &sp_core::H160,
		data: &[u8],
	) -> Result<u64, (u64, DispatchError)> {
		let mut weight = 0;
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
			let token_ids: Result<
				Vec<BoundedVec<U256, T::MaxTokensPerCollection>>,
				(u64, DispatchError),
			> = token_ids
				.iter()
				.map(|k| {
					if let Token::Array(token_ids) = k {
						let new: Vec<U256> = token_ids
							.iter()
							.filter_map(|j| {
								if let Token::Uint(token_id) = j {
									Some(token_id.clone())
								} else {
									None
								}
							})
							.collect();
						BoundedVec::try_from(new)
							.map_err(|_| (weight, Error::<T>::ExceedsMaxTokens.into()))
					} else {
						Err((weight, Error::<T>::ExceedsMaxTokens.into()))
					}
				})
				.collect();

			let token_ids: BoundedVec<
				BoundedVec<U256, T::MaxTokensPerCollection>,
				T::MaxAddresses,
			> = BoundedVec::try_from(token_ids?)
				.map_err(|_| (weight, Error::<T>::ExceedsMaxAddresses.into()))?;

			let do_deposit_weight = Self::do_deposit(token_addresses, token_ids, *destination)
				.map_err(|err| (weight, err))?;

			weight = T::DbWeight::get().writes(1).saturating_add(do_deposit_weight);

			Ok(weight)
		} else {
			// input data should be valid, we do not expect to fail here
			Err((weight, Error::<T>::InvalidAbiEncoding.into()))
		}
	}

	// TODO implement state sync feature for collection_owner, name and metadata
	fn decode_state_sync_event(_data: &[u8]) -> Result<u64, (u64, DispatchError)> {
		Err((0, Error::<T>::StateSyncDisabled.into()))
	}

	// Accept some representation of one or more tokens from an outside source, and create a
	// Root-side representation of them Expects ERC721 tokens sent and verified through the existing
	// bridge
	fn do_deposit(
		// Addresses of the tokens
		token_addresses: BoundedVec<H160, T::MaxAddresses>,
		// Lists of token ids for the above addresses(For a given address `n`, its tokens are at
		// `token_ids[n]`)
		token_ids: BoundedVec<BoundedVec<U256, T::MaxTokensPerCollection>, T::MaxAddresses>,
		// Root address to deposit the tokens into
		destination: H160,
	) -> Result<u64, DispatchError> {
		let mut weight = 0;

		let initial_issuance: u32 = token_addresses.len() as u32;
		let max_issuance = None;
		let royalties_schedule = None;
		let destination: T::AccountId = destination.into();
		let source_chain = OriginChain::Ethereum;
		let metadata_scheme = pallet_nft::MetadataScheme::Ethereum(Self::contract_address());
		let name = "".encode();

		ensure!(token_addresses.len() == token_ids.len(), Error::<T>::UnequalTokenCount);

		for (collection_idx, address) in token_addresses.iter().enumerate() {
			// Get the list of token ids corresponding to the current collection
			let current_collections_tokens = &token_ids[collection_idx];
			// Assign collection owner to pallet. User can claim it later
			let collection_owner_account =
				<T as pallet_nft::Config>::PalletId::get().into_account_truncating();

			// Weight for do_mint_multiple. TODO: return from do_mint_multiple
			weight = (current_collections_tokens.len() as u64).saturating_mul(
				(T::DbWeight::get().writes(2).saturating_add(T::DbWeight::get().reads(1))).saturating_add(
					T::DbWeight::get().writes(2).saturating_add(T::DbWeight::get().reads(2))
			));
			// Check if incoming collection is in CollectionMapping, if not, create as
			// new collection along with its Eth > Root mapping
			if let Some(root_collection_id) = Self::eth_to_root_nft(address) {
				pallet_nft::Pallet::<T>::do_mint_multiple(
					&destination,
					root_collection_id,
					current_collections_tokens,
				)?;
			} else {
				let new_collection_id = pallet_nft::Pallet::<T>::do_create_collection(
					collection_owner_account,
					name.clone(),
					initial_issuance,
					max_issuance,
					Some(destination.clone()),
					metadata_scheme.clone(),
					royalties_schedule.clone(),
					source_chain.clone(),
				)?;

				// Populate both mappings, building the relationship between the bridged chain
				// token, and this chain's token
				EthToRootNft::<T>::insert(address, new_collection_id);
				RootNftToErc721::<T>::insert(new_collection_id, address);
				pallet_nft::Pallet::<T>::do_mint_multiple(
					&destination,
					new_collection_id,
					current_collections_tokens,
				)?;
				weight = weight.saturating_add(T::DbWeight::get().writes(2));
			}
		}
		Ok(weight)
	}

	// Accepts one or more Ethereum originated ERC721 tokens to be sent back over the bridge
	pub fn do_withdraw(
		who: H160,
		collection_ids: Vec<CollectionUuid>,
		token_ids: Vec<Vec<SerialNumber>>,
		// Root address to deposit the tokens into
		destination: H160,
	) -> Result<(), DispatchError> {
		let mut source_collection_ids = vec![];
		let mut source_token_ids = vec![];

		for (idx, collection_id) in collection_ids.into_iter().enumerate() {
			if let Some(collection_info) = pallet_nft::Pallet::<T>::collection_info(collection_id) {
				// At the time of writing, only Ethereum-originated NFTs can be bridged back.
				ensure!(
					collection_info.source_chain == OriginChain::Ethereum,
					Error::<T>::NoPermissionToBridge
				);
			} else {
				fail!(Error::<T>::NoCollectionInfo);
			}

			// Allocate space
			source_token_ids.push(vec![]);

			// Tokens stored here, as well as the outer loop should be bounded, so iterations are
			// somewhat bounded as well, but there should be a way to reduce this complexity
			for token_id in &token_ids[idx] {
				pallet_nft::Pallet::<T>::do_burn(&who.into(), collection_id, token_id)?;
				source_token_ids[idx].push(Token::Uint(U256::from(token_id.clone())))
			}

			// Lookup the source chain token id for this token and remove it from the mapping
			let token_address = Pallet::<T>::root_to_eth_nft(collection_id)
				.ok_or(Error::<T>::NoMappedTokenExists)?;
			source_collection_ids.push(Token::Address(token_address));
		}

		let source = <T as pallet::Config>::PalletId::get().into_account_truncating();
		let source_token_ids = source_token_ids.into_iter().map(|k| Token::Array(k)).collect();

		let message = ethabi::encode(&[
			Token::Array(source_collection_ids),
			Token::Array(source_token_ids),
			Token::Address(destination),
		]);

		T::EthBridge::send_event(&source, &Pallet::<T>::contract_address(), &message)?;

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
		let weight = 0;

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

			match prefix {
				1_u32 => Self::decode_deposit_event(source, data),
				2_u32 => Self::decode_state_sync_event(data),
				_ => Err((weight, Error::<T>::InvalidAbiPrefix.into())),
			}
		} else {
			return Err((weight, Error::<T>::InvalidAbiPrefix.into()))
		}
	}
}
