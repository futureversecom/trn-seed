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

use crate::*;
use codec::{Decode, Encode, MaxEncodedLen};
use core::{fmt, fmt::Write};
use scale_info::TypeInfo;
use serde::{
	de::{Error, SeqAccess, Visitor},
	ser::SerializeStruct,
	Deserialize, Deserializer, Serialize, Serializer,
};
use sp_core::Get;
use sp_runtime::{traits::ConstU32, BoundedVec, PerThing, Permill};
use sp_std::prelude::*;

/// Defines the length limit of the type MetadataScheme.
/// To avoid overly complex primitives, local const is used here instead of a runtime configurable
/// constant
const METADATA_SCHEME_LIMIT: u32 = 200;

/// The maximum number of entitlements any royalties schedule can have in totality
pub const MAX_ENTITLEMENTS: u32 = 8;

/// The maximum number of entitlements a single collection can have. This is 2 less then
/// MAX_ENTITLEMENTS due to the network and marketplace royalties being added in the listing step
/// By restricting to 2 less, we avoid these listings failing when created.
pub const MAX_COLLECTION_ENTITLEMENTS: u32 = MAX_ENTITLEMENTS - 2;

/// Unique Id for a listing
pub type ListingId = u128;

/// Describes the chain that the bridged resource originated from
#[derive(Decode, Encode, Debug, Deserialize, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum OriginChain {
	Ethereum,
	Root,
}

impl Serialize for OriginChain {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		match *self {
			OriginChain::Ethereum =>
				serializer.serialize_unit_variant("OriginChain", 0, "Ethereum"),
			OriginChain::Root => serializer.serialize_unit_variant("OriginChain", 1, "Root"),
		}
	}
}

impl Default for OriginChain {
	fn default() -> Self {
		Self::Root
	}
}
/// Reason for an NFT being locked (un-transferrable)
#[derive(Decode, Encode, Debug, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum TokenLockReason {
	/// Token is listed for sale
	Listed(ListingId),
}

/// Denotes the metadata URI referencing scheme used by a collection
/// MetadataScheme guarantees the data length not exceed the given limit, and the content won't be
/// checked and needs to be taken care by callers
#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
pub struct MetadataScheme(pub BoundedVec<u8, ConstU32<METADATA_SCHEME_LIMIT>>);

