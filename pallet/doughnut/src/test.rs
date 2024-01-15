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
	BlockedDoughnuts, BlockedHolders, Call, Error,
};
use codec::Encode;
use doughnut_rs::{
	doughnut::{Doughnut, DoughnutV1},
	signature::{sign_ecdsa, verify_signature, SignatureVersion},
	traits::{DoughnutVerify, FeeMode, PayloadVersion, Signing},
};
use frame_support::traits::fungibles::Mutate;
use hex_literal::hex;
use seed_pallet_common::test_prelude::*;
use sp_core::{bytes::to_hex, ecdsa, ecdsa::Public, keccak_256, ByteArray, Pair};

// Helper struct for a test account which provides common methods over that account
struct TestAccount {
	pub seed: &'static str,
}

impl TestAccount {
	// Create a new test account
	pub fn new(seed: &'static str) -> Self {
		Self { seed }
	}

	// Return the ECDSA pair for this account
	pub fn pair(&self) -> ecdsa::Pair {
		Pair::from_string(self.seed, None).unwrap()
	}

	// Return the public key for this account
	pub fn public(&self) -> Public {
		self.pair().public()
	}

	// Return the private key for this account
	pub fn private(&self) -> [u8; 32] {
		self.pair().seed().into()
	}

	// Return the AccountId type for this account
	pub fn address(&self) -> AccountId {
		DoughnutPallet::get_address(self.public().0.into()).unwrap()
	}

	// Sign a message using ECDSA
	pub fn sign_ecdsa(&self, message: &[u8]) -> [u8; 64] {
		sign_ecdsa(&self.private(), message).unwrap()
	}
}

// BOB TestAccount
const BOB: TestAccount = TestAccount { seed: "//Bob" };

// ALICE TestAccount
const ALICE: TestAccount = TestAccount { seed: "//Alice" };

fn make_doughnut(
	holder: &TestAccount,
	issuer: &TestAccount,
	fee_mode: FeeMode,
	domain: &str,
	domain_payload: Vec<u8>,
) -> Doughnut {
	let mut doughnut_v1 = DoughnutV1 {
		holder: holder.public().as_slice().try_into().expect("should not fail"),
		issuer: issuer.public().as_slice().try_into().expect("should not fail"),
		fee_mode: fee_mode as u8,
		domains: vec![(domain.to_string(), domain_payload)],
		expiry: 0,
		not_before: 0,
		payload_version: PayloadVersion::V1 as u16,
		signature_version: SignatureVersion::ECDSA as u8,
		signature: [0_u8; 64],
	};
	// Sign and verify doughnut
	assert_ok!(doughnut_v1.sign_ecdsa(&issuer.private()));
	assert_ok!(doughnut_v1.verify());
	Doughnut::V1(doughnut_v1)
}

#[test]
fn make_doughnut_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		make_doughnut(&ALICE, &BOB, FeeMode::ISSUER, "", vec![]);
	});
}

