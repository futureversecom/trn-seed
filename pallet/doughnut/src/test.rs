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
	doughnut::{Doughnut, DoughnutV0, DoughnutV1},
	signature::{sign_ecdsa, verify_signature, SignatureVersion},
	traits::{DoughnutVerify, FeeMode, PayloadVersion, Signing},
};
use frame_support::traits::fungibles::Mutate;
use seed_pallet_common::test_prelude::*;
use sp_core::{bytes::to_hex, ecdsa, ecdsa::Public, keccak_256, ByteArray, Pair};
use sp_std::default::Default;
use trnnut_rs::TRNNutV0;

// Helper struct for a test account where a seed is supplied and provides common methods to
// receive parts of that account
pub struct TestAccount {
	pub seed: &'static str,
}

impl TestAccount {
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
pub const BOB: TestAccount = TestAccount { seed: "//Bob" };

// ALICE TestAccount
pub const ALICE: TestAccount = TestAccount { seed: "//Alice" };

pub fn make_doughnut(
	holder: &TestAccount,
	issuer: &TestAccount,
	fee_mode: FeeMode,
	domain: &str,
	domain_payload: Vec<u8>,
	signature_version: SignatureVersion,
) -> Doughnut {
	let mut doughnut_v1 = DoughnutV1 {
		holder: holder.public().as_slice().try_into().expect("should not fail"),
		issuer: issuer.public().as_slice().try_into().expect("should not fail"),
		fee_mode: fee_mode as u8,
		domains: vec![(domain.to_string(), domain_payload)],
		expiry: 0,
		not_before: 0,
		payload_version: PayloadVersion::V1 as u16,
		signature_version: signature_version as u8,
		signature: [0_u8; 65],
	};
	// Sign and verify doughnut
	match signature_version {
		SignatureVersion::ECDSA => {
			assert_ok!(doughnut_v1.sign_ecdsa(&issuer.private()));
		},
		SignatureVersion::EIP191 => {
			assert_ok!(doughnut_v1.sign_eip191(&issuer.private()));
		},
		_ => panic!("unsupported signature version"),
	}

	assert_ok!(doughnut_v1.verify());
	Doughnut::V1(doughnut_v1)
}

fn make_trnnut(module: &str, method: &str) -> TRNNut {
	let method_obj = trnnut_rs::v0::method::Method {
		name: method.to_string(),
		block_cooldown: None,
		constraints: None,
	};
	let module_obj = trnnut_rs::v0::module::Module {
		name: module.to_string(),
		block_cooldown: None,
		methods: vec![(method.to_string(), method_obj)],
	};
	TRNNut::V0(TRNNutV0 {
		modules: vec![(module.to_string(), module_obj)],
		contracts: Default::default(),
	})
}

#[test]
fn make_doughnut_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		make_doughnut(&ALICE, &BOB, FeeMode::ISSUER, "", vec![]);
	});
}

#[test]
fn get_address_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let account = ALICE;
		assert_ok!(DoughnutPallet::get_address(account.public().0.into()));
	});
}

#[test]
fn get_address_invalid_public_key_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		// invalid public key should fail
		let pub_key: [u8; 33] = [0_u8; 33];
		assert_noop!(DoughnutPallet::get_address(pub_key), Error::<Test>::UnauthorizedSender);
	});
}

#[test]
fn run_doughnut_common_validations_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let issuer = ALICE;
		let holder = BOB;
		let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "", vec![]);
		let doughnut_encoded = doughnut.encode();

		// Running common validations should work
		assert_ok!(DoughnutPallet::run_doughnut_common_validations(doughnut_encoded));
	});
}

#[test]
fn run_doughnut_common_validations_bad_doughnut_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let issuer = ALICE;
		let holder = BOB;
		let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "", vec![]);
		let mut doughnut_encoded = doughnut.encode();
		// Corrupt the doughnut by removing the last byte
		doughnut_encoded = doughnut_encoded[0..doughnut_encoded.len() - 1].to_vec();

		// Running common validations should fail as the doughnut is corrupt
		assert_noop!(
			DoughnutPallet::run_doughnut_common_validations(doughnut_encoded),
			Error::<Test>::DoughnutDecodeFailed
		);
	});
}

