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

use super::Event;
use crate::mock::{RuntimeEvent as MockEvent, *};
use codec::{Decode, Encode};
use doughnut_rs::{
	signature::sign_ecdsa,
	traits::{DoughnutApi, DoughnutVerify, Signing},
	v0::DoughnutV0,
	Doughnut,
};
use frame_support::{
	assert_ok,
	dispatch::{DispatchClass, GetDispatchInfo},
	traits::fungibles::Mutate,
};
use frame_system::{limits::BlockWeights, RawOrigin};
use libsecp256k1::SecretKey as ECDSASecretKey;
use pallet_transaction_payment::ChargeTransactionPayment;
use seed_pallet_common::{test_prelude::*, CreateExt};
use seed_primitives::AccountId20;
use sp_core::{
	blake2_256,
	bytes::to_hex,
	ecdsa,
	ecdsa::{Pair as ECDSAKeyPair, Public},
	keccak_256, ByteArray, Pair, H512, U256,
};
use sp_runtime::{print, traits::SignedExtension, Perbill};

fn make_doughnut(
	holder: Public,
	issuer: Public,
	domain: &str,
	domain_payload: Vec<u8>,
) -> Doughnut {
	let doughnut_v0 = DoughnutV0 {
		holder: holder.as_slice()[1..].try_into().expect("should not fail"),
		issuer: issuer.as_slice()[1..].try_into().expect("should not fail"),
		domains: vec![(domain.to_string(), domain_payload)],
		expiry: 0,
		not_before: 0,
		payload_version: 0,
		signature_version: 0,
		signature: Default::default(),
	};

	Doughnut::V0(doughnut_v0)
}

// fn generate_ecdsa_keypair(seed: &str) -> (ECDSAKeyPair, ECDSASecretKey) {
// 	let (pair, seed) = ECDSAKeyPair::from_string(seed, None).unwrap();
// 	(pair, ECDSASecretKey::parse(&seed.into()).expect("can not error here"))
// }

#[test]
fn doughnut_transact_call_successful() {
	TestExt::<Test>::default().build().execute_with(|| {
		let issuer = ecdsa::Pair::generate().0;
		let holder = ecdsa::Pair::generate().0;
		let doughnut = make_doughnut(holder.public(), issuer.public(), "", vec![]);

		println!("{:?}", doughnut.encode());
	});
}

#[test]
fn alice_to_bob_doughnut() {
	TestExt::<Test>::default().build().execute_with(|| {
		let issuer: ecdsa::Pair = Pair::from_string("//Alith", None).unwrap();
		let holder: ecdsa::Pair = Pair::from_string("//Baltathar", None).unwrap();
		let doughnut = make_doughnut(holder.public(), issuer.public(), "", vec![]);

		println!("issuer: {:?}", issuer.public());
		println!("holder: {:?}", holder.public());

		let doughnut_encoded = doughnut.encode();
		println!("{:?}", to_hex(doughnut_encoded.clone().as_slice(), false));

		let doughnut_decoded = Doughnut::decode(&mut &doughnut_encoded[..]).unwrap();
		println!("the doughnut {:?}", doughnut_decoded);
	});
}

#[test]
fn alice_to_bob_doughnut_with_signature() {
	TestExt::<Test>::default().build().execute_with(|| {
		// let (issuer_keypair, issuer_secret_key) = generate_ecdsa_keypair("//Alith");
		// let (holder_keypair, holder_secret_key) = generate_ecdsa_keypair("//Baltathar");
		let alice_secret = "cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854";
		let bob_secret = b"79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf";
		let issuer: ecdsa::Pair = Pair::from_string("//Alice", None).unwrap();
		let holder: ecdsa::Pair = Pair::from_string("//Bob", None).unwrap();

		// let maybe_seed = ECDSASecretKey::parse(&"//Alice".into()).expect("can not error here");
		// println!("maybe_seed: {:?}", maybe_seed);
		// let payload = "To a deep sea diver who is swimming with a raincoat".as_bytes();

		// let signed_payload = "To a deep sea diver who is swimming without a raincoat".as_bytes();
		// let signature =
		// 	sign_ecdsa(issuer_secret_key.serialize().as_slice(), signed_payload).unwrap();
		// let signature1 = issuer.sign_prehashed(&keccak_256(payload));
		// let mut sig: [u8; 64] = [0; 64];
		// sig.clone_from_slice(&signature1.0.as_slice()[1..]);
		// let signature = H512::from(sig);
		let mut doughnut = make_doughnut(holder.public(), issuer.public(), "", vec![]);

		let Doughnut::V0(mut doughnut_v0) = doughnut.clone() else {
			panic!("AAAH");
		};
		let seed = issuer.seed();
		println!("Seed: {:?}", seed);
		let signature_sig = doughnut_v0.sign_ecdsa(&seed);
		doughnut = Doughnut::V0(doughnut_v0.clone());

		println!("Payload {:?}", doughnut_v0.clone().payload());
		println!("Signature raw: {:?}", signature_sig);
		println!("Signature {:?}", doughnut_v0.clone().signature());
		println!("Signature Version {:?}", doughnut_v0.clone().signature_version());
		println!("issuer: {:?}", issuer.public());
		println!("holder: {:?}", holder.public());

		let doughnut_encoded = doughnut.encode();
		println!("{:?}", to_hex(doughnut_encoded.clone().as_slice(), false));

		let bob_signature = holder.sign_prehashed(&keccak_256(doughnut_encoded.clone().as_slice()));
		println!("Holder signature: {:?}", bob_signature);

		let doughnut_decoded = Doughnut::decode(&mut &doughnut_encoded[..]).unwrap();
		println!("the doughnut {:?}", doughnut_decoded);

		// Verify Bob's signature of the outer doughnut
		let sig: [u8; 65] = bob_signature.0.as_slice().try_into().unwrap();
		let message: [u8; 32] = keccak_256(doughnut_encoded.as_slice());
		let signer = AccountId20::try_from(holder.public()).unwrap();
		println!("Holder: {:?}", signer);
		let bob_verified = seed_primitives::verify_signature(&sig, &message, &signer);
		assert!(bob_verified);

		let Doughnut::V0(mut doughnut_v0) = doughnut_decoded.clone() else {
			panic!("AAAH");
		};
		assert_ok!(doughnut_v0.verify());

		// let sig: [u8; 65] = signature1.0.as_slice().try_into().unwrap();
		// let message: [u8; 32] = keccak_256(payload);
		// let signer = AccountId20::try_from(issuer.public()).unwrap();
		// let alice_verified = seed_primitives::verify_signature(&sig, &message, &signer);
		// assert!(alice_verified);

		// Alice = issuer
		// Alice gives doughnut to Bob
		// Bob = Holder
		// Bob submits transaction with doughnut
		// Doughnut executes inner call from Alice's account
		// Alice pays gas
	});
}
