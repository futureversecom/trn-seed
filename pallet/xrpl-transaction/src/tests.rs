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
use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use hex_literal::hex;
use seed_pallet_common::test_prelude::*;
use seed_primitives::AccountId20;

mod get_runtime_call_from_xumm_extrinsic {
	use super::*;

	#[test]
	fn test_xumm_get_runtime_call_system_remark() {
		TestExt::<Test>::default().build().execute_with(|| {
			let system_remark_call = mock::RuntimeCall::System(frame_system::Call::remark {
				remark: b"Mischief Managed".to_vec(),
			});
			let scale_encoded_call = system_remark_call.encode();
			let hex_encoded_call = hex::encode(&scale_encoded_call);
			assert_eq!("0001404d69736368696566204d616e61676564", hex_encoded_call);

			let unsigned_extrinsic =
				UncheckedExtrinsic::<Test>::new_unsigned(system_remark_call.into());
			let scale_encoded_extrinsic = unsigned_extrinsic.encode();
			let hex_encoded_extrinsic = hex::encode(&scale_encoded_extrinsic);

			assert_eq!("50040001404d69736368696566204d616e61676564", hex_encoded_extrinsic);
			assert_ok!(XrplTransaction::get_runtime_call_from_xumm_extrinsic(
				&scale_encoded_extrinsic
			));
		});
	}

	#[test]
	fn test_xumm_get_runtime_call_balance_transfer() {
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
				UncheckedExtrinsic::<Test>::new_unsigned(balance_transfer_call.into());
			let scale_encoded_extrinsic = unsigned_extrinsic.encode();
			let hex_encoded_extrinsic = hex::encode(&scale_encoded_extrinsic);

			assert_eq!(
				"6404010000000000000000000000000000000000000000009101",
				hex_encoded_extrinsic
			);
			assert_ok!(XrplTransaction::get_runtime_call_from_xumm_extrinsic(
				&scale_encoded_extrinsic
			));
		});
	}

	#[test]
	fn test_xumm_get_runtime_call_set_block_number() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sudo_call =
				mock::RuntimeCall::System(frame_system::Call::set_code { code: vec![] });
			let scale_encoded_call = sudo_call.encode();
			let hex_encoded_call = hex::encode(scale_encoded_call);
			assert_eq!("000300", hex_encoded_call);

			let unsigned_extrinsic = UncheckedExtrinsic::<Test>::new_unsigned(sudo_call.into());
			let scale_encoded_extrinsic = unsigned_extrinsic.encode();
			let hex_encoded_extrinsic = hex::encode(&scale_encoded_extrinsic);

			assert_eq!("1004000300", hex_encoded_extrinsic);
			assert_ok!(XrplTransaction::get_runtime_call_from_xumm_extrinsic(
				&scale_encoded_extrinsic
			));
		});
	}
}

mod submit_encoded_xumm_transaction {
	use super::*;

