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

use codec::{alloc::string::ToString, Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{ecdsa, ed25519, H160};
use sp_io::hashing::{blake2_256, keccak_256};
use sp_std::vec::Vec;

#[derive(
	Eq, PartialEq, Copy, Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Default, PartialOrd, Ord,
)]
pub struct AccountId20(pub [u8; 20]);

impl_serde::impl_fixed_hash_serde!(AccountId20, 20);

#[cfg(feature = "std")]
impl std::fmt::Display for AccountId20 {
	// TODO: This is a pretty quck-n-dirty implementation. Perhaps we should add
	// checksum casing here? I bet there is a crate for that.
	// Maybe this one https://github.com/miguelmota/rust-eth-checksum
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", self.0)
	}
}

impl core::fmt::Debug for AccountId20 {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{:?}", H160(self.0))
	}
}

impl From<[u8; 20]> for AccountId20 {
	fn from(bytes: [u8; 20]) -> Self {
		Self(bytes)
	}
}

impl From<AccountId20> for [u8; 20] {
	fn from(value: AccountId20) -> [u8; 20] {
		value.0
	}
}

impl From<H160> for AccountId20 {
	fn from(h160: H160) -> Self {
		Self(h160.0)
	}
}

impl TryFrom<ecdsa::Public> for AccountId20 {
	type Error = &'static str;

	fn try_from(public: ecdsa::Public) -> Result<Self, Self::Error> {
		let decompressed = libsecp256k1::PublicKey::parse_slice(
			&public.0,
			Some(libsecp256k1::PublicKeyFormat::Compressed),
		)
		.map_err(|_| "Wrong compressed public key provided")?
		.serialize();
		let mut m = [0u8; 64];
		m.copy_from_slice(&decompressed[1..65]);
		let account = H160(keccak_256(&m)[12..].try_into().map_err(|_| "Invalid account id")?);
		Ok(Self(account.0))
	}
}

impl TryFrom<ed25519::Public> for AccountId20 {
	type Error = &'static str;

	fn try_from(public: ed25519::Public) -> Result<Self, Self::Error> {
		let account =
			H160(keccak_256(&public.0)[12..].try_into().map_err(|_| "Invalid account id")?);
		Ok(Self(account.0))
	}
}

impl From<AccountId20> for H160 {
	fn from(value: AccountId20) -> H160 {
		H160(value.0)
	}
}

#[cfg(feature = "std")]
impl std::str::FromStr for AccountId20 {
	type Err = &'static str;
	fn from_str(input: &str) -> Result<Self, Self::Err> {
		H160::from_str(input).map(Into::into).map_err(|_| "invalid hex address.")
	}
}

#[derive(
	Eq, PartialEq, Clone, Encode, Decode, sp_core::RuntimeDebug, TypeInfo, Serialize, Deserialize,
)]
pub struct EthereumSignature(pub ecdsa::Signature);

impl From<ecdsa::Signature> for EthereumSignature {
	fn from(x: ecdsa::Signature) -> Self {
		EthereumSignature(x)
	}
}

impl sp_runtime::traits::Verify for EthereumSignature {
	type Signer = EthereumSigner;

	/// Verify this signature is for `msg` produced by `signer`
	///
	/// As a fallback checks if the signature verifies using Ethereum's 'personal sign' scheme
	/// `keccak256(prefix + message.len() + message)`
	fn verify<L: sp_runtime::traits::Lazy<[u8]>>(&self, mut msg: L, signer: &AccountId20) -> bool {
		let message = msg.get();
		let m = keccak_256(message);
		// Standard signature
		if verify_signature(self.0.as_ref(), &m, signer) {
			return true;
		}

		// Ethereum signed signature
		let m = keccak_256(ethereum_signed_message(message).as_slice());
		if verify_signature(self.0.as_ref(), &m, signer) {
			return true;
		}

		// Try blake2_256 hashing the message, this is to prevent invalid characters showing in
		// Metamask
		let m = keccak_256(ethereum_signed_message(&blake2_256(message)[..]).as_slice());
		verify_signature(self.0.as_ref(), &m, signer)
	}
}

