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
      // encoded call for: chain_id = 0, nonce = 0, max_block_number = 5, tip = 0, extrinsic = System::remark
			let call = mock::RuntimeCall::System(frame_system::Call::remark { remark: Default::default() });
      let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102509540919FAACF9AB52146C9AA40DB68172D83777250B28E4679176E49CCDD9F81148E6106F6E98E7B21BFDFBFC3DEBA0EDED28A047AF9EA7C0965787472696E7369637D48303A303A353A303A35633933633236383339613137636235616366323765383961616330306639646433663531643161316161346234383266363930663634333633396665383732E1F1").unwrap();
      assert_ok!(Xrpl::submit_encoded_xrpl_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes.clone()), BoundedVec::default(), Box::new(call)));
    });
	}

	#[test]
	fn extrinsic_cannot_perform_privileged_operations() {
		TestExt::<Test>::default().build().execute_with(|| {
			let call = mock::RuntimeCall::System(frame_system::Call::set_code { code: Default::default() });
      // encoded call for: chain_id = 0, nonce = 0, max_block_number = 5, tip = 0, extrinsic = System::set_code
			let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102509540919FAACF9AB52146C9AA40DB68172D83777250B28E4679176E49CCDD9F81148E6106F6E98E7B21BFDFBFC3DEBA0EDED28A047AF9EA7C0965787472696E7369637D48303A303A353A303A66633730373832313235333862623238393633373338393034303237373630313464393765303033656136393430303533303538386134383434393662333337E1F1").unwrap();

			// executing xrpl encoded transaction fails since caller is not root/sudo account
			assert_noop!(
				Xrpl::submit_encoded_xrpl_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes), BoundedVec::default(), Box::new(call)),
				BadOrigin,
			);
		});
	}

	#[test]
	fn validate_invalid_chain_id() {
		TestExt::<Test>::default()
			.with_asset(XRP_ASSET_ID, "XRP", &[(alice(), 0)]) // create XRP asset
			.build()
			.execute_with(|| {
				let call = mock::RuntimeCall::System(frame_system::Call::remark { remark: Default::default() });

				// encoded call for: chain_id = 1; nonce = 0, max_block_number = 5, tip = 0, extrinsic = System::remark; validates invalid chain id
				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102509540919FAACF9AB52146C9AA40DB68172D83777250B28E4679176E49CCDD9F81148E6106F6E98E7B21BFDFBFC3DEBA0EDED28A047AF9EA7C0965787472696E7369637D48303A303A353A303A66633730373832313235333862623238393633373338393034303237373630313464393765303033656136393430303533303538386134383434393662333337E1F1").unwrap()),
					signature: BoundedVec::truncate_from(hex::decode("304402203D76BEF2D67A3B6FAB7972B7B382A654A5E78E74E16197E548F5494D69498256022017DB22937214C595ED2FFEDD9E99F9D830DF81E5B36A57AFFA021F05A497B9D1").unwrap()),
					call: Box::new(call.clone()),
				}));
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);
			});
	}

	#[test]
	fn validate_nonce_too_high() {
		TestExt::<Test>::default()
			.with_asset(XRP_ASSET_ID, "XRP", &[(alice(), 0)]) // create XRP asset
			.build()
			.execute_with(|| {
				let call = mock::RuntimeCall::System(frame_system::Call::remark { remark: Default::default() });

				// encoded call for: chain_id = 0, nonce = 5, max_block_number = 5, tip = 0, extrinsic = System::remark; validates nonce too high
				let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102509540919FAACF9AB52146C9AA40DB68172D83777250B28E4679176E49CCDD9F81148E6106F6E98E7B21BFDFBFC3DEBA0EDED28A047AF9EA7C0965787472696E7369637D48303A353A353A303A35633933633236383339613137636235616366323765383961616330306639646433663531643161316161346234383266363930663634333633396665383732E1F1").unwrap();
				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(tx_bytes.clone()),
					signature: BoundedVec::truncate_from(hex::decode("3044022038D2943A83270CFED21127AA72990F5B9D752AA7293743C872E0F65AAB5BEB8F02200634DD843C0276C36815648D133FE2B1854D55783207CFBC96F9C67E4775E0E3").unwrap()),
					call: Box::new(call.clone()),
				}));

				// fund the user with XRP (to pay for tx fees)
				let tx = XRPLTransaction::try_from(tx_bytes.as_bytes_ref()).unwrap();
				let caller: AccountId20 = tx.get_account().unwrap().into();
				assert_ok!(AssetsExt::mint_into(2, &caller, 2_000_000));
				// validate transaction is successful
				assert_ok!(Executive::validate_transaction(TransactionSource::External, xt.clone().into(), H256::default()));
				// validate that applying extrinsic fails; the pre-dispatch validates nonce mismatch
				assert_err!(
					Executive::apply_extrinsic(xt),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);
			});
	}

	#[test]
	fn validate_hashed_extrinsic() {
		TestExt::<Test>::default()
			.with_asset(XRP_ASSET_ID, "XRP", &[(alice(), 0)]) // create XRP asset
			.build()
			.execute_with(|| {
				let call = mock::RuntimeCall::System(frame_system::Call::remark { remark: Default::default() });

				// validate self contained extrinsic fails, call provided is not signed hashed extrinsic in memo data
				let xt: mock::UncheckedExtrinsicT = fp_self_contained::UncheckedExtrinsic::new_unsigned(mock::RuntimeCall::Xrpl(crate::Call::submit_encoded_xrpl_transaction {
					encoded_msg: BoundedVec::truncate_from(hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102509540919FAACF9AB52146C9AA40DB68172D83777250B28E4679176E49CCDD9F81148E6106F6E98E7B21BFDFBFC3DEBA0EDED28A047AF9EA7C0965787472696E7369637D0A303A303A353A303A3030E1F1").unwrap()),
					signature: BoundedVec::truncate_from(hex::decode("304502210081C0EFD0B5C85AC8C20765B95B44DCD0891619E83529A63A2350907B341EE168022006365C3AB530A1D529606D6EDDE18C76ECF42334FF0DC2140AD392C20305F898").unwrap()),
					call: Box::new(call.clone()),
				}));
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);
			});
	}

	#[test]
	fn validate_transaction_signature() {
		TestExt::<Test>::default()
			.with_asset(XRP_ASSET_ID, "XRP", &[(alice(), 0)]) // create XRP asset
			.build()
			.execute_with(|| {
				let call = mock::RuntimeCall::System(frame_system::Call::remark { remark: Default::default() });

				// encoded call for: chain_id = 0, nonce = 0, max_block_number = 5, tip = 0, extrinsic = System::remark
				let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C25807321021A765BED04797D2DD723C9FDC1ED9D20FEC478F7E8E7D16236F8504C5740C10781145FF8490F22ABFA576788227DB2E80D3F5F104654F9EA7C0965787472696E7369637D48303A303A353A303A35633933633236383339613137636235616366323765383961616330306639646433663531643161316161346234383266363930663634333633396665383732E1F1").unwrap();

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
					signature: BoundedVec::truncate_from(hex::decode("3045022100A6E6546A845ED811FF833789ABE96A5D196737D6FAE0612F40639344DB3ABC2202205D4E3A3753EBC50CB5EBC1A0E861BE0DABA1EE062C08BB40DA9F65F20DEF0CF8").unwrap()),
					call: Box::new(call.clone()),
				}));
				assert_err!(
					Executive::validate_transaction(TransactionSource::External, xt.clone().into(), H256::default()),
					TransactionValidityError::Invalid(InvalidTransaction::BadProof),
				);
				// validate same transaction is successful after funding caller
				let tx = XRPLTransaction::try_from(tx_bytes.as_bytes_ref()).unwrap();
				let caller: AccountId20 = tx.get_account().unwrap().into();
				assert_ok!(AssetsExt::mint_into(2, &caller, 2_000_000));
				assert_ok!(Executive::validate_transaction(TransactionSource::External, xt.clone().into(), H256::default()));
				assert_ok!(Executive::apply_extrinsic(xt));
    	});
	}

	#[test]
	fn system_remark_extrinsic_from_message_success() {
		TestExt::<Test>::default()
			.with_asset(XRP_ASSET_ID, "XRP", &[(alice(), 0)]) // create XRP asset
			.build()
			.execute_with(|| {
				let call = mock::RuntimeCall::System(frame_system::Call::remark { remark: b"Mischief Managed".to_vec() });

      	// encoded call for: chain_id = 0, nonce = 0, max_block_number = 5, tip = 0, extrinsic = System::remark
				let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102509540919FAACF9AB52146C9AA40DB68172D83777250B28E4679176E49CCDD9F81148E6106F6E98E7B21BFDFBFC3DEBA0EDED28A047AF9EA7C0965787472696E7369637D48303A303A353A303A33623832663037383031653632636437383966316233636333353936383236313436613163353136666165613766633633333263643362323563646666316331E1F1").unwrap();
				let signature = hex::decode("3045022100CD796AA2993088249A5076CDD6518BFA31306FDBD5A422594AA0F03854A356F302202DE986F8C9DC08AA95E079241C4BBECC527FAFA4A1B9F7B297C208220C9208AA").unwrap();

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
						public_key: [2,80,149,64,145,159,170,207,154,181,33,70,201,170,64,219,104,23,45,131,119,114,80,178,142,70,121,23,110,73,204,221,159],
						caller,
						r_address: "rDyqBotBNJeXv8PBHY18ABjyw6FQuWXQnu".to_string(),
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
