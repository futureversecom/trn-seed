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
use codec::Encode;
use frame_support::{assert_noop, assert_ok, error::BadOrigin, traits::fungibles::Mutate};
use seed_pallet_common::test_prelude::*;
use seed_primitives::AccountId20;

mod get_runtime_call_from_xrpl_extrinsic {
	use super::*;

	#[test]
	fn test_xrpl_get_runtime_call_system_remark() {
		TestExt::<Test>::default().build().execute_with(|| {
			let system_remark_call = mock::RuntimeCall::System(frame_system::Call::remark {
				remark: b"Mischief Managed".to_vec(),
			});
			let scale_encoded_call = system_remark_call.encode();
			let hex_encoded_call = hex::encode(&scale_encoded_call);
			assert_eq!("0001404d69736368696566204d616e61676564", hex_encoded_call);

			let unsigned_extrinsic =
				UncheckedExtrinsic::<Test>::new_unsigned(system_remark_call.clone().into());
			let scale_encoded_extrinsic = unsigned_extrinsic.encode();
			let hex_encoded_extrinsic = hex::encode(&scale_encoded_extrinsic);

			assert_eq!("50040001404d69736368696566204d616e61676564", hex_encoded_extrinsic);

			let decoded_call =
				Xrpl::get_runtime_call_from_xrpl_extrinsic(&scale_encoded_extrinsic).unwrap();
			assert_eq!(decoded_call, system_remark_call);
		});
	}

	#[test]
	fn test_xrpl_get_runtime_call_balance_transfer() {
		TestExt::<Test>::default().build().execute_with(|| {
			let balance_transfer_call =
				mock::RuntimeCall::Balances(pallet_balances::Call::transfer {
					dest: Default::default(),
					value: 100,
				});
			let scale_encoded_call = balance_transfer_call.encode();
			let hex_encoded_call = hex::encode(scale_encoded_call);
			assert_eq!("010000000000000000000000000000000000000000009101", hex_encoded_call);

			let unsigned_extrinsic =
				UncheckedExtrinsic::<Test>::new_unsigned(balance_transfer_call.clone().into());
			let scale_encoded_extrinsic = unsigned_extrinsic.encode();
			let hex_encoded_extrinsic = hex::encode(&scale_encoded_extrinsic);

			assert_eq!(
				"6404010000000000000000000000000000000000000000009101",
				hex_encoded_extrinsic
			);

			let decoded_call =
				Xrpl::get_runtime_call_from_xrpl_extrinsic(&scale_encoded_extrinsic).unwrap();
			assert_eq!(decoded_call, balance_transfer_call);
		});
	}

	#[test]
	fn test_xrpl_get_runtime_call_set_block_number() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sudo_call =
				mock::RuntimeCall::System(frame_system::Call::set_code { code: vec![] });
			let scale_encoded_call = sudo_call.encode();
			let hex_encoded_call = hex::encode(scale_encoded_call);
			assert_eq!("000300", hex_encoded_call);

			let unsigned_extrinsic =
				UncheckedExtrinsic::<Test>::new_unsigned(sudo_call.clone().into());
			let scale_encoded_extrinsic = unsigned_extrinsic.encode();
			let hex_encoded_extrinsic = hex::encode(&scale_encoded_extrinsic);

			assert_eq!("1004000300", hex_encoded_extrinsic);

			let decoded_call =
				Xrpl::get_runtime_call_from_xrpl_extrinsic(&scale_encoded_extrinsic).unwrap();
			assert_eq!(decoded_call, sudo_call);
		});
	}
}

mod self_contained_call {
	use super::*;

