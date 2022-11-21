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

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::Permill;

use crate::Pallet as Nft;

/// payment asset
// const PAYMENT_ASSET: u32 = 16_000;
/// sale price, 1 million 4dp asset
// const PRICE: u128 = 1_000_000 * 10_000;
/// QUANTITY
// const QUANTITY: u32 = 100;

// Create an NFT collection
// Returns the created `collection_id`
fn setup_collection<T: Config>(
	owner: T::AccountId,
) -> (CollectionUuid, RoyaltiesSchedule<T::AccountId>) {
	let collection_id = <Nft<T>>::next_collection_uuid().unwrap();
	let collection_name = [1_u8; MAX_COLLECTION_NAME_LENGTH as usize].to_vec();
	let metadata_scheme = MetadataScheme::IpfsDir(
		b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".to_vec(),
	);
	let royalties = RoyaltiesSchedule::<T::AccountId> {
		entitlements: (0..MAX_ENTITLEMENTS)
			.map(|_| (owner.clone(), Permill::from_percent(1)))
			.collect::<Vec<(T::AccountId, Permill)>>(),
	};

	assert_ok!(<Nft<T>>::create_collection(
		RawOrigin::Signed(owner).into(),
		collection_name,
		0,
		None,
		None,
		metadata_scheme,
		Some(royalties.clone()),
	));

	(collection_id, royalties)
}

// Create a token for benchmarking
/* fn setup_token<T: Config>(owner: T::AccountId) -> CollectionUuid {
	let collection_owner: T::AccountId = whitelisted_caller();
	let (collection_id, _) = setup_collection::<T>(collection_owner.clone());

	<Nft<T>>::mint(
		RawOrigin::Signed(collection_owner).into(),
		collection_id,
		QUANTITY,
		Some(owner).clone(),
	);

	collection_id
}
*/

benchmarks! {
	set_owner {
		let creator: T::AccountId = account("creator", 0, 0);
		let new_owner: T::AccountId = account("new_owner", 0, 0);
		let (collection_id, royalties) = setup_collection::<T>(creator.clone());

	}: _(RawOrigin::Signed(creator.clone()), collection_id, new_owner.clone())
	verify {
		assert_eq!(<Nft<T>>::collection_info(&collection_id).unwrap().owner, new_owner);
	}

	mint {
		let q in 1 .. 10;
		let creator: T::AccountId = whitelisted_caller();
		let owner: T::AccountId = account("owner", 0, 0);
		let (collection_id, _ ) = setup_collection::<T>(creator.clone());

	}: _(RawOrigin::Signed(creator), collection_id, q.into(), Some(owner))
	verify {
		assert_eq!(<Nft<T>>::next_serial_number(collection_id).unwrap(), q);
	}
}

impl_benchmark_test_suite!(Nft, crate::mock::new_test_ext(), crate::mock::Test,);
