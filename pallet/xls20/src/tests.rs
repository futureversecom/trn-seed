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

#![cfg(test)]
use super::*;
use crate::mock::{
	AssetsExt, MaxTokensPerXls20Mint, Nft, RuntimeEvent as MockEvent,
	System, Test, Xls20, Xls20PaymentAsset,
};
use frame_support::traits::fungibles::Inspect;
use hex_literal::hex;
use pallet_nft::test_utils::NftBuilder;
use pallet_nft::CollectionInfo;
use seed_pallet_common::test_prelude::*;
use seed_primitives::{xrpl::Xls20TokenId, MetadataScheme, CrossChainCompatibility};

// Create an NFT collection with xls20 compatibility
// Returns the created `collection_id`
fn setup_xls20_collection(owner: AccountId, xls_compatible: bool) -> CollectionUuid {
	NftBuilder::<Test>::new(owner)
		.name("test-xls20-collection")
		.cross_chain_compatibility(CrossChainCompatibility { xrpl: xls_compatible })
		.build()
}

#[test]
fn decode_xls20_token_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		//  000B 0C44 95F14B0E44F78A264E41713C64B5F89242540EE2 BC8B858E 00000D65
		// 	+--- +--- +--------------------------------------- +------- +-------
		// 	|    |    |                                        |        |
		// 	|    |    |                                        |        `---> Sequence: 3,429
		// 	|    |    |                                        |
		//  |    |    |                                        `---> Taxon: 146,999,694
		// 	|    |    |
		// 	|    |    `---> Issuer: rNCFjv8Ek5oDrNiMJ3pw6eLLFtMjZLJnf2
		// 	|    |
		//  |    `---> TransferFee: 314.0 bps or 3.140%
		// 	|
		//  `---> Flags: 12 -> lsfBurnable, lsfOnlyXRP and lsfTransferable

		let token = hex!("000B0C4495F14B0E44F78A264E41713C64B5F89242540EE2BC8B858E00000D65");
		let expected = Xls20Token {
			flags: 11,
			transfer_fee: Permill::from_rational(314u32, 10_000),
			issuer: H160::from(hex!("95F14B0E44F78A264E41713C64B5F89242540EE2")),
			taxon: 146_999_694,
			sequence: 3429,
		};
		let actual = Xls20Token::from(token);
		assert_eq!(actual, expected);
	});
}

#[test]
fn set_relayer_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let alice = create_account(10);
		let bob = create_account(11);

		// Not sudo should fail
		assert_noop!(Xls20::set_relayer(RawOrigin::Signed(alice).into(), alice), BadOrigin);
		assert_eq!(Relayer::<Test>::get(), None);

		// Set relayer to Alice
		assert_ok!(Xls20::set_relayer(RawOrigin::Root.into(), alice));
		assert_eq!(Relayer::<Test>::get(), Some(alice));

		// Check event
		System::assert_last_event(MockEvent::Xls20(crate::Event::RelayerSet { account: alice }));

		// Set relayer to Bob
		assert_ok!(Xls20::set_relayer(RawOrigin::Root.into(), bob));
		assert_eq!(Relayer::<Test>::get(), Some(bob));
	});
}

#[test]
fn set_xls20_fee_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let alice = create_account(10);
		let new_fee: Balance = 100;

		// Not sudo should fail
		assert_noop!(Xls20::set_xls20_fee(RawOrigin::Signed(alice).into(), new_fee), BadOrigin);
		assert_eq!(Xls20MintFee::<Test>::get(), 0);

		// Set fee to 100
		assert_ok!(Xls20::set_xls20_fee(RawOrigin::Root.into(), new_fee));
		assert_eq!(Xls20MintFee::<Test>::get(), new_fee);

		// Check event
		System::assert_last_event(MockEvent::Xls20(crate::Event::Xls20MintFeeSet { new_fee }));

		// Set fee to 200
		let new_fee: Balance = 200;
		assert_ok!(Xls20::set_xls20_fee(RawOrigin::Root.into(), new_fee));
		assert_eq!(Xls20MintFee::<Test>::get(), new_fee);

		// Set fee back to 0
		let new_fee: Balance = 0;
		assert_ok!(Xls20::set_xls20_fee(RawOrigin::Root.into(), new_fee));
		assert_eq!(Xls20MintFee::<Test>::get(), new_fee);
	});
}