#[test]
fn bob_to_alice_doughnut() {
	TestExt::<Test>::default().build().execute_with(|| {
		let issuer: TestAccount = BOB;
		let holder: TestAccount = ALICE;
		let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "", vec![]);

		println!("issuer address (Bob): {:?}", to_hex(issuer.address().0.as_slice(), false));
		println!("holder address (Alice): {:?}", to_hex(holder.address().0.as_slice(), false));

		let doughnut_encoded = doughnut.encode();
		println!("Encoded doughnut");
		println!("{:?}", to_hex(doughnut_encoded.clone().as_slice(), false));

		// Print Alice's signature over the doughnut
		let alice_signature = holder.sign_ecdsa(&doughnut_encoded.as_slice());
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
		let issuer = ALICE;
		let holder = BOB;
		let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "1", vec![]);

		println!("doughnut: {:?}", doughnut);
		println!("issuer address (Alice): {:?}", to_hex(issuer.address().0.as_slice(), false));
		println!("holder address (Bob): {:?}", to_hex(holder.address().0.as_slice(), false));

		let doughnut_encoded = doughnut.encode();
		println!("Encoded doughnut");
		println!("{:?}", to_hex(doughnut_encoded.clone().as_slice(), false));

		// Print Bob's signature over the doughnut
		let bob_signature = sign_ecdsa(&holder.private(), &doughnut_encoded.as_slice()).unwrap();
		println!("Holder signature: {:?}", to_hex(holder.private().as_slice(), false));

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
	let issuer = ALICE;
	let initial_balance = 10_000;
	TestExt::<Test>::default()
		.with_balances(&[(issuer.address(), initial_balance)])
		.build()
		.execute_with(|| {
			let holder = BOB;
			let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "1", vec![]);
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
				Some(holder.address()).into(),
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
			assert_eq!(
				Balances::free_balance(&issuer.address()),
				initial_balance - transfer_amount
			);
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
		let issuer = ALICE;
		let holder = BOB;
		let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "1", vec![]);
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
		let issuer = ALICE;
		let holder = BOB;
		let mut doughnut_v1 = DoughnutV1 {
			holder: holder.public().as_slice().try_into().expect("should not fail"),
			issuer: issuer.public().as_slice().try_into().expect("should not fail"),
			fee_mode: 0,
			domains: vec![(String::from(""), vec![])],
			expiry: 0,
			not_before: 0,
			payload_version: 0,
			signature_version: SignatureVersion::ECDSA as u8,
			signature: [0_u8; 64],
		};

		// Sign the doughnut with Bobs private key (The holder, not the issuer)
		assert_ok!(doughnut_v1.sign_ecdsa(&holder.private()));
		let doughnut = Doughnut::V1(doughnut_v1);
		let doughnut_encoded = doughnut.encode();

		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::<Test>::remark { remark: b"Mischief Managed".to_vec() }.into();

		// Attempting to transact the doughnut as a random account should fail as it was not
		// signed by Alice
		assert_noop!(
			DoughnutPallet::transact(
				Some(holder.address()).into(),
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
		let issuer = ALICE;
		let holder = BOB;
		let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "1", vec![]);
		let doughnut_encoded = doughnut.encode();

		assert_ok!(DoughnutPallet::revoke_doughnut(
			Some(issuer.address()).into(),
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
				Some(holder.address()).into(),
				Box::new(call.clone()),
				doughnut_encoded.clone(),
				0,
				vec![]
			),
			Error::<Test>::DoughnutRevoked
		);

		// Remove revoke
		assert_ok!(DoughnutPallet::revoke_doughnut(
			Some(issuer.address()).into(),
			doughnut_encoded.clone(),
			false
		));
		assert_eq!(BlockedDoughnuts::<Test>::get(doughnut_hash), false);

		// Attempting to transact the doughnut should now succeed
		assert_ok!(DoughnutPallet::transact(
			Some(holder.address()).into(),
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
		let issuer = ALICE;
		let holder = BOB;
		let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "1", vec![]);
		let doughnut_encoded = doughnut.encode();

		assert_noop!(
			DoughnutPallet::revoke_doughnut(
				Some(holder.address()).into(),
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
		let issuer = ALICE;
		let holder = BOB;
		let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "1", vec![]);
		let doughnut_encoded = doughnut.encode();

		assert_ok!(DoughnutPallet::revoke_holder(
			Some(issuer.address()).into(),
			holder.address().clone(),
			true
		));

		// Check storage updated
		assert_eq!(BlockedHolders::<Test>::get(issuer.address(), holder.address()), true);

		// Attempting to transact the doughnut should fail as the holder is revoked
		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::<Test>::remark { remark: b"Mischief Managed".to_vec() }.into();
		assert_noop!(
			DoughnutPallet::transact(
				Some(holder.address()).into(),
				Box::new(call.clone()),
				doughnut_encoded.clone(),
				0,
				vec![]
			),
			Error::<Test>::HolderRevoked
		);

		// Remove revoke
		assert_ok!(DoughnutPallet::revoke_holder(
			Some(issuer.address()).into(),
			holder.address().clone(),
			false
		));
		assert_eq!(BlockedHolders::<Test>::get(issuer.address(), holder.address()), false);

		// Attempting to transact the doughnut should now succeed
		assert_ok!(DoughnutPallet::transact(
			Some(holder.address()).into(),
			Box::new(call),
			doughnut_encoded,
			0,
			vec![]
		));
	});
}

#[test]
fn generate_alice_to_bob_outer_signature() {
	let issuer = ALICE;
	let initial_balance = 10_000;
	TestExt::<Test>::default()
		.with_balances(&[(issuer.address(), initial_balance)])
		.build()
		.execute_with(|| {
			let holder = BOB;
			let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "1", vec![]);
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
			let outer_call = Call::<Test>::transact {
				call: Box::new(call),
				doughnut: doughnut_encoded.clone(),
				nonce: 0,
				signature: vec![],
			};

			let mut outer_call_payload: Vec<u8> = outer_call.encode();
			outer_call_payload.as_mut_slice()[1] = 0x05; // TODO - for some reason, actual runtime encoded
											 // call has this byte as 0x05. check this.
			let outer_signature = holder.sign_ecdsa(&outer_call_payload.as_slice());
			println!("doughnut: {:?}", to_hex(doughnut_encoded.as_slice(), false));
			println!("outer call: {:?}", outer_call);
			println!("outer call payload: {:?}", to_hex(outer_call_payload.as_slice(), false));
			println!("outer call signature: {:?}", to_hex(&outer_signature, false));
		});
}

