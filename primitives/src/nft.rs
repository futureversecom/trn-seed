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
use sp_runtime::{traits::ConstU32, BoundedVec};
use sp_std::prelude::*;

/// Denotes the metadata URI referencing scheme used by a collection
/// MetadataScheme guarantees the data length not exceed the given limit, and the content won't be
/// checked and needs to be taken care by callers
#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
pub struct MetadataScheme(BoundedVec<u8, ConstU32<1_000>>);

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

impl TryFrom<Vec<u8>> for MetadataScheme {
	type Error = &'static str;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		let bounded_vec: BoundedVec<u8, ConstU32<1_000>> =
			BoundedVec::try_from(value).map_err(|_| "Too large input vec")?;

		Ok(MetadataScheme(bounded_vec))
	}
}

#[cfg(test)]
mod test {
	use super::MetadataScheme;

	#[test]
	fn test_construct_token_uri() {
		assert_eq!(
			MetadataScheme::try_from(b"http://test.com/defg/hijkl/".to_vec())
				.unwrap()
				.construct_token_uri(1),
			b"http://test.com/defg/hijkl/1".to_vec()
		);
	}

	#[test]
	fn test_try_from_succeeds() {
		assert_eq!(
			MetadataScheme::try_from(b"http://test.com/defg/hijkl/".to_vec())
				.unwrap()
				.0
				.to_vec(),
			b"http://test.com/defg/hijkl/".to_vec()
		)
	}

	#[test]
	fn test_try_from_fails() {
		assert!(MetadataScheme::try_from(vec![0; 1001]).is_err())
	}
}