#[test]
fn run_doughnut_common_validations_invalid_doughnut_version_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let issuer = ALICE;
		let holder = BOB;
		let doughnut_v0 = DoughnutV0 {
			holder: holder.public().as_slice()[0..32].try_into().expect("should not fail"),
			issuer: issuer.public().as_slice()[0..32].try_into().expect("should not fail"),
			domains: vec![(String::default(), vec![])],
			expiry: 0,
			not_before: 0,
			payload_version: PayloadVersion::V0 as u16,
			signature_version: SignatureVersion::ECDSA as u8,
			signature: Default::default(),
		};
		let doughnut = Doughnut::V0(doughnut_v0);
		let doughnut_encoded = doughnut.encode();

		// Running common validations should fail as the doughnut is V0
		assert_noop!(
			DoughnutPallet::run_doughnut_common_validations(doughnut_encoded),
			Error::<Test>::UnsupportedDoughnutVersion
		);
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
fn alice_to_bob_doughnut_for_balance_trnasfer() {
	TestExt::<Test>::default().build().execute_with(|| {
		let issuer = ALICE;
		let holder = BOB;
		let trnnut = make_trnnut("Balances", "transfer");
		let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "trn", trnnut.encode());

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
			let trnnut = make_trnnut("Balances", "transfer");
			let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "trn", trnnut.encode());
			let doughnut_encoded = doughnut.encode();

			// add BOB to whitelisted holders
			WhitelistedHolders::<Test>::insert(BOB.address(), true);

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
				RawOrigin::None.into(),
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
				RawOrigin::None.into(),
				Box::new(call),
				vec![], // Invalid doughnut
				0,
				vec![]
			),
			Error::<Test>::DoughnutDecodeFailed
		);
	});
}

// TODO: enable this test
// #[test]
// fn transact_holder_not_sender_fails() {
// 	TestExt::<Test>::default().build().execute_with(|| {
// 		let issuer = ALICE;
// 		let holder = BOB;
// 		let trnnut = make_trnnut("System", "remark");
// 		let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "trn", trnnut.encode());
// 		let doughnut_encoded = doughnut.encode();
//
// 		let call: <Test as frame_system::Config>::RuntimeCall =
// 			frame_system::Call::<Test>::remark { remark: b"Mischief Managed".to_vec() }.into();
//
// 		// Attempting to transact the doughnut as a random account should fail
// 		assert_noop!(
// 			DoughnutPallet::transact(
// 				RawOrigin::None.into(),
// 				Box::new(call),
// 				doughnut_encoded,
// 				0,
// 				vec![]
// 			),
// 			Error::<Test>::UnauthorizedSender
// 		);
// 	});
// }