#[test]
fn xls20_mint_throws_event() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let collection_id = setup_xls20_collection(collection_owner, true);
		let quantity: TokenCount = 5;
		let token_owner = create_account(11);

		// Mint tokens
		assert_ok!(Nft::mint(
			Some(collection_owner).into(),
			collection_id,
			quantity,
			Some(token_owner),
		));

		// Check event is thrown with all serial numbers and token_uris
		let serial_numbers: Vec<SerialNumber> = vec![0, 1, 2, 3, 4];
		let token_uris: Vec<Vec<u8>> = vec![
			b"https://example.com/0".to_vec(),
			b"https://example.com/1".to_vec(),
			b"https://example.com/2".to_vec(),
			b"https://example.com/3".to_vec(),
			b"https://example.com/4".to_vec(),
		];
		System::assert_has_event(
			Event::<Test>::Xls20MintRequest { collection_id, serial_numbers, token_uris }.into(),
		);

		// Mint 2 more tokens for sanity
		let quantity: TokenCount = 2;
		assert_ok!(Nft::mint(
			Some(collection_owner).into(),
			collection_id,
			quantity,
			Some(token_owner),
		));

		// Check event is thrown with all serial numbers and token_uris
		let serial_numbers: Vec<SerialNumber> = vec![5, 6];
		let token_uris: Vec<Vec<u8>> =
			vec![b"https://example.com/5".to_vec(), b"https://example.com/6".to_vec()];
		System::assert_has_event(
			Event::<Test>::Xls20MintRequest { collection_id, serial_numbers, token_uris }.into(),
		);
	});
}

#[test]
fn xls20_mint_with_fee() {
	let collection_owner = create_account(10);
	let initial_balance = 10000;

	TestExt::<Test>::default()
		.with_xrp_balances(&[(collection_owner, initial_balance)])
		.build()
		.execute_with(|| {
			let collection_id = setup_xls20_collection(collection_owner, true);
			let quantity: TokenCount = 5;
			let relayer = create_account(11);
			let new_fee: Balance = 100;

			// Set fee to 100
			assert_ok!(Xls20::set_xls20_fee(RawOrigin::Root.into(), new_fee));
			assert_eq!(Xls20MintFee::<Test>::get(), new_fee);

			// Set relayer to Bob
			assert_ok!(Xls20::set_relayer(RawOrigin::Root.into(), relayer));
			assert_eq!(Relayer::<Test>::get(), Some(relayer));

			// Mint tokens with correct fee works
			assert_ok!(Nft::mint(Some(collection_owner).into(), collection_id, quantity, None,));

			// Check balances are correct after fees are paid.
			let payment_amount = new_fee * quantity as u128; // 500
			let balance_owner = AssetsExt::balance(Xls20PaymentAsset::get(), &collection_owner);
			assert_eq!(balance_owner, initial_balance - payment_amount);

			let balance_relayer = AssetsExt::balance(Xls20PaymentAsset::get(), &relayer);
			assert_eq!(balance_relayer, payment_amount);
		});
}

