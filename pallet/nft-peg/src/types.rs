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

use crate::{Config, Pallet};
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::BoundedVec;
use scale_info::TypeInfo;
use seed_primitives::{BlockNumber, CollectionUuid, SerialNumber};
use sp_core::H160;
use sp_runtime::{traits::Get, RuntimeDebug};
use sp_std::{marker::PhantomData, vec::Vec};

#[derive(Debug, PartialEq, Clone, Encode, Decode, TypeInfo)]
/// Contains information about a token
pub struct TokenInfo<T: Config> {
	// The address of the contract
	pub token_address: H160,
	// The ids of the tokens belonging to the contract
	pub token_ids: BoundedVec<SerialNumber, T::MaxTokensPerMint>,
}

#[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct RoadBlockedTokens<T: Config> {
	pub block_number: BlockNumber,
	pub collection_id: CollectionUuid,
	pub serial_numbers: BoundedVec<
		BoundedVec<SerialNumber, T::MaxSerialsPerWithdraw>,
		T::MaxCollectionsPerWithdraw,
	>,
}

pub struct GroupedTokenInfo<T: Config> {
	pub tokens: Vec<TokenInfo<T>>,
	pub destination: T::AccountId,
}

impl<T: Config> GroupedTokenInfo<T> {
	pub fn new(
		token_ids: BoundedVec<BoundedVec<SerialNumber, T::MaxTokensPerMint>, T::MaxAddresses>,
		token_addresses: BoundedVec<H160, T::MaxAddresses>,
		destination: T::AccountId,
	) -> Self {
		let token_information: Vec<TokenInfo<T>> = token_ids
			.into_iter()
			.zip(token_addresses.into_iter())
			.map(|(token_ids, token_address)| TokenInfo { token_address, token_ids })
			.collect();
		GroupedTokenInfo { tokens: token_information, destination }
	}
}

/// Used to get the contract address for use in the EthereumEventSubscriber
pub struct GetContractAddress<T>(PhantomData<T>);

impl<T: Config> Get<H160> for GetContractAddress<T> {
	fn get() -> H160 {
		Pallet::<T>::contract_address()
	}
}

/// The destination of an incoming event from the bridge
pub enum MessageDestination {
	Deposit,
	StateSync,
	Other,
}

impl From<u32> for MessageDestination {
	fn from(index: u32) -> Self {
		match index {
			1 => MessageDestination::Deposit,
			2 => MessageDestination::StateSync,
			_ => MessageDestination::Other,
		}
	}
}
