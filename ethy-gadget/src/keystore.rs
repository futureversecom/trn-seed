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

use sp_application_crypto::RuntimeAppPublic;
use sp_core::keccak_256;
use sp_keystore::{Keystore, KeystorePtr};

pub use seed_primitives::ethy::EthyEcdsaToEthereum;
use seed_primitives::ethy::{
	crypto::{AuthorityId as Public, AuthoritySignature as Signature},
	ETHY_KEY_TYPE,
};

use crate::error;

/// An Ethy specific keystore implemented as a `Newtype`. This is basically a
/// wrapper around [`sp_keystore::Keystore`] and allows to customize
/// common cryptographic functionality.
pub(crate) struct EthyKeystore(Option<KeystorePtr>);

impl EthyKeystore {
	/// Check if the keystore contains a private key for one of the public keys
	/// contained in `keys`. A public key with a matching private key is known
	/// as a local authority id.
	///
	/// Return the public key for which we also do have a private key. If no
	/// matching private key is found, `None` will be returned.
	pub fn authority_id(&self, keys: &[Public]) -> Option<Public> {
		let store = self.0.clone()?;

		for key in keys {
			if Keystore::has_keys(&*store, &[(key.to_raw_vec(), ETHY_KEY_TYPE)]) {
				return Some(key.clone());
			}
		}

		None
	}

	/// Sign `message` with the `public` key.
	///
	/// Return the message signature or an error in case of failure.
	pub fn sign_prehashed(
		&self,
		public: &Public,
		message: &[u8; 32],
	) -> Result<Signature, error::Error> {
		let store = self.0.clone().ok_or_else(|| error::Error::Keystore("no Keystore".into()))?;

		let public = public.as_ref();

		// Sign the message (it is already)
		// use `_prehashed` to avoid any changes to the message
		let sig = Keystore::ecdsa_sign_prehashed(&*store, ETHY_KEY_TYPE, public, message)
			.map_err(|e| error::Error::Keystore(e.to_string()))?
			.ok_or_else(|| error::Error::Signature("ecdsa_sign_prehashed() failed".to_string()))?;

		Ok(sig.into())
	}

	/// Returns a vector of Public keys which are currently supported
	/// (i.e. found in the keystore).
	#[allow(dead_code)]
	pub fn public_keys(&self) -> Result<Vec<Public>, error::Error> {
		let store = self.0.clone().ok_or_else(|| error::Error::Keystore("no Keystore".into()))?;

		let pk: Vec<Public> = Keystore::ecdsa_public_keys(&*store, ETHY_KEY_TYPE)
			.drain(..)
			.map(Public::from)
			.collect();

		Ok(pk)
	}

	/// Use the `public` key to verify that `sig` is a valid signature for `message`.
	///
	/// Return `true` if the signature is authentic, `false` otherwise.
	#[allow(dead_code)]
	pub fn verify(public: &Public, sig: &Signature, message: &[u8]) -> bool {
		let msg = keccak_256(message);
		let sig = sig.as_ref();
		let public = public.as_ref();

		sp_core::ecdsa::Pair::verify_prehashed(sig, &msg, public)
	}

	/// Use the `public` key to verify that `sig` is a valid signature for `digest`.
	///
	/// Return `true` if the signature is authentic, `false` otherwise.
	pub fn verify_prehashed(public: &Public, sig: &Signature, digest: &[u8; 32]) -> bool {
		sp_core::ecdsa::Pair::verify_prehashed(sig.as_ref(), digest, public.as_ref())
	}
}

impl From<Option<KeystorePtr>> for EthyKeystore {
	fn from(store: Option<KeystorePtr>) -> EthyKeystore {
		EthyKeystore(store)
	}
}

#[cfg(test)]
mod tests {
	use sp_application_crypto::Pair as _PairT;
	use sp_core::{ecdsa, keccak_256};
	use sp_keystore::Keystore;

	use seed_primitives::ethy::{
		crypto::{AuthorityId as Public, AuthorityPair as Pair},
		ETHY_KEY_TYPE,
	};

	use super::EthyKeystore;
	use crate::{
		error::Error,
		testing::{keystore, Keyring},
	};

	#[test]
	fn verify_should_work() {
		let msg = keccak_256(b"I am Alice!");
		let sig = Keyring::Alice.sign(b"I am Alice!");

		assert!(ecdsa::Pair::verify_prehashed(
			&sig.clone().into(),
			&msg,
			&Keyring::Alice.public().into(),
		));

		// different public key -> fail
		assert!(!ecdsa::Pair::verify_prehashed(
			&sig.clone().into(),
			&msg,
			&Keyring::Bob.public().into(),
		));

		let msg = keccak_256(b"I am not Alice!");

		// different msg -> fail
		assert!(
			!ecdsa::Pair::verify_prehashed(&sig.into(), &msg, &Keyring::Alice.public().into(),)
		);
	}