#[test]
fn signed_extension_validations_succeed() {
	TestExt::<Test>::default()
		.with_asset(XRP_ASSET_ID, "XRP", &[]) // create XRP asset
		.build()
		.execute_with(|| {
			let issuer = ALICE;
			let holder = BOB;
			let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "1", vec![]);
			let doughnut_encoded = doughnut.encode();

			// Fund the issuer so they can pass the validations for paying gas
			assert_ok!(AssetsExt::mint_into(XRP_ASSET_ID, &issuer.address(), 5000000));
			let call = mock::RuntimeCall::System(frame_system::Call::remark_with_event {
				remark: b"Mischief Managed".to_vec(),
			});
			let doughnut_encoded = doughnut.encode();
			let nonce = 0;

			// Print Bob's signature over the doughnut
			let outer_call = Call::<Test>::transact {
				call: Box::new(call.clone()),
				doughnut: doughnut_encoded.clone(),
				nonce,
				signature: vec![],
			};
			let outer_signature = holder.sign_ecdsa(&outer_call.encode().as_slice());

			// validate self contained extrinsic is invalid (invalid signature)
			let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(
				mock::RuntimeCall::Doughnut(crate::Call::transact {
					call: Box::new(call.clone()),
					doughnut: doughnut_encoded,
					nonce,
					signature: outer_signature.into(),
				}),
			);

			// Validate transaction should succeed
			assert_ok!(Executive::validate_transaction(
				TransactionSource::External,
				xt.clone().into(),
				H256::default()
			),);

			// execute the extrinsic with the provided signed extras
			assert_ok!(Executive::apply_extrinsic(xt.clone()));

			// validate account nonce is incremented
			assert_eq!(System::account_nonce(&holder.address()), 1);

			// Check event is thrown as the doughnut was successfully executed
			System::assert_has_event(
				Event::DoughnutCallExecuted { result: DispatchResult::Ok(()) }.into(),
			);
		});
}