	#[test]
	fn submit_encoded_xrpl_transaction_validations() {
		TestExt::<Test>::default().build().execute_with(|| {
      // short extrinsic (2 bytes - FF)
      let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732103A4AD384B234BDB4C89216F165FEA6FAAE80F7341370DAF9D82BB2C3F447A33628114937576E59534F15B8793968AF2839B659ACBC881F9EA7C0965787472696E7369637D08303A303A353A4646E1F1").unwrap();
      assert_noop!(
        Xrpl::submit_encoded_xrpl_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes), BoundedVec::default()),
        Error::<Test>::XRPLTransactionExtrinsicLengthInvalid,
      );

      // unknown extrinsic (FFFF)
      let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C25807321028A465BFF70ECFDA31D60D7D9B924A610043519BAF451A6670A55A356DCD0ADA5811475F3513984D423C4E193C6DE08DFCED4CFD150B6F9EA7C0965787472696E7369637D0A303A303A353A46464646E1F1").unwrap();
      assert_noop!(
        Xrpl::submit_encoded_xrpl_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes), BoundedVec::default()),
        Error::<Test>::XRPLTransactionExtrinsicLengthInvalid,
      );

      // known extrinsic; chainIid = 0, nonce = 0, max_block_number = 5, extrinsic = System::remark
      let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C25807321026577EEF1DDBC8B7B883BF19457A5FA4CCBD1EEAF29A51AD2D8370CB3E2DC9F2B81149308E2A8716F3F4BCBE49EFA6FA9DAF75AA31D0DF9EA7C0965787472696E7369637D30303A303A353A353030343030303134303464363937333633363836393635363632303464363136653631363736353634E1F1").unwrap();
      assert_ok!(Xrpl::submit_encoded_xrpl_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes.clone()), BoundedVec::default()));
    });
	}

	#[test]
	fn extrinsic_cannot_perform_privileged_operations() {
		TestExt::<Test>::default().build().execute_with(|| {
			// encoded call for: extrinsic = System::set_code, nonce = 0, max_block_number = 5
			let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732103C7F5304313F8C3CE00E36D0F09A8E08F7EABCD7144E3384FC4E66E75E5522F9D81148AB02D60F912ED0AD339A883C60DB9639311F329F9EA7C0965787472696E7369637D14303A303A353A3138303430303033303831323334E1F1").unwrap();

			// executing xrpl encoded transaction fails since caller is not root/sudo account
			assert_noop!(
				Xrpl::submit_encoded_xrpl_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes), BoundedVec::default()),
				BadOrigin,
			);
		});
	}

	#[test]
	fn signed_extension_validations() {
		TestExt::<Test>::default()
			.with_asset(XRP_ASSET_ID, "XRP", &[(alice(), 0)]) // create XRP asset
			.build()
			.execute_with(|| {
				// encoded call for: chain_id = 1; nonce = 0, max_block_number = 5, extrinsic = System::remark; validates invalid chain id
				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C25807321033A0663EEAAD786F132CDBC25C7D2A6F8C14D55DB7B9AB52AFFB0D8A1C9A1010D81145B3DE9CEA3A77D69DD12F99C12E542907EE49E44F9EA7C0965787472696E7369637D30313A303A353A353030343030303134303464363937333633363836393635363632303464363136653631363736353634E1F1").unwrap()),
					signature: BoundedVec::truncate_from(hex::decode("3045022100B877466D021B990299F5177E33AF2B2D4B40D2D01CF0889C26247BAEF7995C6F02207EEAD77A28D990F94C6F425EC134EAAD456282FD74CB09005656CA208E8A1476").unwrap()),
				}));
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);

				// encoded call for: chain_id = 0, nonce = 1, max_block_number = 5, extrinsic = System::remark; validates nonce too high
				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732103559940F18727930969416A900738B4525FE104C5812C5305365E7B30316BDAA68114DD5493DF89C562B62E277B496FB2DF1043302932F9EA7C0965787472696E7369637D30303A313A353A353030343030303134303464363937333633363836393635363632303464363136653631363736353634E1F1").unwrap()),
					signature: BoundedVec::truncate_from(hex::decode("304402200802EBDB16E1568788BD111BD39DA826D19386F0E3F26CB79D95A0ED4E08052102205784BB0EC6005F59423A886496083AA3626A66224B372FE3977598195E74C73D").unwrap()),
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
				}));
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.clone().into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);

				// validate self contained extrinsic is invalid (invalid signature)
				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(tx_bytes.clone()),
					signature: BoundedVec::truncate_from(hex::decode("304402205CD628B33CD2A89D735EBC139F21A3F2F138F7D687BBAF3E2CDFBBF8951919DC02204B65FC7FF3C2C1B1EEF10186CF6BDAA1C96E8F0814099EE5811C12F65E26A81E").unwrap()),
				}));
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.clone().into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);

				// validate self contained extrinsic fails, user does not have funds to pay for transaction
				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(tx_bytes.clone()),
					signature: BoundedVec::truncate_from(hex::decode("3045022100BD734A38F9C5C210CC7E1D57AEA6DA45039D0068E3ABBA348189A5EBC6A0757D022077B4212F023C66B6C99FB68DC7AEF7921A1BAFF2A85AC6C5E70000C50009231C").unwrap()),
				}));
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.clone().into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);

				// validate self contained extrinsic fails, nested submit_encoded_xrpl_transaction call fails
				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C25807321038504F5A3B50DCC5E2324DB63398D65DD654C1BDC4B62A0D599306E154CC064D581145B1593D888A6767A14FAFFDD519EBB11BBF0412FF9EA7C0965787472696E7369637D20303A353A33343034323330303130303030303030303031303030303030303030E1F1").unwrap()),
					signature: BoundedVec::truncate_from(hex::decode("3045022100BD734A38F9C5C210CC7E1D57AEA6DA45039D0068E3ABBA348189A5EBC6A0757D022077B4212F023C66B6C99FB68DC7AEF7921A1BAFF2A85AC6C5E70000C50009231C").unwrap()),
				}));
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::Call),
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
				// encoded call for: extrinsic = System::remark, nonce = 0, max_block_number = 5
				let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102A56E68CCA82AB7F17CD9FBB582B0E72A554374C309FB5368D6D58E018A7D90B58114F27BEF6F025319DF099741A8DD4B9097CD744FA2F9EA7C0965787472696E7369637D30303A303A353A353030343030303134303464363937333633363836393635363632303464363136653631363736353634E1F1").unwrap();

				// fund the user with XRP (to pay for tx fees)
				let tx = XRPLTransaction::try_from(tx_bytes.as_bytes_ref()).unwrap();
				let caller: AccountId20 = tx.get_account().unwrap().into();
				assert_ok!(AssetsExt::mint_into(2, &caller, 2_000_000));

				let balance_before = Assets::balance(XRP_ASSET_ID, &caller);

				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(tx_bytes.clone()),
					signature: BoundedVec::truncate_from(hex::decode("3044022025E7CB73BBC9517E6FB50416C2882D2637C2ECE9EFA3CBFD9E03C16530CECD6202200759DA60251B262D8D0D0F2754C8F6132DEE4BDDD82BBAFD07E9DC15E4FC4167").unwrap()),
				}));

				// validate self contained extrinsic fails if block_number is exceeded
				System::set_block_number(5);
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
						public_key: [ 2, 165, 110, 104, 204, 168,42, 183, 241, 124, 217, 251, 181, 130, 176, 231, 42,85,67, 116, 195, 9, 251,83, 104, 214, 213, 142, 1, 138, 125, 144, 181 ],
						caller,
						call: mock::RuntimeCall::System(frame_system::Call::remark { remark: b"Mischief Managed".to_vec() }),
					}.into(),
				);

				// verify extrinsic success event
				System::assert_last_event(mock::RuntimeEvent::System(
					frame_system::Event::ExtrinsicSuccess {
						dispatch_info: DispatchInfo {
							weight: Weight::from_ref_time(690_622_000),
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
