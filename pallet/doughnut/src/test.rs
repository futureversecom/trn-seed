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
	signature::{SignatureVersion, sign_ecdsa, verify_signature},
	traits::{DoughnutApi, DoughnutVerify, Signing},
	doughnut::{Doughnut, DoughnutV0, DoughnutV1},
};
use frame_support::{
	assert_ok,
	dispatch::{DispatchClass, GetDispatchInfo},
	traits::fungibles::Mutate,
};
use frame_system::{limits::BlockWeights, RawOrigin};
use hex_literal::hex;
use pallet_transaction_payment::ChargeTransactionPayment;
use seed_pallet_common::{test_prelude::*, CreateExt};
use sp_core::{bytes::to_hex, ecdsa, ecdsa::Public, keccak_256, ByteArray, Pair, H512, U256};
use sp_runtime::{print, traits::SignedExtension, Perbill};

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
	let signature = doughnut_v1.sign_ecdsa(issuer_secret_key).unwrap();
	println!("sig {:?}", signature);

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
		let issuer: ecdsa::Pair = Pair::from_string("//Bob", None).unwrap();
		let holder: ecdsa::Pair = Pair::from_string("//Alice", None).unwrap();
		let mut doughnut = make_doughnut(
			holder.public(),
			issuer.public(),
			&hex!("79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf"),
			"",
			vec![],
		);
		// doughnut.sign_ecdsa(&hex!("
		// cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"));
		// if let Doughnut::V0(doughnut_v0) = doughnut {
		// 	doughnut_v0.signature =
		// doughnut_rs::signature::sign_ecdsa(&hex!("
		// cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"),
		// &doughnut.encode()).unwrap();
		//
		// }

		println!("issuer: {:?}", issuer.public());
		println!("holder: {:?}", holder.public());

		let issuer_address: AccountId =
			crate::pallet::Pallet::<Test>::get_address(issuer.public().0.try_into().unwrap())
				.unwrap();
		let holder_address: AccountId =
			crate::pallet::Pallet::<Test>::get_address(holder.public().0.try_into().unwrap())
				.unwrap();

		println!("issuer address: {:?}", to_hex(issuer_address.0.as_slice(), false));
		println!("holder address: {:?}", to_hex(holder_address.0.as_slice(), false));

		let doughnut_encoded = doughnut.encode();
		println!("{:?}", to_hex(doughnut_encoded.clone().as_slice(), false));

		let doughnut_decoded = Doughnut::decode(&mut &doughnut_encoded[..]).unwrap();
		println!("the doughnut {:?}", doughnut_decoded);

		// Verify doughnut works
		let Doughnut::V1(doughnut_v1) = doughnut_decoded.clone() else {
			panic!("Wrong doughnut version");
		};
		println!("Sig bytes length: {:?}", doughnut_v1.signature().len());
		assert_ok!(doughnut_v1.verify());

		// Print bobs signature over the doughnut
		let bob_private = hex!("79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf");
		// Actually Alices private
		let bob_private = hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854");
		let bob_signature = sign_ecdsa(&bob_private, &doughnut_encoded.as_slice()).unwrap();
		println!("Holder signature: {:?}", to_hex(bob_signature.as_slice(), false));

		// Verify Bob's signature
		assert_ok!(verify_signature(
			SignatureVersion::ECDSA as u8,
			&bob_signature,
			&doughnut_v1.holder(),
			&doughnut_encoded.clone()
		));
	});
}
