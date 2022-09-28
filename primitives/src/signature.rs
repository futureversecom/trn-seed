use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{ecdsa, H160};
use sp_io::hashing::keccak_256;
use sp_std::vec::Vec;

#[derive(
	Eq, PartialEq, Copy, Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Default, PartialOrd, Ord,
)]
pub struct AccountId20(pub [u8; 20]);

#[cfg(feature = "std")]
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

impl Into<[u8; 20]> for AccountId20 {
	fn into(self) -> [u8; 20] {
		self.0
	}
}

impl From<H160> for AccountId20 {
	fn from(h160: H160) -> Self {
		Self(h160.0)
	}
}

impl Into<H160> for AccountId20 {
	fn into(self) -> H160 {
		H160(self.0)
	}
}

#[cfg(feature = "std")]
impl std::str::FromStr for AccountId20 {
	type Err = &'static str;
	fn from_str(input: &str) -> Result<Self, Self::Err> {
		H160::from_str(input).map(Into::into).map_err(|_| "invalid hex address.")
	}
}

#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Eq, PartialEq, Clone, Encode, Decode, sp_core::RuntimeDebug, TypeInfo)]
pub struct EthereumSignature(ecdsa::Signature);

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

		let native_signature_valid =
			match sp_io::crypto::secp256k1_ecdsa_recover(self.0.as_ref(), &m) {
				Ok(pubkey) => AccountId20(keccak_256(&pubkey)[12..].try_into().unwrap()) == *signer,
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
			};
		if native_signature_valid {
			return true
		}

		let m = keccak_256(personal_sign_message(message).as_slice());
		match sp_io::crypto::secp256k1_ecdsa_recover(self.0.as_ref(), &m) {
			Ok(pubkey) => AccountId20(keccak_256(&pubkey)[12..].try_into().unwrap()) == *signer,
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
}

/// Constructs the message that Ethereum RPC's `personal_sign` and `eth_sign` would sign.
pub fn personal_sign_message(message: &[u8]) -> Vec<u8> {
	let mut l = message.len();
	let mut rev = Vec::new();
	while l > 0 {
		rev.push(b'0' + (l % 10) as u8);
		l /= 10;
	}
	let mut v = b"\x19Ethereum Signed Message:\n".to_vec();
	v.extend(rev.into_iter().rev());
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
		let account = H160(keccak_256(&m)[12..].try_into().unwrap());
		EthereumSigner(account.into())
	}
}

impl From<libsecp256k1::PublicKey> for EthereumSigner {
	fn from(x: libsecp256k1::PublicKey) -> Self {
		let mut m = [0u8; 64];
		m.copy_from_slice(&x.serialize()[1..65]);
		let account = H160(keccak_256(&m)[12..].try_into().unwrap());
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
	fn verify_personal_sign_works() {
		let msg = "test eth signed message";
		let pair = ecdsa::Pair::from_seed(&hex![
			"7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c"
		]);
		let address: EthereumSigner = pair.public().into();
		let signature: EthereumSignature = ecdsa::Signature(hex!["dd0992d40e5cdf99db76bed162808508ac65acd7ae2fdc8573594f03ed9c939773e813181788fc02c3c68f3fdc592759b35f6354484343e18cb5317d34dab6c61b"]).into();

		assert!(signature.verify(msg.as_ref(), &address.into_account()));
	}

	#[test]
	fn verify_fails() {
		let msg = "test eth signed message";
		let pair = ecdsa::Pair::from_seed(&hex![
			"7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c"
		]);
		let address: EthereumSigner = pair.public().into();
		let signature: EthereumSignature = ecdsa::Signature(hex!["ad0992d40e5cdf99db76bed162808508ac65acd7ae2fdc8573594f03ed9c939773e813181788fc02c3c68f3fdc592759b35f6354484343e18cb5317d34dab6c61b"]).into();

		assert!(!signature.verify(msg.as_ref(), &address.into_account()));
	}
}