#[test]
fn xls20_mint_with_fee_no_balance_fails() {
	let collection_owner = create_account(10);
	let initial_balance = 499; // Balance too low

	TestExt::<Test>::default()
		.with_xrp_balances(&[(collection_owner, initial_balance)])
		.build()
		.execute_with(|| {
			let collection_id = setup_xls20_collection(collection_owner, true);
			let quantity: TokenCount = 5;
			let relayer = create_account(11);
			let new_fee: Balance = 100;

			// Set fee to 100
			assert_ok!(Xls20::set_xls20_fee(RawOrigin::Root.into(), new_fee));
			assert_eq!(Xls20MintFee::<Test>::get(), new_fee);

			// Set relayer to Bob
			assert_ok!(Xls20::set_relayer(RawOrigin::Root.into(), relayer));
			assert_eq!(Relayer::<Test>::get(), Some(relayer));

			// Mint tokens with correct fee works
			assert_noop!(
				Nft::mint(Some(collection_owner).into(), collection_id, quantity, None,),
				ArithmeticError::Underflow
			);
		});
}

#[test]
fn re_request_xls20_mint_works() {
	let collection_owner = create_account(10);
	let initial_balance = 10000;

	TestExt::<Test>::default()
		.with_xrp_balances(&[(collection_owner, initial_balance)])
		.build()
		.execute_with(|| {
			let collection_id = setup_xls20_collection(collection_owner, true);
			let relayer = create_account(11);
			let mint_fee: Balance = 100;
			let specified_fee: Balance = 400; // The fee specified by the caller of mint
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerXls20Mint> =
				BoundedVec::try_from(vec![0, 1, 2, 3]).unwrap();

			// Mint tokens
			assert_ok!(Nft::mint(Some(collection_owner).into(), collection_id, 4, None));

			// Set fee to 100
			assert_ok!(Xls20::set_xls20_fee(RawOrigin::Root.into(), mint_fee));
			assert_eq!(Xls20MintFee::<Test>::get(), mint_fee);

			// Set relayer to Bob
			assert_ok!(Xls20::set_relayer(RawOrigin::Root.into(), relayer));
			assert_eq!(Relayer::<Test>::get(), Some(relayer));

			// Re request should pay fees and throw events
			assert_ok!(Xls20::re_request_xls20_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				serial_numbers,
			));

			// Check balances are correct after fees are paid.
			// Note the min fee will be paid, rather than the specified fee (599)
			let balance_owner = AssetsExt::balance(Xls20PaymentAsset::get(), &collection_owner);
			assert_eq!(balance_owner, initial_balance - specified_fee);

			let balance_relayer = AssetsExt::balance(Xls20PaymentAsset::get(), &relayer);
			assert_eq!(balance_relayer, specified_fee);

			// Check event is thrown with all serial numbers and token_uris
			let serial_numbers: Vec<SerialNumber> = vec![0, 1, 2, 3];
			let token_uris: Vec<Vec<u8>> = vec![
				b"https://example.com/0".to_vec(),
				b"https://example.com/1".to_vec(),
				b"https://example.com/2".to_vec(),
				b"https://example.com/3".to_vec(),
			];
			System::assert_last_event(
				Event::<Test>::Xls20MintRequest { collection_id, serial_numbers, token_uris }
					.into(),
			);
		});
}

#[test]
fn re_request_xls20_mint_not_collection_owner_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let bob = create_account(11);
		let collection_id = setup_xls20_collection(collection_owner, true);
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerXls20Mint> =
			BoundedVec::try_from(vec![0]).unwrap();

		assert_noop!(
			Xls20::re_request_xls20_mint(
				RawOrigin::Signed(bob).into(),
				collection_id,
				serial_numbers,
			),
			Error::<Test>::NotCollectionOwner
		);
	});
}

#[test]
fn re_request_xls20_mint_not_xls20_compatible_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let collection_id = setup_xls20_collection(collection_owner, false);
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerXls20Mint> =
			BoundedVec::try_from(vec![0, 1, 2, 3]).unwrap();

		assert_noop!(
			Xls20::re_request_xls20_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				serial_numbers,
			),
			Error::<Test>::NotXLS20Compatible
		);
	});
}