#[test]
fn transact_holder_not_signed_doughnut_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let issuer = ALICE;
		let holder = BOB;
		let trnnut = make_trnnut("System", "remark");
		let mut doughnut_v1 = DoughnutV1 {
			holder: holder.public().as_slice().try_into().expect("should not fail"),
			issuer: issuer.public().as_slice().try_into().expect("should not fail"),
			fee_mode: 0,
			domains: vec![(String::from("trn"), trnnut.encode())],
			expiry: 0,
			not_before: 0,
			payload_version: PayloadVersion::V1 as u16,
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
				RawOrigin::None.into(),
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
		let trnnut = make_trnnut("System", "remark");
		let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "trn", trnnut.encode());
		let doughnut_encoded = doughnut.encode();

		// add BOB to whitelisted holders
		WhitelistedHolders::<Test>::insert(BOB.address(), true);

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
				RawOrigin::None.into(),
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
			RawOrigin::None.into(),
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
		let trnnut = make_trnnut("System", "remark");
		let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "trn", trnnut.encode());
		let doughnut_encoded = doughnut.encode();

		// add BOB to whitelisted holders
		WhitelistedHolders::<Test>::insert(BOB.address(), true);

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
				RawOrigin::None.into(),
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
			RawOrigin::None.into(),
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
fn generate_alice_to_bob_outer_signature_for_balances_transfer() {
	let issuer = ALICE;
	let initial_balance = 10_000;
	TestExt::<Test>::default()
		.with_balances(&[(issuer.address(), initial_balance)])
		.build()
		.execute_with(|| {
			let holder = BOB;
			let trnnut = make_trnnut("Balances", "transfer");
			let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "trn", trnnut.encode());
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
fn generate_alice_to_bob_outer_signature_for_balances_transfer_keep_alive() {
	let issuer = ALICE;
	let initial_balance = 10_000;
	TestExt::<Test>::default()
		.with_balances(&[(issuer.address(), initial_balance)])
		.build()
		.execute_with(|| {
			let holder = BOB;
			let trnnut = make_trnnut("Balances", "transfer");
			let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "trn", trnnut.encode());
			let doughnut_encoded = doughnut.encode();

			// Create balances transfer call
			let transfer_amount = 1234;
			let destination = create_account(12);
			let call: <Test as frame_system::Config>::RuntimeCall =
				pallet_balances::Call::<Test>::transfer_keep_alive {
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
			let trnnut = make_trnnut("System", "remark_with_event");
			let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "trn", trnnut.encode());
			let doughnut_encoded = doughnut.encode();

			// add BOB to whitelisted holders
			WhitelistedHolders::<Test>::insert(BOB.address(), true);

			// Fund the issuer so they can pass the validations for paying gas
			assert_ok!(AssetsExt::mint_into(XRP_ASSET_ID, &issuer.address(), 5000000));
			let call = mock::RuntimeCall::System(frame_system::Call::remark_with_event {
				remark: b"Mischief Managed".to_vec(),
			});
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
fn signed_extension_validations_invalid_inner_signature_fails() {
	TestExt::<Test>::default()
		.with_asset(XRP_ASSET_ID, "XRP", &[]) // create XRP asset
		.build()
		.execute_with(|| {
			let issuer = ALICE;
			let holder = BOB;
			let doughnut_v1 = DoughnutV1 {
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

			// don't sign doughnut and check that verify fails
			assert_err!(doughnut_v1.verify(), doughnut_rs::error::VerifyError::Invalid);
			let doughnut = Doughnut::V1(doughnut_v1);
			let doughnut_encoded = doughnut.encode();

			// Fund the issuer so they can pass the validations for paying gas
			assert_ok!(AssetsExt::mint_into(XRP_ASSET_ID, &issuer.address(), 5000000));
			let call = mock::RuntimeCall::System(frame_system::Call::remark_with_event {
				remark: b"Mischief Managed".to_vec(),
			});
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

			// Validate transaction should fail as the inner signature is incorrect
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
fn signed_extension_validations_invalid_outer_signature_fails() {
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

#[test]
fn update_whitelisted_holders_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let whitelisted_holder = ALICE;

		assert_ok!(DoughnutPallet::update_whitelisted_holders(
			RawOrigin::Root.into(),
			whitelisted_holder.address(),
			true
		));

		// Check storage updated
		assert_eq!(WhitelistedHolders::<Test>::get(whitelisted_holder.address()), true);
		// Check event is thrown
		System::assert_has_event(
			Event::WhitelistedHoldersUpdated {
				holder: whitelisted_holder.address(),
				enabled: true,
			}
			.into(),
		);

		// only root can update the whitelisted holders list. try to remove alice from the list
		assert_noop!(
			DoughnutPallet::update_whitelisted_holders(
				Some(BOB.address()).into(),
				whitelisted_holder.address(),
				false
			),
			DispatchError::BadOrigin
		);
		assert_eq!(WhitelistedHolders::<Test>::get(whitelisted_holder.address()), true);

		// remove alice from the list by root
		assert_ok!(DoughnutPallet::update_whitelisted_holders(
			RawOrigin::Root.into(),
			whitelisted_holder.address(),
			false
		));

		assert_eq!(WhitelistedHolders::<Test>::get(whitelisted_holder.address()), false);
		// Check event is thrown
		System::assert_has_event(
			Event::WhitelistedHoldersUpdated {
				holder: whitelisted_holder.address(),
				enabled: false,
			}
			.into(),
		);
	});
}

#[test]
fn holder_whitelisting_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let issuer = ALICE;
		let holder = BOB;
		let trnnut = make_trnnut("System", "remark");
		let doughnut = make_doughnut(&holder, &issuer, FeeMode::ISSUER, "trn", trnnut.encode());
		let doughnut_encoded = doughnut.encode();

		// Attempting to transact the doughnut should fail as the holder is not whitelisted
		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::<Test>::remark { remark: b"Mischief Managed".to_vec() }.into();
		assert_noop!(
			DoughnutPallet::transact(
				RawOrigin::None.into(),
				Box::new(call.clone()),
				doughnut_encoded.clone(),
				0,
				vec![]
			),
			Error::<Test>::HolderNotWhitelisted
		);

		// Add BOB to whitelisted holders list
		assert_ok!(DoughnutPallet::update_whitelisted_holders(
			RawOrigin::Root.into(),
			BOB.address(),
			true
		));
		assert_eq!(WhitelistedHolders::<Test>::get(BOB.address()), true);

		// Attempting to transact the doughnut should now succeed
		assert_ok!(DoughnutPallet::transact(
			RawOrigin::None.into(),
			Box::new(call),
			doughnut_encoded,
			0,
			vec![]
		));
	});
}
