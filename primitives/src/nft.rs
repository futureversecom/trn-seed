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
/// Enable token metadata URI construction by clients
#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
pub struct MetadataScheme(BoundedVec<u8, ConstU32<1_000>>);

// {
// 	/// Collection metadata is hosted by an HTTPS server
// 	/// Inner value is the URI without protocol prefix 'https://' or trailing '/'
// 	/// full metadata URI construction: `https://<domain>/<path+>/<serial_number>`
// 	/// Https(b"example.com/metadata")
// 	Https(Vec<u8>),
// 	/// Collection metadata is hosted by an unsecured HTTP server
// 	/// Inner value is the URI without protocol prefix 'http://' or trailing '/'
// 	/// full metadata URI construction: `https://<domain>/<path+>/<serial_number>`
// 	/// Https(b"example.com/metadata")
// 	Http(Vec<u8>),
// 	/// Collection metadata is hosted by an IPFS directory
// 	/// Inner value is the directory's IPFS CID
// 	/// full metadata URI construction: `ipfs://<directory_CID>/<serial_number>`
// 	/// Ipfs(b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi")
// 	Ipfs(Vec<u8>),
// 	// Collection metadata is located on Ethereum in the relevant field on the source token
// 	// ethereum://<contractaddress>/<originalid>
// 	Ethereum(H160),
// }

impl MetadataScheme {
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
		let b: BoundedVec<u8, ConstU32<1_000>> =
			BoundedVec::try_from(value).map_err(|_| "Invalid string")?;

		Ok(MetadataScheme(b))
	}
}

// #[cfg(test)]
// mod test {
// 	use super::MetadataScheme;
// 	use sp_core::H160;

// 	#[test]
// 	fn metadata_path_sanitize() {
// 		// empty
// 		assert_eq!(MetadataScheme::Http(b"".to_vec()).sanitize(), Err("empty path"));

// 		// protocol stripped, trailing slashes
// 		assert_eq!(
// 			MetadataScheme::Http(b" http://test.com/".to_vec()).sanitize(),
// 			Ok(MetadataScheme::Http(b"test.com/".to_vec()))
// 		);
// 		assert_eq!(
// 			MetadataScheme::Https(b"https://test.com/ ".to_vec()).sanitize(),
// 			Ok(MetadataScheme::Https(b"test.com/".to_vec()))
// 		);
// 		assert_eq!(
// 			MetadataScheme::Ipfs(b"ipfs://notarealCIDblah/".to_vec()).sanitize(),
// 			Ok(MetadataScheme::Ipfs(b"notarealCIDblah/".to_vec()))
// 		);

// 		// protocol stripped, nested
// 		assert_eq!(
// 			MetadataScheme::Http(b" http://test.com/abc/".to_vec()).sanitize(),
// 			Ok(MetadataScheme::Http(b"test.com/abc/".to_vec()))
// 		);
// 		assert_eq!(
// 			MetadataScheme::Https(b"https://test.com/def ".to_vec()).sanitize(),
// 			Ok(MetadataScheme::Https(b"test.com/def".to_vec()))
// 		);
// 		assert_eq!(
// 			MetadataScheme::Ipfs(b"ipfs://notarealCIDblah/ghi/jkl/".to_vec()).sanitize(),
// 			Ok(MetadataScheme::Ipfs(b"notarealCIDblah/ghi/jkl/".to_vec()))
// 		);

// 		// untouched
// 		assert_eq!(
// 			MetadataScheme::Http(b"test.com".to_vec()).sanitize(),
// 			Ok(MetadataScheme::Http(b"test.com".to_vec()))
// 		);

// 		assert_eq!(
// 			MetadataScheme::Ethereum(H160::from_low_u64_be(123)).sanitize(),
// 			Ok(MetadataScheme::Ethereum(H160::from_low_u64_be(123)))
// 		);
// 	}

// 	#[test]
// 	fn uri_to_metadata_scheme() {
// 		let scheme: Result<MetadataScheme, &'static str> = b"http://test.com".to_vec().try_into();
// 		assert_eq!(scheme, Ok(MetadataScheme::Http(b"test.com".to_vec())));

// 		let scheme: Result<MetadataScheme, &'static str> = b"https://test.com".to_vec().try_into();
// 		assert_eq!(scheme, Ok(MetadataScheme::Https(b"test.com".to_vec())));

// 		// nested path with trailing slash
// 		let scheme: Result<MetadataScheme, &'static str> =
// 			b"https://test.com/defg/hijkl/".to_vec().try_into();
// 		assert_eq!(scheme, Ok(MetadataScheme::Https(b"test.com/defg/hijkl/".to_vec())));

// 		let scheme: Result<MetadataScheme, &'static str> = b"ipfs://test.com".to_vec().try_into();
// 		assert_eq!(scheme, Ok(MetadataScheme::Ipfs(b"test.com".to_vec())));