#[test]
fn re_request_xls20_mint_no_collection_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let who = create_account(10);
		let collection_id = 1;

		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerXls20Mint> =
			BoundedVec::try_from(vec![0]).unwrap();

		// Collection doesn't exist so should fail
		assert_noop!(
			Xls20::re_request_xls20_mint(
				RawOrigin::Signed(who).into(),
				collection_id,
				serial_numbers,
			),
			pallet_nft::Error::<Test>::NoCollectionFound
		);
	});
}

#[test]
fn re_request_xls20_mint_empty_serial_numbers_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let who = create_account(10);
		let collection_id = 1;

		// Empty serial numbers should fail
		assert_noop!(
			Xls20::re_request_xls20_mint(
				RawOrigin::Signed(who).into(),
				collection_id,
				Default::default(),
			),
			Error::<Test>::NoToken
		);
	});
}

#[test]
fn re_request_xls20_mint_no_token_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let collection_id = setup_xls20_collection(collection_owner, true);
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerXls20Mint> =
			BoundedVec::try_from(vec![0]).unwrap();

		// Token doesn't exist should fail
		assert_noop!(
			Xls20::re_request_xls20_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				serial_numbers,
			),
			Error::<Test>::NoToken
		);
	});
}

#[test]
fn re_request_xls20_mint_duplicate_mapping_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let relayer = create_account(11);
		let collection_id = setup_xls20_collection(collection_owner, true);
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerXls20Mint> =
			BoundedVec::try_from(vec![0]).unwrap();
		let quantity: TokenCount = 1;
		let token_owner = create_account(12);

		let token_mappings = BoundedVec::truncate_from(vec![(
			0,
			hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66"),
		)]);

		// Set relayer to Bob
		assert_ok!(Xls20::set_relayer(RawOrigin::Root.into(), relayer));

		// Mint tokens
		assert_ok!(Nft::mint(
			Some(collection_owner).into(),
			collection_id,
			quantity,
			Some(token_owner),
		));

		// call fulfill and add mappings to storage
		assert_ok!(Xls20::fulfill_xls20_mint(
			RawOrigin::Signed(relayer).into(),
			collection_id,
			token_mappings.clone()
		));

		// Mapping already exists should fail
		assert_noop!(
			Xls20::re_request_xls20_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				serial_numbers,
			),
			Error::<Test>::MappingAlreadyExists
		);
	});
}

#[test]
fn fulfill_xls20_mint_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let collection_id = setup_xls20_collection(collection_owner, true);
		let relayer = create_account(11);
		let token_mappings = BoundedVec::truncate_from(vec![
			(0, hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66")),
			(1, hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d67")),
			(2, hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d68")),
			(3, hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d69")),
		]);

		// Set relayer to Bob
		assert_ok!(Xls20::set_relayer(RawOrigin::Root.into(), relayer));
		assert_eq!(Relayer::<Test>::get(), Some(relayer));

		// Mint tokens
		assert_ok!(Nft::mint(
			Some(collection_owner).into(),
			collection_id,
			token_mappings.len() as u32,
			None,
		));

		// call fulfill and add mappings to storage
		assert_ok!(Xls20::fulfill_xls20_mint(
			RawOrigin::Signed(relayer).into(),
			collection_id,
			token_mappings.clone()
		));

		// Check all mappings have been stored
		for (serial_number, xls20_token_id) in token_mappings.clone().iter() {
			assert_eq!(
				Xls20TokenMap::<Test>::get(collection_id, serial_number),
				Some(*xls20_token_id)
			);
		}

		// Check event is thrown with new mappings
		System::assert_last_event(
			Event::<Test>::Xls20MappingSet { collection_id, mappings: token_mappings.into_inner() }
				.into(),
		);
	});
}

#[test]
fn fulfill_xls20_empty_token_map_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let collection_id = setup_xls20_collection(collection_owner, true);
		let relayer = create_account(11);
		let token_mappings: BoundedVec<(SerialNumber, Xls20TokenId), MaxTokensPerXls20Mint> =
			BoundedVec::try_from(vec![]).unwrap();

		// Set relayer to Bob
		assert_ok!(Xls20::set_relayer(RawOrigin::Root.into(), relayer));
		assert_eq!(Relayer::<Test>::get(), Some(relayer));

		// call fulfill and add mappings to storage
		assert_noop!(
			Xls20::fulfill_xls20_mint(
				RawOrigin::Signed(relayer).into(),
				collection_id,
				token_mappings.clone()
			),
			Error::<Test>::NoToken
		);
	});
}

