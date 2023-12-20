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

use crate::mock::*;
use codec::Encode;
use doughnut_rs::{
	doughnut::{Doughnut, DoughnutV1},
	signature::{sign_ecdsa, verify_signature, SignatureVersion},
	traits::{DoughnutVerify, Signing},
};
use frame_support::assert_ok;
use hex_literal::hex;
use seed_pallet_common::test_prelude::*;
use sp_core::{bytes::to_hex, ecdsa, ecdsa::Public, ByteArray, Pair};

fn make_doughnut(
	holder: Public,
	issuer: Public,
	issuer_secret_key: &[u8; 32],
	domain: &str,
	domain_payload: Vec<u8>,
) -> Doughnut {
	let mut doughnut_v1 = DoughnutV1 {
		holder: holder.as_slice().try_into().expect("should not fail"),
		issuer: issuer.as_slice().try_into().expect("should not fail"),
		domains: vec![(domain.to_string(), domain_payload)],
		expiry: 0,
		not_before: 0,
		payload_version: 0,
		signature_version: SignatureVersion::ECDSA as u8,
		signature: [0_u8; 64],
	};
	// Sign and verify doughnut
	assert_ok!(doughnut_v1.sign_ecdsa(issuer_secret_key));
	assert_ok!(doughnut_v1.verify());
	Doughnut::V1(doughnut_v1)
}

#[test]
fn doughnut_transact_call_successful() {
	TestExt::<Test>::default().build().execute_with(|| {
		let issuer = ecdsa::Pair::generate().0;
		let holder = ecdsa::Pair::generate().0;
		let doughnut = make_doughnut(
			holder.public(),
			issuer.public(),
			&hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"),
			"",
			vec![],
		);

		println!("{:?}", doughnut.encode());
	});
}

#[test]
fn bob_to_alice_doughnut() {
	TestExt::<Test>::default().build().execute_with(|| {
		let bob_private = hex!("79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf");
		let alice_private =
			hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854");
		let issuer: ecdsa::Pair = Pair::from_string("//Bob", None).unwrap();
		let holder: ecdsa::Pair = Pair::from_string("//Alice", None).unwrap();
		let doughnut = make_doughnut(holder.public(), issuer.public(), &bob_private, "", vec![]);

		let issuer_address: AccountId =
			crate::pallet::Pallet::<Test>::get_address(issuer.public().0.try_into().unwrap())
				.unwrap();
		let holder_address: AccountId =
			crate::pallet::Pallet::<Test>::get_address(holder.public().0.try_into().unwrap())
				.unwrap();

		println!("issuer address: {:?}", to_hex(issuer_address.0.as_slice(), false));
		println!("holder address: {:?}", to_hex(holder_address.0.as_slice(), false));

		let doughnut_encoded = doughnut.encode();
		println!("Encoded doughnut");
		println!("{:?}", to_hex(doughnut_encoded.clone().as_slice(), false));

		// let doughnut_decoded = Doughnut::decode(&mut &doughnut_encoded[..]).unwrap();
		// println!("the doughnut {:?}", doughnut_decoded);

		// Print Alice's signature over the doughnut
		let alice_signature = sign_ecdsa(&alice_private, &doughnut_encoded.as_slice()).unwrap();
		println!("Holder signature: {:?}", to_hex(alice_signature.as_slice(), false));

		// Verify Alice's signature
		assert_ok!(verify_signature(
			SignatureVersion::ECDSA as u8,
			&alice_signature,
			&holder.public().as_slice(),
			&doughnut_encoded.clone()
		));
	});
}

#[test]
fn alice_to_bob_doughnut() {
	TestExt::<Test>::default().build().execute_with(|| {
		let bob_private = hex!("79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf");
		let alice_private =
			hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854");
		let issuer: ecdsa::Pair = Pair::from_string("//Alice", None).unwrap();
		let holder: ecdsa::Pair = Pair::from_string("//Bob", None).unwrap();
		let doughnut = make_doughnut(holder.public(), issuer.public(), &alice_private, "1", vec![]);

		let issuer_address: AccountId =
			crate::pallet::Pallet::<Test>::get_address(issuer.public().0.try_into().unwrap())
				.unwrap();
		let holder_address: AccountId =
			crate::pallet::Pallet::<Test>::get_address(holder.public().0.try_into().unwrap())
				.unwrap();

		println!("issuer address: {:?}", to_hex(issuer_address.0.as_slice(), false));
		println!("holder address: {:?}", to_hex(holder_address.0.as_slice(), false));

		let doughnut_encoded = doughnut.encode();
		println!("Encoded doughnut");
		println!("{:?}", to_hex(doughnut_encoded.clone().as_slice(), false));

		// Print Bob's signature over the doughnut
		let bob_signature = sign_ecdsa(&bob_private, &doughnut_encoded.as_slice()).unwrap();
		println!("Holder signature: {:?}", to_hex(bob_signature.as_slice(), false));

		// Verify Bob's signature
		assert_ok!(verify_signature(
			SignatureVersion::ECDSA as u8,
			&bob_signature,
			&holder.public().as_slice(),
			&doughnut_encoded.clone()
		));
	});
}
