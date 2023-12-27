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
use crate::{mock::*, types::*};
use frame_support::{assert_noop, assert_ok, error::BadOrigin, traits::fungibles::Mutate};
use seed_pallet_common::test_prelude::*;
use seed_primitives::AccountId20;

mod self_contained_call {
	use super::*;

	#[test]
	fn submit_encoded_xrpl_transaction_validations() {
		TestExt::<Test>::default().build().execute_with(|| {
      // encoded call for: chainIid = 0, nonce = 0, max_block_number = 5, extrinsic = System::remark
			let call = mock::RuntimeCall::System(frame_system::Call::remark { remark: Default::default() });
      let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C25807321026577EEF1DDBC8B7B883BF19457A5FA4CCBD1EEAF29A51AD2D8370CB3E2DC9F2B81149308E2A8716F3F4BCBE49EFA6FA9DAF75AA31D0DF9EA7C0965787472696E7369637D30303A303A353A353030343030303134303464363937333633363836393635363632303464363136653631363736353634E1F1").unwrap();
      assert_ok!(Xrpl::submit_encoded_xrpl_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes.clone()), BoundedVec::default(), Box::new(call)));
    });
	}

	#[test]
	fn extrinsic_cannot_perform_privileged_operations() {
		TestExt::<Test>::default().build().execute_with(|| {
			let call = mock::RuntimeCall::System(frame_system::Call::set_code { code: Default::default() });
      // encoded call for: chainIid = 0, nonce = 0, max_block_number = 5, extrinsic = System::set_code
			let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732103C7F5304313F8C3CE00E36D0F09A8E08F7EABCD7144E3384FC4E66E75E5522F9D81148AB02D60F912ED0AD339A883C60DB9639311F329F9EA7C0965787472696E7369637D14303A303A353A3138303430303033303831323334E1F1").unwrap();

			// executing xrpl encoded transaction fails since caller is not root/sudo account
			assert_noop!(
				Xrpl::submit_encoded_xrpl_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes), BoundedVec::default(), Box::new(call)),
				BadOrigin,
			);
		});
	}

	// TODO: remove
	// #[test]
	// fn custom() {
	// 	let encoded_call: &[u8] = &[ 0, 1, 0 ];
	// 	println!("encoded_call: {:?}", encoded_call); // 00 01 00
	// 	println!("hex encoded_call: {:?}", hex::encode(&[ 0, 1, 0 ])); // 00 01 00
	// 	let hashed_call = sp_io::hashing::blake2_256(&encoded_call);
	// 	println!("hashed_call: {:?}", hashed_call);
	// 	let encoded_call = hex::encode(hashed_call);
	// 	println!("encoded_call: {:?}", encoded_call);
	// 	assert!(false);
	// }

	#[test]
	fn signed_extension_validations() {
		TestExt::<Test>::default()
			.with_asset(XRP_ASSET_ID, "XRP", &[(alice(), 0)]) // create XRP asset
			.build()
			.execute_with(|| {
				// encoded call for: chain_id = 1; nonce = 0, max_block_number = 5, extrinsic = System::remark; validates invalid chain id
				let call = mock::RuntimeCall::System(frame_system::Call::remark { remark: Default::default() });
				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C25807321033A0663EEAAD786F132CDBC25C7D2A6F8C14D55DB7B9AB52AFFB0D8A1C9A1010D81145B3DE9CEA3A77D69DD12F99C12E542907EE49E44F9EA7C0965787472696E7369637D30313A303A353A353030343030303134303464363937333633363836393635363632303464363136653631363736353634E1F1").unwrap()),
					signature: BoundedVec::truncate_from(hex::decode("3045022100B877466D021B990299F5177E33AF2B2D4B40D2D01CF0889C26247BAEF7995C6F02207EEAD77A28D990F94C6F425EC134EAAD456282FD74CB09005656CA208E8A1476").unwrap()),
					call: Box::new(call.clone()),
				}));
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);

				// encoded call for: chain_id = 0, nonce = 1, max_block_number = 5, extrinsic = System::remark; validates nonce too high
				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732103559940F18727930969416A900738B4525FE104C5812C5305365E7B30316BDAA68114DD5493DF89C562B62E277B496FB2DF1043302932F9EA7C0965787472696E7369637D30303A313A353A353030343030303134303464363937333633363836393635363632303464363136653631363736353634E1F1").unwrap()),
					signature: BoundedVec::truncate_from(hex::decode("304402200802EBDB16E1568788BD111BD39DA826D19386F0E3F26CB79D95A0ED4E08052102205784BB0EC6005F59423A886496083AA3626A66224B372FE3977598195E74C73D").unwrap()),
					call: Box::new(call.clone()),
				}));
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);

				// encoded call for: chain_id = 0, nonce = 0, max_block_number = 5, extrinsic = System::remark
				let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732103480D32221603422F1B1EB9B9446288DBE3ABDA4194E735457B28793E6B411690811438F9D2B0136BC20A6C3428EC77B0D4CCFDE9F01BF9EA7C0965787472696E7369637D30303A303A353A353030343030303134303464363937333633363836393635363632303464363136653631363736353634E1F1").unwrap();

				// validate self contained extrinsic is invalid (no signature)
				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(tx_bytes.clone()),
					signature: BoundedVec::default(),
					call: Box::new(call.clone()),
				}));
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.clone().into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);

				// validate self contained extrinsic is invalid (invalid signature)
				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(tx_bytes.clone()),
					signature: BoundedVec::truncate_from(hex::decode("304402205CD628B33CD2A89D735EBC139F21A3F2F138F7D687BBAF3E2CDFBBF8951919DC02204B65FC7FF3C2C1B1EEF10186CF6BDAA1C96E8F0814099EE5811C12F65E26A81E").unwrap()),
					call: Box::new(call.clone()),
				}));
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.clone().into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);

				// validate self contained extrinsic fails, user does not have funds to pay for transaction
				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(tx_bytes.clone()),
					signature: BoundedVec::truncate_from(hex::decode("3045022100BD734A38F9C5C210CC7E1D57AEA6DA45039D0068E3ABBA348189A5EBC6A0757D022077B4212F023C66B6C99FB68DC7AEF7921A1BAFF2A85AC6C5E70000C50009231C").unwrap()),
					call: Box::new(call.clone()),
				}));
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.clone().into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);

				// validate self contained extrinsic fails, call provided is not signed hashed extrinsic in memo data
				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C258073210355C6CA1FB82188356AE57E3874EFB7EC5EDC361251876DE065427F063AAD4438811481717D1D4D3B764AA57641C9726345BB2B9F43F7F9EA7C0965787472696E7369637D48303A303A3334353A38303830373738633330633230666132656263306564313864326362636131663330623032373632356337643964393766356435383937323163393161656236E1F1").unwrap()),
					signature: BoundedVec::truncate_from(hex::decode("304402203E09C8C178F4523863DAD10B7A0908699CFE36C6349A2C3931B7EEBA04844BEF0220506F5CC85C4B254DAB2E73BE94236809C0D974B47A465AD8A087975F6EF27628").unwrap()),
					call: Box::new(call.clone()),
				}));
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);

				// validate account nonce is not incremented from any of the failure scanerios above
				let tx = XRPLTransaction::try_from(tx_bytes.as_bytes_ref()).unwrap();
				let caller: AccountId20 = tx.get_account().unwrap().into();
				assert_eq!(System::account_nonce(&caller), 0);
    	});
	}

	#[test]
	fn system_remark_extrinsic_from_message_success() {
		TestExt::<Test>::default()
			.with_asset(XRP_ASSET_ID, "XRP", &[(alice(), 0)]) // create XRP asset
			.build()
			.execute_with(|| {
				let call = mock::RuntimeCall::System(frame_system::Call::remark { remark: b"Mischief Managed".to_vec() });

      	// encoded call for: chainIid = 0, nonce = 0, max_block_number = 5, extrinsic = System::remark
				let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C25807321029259980381C9BD1E3C174436F99C179504ED18A34A81FE39A5458E9D836285258114EE0B375F1B10624DDDCF6F200B531C8674324D15F9EA7C0965787472696E7369637D46303A303A353A33623832663037383031653632636437383966316233636333353936383236313436613163353136666165613766633633333263643362323563646666316331E1F1").unwrap();
				let signature = hex::decode("304402202E02877C195085F54FA1D8EA2440FFDD15F871AE0C2386DD5F486C3B86C4CA2C02207D1071BFF1A51178E9262C07B72FB74CE8B35DBCFE8E556EEC28B20D7C6AF24E").unwrap();

				// fund the user with XRP (to pay for tx fees)
				let tx = XRPLTransaction::try_from(tx_bytes.as_bytes_ref()).unwrap();
				let caller: AccountId20 = tx.get_account().unwrap().into();
				assert_ok!(AssetsExt::mint_into(2, &caller, 2_000_000));

				let balance_before = Assets::balance(XRP_ASSET_ID, &caller);

				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(tx_bytes.clone()),
					signature: BoundedVec::truncate_from(signature.clone()),
					call: Box::new(call.clone()),
				}));

				// validate self contained extrinsic fails if block_number is exceeded
				System::set_block_number(10);
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.clone().into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);

				// reset block number, extrinsic validation should pass now
				System::set_block_number(1);
				assert_ok!(Executive::validate_transaction(TransactionSource::External, xt.clone().into(), H256::default()));

				// execute the extrinsic with the provided signed extras
				assert_ok!(Executive::apply_extrinsic(xt.clone()));

				// verify the event was emitted for successful extrinsic with nested system remark call
				System::assert_has_event(
					Event::XRPLExtrinsicExecuted {
						public_key: [2,146,89,152,3,129,201,189,30,60,23,68,54,249,156,23,149,4,237,24,163,74,129,254,57,165,69,142,157,131,98,133,37],
						caller,
						call: mock::RuntimeCall::System(frame_system::Call::remark { remark: b"Mischief Managed".to_vec() }),
					}.into(),
				);

				// verify extrinsic success event
				System::assert_last_event(mock::RuntimeEvent::System(
					frame_system::Event::ExtrinsicSuccess {
						dispatch_info: DispatchInfo {
							weight: Weight::from_ref_time(311_960_000),
							class: DispatchClass::Normal,
							pays_fee: Pays::Yes,
						},
					},
				));

				// validate account nonce is incremented
				assert_eq!(System::account_nonce(&caller), 1);

				// validate account balance is decremented
				assert!(Assets::balance(XRP_ASSET_ID, &caller) < balance_before);

				// validate the same extrinsic will fail (nonce mismatch) - preventing replays
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.clone().into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);
  		});
	}
}
