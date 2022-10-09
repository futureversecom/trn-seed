#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;

use ethabi::{ParamType, Token};
// Pallet for managing NFTs bridged from *x* chain
use frame_support::{
	ensure,
	traits::{
		Get,
	},
	PalletId,
};
use pallet_nft::TokenCount;
use scale_info::TypeInfo;
use sp_core::{H160, U256};
use sp_runtime::{traits::{AccountIdConversion, Zero}, DispatchError};

use codec::{Decode, Encode, MaxEncodedLen};
pub use pallet::*;
use seed_pallet_common::{log, EthereumEventSubscriber};
use seed_primitives::{Balance, EthAddress, AccountId20, CollectionUuid, SerialNumber};
use sp_std::{boxed::Box, vec::Vec};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use seed_primitives::EthAddress;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_nft::Config {
		type PalletId: Get<PalletId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn contract_address)]
	pub type ContractAddress<T> = StorageValue<_, EthAddress, ValueQuery>;


	// Map Ethereum Collection ids to Root collection ids
	#[pallet::storage]
	#[pallet::getter(fn mapped_collections)]
	pub type CollectionsMapping<T: Config> =
		StorageMap<_, Twox64Concat, H160, u32, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		InvalidAbiEncoding,
		/// Multiple tokens were passed from contract, but amounts were unqeual per each array
		UnequalTokenCount
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

impl<T: Config> Pallet<T> {
	// fn process_nfts(
	// 	owner: Option<&T::AccountId>,
	// 	name: Vec<u8>,
	// 	source_collection_id: u32,
	// 	serial_number: SerialNumber,
	// 	quantity: TokenCount,
	// ) -> Result<(), DispatchError> {
	// 	let root_collection_id = if !CollectionsMapping::<T>::contains_key(source_collection_id) {
	// 		// Assumed values. We may need to change this later
	// 		let initial_issuance = quantity;
	// 		let max_issuance = None;
	// 		let royalties_schedule = None;

	// 		let metadata_scheme = pallet_nft::MetadataScheme::Ethereum(Self::contract_address());

	// 		let root_collection_id = pallet_nft::Pallet::<T>::do_create_collection(
	// 			// owner.unwrap().clone(),
	// 			name,
	// 			initial_issuance,
	// 			max_issuance,
	// 			// Token owner
	// 			Some(owner.unwrap().clone()),
	// 			metadata_scheme,
	// 			royalties_schedule,
	// 			Some(source_collection_id)
	// 		)?;
	// 		CollectionsMapping::<T>::insert(source_collection_id, root_collection_id);
	// 		root_collection_id
	// 	} else {
	// 		Self::mapped_collections(source_collection_id)
	// 	};

	// 	// NFTs without a given owner are understood to be temporarily owned by the pallet.
	// 	// The pallet will own it until properly claimed by the true owner
	// 	let owner = 
	// 	if owner.is_none() {
	// 		&T::PalletId::get().into_account_truncating()
	// 	} else {
	// 		owner.unwrap()
	// 	}; 

	// Non functional atm. This needs to accept and process multiple tokens
	fn process_nfts_multiple(
		token_addresses: Vec<Token>,
		token_ids: Vec<Token>,
		contract_owners: Vec<Token>,
		destination: H160,
	) -> Result<(), DispatchError> {
		// Spiked singular logic, commented out here
		// let root_collection_id = if !CollectionsMapping::<T>::contains_key(source_collection_id) {
		// 	// Assumed values for each. We may need to change this later
		// 	let initial_issuance = token_addresses.len();
		// 	let max_issuance = None;
		// 	let royalties_schedule = None;

		// 	let metadata_scheme = pallet_nft::MetadataScheme::Ethereum(Self::contract_address());

		// 	let root_collection_id = pallet_nft::Pallet::<T>::do_create_collection(
		// 		// owner.unwrap().clone(),
		// 		name,
		// 		initial_issuance,
		// 		max_issuance,
		// 		// Token owner
		// 		Some(owner.unwrap().clone()),
		// 		metadata_scheme,
		// 		royalties_schedule,
		// 		Some(source_collection_id)
		// 	)?;
		// 	CollectionsMapping::<T>::insert(source_collection_id, root_collection_id);
		// 	root_collection_id
		// } else {
		// 	Self::mapped_collections(source_collection_id)
		// };

		// // NFTs without a given owner are understood to be temporarily owned by the pallet.
		// // The pallet will own it until properly claimed by the true owner
		// let owner = 
		// if owner.is_none() {
		// 	&T::PalletId::get().into_account_truncating()
		// } else {
		// 	owner.unwrap()
		// }; 

		// pallet_nft::Pallet::<T>::do_mint(
		// 	owner,
		// 	root_collection_id,
		// 	serial_number,
		// 	quantity
		// )

		ensure!(token_addresses.len() == token_ids.len(), Error::<T>::UnequalTokenCount);
		ensure!(token_addresses.len() == contract_owners.len(), Error::<T>::UnequalTokenCount);


		// For each, mint:
		// 	pallet_nft::Pallet::<T>::do_mint(
		// 		owner,
		// 		root_collection_id,
		// 		serial_number,
		// 		quantity
		// 	)
		// }

		Ok(())
	}

}

impl<T: Config> EthereumEventSubscriber for Pallet<T>
where <T as frame_system::Config>::AccountId: From<H160>
{
	type Address = <T as pallet::Config>::PalletId;
	type SourceAddress = GetEthAddress<T>;

	fn on_event(source: &sp_core::H160, data: &[u8]) -> seed_pallet_common::OnEventResult {
		let mut weight = 10000;
		let abi_decoded = match ethabi::decode(
			&[
				ParamType::Array(Box::new(ParamType::Address)),
				// TODO: Check nesting here, double check the decoding of the uint correctly maps to some uint we care about
				ParamType::Array(Box::new(
					ParamType::Array(Box::new(ParamType::Uint(32)))
				)),
				ParamType::Array(Box::new(ParamType::Address)),
				ParamType::Address,
				],
			data,
		) {
			Ok(abi) => abi,
			Err(_) => return Err((42, Error::<T>::InvalidAbiEncoding.into())),
		};

		if let [
				Token::Array(token_addresses),
				Token::Array(token_ids), // Pull inner vec out
				Token::Array(contract_owners),
				Token::Address(destination)
			] =
			abi_decoded.as_slice()
		{

			Self::process_nfts_multiple(
				token_addresses.clone(),
				token_ids.clone(),
				contract_owners.clone(),
				destination.clone(),
			);

			Ok(weight)
		} else {
			// input data should be valid, we do not expect to fail here
			Err((weight, Error::<T>::InvalidAbiEncoding.into()))
		}
	}
}