#[test]
fn fulfill_xls20_mint_not_relayer_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let collection_id = setup_xls20_collection(collection_owner, true);
		let relayer = create_account(11);
		let token_mappings = BoundedVec::truncate_from(vec![(
			0,
			hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66"),
		)]);

		// Set relayer to Bob
		assert_ok!(Xls20::set_relayer(RawOrigin::Root.into(), relayer));
		assert_eq!(Relayer::<Test>::get(), Some(relayer));

		// call fulfill and add mappings to storage
		assert_noop!(
			Xls20::fulfill_xls20_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				token_mappings.clone()
			),
			Error::<Test>::NotRelayer
		);
	});
}

#[test]
fn fulfill_xls20_mint_no_collection_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_id = 1;
		let relayer = create_account(11);
		let token_mappings = BoundedVec::truncate_from(vec![(
			0,
			hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66"),
		)]);

		// Set relayer to Bob
		assert_ok!(Xls20::set_relayer(RawOrigin::Root.into(), relayer));
		assert_eq!(Relayer::<Test>::get(), Some(relayer));

		// call fulfill and add mappings to storage
		assert_noop!(
			Xls20::fulfill_xls20_mint(
				RawOrigin::Signed(relayer).into(),
				collection_id,
				token_mappings.clone()
			),
			pallet_nft::Error::<Test>::NoCollectionFound
		);
	});
}

