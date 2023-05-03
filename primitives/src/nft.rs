// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use crate::*;
use codec::{Decode, Encode};
use core::fmt::Write;
use scale_info::TypeInfo;
use sp_core::H160;
use sp_runtime::{PerThing, Permill};
use sp_std::prelude::*;

/// The max. number of entitlements any royalties schedule can have
/// just a sensible upper bound
pub const MAX_ENTITLEMENTS: usize = 8;

/// Describes the chain that the bridged resource originated from
#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
pub enum OriginChain {
	Ethereum,
	Root,
}

/// Denotes the metadata URI referencing scheme used by a collection
/// MetadataScheme guarantees the data length not exceed the given limit, and the content won't be
/// checked and needs to be taken care by callers
#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
pub struct MetadataScheme(BoundedVec<u8, ConstU32<METADATA_SCHEME_LIMIT>>);

impl MetadataScheme {
	/// This function simply concatenates the stored data with the given serial_number
	/// Returns the full token_uri for a token
	pub fn construct_token_uri(&self, serial_number: SerialNumber) -> Vec<u8> {
		let mut token_uri = sp_std::Writer::default();
		write!(&mut token_uri, "{}{}", core::str::from_utf8(&self.0).unwrap_or(""), serial_number)
			.expect("Not written");
		token_uri.inner().clone()
	}
}

impl TryFrom<&[u8]> for MetadataScheme {
	type Error = &'static str;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		let bounded_vec: BoundedVec<u8, ConstU32<METADATA_SCHEME_LIMIT>> =
			BoundedVec::try_from(value.to_vec()).map_err(|_| "Too large input vec")?;

		Ok(MetadataScheme(bounded_vec))
	}
}

/// Describes the royalty scheme for secondary sales for an NFT collection/token
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub struct RoyaltiesSchedule<AccountId> {
	/// Entitlements on all secondary sales, (beneficiary, % of sale price)
	pub entitlements: Vec<(AccountId, Permill)>,
}

impl<AccountId> RoyaltiesSchedule<AccountId> {
	/// True if entitlements are within valid parameters
	/// - not overcommitted (> 100%)
	/// - < MAX_ENTITLEMENTS
	pub fn validate(&self) -> bool {
		!self.entitlements.is_empty() &&
			self.entitlements.len() <= MAX_ENTITLEMENTS &&
			self.entitlements
				.iter()
				.map(|(_who, share)| share.deconstruct() as u32)
				.sum::<u32>() <= Permill::ACCURACY
	}
	/// Calculate the total % entitled for royalties
	/// It will return `0` if the `entitlements` are overcommitted
	pub fn calculate_total_entitlement(&self) -> Permill {
		// if royalties are in a strange state
		if !self.validate() {
			return Permill::zero()
		}
		Permill::from_parts(
			self.entitlements.iter().map(|(_who, share)| share.deconstruct()).sum::<u32>(),
		)
	}
}

impl<AccountId> Default for RoyaltiesSchedule<AccountId> {
	fn default() -> Self {
		Self { entitlements: vec![] }
	}
}

#[cfg(test)]
mod test {
	use super::{MetadataScheme, RoyaltiesSchedule};
	use sp_runtime::Permill;

	#[test]
	fn valid_royalties_plan() {
		assert!(RoyaltiesSchedule::<u32> { entitlements: vec![(1_u32, Permill::from_float(0.1))] }
			.validate());

		// explicitally specifying zero royalties is odd but fine
		assert!(RoyaltiesSchedule::<u32> { entitlements: vec![(1_u32, Permill::from_float(0.0))] }
			.validate());

		let plan = RoyaltiesSchedule::<u32> {
			entitlements: vec![
				(1_u32, Permill::from_float(1.01)), // saturates at 100%
			],
		};
		assert_eq!(plan.entitlements[0].1, Permill::one());
		assert!(plan.validate());
	}

	#[test]
	fn invalid_royalties_plan() {
		// overcommits > 100% to royalties
		assert!(!RoyaltiesSchedule::<u32> {
			entitlements: vec![
				(1_u32, Permill::from_float(0.2)),
				(2_u32, Permill::from_float(0.81)),
			],
		}
		.validate());
	}

	#[test]
	fn test_construct_token_uri() {
		assert_eq!(
			MetadataScheme::try_from(b"http://test.com/defg/hijkl/".as_slice())
				.unwrap()
				.construct_token_uri(1),
			b"http://test.com/defg/hijkl/1".to_vec()
		);
	}

	#[test]
	fn test_try_from_succeeds() {
		assert_eq!(
			MetadataScheme::try_from(b"http://test.com/defg/hijkl/".as_slice())
				.unwrap()
				.0
				.to_vec(),
			b"http://test.com/defg/hijkl/".to_vec()
		)
	}

	#[test]
	fn test_try_from_fails() {
		assert!(MetadataScheme::try_from(vec![0; 1001].as_slice()).is_err())
	}
}