	#[test]
	fn pair_works() {
		let want = Pair::from_string("//Alice", None).expect("Pair failed").to_raw_vec();
		let got = Keyring::Alice.pair().to_raw_vec();
		assert_eq!(want, got);

		let want = Pair::from_string("//Bob", None).expect("Pair failed").to_raw_vec();
		let got = Keyring::Bob.pair().to_raw_vec();
		assert_eq!(want, got);

		let want = Pair::from_string("//Charlie", None).expect("Pair failed").to_raw_vec();
		let got = Keyring::Charlie.pair().to_raw_vec();
		assert_eq!(want, got);

		let want = Pair::from_string("//Dave", None).expect("Pair failed").to_raw_vec();
		let got = Keyring::Dave.pair().to_raw_vec();
		assert_eq!(want, got);

		let want = Pair::from_string("//Eve", None).expect("Pair failed").to_raw_vec();
		let got = Keyring::Eve.pair().to_raw_vec();
		assert_eq!(want, got);

		let want = Pair::from_string("//Ferdie", None).expect("Pair failed").to_raw_vec();
		let got = Keyring::Ferdie.pair().to_raw_vec();
		assert_eq!(want, got);

		let want = Pair::from_string("//One", None).expect("Pair failed").to_raw_vec();
		let got = Keyring::One.pair().to_raw_vec();
		assert_eq!(want, got);

		let want = Pair::from_string("//Two", None).expect("Pair failed").to_raw_vec();
		let got = Keyring::Two.pair().to_raw_vec();
		assert_eq!(want, got);
	}

	#[test]
	fn authority_id_works() {
		let store = keystore();

		let alice: Public =
			Keystore::ecdsa_generate_new(&*store, ETHY_KEY_TYPE, Some(&Keyring::Alice.to_seed()))
				.ok()
				.unwrap()
				.into();

		let bob = Keyring::Bob.public();
		let charlie = Keyring::Charlie.public();

		let store: EthyKeystore = Some(store).into();

		let mut keys = vec![bob, charlie];

		let id = store.authority_id(keys.as_slice());
		assert!(id.is_none());

		keys.push(alice.clone());

		let id = store.authority_id(keys.as_slice()).unwrap();
		assert_eq!(id, alice);
	}

	#[test]
	fn sign_works() {
		let store = keystore();

		let alice: Public =
			Keystore::ecdsa_generate_new(&*store, ETHY_KEY_TYPE, Some(&Keyring::Alice.to_seed()))
				.ok()
				.unwrap()
				.into();

		let store: EthyKeystore = Some(store).into();

		let msg = b"are you involved or committed?";
		let sig1 = store.sign_prehashed(&alice, &keccak_256(msg)).unwrap();
		let sig2 = Keyring::Alice.sign(msg);

		assert_eq!(sig1, sig2);
	}

	#[test]
	fn sign_error() {
		let store = keystore();

		let _ = Keystore::ecdsa_generate_new(&*store, ETHY_KEY_TYPE, Some(&Keyring::Bob.to_seed()))
			.ok()
			.unwrap();

		let store: EthyKeystore = Some(store).into();

		let alice = Keyring::Alice.public();

		let msg = b"are you involved or committed?";
		let sig = store.sign_prehashed(&alice, &keccak_256(msg)).err().unwrap();
		let err = Error::Signature("ecdsa_sign_prehashed() failed".to_string());

		assert_eq!(sig, err);
	}

	#[test]
	fn sign_no_keystore() {
		let store: EthyKeystore = None.into();

		let alice = Keyring::Alice.public();
		let msg = b"are you involved or committed?";
		let sig = store.sign_prehashed(&alice, &keccak_256(msg)).err().unwrap();
		let err = Error::Keystore("no Keystore".to_string());
		assert_eq!(sig, err);
	}

	#[test]
	fn verify_works() {
		let store = keystore();

		let alice: Public =
			Keystore::ecdsa_generate_new(&*store, ETHY_KEY_TYPE, Some(&Keyring::Alice.to_seed()))
				.ok()
				.unwrap()
				.into();

		let store: EthyKeystore = Some(store).into();

		// `msg` and `sig` match
		let msg = b"are you involved or committed?";
		let sig = store.sign_prehashed(&alice, &keccak_256(msg)).unwrap();
		assert!(EthyKeystore::verify(&alice, &sig, msg));

		// `msg and `sig` don't match
		let msg = b"you are just involved";
		assert!(!EthyKeystore::verify(&alice, &sig, msg));
	}

	// Note that we use keys with and without a seed for this test.
	#[test]
	fn public_keys_works() {
		const TEST_TYPE: sp_application_crypto::KeyTypeId =
			sp_application_crypto::KeyTypeId(*b"test");

		let store = keystore();

		let add_key = |key_type, seed: Option<&str>| {
			Keystore::ecdsa_generate_new(&*store, key_type, seed).unwrap()
		};

		// test keys
		let _ = add_key(TEST_TYPE, Some(Keyring::Alice.to_seed().as_str()));
		let _ = add_key(TEST_TYPE, Some(Keyring::Bob.to_seed().as_str()));

		let _ = add_key(TEST_TYPE, None);
		let _ = add_key(TEST_TYPE, None);

		// Ethy keys
		let _ = add_key(ETHY_KEY_TYPE, Some(Keyring::Dave.to_seed().as_str()));
		let _ = add_key(ETHY_KEY_TYPE, Some(Keyring::Eve.to_seed().as_str()));

		let key1: Public = add_key(ETHY_KEY_TYPE, None).into();
		let key2: Public = add_key(ETHY_KEY_TYPE, None).into();

		let store: EthyKeystore = Some(store).into();

		let keys = store.public_keys().ok().unwrap();

		assert!(keys.len() == 4);
		assert!(keys.contains(&Keyring::Dave.public()));
		assert!(keys.contains(&Keyring::Eve.public()));
		assert!(keys.contains(&key1));
		assert!(keys.contains(&key2));
	}
}
