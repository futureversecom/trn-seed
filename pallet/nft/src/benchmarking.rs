// /* Copyright 2019-2021 Centrality Investments Limited
// *
// * Licensed under the LGPL, Version 3.0 (the "License");
// * you may not use this file except in compliance with the License.
// * Unless required by applicable law or agreed to in writing, software
// * distributed under the License is distributed on an "AS IS" BASIS,
// * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// * See the License for the specific language governing permissions and
// * limitations under the License.
// * You may obtain a copy of the License at the root of this project source code,
// * or at:
// * https://centrality.ai/licenses/gplv3.txt
// * https://centrality.ai/licenses/lgplv3.txt
// */
//! NFT benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{
	account as bench_account, benchmarks, impl_benchmark_test_suite, whitelisted_caller,
};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::Permill;

use crate::Pallet as Nft;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

struct CollectionBuilder<T: Config> {
	caller: T::AccountId,
	name: CollectionNameType,
	initial_issuance: TokenCount,
	max_issuance: Option<TokenCount>,
	token_owner: Option<T::AccountId>,
	metadata_scheme: MetadataScheme,
	royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
}

impl<T: Config> CollectionBuilder<T> {
	pub fn default() -> Self {
		let metadata_scheme = MetadataScheme::IpfsDir(
			b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".to_vec(),
		);
		Self {
			caller: account::<T>("Alice"),
			name: "New Collection".into(),
			initial_issuance: 0,
			max_issuance: None,
			token_owner: None,
			metadata_scheme,
			royalties_schedule: None,
		}
	}

	pub fn caller(&mut self, value: T::AccountId) -> &mut Self {
		self.caller = value;
		self
	}

	pub fn build(&self) -> CollectionUuid {
		let id = Nft::<T>::next_collection_uuid().unwrap();
		Nft::<T>::create_collection(
			origin::<T>(&self.caller).into(),
			self.name.clone(),
			self.initial_issuance.clone(),
			self.max_issuance.clone(),
			self.token_owner.clone(),
			self.metadata_scheme.clone(),
			self.royalties_schedule.clone(),
		)
		.unwrap();

		id
	}
}

benchmarks! {
	claim_unowned_collection {
		let collection_id = CollectionBuilder::<T>::default().caller(Nft::<T>::account_id()).build();
	}: _(RawOrigin::Root, collection_id, account::<T>("Alice"))
}

impl_benchmark_test_suite!(Nft, crate::mock::new_test_ext(), crate::mock::Test,);