pub fn verify_signature(signature: &[u8; 65], message: &[u8; 32], signer: &AccountId20) -> bool {
	match sp_io::crypto::secp256k1_ecdsa_recover(signature, message) {
		Ok(pubkey) => {
			AccountId20(keccak_256(&pubkey)[12..].try_into().expect("Expected 20 bytes")) == *signer
		},
		Err(sp_io::EcdsaVerifyError::BadRS) => {
			log::error!(target: "evm", "Error recovering: Incorrect value of R or S");
			false
		},
		Err(sp_io::EcdsaVerifyError::BadV) => {
			log::error!(target: "evm", "Error recovering: Incorrect value of V");
			false
		},
		Err(sp_io::EcdsaVerifyError::BadSignature) => {
			log::error!(target: "evm", "Error recovering: Invalid signature");
			false
		},
	}
}

/// Constructs the message that Ethereum RPC's `ethereum_sign` and `eth_sign` would sign.
pub fn ethereum_signed_message(message: &[u8]) -> Vec<u8> {
	let mut v = b"\x19Ethereum Signed Message:\n".to_vec();
	v.extend(message.len().to_string().as_bytes());
	v.extend_from_slice(message);
	v
}

/// Public key for an Ethereum / Moonbeam compatible account
#[derive(
	Eq, PartialEq, Ord, PartialOrd, Clone, Encode, Decode, sp_core::RuntimeDebug, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct EthereumSigner([u8; 20]);

impl sp_runtime::traits::IdentifyAccount for EthereumSigner {
	type AccountId = AccountId20;
	fn into_account(self) -> AccountId20 {
		AccountId20(self.0)
	}
}

impl From<[u8; 20]> for EthereumSigner {
	fn from(x: [u8; 20]) -> Self {
		EthereumSigner(x)
	}
}

impl From<ecdsa::Public> for EthereumSigner {
	fn from(x: ecdsa::Public) -> Self {
		let decompressed = libsecp256k1::PublicKey::parse_slice(
			&x.0,
			Some(libsecp256k1::PublicKeyFormat::Compressed),
		)
		.expect("Wrong compressed public key provided")
		.serialize();
		let mut m = [0u8; 64];
		m.copy_from_slice(&decompressed[1..65]);
		let account = H160(keccak_256(&m)[12..].try_into().expect("Expected 20 bytes"));
		EthereumSigner(account.into())
	}
}