#[test]
fn fulfill_xls20_mint_no_token_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let collection_id = setup_xls20_collection(collection_owner, true);
		let relayer = create_account(11);
		let token_mappings = BoundedVec::truncate_from(vec![
			(0, hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66")),
			(1, hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d67")),
			(2, hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d68")),
			(3, hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d69")),
		]);
		// Set relayer to Bob
		assert_ok!(Xls20::set_relayer(RawOrigin::Root.into(), relayer));
		assert_eq!(Relayer::<Test>::get(), Some(relayer));

		// Mint one less token than we submit mappings for
		assert_ok!(Nft::mint(
			Some(collection_owner).into(),
			collection_id,
			token_mappings.len() as u32 - 1_u32,
			None,
		));

		// call fulfill should fail as we have specified a serial number that does not exist
		assert_noop!(
			Xls20::fulfill_xls20_mint(
				RawOrigin::Signed(relayer).into(),
				collection_id,
				token_mappings.clone()
			),
			Error::<Test>::NoToken
		);
	});
}

#[test]
fn fulfill_xls20_mint_duplicate_mapping_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let collection_id = setup_xls20_collection(collection_owner, true);
		let relayer = create_account(11);
		let token_mappings = BoundedVec::truncate_from(vec![
			(0, hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66")),
			(0, hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66")),
		]);

		// Set relayer to Bob
		assert_ok!(Xls20::set_relayer(RawOrigin::Root.into(), relayer));
		assert_eq!(Relayer::<Test>::get(), Some(relayer));

		// Mint tokens
		assert_ok!(Nft::mint(
			Some(collection_owner).into(),
			collection_id,
			token_mappings.len() as u32,
			None,
		));

		// call fulfill should fail due to duplicate token ids in token_mappings
		assert_noop!(
			Xls20::fulfill_xls20_mint(
				RawOrigin::Signed(relayer).into(),
				collection_id,
				token_mappings.clone()
			),
			Error::<Test>::MappingAlreadyExists
		);

		// Submit successful token mappings to add to storage
		let serial_number: SerialNumber = 0;
		let token_mappings = BoundedVec::truncate_from(vec![(
			serial_number,
			hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66"),
		)]);

		assert_ok!(Xls20::fulfill_xls20_mint(
			RawOrigin::Signed(relayer).into(),
			collection_id,
			token_mappings.clone()
		));
		// Check it's added to storage
		assert_eq!(
			Xls20TokenMap::<Test>::get(collection_id, serial_number),
			Some(hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66"))
		);

		// Subsequent call should fail on same token id
		assert_noop!(
			Xls20::fulfill_xls20_mint(
				RawOrigin::Signed(relayer).into(),
				collection_id,
				token_mappings.clone()
			),
			Error::<Test>::MappingAlreadyExists
		);

		// Different serial should work fine
		let serial_number: SerialNumber = 1;
		let token_mappings = BoundedVec::truncate_from(vec![(
			serial_number,
			hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d67"),
		)]);

		assert_ok!(Xls20::fulfill_xls20_mint(
			RawOrigin::Signed(relayer).into(),
			collection_id,
			token_mappings.clone()
		));
		// Again, check it's added to storage
		assert_eq!(
			Xls20TokenMap::<Test>::get(collection_id, serial_number),
			Some(hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d67"))
		);
	});
}

#[test]
fn enable_xls20_compatibility_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let collection_id = setup_xls20_collection(collection_owner, false);

		// XLS-20 compatibility disabled
		assert_eq!(
			CollectionInfo::<Test>::get(collection_id).unwrap().cross_chain_compatibility,
			CrossChainCompatibility { xrpl: false },
		);

		// Can successfully enable XLS-20 compatibility
		assert_ok!(Xls20::enable_xls20_compatibility(
			RawOrigin::Signed(collection_owner).into(),
			collection_id,
		));

		// Check event
		System::assert_last_event(MockEvent::Xls20(crate::Event::Xls20CompatibilityEnabled {
			collection_id,
		}));

		// XLS-20 compatibility now enabled
		assert_eq!(
			CollectionInfo::<Test>::get(collection_id).unwrap().cross_chain_compatibility,
			CrossChainCompatibility { xrpl: true },
		);
	});
}

#[test]
fn enable_xls20_compatibility_no_collection_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let collection_id = 1;

		// Can not enable compatibility if collection doesn't exist
		assert_noop!(
			Xls20::enable_xls20_compatibility(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
			),
			pallet_nft::Error::<Test>::NoCollectionFound
		);
	});
}

#[test]
fn enable_xls20_compatibility_not_collection_owner_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let bob = create_account(11);
		let collection_id = setup_xls20_collection(collection_owner, false);

		// Can not enable compatibility if not owner
		assert_noop!(
			Xls20::enable_xls20_compatibility(RawOrigin::Signed(bob).into(), collection_id,),
			pallet_nft::Error::<Test>::NotCollectionOwner
		);
	});
}

#[test]
fn enable_xls20_compatibility_non_zero_issuance_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let collection_id = setup_xls20_collection(collection_owner, false);

		// Mint 1 token
		assert_ok!(Nft::mint(Some(collection_owner).into(), collection_id, 1, None));

		// Can not enable compatibility if tokens are minted in collection
		assert_noop!(
			Xls20::enable_xls20_compatibility(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
			),
			pallet_nft::Error::<Test>::CollectionIssuanceNotZero
		);
	});
}

mod set_collection_mappings {
	use super::*;

	#[test]
	fn set_collection_mappings_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let collection_mappings = vec![
				(12, Xls20Collection::new(H160::from_low_u64_be(12), 123)),
				(22, Xls20Collection::new(H160::from_low_u64_be(22), 223)),
				(32, Xls20Collection::new(H160::from_low_u64_be(32), 323)),
			];