	#[test]
	fn missing_memo_fields() {
		TestExt::<Test>::default().build().execute_with(|| {
      // nonce = 1 (too high)
      let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C258073210339C5E8A028ECCF4977B951EC31993160CFBE3E5F231798237FC3C6385C626C588114C32E15B4D5A3C107EB37FCD5703B9CFA66D62107F9EA7C0965787472696E7369637D2E313A303A353030343030303134303464363937333633363836393635363632303464363136653631363736353634E1F1").unwrap();
      assert_noop!(
        XrplTransaction::submit_encoded_xumm_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes), BoundedVec::default()),
        Error::<Test>::NonceMismatch,
      );

      // short extrinsic (2 bytes - FF)
      let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A811424A53BB5CAAD40A961836FEF648E8424846EC75AF9EA7C0965787472696E7369637D06303A313A4646E1F1").unwrap();
      assert_noop!(
        XrplTransaction::submit_encoded_xumm_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes), BoundedVec::default()),
        Error::<Test>::XUMMTransactionExtrinsicLengthInvalid,
      );

      // unknown extrinsic
      let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A811424A53BB5CAAD40A961836FEF648E8424846EC75AF9EA7C0965787472696E7369637D08303A313A46464646E1F1").unwrap();
      assert_noop!(
        XrplTransaction::submit_encoded_xumm_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes), BoundedVec::default()),
        Error::<Test>::XUMMTransactionExtrinsicLengthInvalid,
      );

      // known extrinsic (system remark), nonce = 0, max_block_number = 1
      let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A811424A53BB5CAAD40A961836FEF648E8424846EC75AF9EA7C0965787472696E7369637D2E303A313A353030343030303134303464363937333633363836393635363632303464363136653631363736353634E1F1").unwrap();
      assert_ok!(XrplTransaction::submit_encoded_xumm_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes.clone()), BoundedVec::default()));

      // test the same tx fails due to re-used nonce (replay prevention)
      assert_noop!(
        XrplTransaction::submit_encoded_xumm_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes.clone()), BoundedVec::default()),
        Error::<Test>::NonceMismatch,
      );

      // test the same tx fails if block number is exceeded (with nonce = 1)
      System::set_block_number(2);
      let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A811424A53BB5CAAD40A961836FEF648E8424846EC75AF9EA7C0965787472696E7369637D2E313A313A353030343030303134303464363937333633363836393635363632303464363136653631363736353634E1F1").unwrap();
      assert_noop!(
        XrplTransaction::submit_encoded_xumm_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes.clone()), BoundedVec::default()),
        Error::<Test>::MaxBlockNumberExceeded,
      );
    });
	}

	#[test]
	fn system_remark_extrinsic_from_message() {
		let caller = AccountId20::from(hex!("a2ea53a4f8f920f82a5cc0aa665f75403916dc8a"));

		TestExt::<Test>::default().build().execute_with(|| {
      let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A811424A53BB5CAAD40A961836FEF648E8424846EC75AF9EA7C0965787472696E7369637D2E303A313A353030343030303134303464363937333633363836393635363632303464363136653631363736353634E1F1").unwrap();
      let tx = XUMMTransaction::try_from(tx_bytes.as_bytes_ref()).unwrap();
      let call_data = tx.get_extrinsic_data().unwrap().call;
      let remark_call = XrplTransaction::get_runtime_call_from_xumm_extrinsic(&call_data).unwrap();

      // execute xumm encoded transaction
      assert_ok!(XRPLTransaction::submit_encoded_xumm_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes), BoundedVec::default()));

      System::assert_last_event(mock::RuntimeEvent::XRPLTransaction(
        Event::XUMMExtrinsicExecuted { caller, nonce: 0, call: remark_call }
      ));

      // validate account nonce is incremented
      assert_eq!(System::account_nonce(&caller), 1);
    });
	}

	#[test]
	fn balance_transfer_extrinsic_from_message() {
		let caller = AccountId20::from(hex!("a2ea53a4f8f920f82a5cc0aa665f75403916dc8a"));
		let endowed = [(caller, 1_000_000)];

		TestExt::<Test>::default()
      .with_balances(&endowed) // endow the caller with ROOT
      .build()
      .execute_with(|| {
        let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A811424A53BB5CAAD40A961836FEF648E8424846EC75AF9EA7C0965787472696E7369637D38303A313A36343034303130303030303030303030303030303030303030303030303030303030303030303030303030303030303039313031E1F1").unwrap();
        let tx = XUMMTransaction::try_from(tx_bytes.as_bytes_ref()).unwrap();
        let call_data = tx.get_extrinsic_data().unwrap().call;
        let balance_transfer_call = XrplTransaction::get_runtime_call_from_xumm_extrinsic(&call_data).unwrap();

        // execute xumm encoded transaction
        assert_ok!(XrplTransaction::submit_encoded_xumm_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes), BoundedVec::default()));

        // extracted balance transfer call from xumm encoded transaction successfully executed
        System::assert_has_event(
          pallet_balances::Event::<Test>::Transfer {
            from: caller,
            to: Default::default(),
            amount: 100,
          }
          .into(),
        );

        // xumm transaction/extrinsic successfully executed
        System::assert_last_event(mock::RuntimeEvent::XrplTransaction(
          Event::XUMMExtrinsicExecuted { caller, call: balance_transfer_call }
        ));

        // validate account nonce is incremented
        assert_eq!(System::account_nonce(&caller), 1);
      });
	}

	#[test]
	fn extrinsic_cannot_perform_privileged_operations() {
		let caller = AccountId20::from(hex!("a2ea53a4f8f920f82a5cc0aa665f75403916dc8a"));
		let endowed = [(caller, 1_000_000)];

		TestExt::<Test>::default()
      .with_balances(&endowed) // endow the caller with ROOT
      .build()
      .execute_with(|| {
        // encoded call for System::set_code
        let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A811424A53BB5CAAD40A961836FEF648E8424846EC75AF9EA7C0965787472696E7369637D0E303A313A31303034303030333030E1F1").unwrap();

        // executing xumm encoded transaction fails with since caller is not root/sudo account
        assert_noop!(
          XrplTransaction::submit_encoded_xumm_transaction(frame_system::RawOrigin::None.into(), BoundedVec::truncate_from(tx_bytes), BoundedVec::default()),
          BadOrigin,
        );
      });
	}
}

// TODO: setup PR for this branch
// - use release template structure
// TODO: test is not the same call nested in the extrinsic
// TODO: test signature verification e2e in `validate_unsigned` trait impl