#[test]
fn signed_extension_validations_low_balance_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let issuer = ALICE;
		let holder = BOB;
		let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "1", vec![]);
		let doughnut_encoded = doughnut.encode();

		let call = mock::RuntimeCall::System(frame_system::Call::remark_with_event {
			remark: b"Mischief Managed".to_vec(),
		});
		let doughnut_encoded = doughnut.encode();
		let nonce = 0;

		// Print Bob's signature over the doughnut
		let outer_call = Call::<Test>::transact {
			call: Box::new(call.clone()),
			doughnut: doughnut_encoded.clone(),
			nonce,
			signature: vec![],
		};
		let outer_signature = holder.sign_ecdsa(&outer_call.encode().as_slice());

		// validate self contained extrinsic is invalid (invalid signature)
		let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(
			mock::RuntimeCall::Doughnut(crate::Call::transact {
				call: Box::new(call.clone()),
				doughnut: doughnut_encoded,
				nonce,
				signature: outer_signature.into(),
			}),
		);

		// Validate transaction should fail as the holder does not have enough XRP to cover
		// the fee payment
		assert_err!(
			Executive::validate_transaction(
				TransactionSource::External,
				xt.clone().into(),
				H256::default()
			),
			TransactionValidityError::Invalid(InvalidTransaction::BadProof)
		);
	});
}

#[test]
fn apply_extrinsic_invalid_nonce_fails() {
	TestExt::<Test>::default()
		.with_asset(XRP_ASSET_ID, "XRP", &[]) // create XRP asset
		.build()
		.execute_with(|| {
			let issuer = ALICE;
			let holder = BOB;
			let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "1", vec![]);
			let doughnut_encoded = doughnut.encode();

			// Fund the issuer so they can pass the validations for paying gas
			assert_ok!(AssetsExt::mint_into(XRP_ASSET_ID, &issuer.address(), 5000000));
			let call = mock::RuntimeCall::System(frame_system::Call::remark_with_event {
				remark: b"Mischief Managed".to_vec(),
			});
			let doughnut_encoded = doughnut.encode();
			let nonce = 1;

			// Print Bob's signature over the doughnut
			let outer_call = Call::<Test>::transact {
				call: Box::new(call.clone()),
				doughnut: doughnut_encoded.clone(),
				nonce,
				signature: vec![],
			};
			let outer_signature = holder.sign_ecdsa(&outer_call.encode().as_slice());

			// validate self contained extrinsic is invalid (invalid signature)
			let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(
				mock::RuntimeCall::Doughnut(crate::Call::transact {
					call: Box::new(call.clone()),
					doughnut: doughnut_encoded,
					nonce,
					signature: outer_signature.into(),
				}),
			);

			// Validate transaction should succeed
			assert_ok!(Executive::validate_transaction(
				TransactionSource::External,
				xt.clone().into(),
				H256::default()
			),);
			// Validate transaction should fail as the nonce is too high
			assert_err!(
				Executive::apply_extrinsic(xt.clone()),
				TransactionValidityError::Invalid(InvalidTransaction::BadProof)
			);
		});
}

#[test]
fn signed_extension_validations_invalid_signature_fails() {
	TestExt::<Test>::default()
		.with_asset(XRP_ASSET_ID, "XRP", &[]) // create XRP asset
		.build()
		.execute_with(|| {
			let issuer = ALICE;
			let holder = BOB;
			let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "1", vec![]);
			let doughnut_encoded = doughnut.encode();

			// Fund the issuer so they can pass the validations for paying gas
			assert_ok!(AssetsExt::mint_into(XRP_ASSET_ID, &issuer.address(), 5000000));
			let call = mock::RuntimeCall::System(frame_system::Call::remark_with_event {
				remark: b"Mischief Managed".to_vec(),
			});
			let doughnut_encoded = doughnut.encode();
			let nonce = 0;

			// Sign the signature with just the doughnut which is invalid
			let outer_signature = holder.sign_ecdsa(&doughnut_encoded.as_slice());

			// validate self contained extrinsic is invalid (invalid signature)
			let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(
				mock::RuntimeCall::Doughnut(crate::Call::transact {
					call: Box::new(call.clone()),
					doughnut: doughnut_encoded,
					nonce,
					signature: outer_signature.into(),
				}),
			);

			// Validate transaction should fail as the outer signature is incorrect
			assert_err!(
				Executive::validate_transaction(
					TransactionSource::External,
					xt.clone().into(),
					H256::default()
				),
				TransactionValidityError::Invalid(InvalidTransaction::BadProof)
			);
		});
}