			assert_ok!(Xls20::set_collection_mappings(
				RawOrigin::Root.into(),
				collection_mappings.clone()
			));

			for (collection_id, xls20_collection) in collection_mappings.clone() {
				assert_eq!(CollectionMapping::<Test>::get(xls20_collection), Some(collection_id));
			}

			// Check event
			System::assert_last_event(MockEvent::Xls20(crate::Event::Xls20CollectionMappingsSet {
				mappings: collection_mappings,
			}));
		});
	}

	#[test]
	fn set_collection_mappings_not_sudo_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let collection_mappings =
				vec![(12, Xls20Collection::new(H160::from_low_u64_be(12), 123))];

			assert_noop!(
				Xls20::set_collection_mappings(
					RawOrigin::Signed(create_account(10)).into(),
					collection_mappings.clone()
				),
				BadOrigin
			);
		});
	}
}

mod deposit_token {
	use super::*;

	#[test]
	/// Test the flow where a token is deposited to TRN and needs to be minted into the existing
	/// collection
	fn deposit_token_mint_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_xls20_collection(collection_owner, true);
			let beneficiary = create_account(12);
			let xls20_token_id =
				hex!("000B0C4495F14B0E44F78A264E41713C64B5F89242540EE2BC8B858E00000D65");
			let xls20_token = Xls20Token::from(xls20_token_id);
			let xls20_collection = Xls20Collection::new(xls20_token.issuer, xls20_token.taxon);
			CollectionMapping::<Test>::insert(xls20_collection, collection_id);

			// Deposit token
			let weight = Xls20::deposit_xls20_token(&beneficiary, xls20_token_id)
				.expect("Failed to deposit token");
			assert_eq!(weight, <Test as Config>::WeightInfo::deposit_token_mint());

			// Token was minted into beneficiary address
			let new_owner =
				<Test as Config>::NFTExt::get_token_owner(&(collection_id, xls20_token.sequence))
					.unwrap();
			assert_eq!(new_owner, beneficiary);

			let collection_info = <CollectionInfo<Test>>::get(collection_id).unwrap();
			assert_eq!(collection_info.collection_issuance, 1);
		});
	}

	#[test]
	/// Test the flow where a token is deposited to TRN and the collection needs to be created
	/// and the token minted into the beneficiary address
	fn deposit_token_create_collection_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let beneficiary = create_account(12);
			let xls20_token_id =
				hex!("000B0C4495F14B0E44F78A264E41713C64B5F89242540EE2BC8B858E00000D65");
			let xls20_token = Xls20Token::from(xls20_token_id);
			let serial_number = xls20_token.sequence;
			let collection_id = <Test as Config>::NFTExt::next_collection_uuid()
				.expect("Failed to get next collection uuid");

			// Deposit token
			let weight = Xls20::deposit_xls20_token(&beneficiary, xls20_token_id)
				.expect("Failed to deposit token");
			assert_eq!(weight, <Test as Config>::WeightInfo::deposit_token_create_collection());

			// Token was minted into beneficiary address
			let new_owner =
				<Test as Config>::NFTExt::get_token_owner(&(collection_id, serial_number)).unwrap();
			assert_eq!(new_owner, beneficiary);

			let collection_info = <CollectionInfo<Test>>::get(collection_id).unwrap();
			assert_eq!(collection_info.collection_issuance, 1);
			// Cross chain compatibility should be enabled
			assert!(collection_info.cross_chain_compatibility.xrpl);
			// Origin chain is XRPL
			assert_eq!(collection_info.origin_chain, OriginChain::XRPL);
			// collection mapping set
			let xls20_collection = Xls20Collection::new(xls20_token.issuer, xls20_token.taxon);
			assert_eq!(CollectionMapping::<Test>::get(xls20_collection), Some(collection_id));
		});
	}
}