impl From<libsecp256k1::PublicKey> for EthereumSigner {
	fn from(x: libsecp256k1::PublicKey) -> Self {
		let mut m = [0u8; 64];
		m.copy_from_slice(&x.serialize()[1..65]);
		let account = H160(keccak_256(&m)[12..].try_into().expect("Expected 20 bytes"));
		EthereumSigner(account.into())
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for EthereumSigner {
	fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(fmt, "ethereum signature: {:?}", H160::from_slice(&self.0))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use hex_literal::hex;
	use sp_core::{ecdsa, Pair};
	use sp_runtime::traits::{IdentifyAccount, Verify};

	#[test]
	fn test_account_derivation_1() {
		// Test from https://asecuritysite.com/encryption/ethadd
		let secret_key =
			hex::decode("502f97299c472b88754accd412b7c9a6062ef3186fba0c0388365e1edec24875")
				.unwrap();
		let mut expected_hex_account = [0u8; 20];
		hex::decode_to_slice("976f8456e4e2034179b284a23c0e0c8f6d3da50c", &mut expected_hex_account)
			.expect("example data is 20 bytes of valid hex");

		let public_key = ecdsa::Pair::from_seed_slice(&secret_key).unwrap().public();
		let account: EthereumSigner = public_key.into();
		let expected_account = AccountId20::from(expected_hex_account);
		assert_eq!(account.into_account(), expected_account);
	}

	#[test]
	fn test_account_derivation_2() {
		// Test from https://asecuritysite.com/encryption/ethadd
		let secret_key =
			hex::decode("0f02ba4d7f83e59eaa32eae9c3c4d99b68ce76decade21cdab7ecce8f4aef81a")
				.unwrap();
		let mut expected_hex_account = [0u8; 20];
		hex::decode_to_slice("420e9f260b40af7e49440cead3069f8e82a5230f", &mut expected_hex_account)
			.expect("example data is 20 bytes of valid hex");

		let public_key = ecdsa::Pair::from_seed_slice(&secret_key).unwrap().public();
		let account: EthereumSigner = public_key.into();
		let expected_account = AccountId20::from(expected_hex_account);
		assert_eq!(account.into_account(), expected_account);
	}

	#[test]
	fn test_account_derivation_raw() {
		let message = keccak_256(b"\x19Ethereum Signed Message:\n7Testing");
		// Hash of message: 0x7cb2a5416e92bcb656bb09626c685f876b57e2962dc20b3376bfdf1c01ef863f
		let signature_raw = hex!["a2681e584058b5725e86b7d00c9a05963eff07543c9fb7e7f1a9b9980b5ae17b5428b8ccc0306c37923f5ef3e7762fd6ddf5f0278aae976f5fa8363f14e71aaa1c"];

		let mut expected_hex_account = [0u8; 20];
		hex::decode_to_slice("3DA64aDE0Fd4354c3c7FF6A45A849b8CB94e3D2b", &mut expected_hex_account)
			.expect("example data is 20 bytes of valid hex");

		// Test ecdsa recover function
		let result = sp_io::crypto::secp256k1_ecdsa_recover(&signature_raw, &message);
		match result {
			Ok(pub_key) => {
				println!("{:?}", pub_key);
				assert_eq!(
					AccountId20::from(expected_hex_account),
					AccountId20(keccak_256(&pub_key)[12..].try_into().unwrap())
				)
			},
			_ => panic!(),
		}

		// Now check verify function
		let pair = ecdsa::Pair::from_seed(&hex![
			"bbf3533f0c06a5715eed1c9887c0ec55398dfed316798d8df9c2ff4b3f4765c4"
		]);
		let address: EthereumSigner = pair.public().into(); // 0x3DA64aDE0Fd4354c3c7FF6A45A849b8CB94e3D2b
		let signature: EthereumSignature = ecdsa::Signature(signature_raw).into();
		let message = "Testing";
		assert!(signature.verify(message.as_ref(), &address.into_account()));
	}

	#[test]
	fn ed25519_to_ethereum() {
		let public_key = "FB2A3A850B43E24D2700532EF1F9CCB2475DFF4F62B634B0C58845F23C263965";
		let public_key_bytes = hex::decode(public_key).unwrap();
		let public = ed25519::Public::from_raw(public_key_bytes.try_into().unwrap());
		let account: AccountId20 = public.try_into().unwrap();

		assert_eq!(hex!("83a6dd17b5db4f87b9d877a38e172f3bff0cde46"), account.0);
	}

	#[test]
	fn verify_ethereum_sign_works_0() {
		let msg = "test eth signed message";
		let pair = ecdsa::Pair::from_seed(&hex![
			"7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c"
		]);
		let address: EthereumSigner = pair.public().into(); // 0x420aC537F1a4f78d4Dfb3A71e902be0E3d480AFB
		let eth_signed_msg = &keccak_256(ethereum_signed_message(msg.as_bytes()).as_ref());
		// let signature: EthereumSignature =
		// ecdsa::Signature(hex!["
		// dd0992d40e5cdf99db76bed162808508ac65acd7ae2fdc8573594f03ed9c939773e813181788fc02c3c68f3fdc592759b35f6354484343e18cb5317d34dab6c600"
		// ]).into();
		let signature: EthereumSignature = pair.sign_prehashed(eth_signed_msg).into();

		assert!(signature.verify(msg.as_ref(), &address.into_account()));
	}

	#[test]
	fn verify_ethereum_sign_works_2() {
		let msg = "hello world";
		let pair = ecdsa::Pair::from_seed(&hex![
			"cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"
		]);
		let address: EthereumSigner = pair.public().into(); // 0xE04CC55ebEE1cBCE552f250e85c57B70B2E2625b
		let eth_signed_msg = &keccak_256(ethereum_signed_message(msg.as_bytes()).as_ref());
		let signature: EthereumSignature = pair.sign_prehashed(eth_signed_msg).into();

		assert!(signature.verify(msg.as_ref(), &address.into_account()));
	}

	#[test]
	fn verify_ethereum_sign_works_3() {
		let msg = "0x6460040300ff64d3f6efe2317ee2807d223a0bdc4c0c49dfdb44460020000400000001000000ff752da18c6a9310be5f586409e414696b4fb6b459f5cd7022eb62f2e2199521aaa3c6ff03969eede192b85b5ab05606317dfca02c0c9a2dac573ef447703680";
		let pair = ecdsa::Pair::from_seed(&hex![
			"cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"
		]);
		let address: EthereumSigner = pair.public().into(); // 0xE04CC55ebEE1cBCE552f250e85c57B70B2E2625b
		let eth_signed_msg = &keccak_256(ethereum_signed_message(msg.as_bytes()).as_ref());
		let signature: EthereumSignature = pair.sign_prehashed(eth_signed_msg).into();

		assert!(signature.verify(msg.as_ref(), &address.into_account()));
	}

	#[test]
	fn verify_fails() {
		let msg = "test eth signed message";
		let pair = ecdsa::Pair::from_seed(&hex![
			"7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c"
		]);
		let address: EthereumSigner = pair.public().into(); // 0x420aC537F1a4f78d4Dfb3A71e902be0E3d480AFB
		let signature: EthereumSignature = ecdsa::Signature(hex!["ad0992d40e5cdf99db76bed162808508ac65acd7ae2fdc8573594f03ed9c939773e813181788fc02c3c68f3fdc592759b35f6354484343e18cb5317d34dab6c61b"]).into();

		assert!(!signature.verify(msg.as_ref(), &address.into_account()));
	}

	#[test]
	fn construct_and_verify_message() {
		let msg_hash: &[u8; 32] =
			&hex!("4bb8b8a113de9a87a8c02cace5c8a9f61e478eaaa8f8100773a4c207f2c06662");
		let msg_hash_str: &str = &hex::encode(msg_hash);
		let eth_signed_msg = &keccak_256(ethereum_signed_message(msg_hash_str.as_bytes()).as_ref()); // 71ea60525c727e50bfa2358ef14e7456bae41fe483ed104341e1376ab3141338

		let pair = ecdsa::Pair::from_seed(&hex![
			"7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c"
		]);
		let address: EthereumSigner = pair.public().into(); // 0x420aC537F1a4f78d4Dfb3A71e902be0E3d480AFB

		let signature: EthereumSignature = pair.sign_prehashed(eth_signed_msg).into();
		assert!(signature.verify(msg_hash_str.as_ref(), &address.into_account()));

		match sp_io::crypto::secp256k1_ecdsa_recover(&signature.0 .0, eth_signed_msg) {
			Ok(pubkey_bytes) => {
				let account = AccountId20(keccak_256(&pubkey_bytes)[12..].try_into().unwrap());
				assert_eq!(
					AccountId20::from(hex!("420aC537F1a4f78d4Dfb3A71e902be0E3d480AFB")),
					account
				);
			},
			_ => panic!(),
		};
	}
}