// 		// eth address without 0x prefix
// 		let scheme: Result<MetadataScheme, &'static str> =
// 			b"ethereum://E04CC55ebEE1cBCE552f250e85c57B70B2E2625b".to_vec().try_into();
// 		assert_eq!(
// 			scheme,
// 			Ok(MetadataScheme::Ethereum(H160::from_slice(
// 				&hex::decode("E04CC55ebEE1cBCE552f250e85c57B70B2E2625b").unwrap()
// 			)))
// 		);

// 		// eth address with 0x prefix
// 		let scheme: Result<MetadataScheme, &'static str> =
// 			b"ethereum://0xE04CC55ebEE1cBCE552f250e85c57B70B2E2625b".to_vec().try_into();
// 		assert_eq!(
// 			scheme,
// 			Ok(MetadataScheme::Ethereum(H160::from_slice(
// 				&hex::decode("E04CC55ebEE1cBCE552f250e85c57B70B2E2625b").unwrap()
// 			)))
// 		);

// 		// eth address with trailing slash
// 		let scheme: Result<MetadataScheme, &'static str> =
// 			b"ethereum://0xE04CC55ebEE1cBCE552f250e85c57B70B2E2625b/".to_vec().try_into();
// 		assert_eq!(
// 			scheme,
// 			Ok(MetadataScheme::Ethereum(H160::from_slice(
// 				&hex::decode("E04CC55ebEE1cBCE552f250e85c57B70B2E2625b").unwrap()
// 			)))
// 		);

// 		// invalid protocol
// 		let scheme: Result<MetadataScheme, &'static str> =
// 			b"tcp://notarealCIDblah".to_vec().try_into();
// 		assert_eq!(scheme, Err("scheme not supported"));

// 		// missing protocol
// 		let scheme: Result<MetadataScheme, &'static str> = b"notarealCIDblah".to_vec().try_into();
// 		assert_eq!(scheme, Err("Invalid URI"));

// 		// empty path
// 		let scheme: Result<MetadataScheme, &'static str> = b"".to_vec().try_into();
// 		assert_eq!(scheme, Err("Invalid URI"));

// 		// everything after 2nd `://` is stripped out
// 		let scheme: Result<MetadataScheme, &'static str> = b"https://://".to_vec().try_into();
// 		assert_eq!(scheme, Err("empty path"));

// 		let scheme: Result<MetadataScheme, &'static str> = b"https://a://".to_vec().try_into();
// 		assert_eq!(scheme, Ok(MetadataScheme::Https(b"a".to_vec())));

// 		let scheme: Result<MetadataScheme, &'static str> =
// 			b"https://a://-----all-ignored".to_vec().try_into();
// 		assert_eq!(scheme, Ok(MetadataScheme::Https(b"a".to_vec())));

// 		// duplicate protocol - everything after 2nd `://` is stripped out
// 		let scheme: Result<MetadataScheme, &'static str> =
// 			b"https://httpsa://everything-here-ignored".to_vec().try_into();
// 		assert_eq!(scheme, Ok(MetadataScheme::Https(b"httpsa".to_vec())));

// 		let scheme: Result<MetadataScheme, &'static str> =
// 			b"https://https://https://httpsa://a".to_vec().try_into();
// 		assert_eq!(scheme, Ok(MetadataScheme::Https(b"https".to_vec())));
// 	}

// 	#[test]
// 	fn test_construct_token_uri() {
// 		// no `/` seperator
// 		assert_eq!(
// 			MetadataScheme::Http(b"test.com".to_vec()).construct_token_uri(1),
// 			b"http://test.com1".to_vec()
// 		);

// 		assert_eq!(
// 			MetadataScheme::Https(b"test.com".to_vec()).construct_token_uri(1),
// 			b"https://test.com1".to_vec()
// 		);

// 		assert_eq!(
// 			MetadataScheme::Ipfs(b"test.com".to_vec()).construct_token_uri(1),
// 			b"ipfs://test.com1".to_vec()
// 		);

// 		assert_eq!(
// 			MetadataScheme::Ethereum(H160::from_slice(
// 				&hex::decode("E04CC55ebEE1cBCE552f250e85c57B70B2E2625b").unwrap()
// 			))
// 			.construct_token_uri(1),
// 			b"ethereum://0xe04cc55ebee1cbce552f250e85c57b70b2e2625b/1".to_vec() /* trailing slash always
// added for eth address */ 		);

// 		// nested path with trailing slash
// 		assert_eq!(
// 			MetadataScheme::Http(b"test.com/defg/hijkl/".to_vec()).construct_token_uri(1),
// 			b"http://test.com/defg/hijkl/1".to_vec()
// 		);

// 		assert_eq!(
// 			MetadataScheme::Https(b"test.com/defg/hijkl/".to_vec()).construct_token_uri(123),
// 			b"https://test.com/defg/hijkl/123".to_vec()
// 		);

// 		assert_eq!(
// 			MetadataScheme::Ipfs(b"test.com/defg/hijkl/".to_vec()).construct_token_uri(123),
// 			b"ipfs://test.com/defg/hijkl/123".to_vec()
// 		);
// 	}
// }
