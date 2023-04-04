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

use crate::mock::*;
use frame_support::{assert_noop, assert_ok, pallet_prelude::*};
use seed_primitives::{MetadataScheme, RoyaltiesSchedule};
use sp_runtime::Permill;

#[test]
fn create_collection_works() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = alice();
		let collection_name = b"test".to_vec();
		let token_owner = bob();
		let metadata_scheme = MetadataScheme::Https(b"example.com/metadata".to_vec());
		let royalties_schedule =
			RoyaltiesSchedule { entitlements: vec![(collection_owner, Permill::one())] };

		assert_ok!(Sft::create_collection(
			Some(collection_owner).into(),
			collection_name,
			Some(token_owner),
			metadata_scheme,
			Some(royalties_schedule)
		));
	});
}