impl MetadataScheme {
	/// This function simply concatenates the stored data with the given serial_number
	/// Returns the full token_uri for a token
	pub fn construct_token_uri(&self, serial_number: SerialNumber) -> Vec<u8> {
		let mut token_uri = sp_std::Writer::default();
		write!(&mut token_uri, "{}{}", core::str::from_utf8(&self.0).unwrap_or(""), serial_number)
			.expect("Not written");
		token_uri.into_inner()
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

/// A bounded vector.
///
/// It has implementations for efficient append and length decoding, as with a normal `Vec<_>`, once
/// put into storage as a raw value, map or double-map.
///
/// As the name suggests, the length of the queue is always bounded. All internal operations ensure
/// this bound is respected.
// #[cfg_attr(feature = "std", derive(Serialize), serde(transparent))]
// #[derive(Encode, scale_info::TypeInfo)]
// #[scale_info(skip_type_params(S))]
// pub struct BoundedVector<T, S>(
// 	pub(super) Vec<T>,
// 	#[cfg_attr(feature = "std", serde(skip_serializing))] PhantomData<S>,
// );
//
// #[cfg(feature = "std")]
// impl<'de, T, S: Get<u32>> Deserialize<'de> for BoundedVector<T, S>
// 	where
// 		T: Deserialize<'de>,
// {
// 	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
// 		where
// 			D: Deserializer<'de>,
// 	{
// 		struct VecVisitor<T, S: Get<u32>>(PhantomData<(T, S)>);
//
// 		impl<'de, T, S: Get<u32>> Visitor<'de> for VecVisitor<T, S>
// 			where
// 				T: Deserialize<'de>,
// 		{
// 			type Value = Vec<T>;
//
// 			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
// 				formatter.write_str("a sequence")
// 			}
//
// 			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
// 				where
// 					A: SeqAccess<'de>,
// 			{
// 				let size = seq.size_hint().unwrap_or(0);
// 				let max = match usize::try_from(S::get()) {
// 					Ok(n) => n,
// 					Err(_) => return Err(A::Error::custom("can't convert to usize")),
// 				};
// 				if size > max {
// 					Err(A::Error::custom("out of bounds"))
// 				} else {
// 					let mut values = Vec::with_capacity(size);
//
// 					while let Some(value) = seq.next_element()? {
// 						values.push(value);
// 						if values.len() > max {
// 							return Err(A::Error::custom("out of bounds"))
// 						}
// 					}
//
// 					Ok(values)
// 				}
// 			}
// 		}
//
// 		let visitor: VecVisitor<T, S> = VecVisitor(PhantomData);
// 		deserializer
// 			.deserialize_seq(visitor)
// 			.map(|v| BoundedVector::<T, S>::try_from(v).map_err(|_| Error::custom("out of bounds")))?
// 	}
// }

/// Describes the royalty scheme for secondary sales for an NFT collection/token
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct RoyaltiesSchedule<AccountId> {
	/// Entitlements on all secondary sales, (beneficiary, % of sale price)
	pub entitlements: BoundedVec<(AccountId, Permill), ConstU32<MAX_ENTITLEMENTS>>,
}

// impl Serialize for RoyaltiesSchedule<AccountId> {
// 	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
// 		where
// 			S: serde::Serializer,
// 	{
// 		let mut s = serializer.serialize_struct("RoyaltiesSchedule", 1)?;
// 		s.serialize_field("entitlements", &self.entitlements)?;
// 		s.end()
// 	}
// }

// impl<'a> Deserialize<'a> for RoyaltiesSchedule<AccountId> {
// 	fn deserialize<D>(deserializer: D) -> Result<RoyaltiesSchedule<AccountId>, D::Error>
// 		where
// 			D: Deserializer<'a>,
// 	{
// 		deserializer.deserialize_any(RoyaltiesScheduleVisitor)
// 	}
// }

// struct RoyaltiesScheduleVisitor;
//
// impl<'a> Visitor<'a> for RoyaltiesScheduleVisitor {
// 	type Value = RoyaltiesSchedule<AccountId>;
//
// 	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
// 		formatter.write_str("valid abi spec file")
// 	}
//
// 	fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
// 		where
// 			A: SeqAccess<'a>,
// 	{
// 		let mut result = RoyaltiesSchedule::default();
// 		// while let Some(operation) = seq.next_element::<A>()? {
// 		// 	match operation {
// 		// 		xrpl => {
// 		// 			result.entitlements = false;
// 		// 		}
// 		// 	}
// 		// }
//
//
// 		Ok(result)
// 	}
// }

// impl<T> Serialize for RoyaltiesSchedule<T>
// 	where T: Serialize {
// 	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
// 		where
// 			S: serde::Serializer,
// 	{
// 		let mut s = serializer.serialize_struct("RoyaltiesSchedule", 1)?;
// 		s.serialize_field("entitlements", &self.entitlements)?;
// 		s.end()
// 	}
// }

impl<AccountId> RoyaltiesSchedule<AccountId> {
	/// True if entitlements are within valid parameters
	/// - not overcommitted (> 100%)
	/// - < MAX_ENTITLEMENTS
	pub fn validate(&self) -> bool {
		!self.entitlements.is_empty() &&
			self.entitlements.len() <= MAX_ENTITLEMENTS as usize &&
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
		Self { entitlements: BoundedVec::default() }
	}
}

#[cfg(test)]
mod test {
	use super::{MetadataScheme, RoyaltiesSchedule};
	use sp_runtime::{BoundedVec, Permill};

	#[test]
	fn valid_royalties_plan() {
		assert!(RoyaltiesSchedule::<u32> {
			entitlements: BoundedVec::truncate_from(vec![(1_u32, Permill::from_float(0.1))])
		}
		.validate());

		// explicitally specifying zero royalties is odd but fine
		assert!(RoyaltiesSchedule::<u32> {
			entitlements: BoundedVec::truncate_from(vec![(1_u32, Permill::from_float(0.0))])
		}
		.validate());

		let plan = RoyaltiesSchedule::<u32> {
			entitlements: BoundedVec::truncate_from(vec![
				(1_u32, Permill::from_float(1.01)), // saturates at 100%
			]),
		};
		assert_eq!(plan.entitlements[0].1, Permill::one());
		assert!(plan.validate());
	}

	#[test]
	fn invalid_royalties_plan() {
		// overcommits > 100% to royalties
		assert!(!RoyaltiesSchedule::<u32> {
			entitlements: BoundedVec::truncate_from(vec![
				(1_u32, Permill::from_float(0.2)),
				(2_u32, Permill::from_float(0.81)),
			]),
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
