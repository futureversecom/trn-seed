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

use super::*;
use crate::{
	mock::{Doughnut as DoughnutPallet, *},
	BlockedDoughnuts, BlockedHolders, Error,
};
use codec::Encode;
use doughnut_rs::{
	doughnut::{Doughnut, DoughnutV1},
	signature::{sign_ecdsa, verify_signature, SignatureVersion},
	traits::{DoughnutVerify, Signing},
};
use hex_literal::hex;
use seed_pallet_common::test_prelude::*;
use sp_core::{bytes::to_hex, ecdsa, ecdsa::Public, keccak_256, ByteArray, Pair};

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
fn make_doughnut_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let alice_private =
			hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854");
		let issuer: ecdsa::Pair = Pair::from_string("//Alice", None).unwrap();
		let holder: ecdsa::Pair = Pair::from_string("//Bob", None).unwrap();
		make_doughnut(holder.public(), issuer.public(), &alice_private, "", vec![]);
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

		let issuer_address = DoughnutPallet::get_address(issuer.public().0.into()).unwrap();
		let holder_address = DoughnutPallet::get_address(holder.public().0.into()).unwrap();

		println!("issuer address (Bob): {:?}", to_hex(issuer_address.0.as_slice(), false));
		println!("holder address (Alice): {:?}", to_hex(holder_address.0.as_slice(), false));

		let doughnut_encoded = doughnut.encode();
		println!("Encoded doughnut");
		println!("{:?}", to_hex(doughnut_encoded.clone().as_slice(), false));

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

		let issuer_address = DoughnutPallet::get_address(issuer.public().0.into()).unwrap();
		let holder_address = DoughnutPallet::get_address(holder.public().0.into()).unwrap();

		println!("issuer address (Alice): {:?}", to_hex(issuer_address.0.as_slice(), false));
		println!("holder address (Bob): {:?}", to_hex(holder_address.0.as_slice(), false));

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

#[test]
fn transact_works() {
	let issuer: ecdsa::Pair = Pair::from_string("//Alice", None).unwrap();
	let issuer_address = DoughnutPallet::get_address(issuer.public().0.into()).unwrap();
	let initial_balance = 10_000;
	TestExt::<Test>::default()
		.with_balances(&[(issuer_address, initial_balance)])
		.build()
		.execute_with(|| {
			let alice_private =
				hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854");
			let holder: ecdsa::Pair = Pair::from_string("//Bob", None).unwrap();
			let holder_address = DoughnutPallet::get_address(holder.public().0.into()).unwrap();
			let doughnut =
				make_doughnut(holder.public(), issuer.public(), &alice_private, "1", vec![]);
			let doughnut_encoded = doughnut.encode();

			// Create balances transfer call
			let transfer_amount = 1234;
			let destination = create_account(12);
			let call: <Test as frame_system::Config>::RuntimeCall =
				pallet_balances::Call::<Test>::transfer {
					dest: destination,
					value: transfer_amount,
				}
				.into();

			// Attempting to transact the doughnut should succeed
			assert_ok!(DoughnutPallet::transact(
				Some(holder_address).into(),
				Box::new(call),
				doughnut_encoded,
				0,
				vec![]
			));

			// Check event is thrown
			System::assert_has_event(
				Event::DoughnutCallExecuted { result: DispatchResult::Ok(()) }.into(),
			);
			// Check balance of destination and issuer is correct
			assert_eq!(Balances::free_balance(&destination), transfer_amount);
			assert_eq!(Balances::free_balance(&issuer_address), initial_balance - transfer_amount);
		});
}

#[test]
fn transact_invalid_doughnut_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::<Test>::remark { remark: b"Mischief Managed".to_vec() }.into();

		// Attempting to transact the doughnut should fail as the doughnut is invalid
		assert_noop!(
			DoughnutPallet::transact(
				Some(create_account(10)).into(),
				Box::new(call),
				vec![], // Invalid doughnut
				0,
				vec![]
			),
			Error::<Test>::DoughnutDecodeFailed
		);
	});
}

#[test]
fn transact_holder_not_sender_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let alice_private =
			hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854");
		let holder: ecdsa::Pair = Pair::from_string("//Bob", None).unwrap();
		let issuer: ecdsa::Pair = Pair::from_string("//Alice", None).unwrap();
		let doughnut = make_doughnut(holder.public(), issuer.public(), &alice_private, "1", vec![]);
		let doughnut_encoded = doughnut.encode();

		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::<Test>::remark { remark: b"Mischief Managed".to_vec() }.into();

		// Attempting to transact the doughnut as a random account should fail
		assert_noop!(
			DoughnutPallet::transact(
				Some(create_account(10)).into(),
				Box::new(call),
				doughnut_encoded,
				0,
				vec![]
			),
			Error::<Test>::UnauthorizedSender
		);
	});
}

