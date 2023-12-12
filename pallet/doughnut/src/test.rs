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

use doughnut_rs::Doughnut;
use doughnut_rs::v0::DoughnutV0;
use super::Event;
use crate::{
	mock::{RuntimeEvent as MockEvent, *},
};
use frame_support::{
	assert_ok,
	dispatch::{DispatchClass, GetDispatchInfo},
	traits::fungibles::Mutate,
};
use frame_system::{limits::BlockWeights, RawOrigin};
use pallet_transaction_payment::ChargeTransactionPayment;
use seed_pallet_common::CreateExt;
use sp_core::{ByteArray, ecdsa, Pair, U256};
use sp_core::ecdsa::Public;
use sp_runtime::{traits::SignedExtension, Perbill, print};
use seed_pallet_common::test_prelude::*;
use codec::{Decode, Encode};
use sp_core::bytes::to_hex;

fn make_doughnut(holder: Public, issuer: Public, domain: &str, domain_payload: Vec<u8>) -> Doughnut {
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
		println!("{:?}", to_hex(doughnut_encoded.clone().as_slice(),false));

		let doughnut_decoded =  Doughnut::decode(&mut &doughnut_encoded[..]).unwrap();
		println!("the doughnut {:?}", doughnut_decoded);
	});
}