#[test]
fn transact_holder_not_signed_doughnut_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let bob_private = hex!("79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf");
		let holder: ecdsa::Pair = Pair::from_string("//Bob", None).unwrap();
		let holder_address = DoughnutPallet::get_address(holder.public().0.into()).unwrap();
		let issuer: ecdsa::Pair = Pair::from_string("//Alice", None).unwrap();
		let mut doughnut_v1 = DoughnutV1 {
			holder: holder.public().as_slice().try_into().expect("should not fail"),
			issuer: issuer.public().as_slice().try_into().expect("should not fail"),
			domains: vec![(String::from(""), vec![])],
			expiry: 0,
			not_before: 0,
			payload_version: 0,
			signature_version: SignatureVersion::ECDSA as u8,
			signature: [0_u8; 64],
		};

		// Sign the doughnut with Bobs private key (The holder, not the issuer)
		assert_ok!(doughnut_v1.sign_ecdsa(&bob_private));
		let doughnut = Doughnut::V1(doughnut_v1);
		let doughnut_encoded = doughnut.encode();

		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::<Test>::remark { remark: b"Mischief Managed".to_vec() }.into();

		// Attempting to transact the doughnut as a random account should fail as it was not
		// signed by Alice
		assert_noop!(
			DoughnutPallet::transact(
				Some(holder_address).into(),
				Box::new(call),
				doughnut_encoded,
				0,
				vec![]
			),
			Error::<Test>::DoughnutVerifyFailed
		);
	});
}

#[test]
fn revoke_doughnut_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let alice_private =
			hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854");
		let issuer: ecdsa::Pair = Pair::from_string("//Alice", None).unwrap();
		let holder: ecdsa::Pair = Pair::from_string("//Bob", None).unwrap();
		let doughnut = make_doughnut(holder.public(), issuer.public(), &alice_private, "1", vec![]);

		let issuer_address = DoughnutPallet::get_address(issuer.public().0.into()).unwrap();
		let holder_address = DoughnutPallet::get_address(holder.public().0.into()).unwrap();

		let doughnut_encoded = doughnut.encode();

		assert_ok!(DoughnutPallet::revoke_doughnut(
			Some(issuer_address).into(),
			doughnut_encoded.clone(),
			true
		));

		// Check storage updated
		let doughnut_hash = keccak_256(&doughnut_encoded);
		assert_eq!(BlockedDoughnuts::<Test>::get(doughnut_hash), true);

		// Attempting to transact the doughnut should fail as the doughnut is revoked
		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::<Test>::remark { remark: b"Mischief Managed".to_vec() }.into();
		assert_noop!(
			DoughnutPallet::transact(
				Some(holder_address).into(),
				Box::new(call.clone()),
				doughnut_encoded.clone(),
				0,
				vec![]
			),
			Error::<Test>::DoughnutRevoked
		);

		// Remove revoke
		assert_ok!(DoughnutPallet::revoke_doughnut(
			Some(issuer_address).into(),
			doughnut_encoded.clone(),
			false
		));
		assert_eq!(BlockedDoughnuts::<Test>::get(doughnut_hash), false);

		// Attempting to transact the doughnut should now succeed
		assert_ok!(DoughnutPallet::transact(
			Some(holder_address).into(),
			Box::new(call),
			doughnut_encoded,
			0,
			vec![]
		));
	});
}

#[test]
fn revoke_doughnut_not_issuer_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let alice_private =
			hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854");
		let issuer: ecdsa::Pair = Pair::from_string("//Alice", None).unwrap();
		let holder: ecdsa::Pair = Pair::from_string("//Bob", None).unwrap();
		let doughnut = make_doughnut(holder.public(), issuer.public(), &alice_private, "1", vec![]);
		let holder_address = DoughnutPallet::get_address(holder.public().0.into()).unwrap();

		let doughnut_encoded = doughnut.encode();

		assert_noop!(
			DoughnutPallet::revoke_doughnut(
				Some(holder_address).into(),
				doughnut_encoded.clone(),
				true
			),
			Error::<Test>::CallerNotIssuer
		);
	});
}

#[test]
fn revoke_doughnut_invalid_doughnut_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		assert_noop!(
			DoughnutPallet::revoke_doughnut(Some(create_account(12)).into(), vec![], true),
			Error::<Test>::UnsupportedDoughnutVersion
		);
	});
}

#[test]
fn revoke_holder_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let alice_private =
			hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854");
		let issuer: ecdsa::Pair = Pair::from_string("//Alice", None).unwrap();
		let holder: ecdsa::Pair = Pair::from_string("//Bob", None).unwrap();
		let doughnut = make_doughnut(holder.public(), issuer.public(), &alice_private, "1", vec![]);

		let issuer_address = DoughnutPallet::get_address(issuer.public().0.into()).unwrap();
		let holder_address = DoughnutPallet::get_address(holder.public().0.into()).unwrap();

		let doughnut_encoded = doughnut.encode();

		assert_ok!(DoughnutPallet::revoke_holder(
			Some(issuer_address).into(),
			holder_address.clone(),
			true
		));

		// Check storage updated
		assert_eq!(BlockedHolders::<Test>::get(issuer_address, holder_address), true);

		// Attempting to transact the doughnut should fail as the holder is revoked
		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::<Test>::remark { remark: b"Mischief Managed".to_vec() }.into();
		assert_noop!(
			DoughnutPallet::transact(
				Some(holder_address).into(),
				Box::new(call.clone()),
				doughnut_encoded.clone(),
				0,
				vec![]
			),
			Error::<Test>::HolderRevoked
		);

		// Remove revoke
		assert_ok!(DoughnutPallet::revoke_holder(
			Some(issuer_address).into(),
			holder_address.clone(),
			false
		));
		assert_eq!(BlockedHolders::<Test>::get(issuer_address, holder_address), false);

		// Attempting to transact the doughnut should now succeed
		assert_ok!(DoughnutPallet::transact(
			Some(holder_address).into(),
			Box::new(call),
			doughnut_encoded,
			0,
			vec![]
		));
	});
}
