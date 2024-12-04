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
use crate::mock::{
	AssetsExt, ChainId, NFINetworkFeePercentage, NativeAssetId, Nfi, Nft,
	RuntimeEvent as MockEvent, Sft, System, Test,
};
use core::ops::Mul;
use frame_support::traits::fungibles::Inspect;
use pallet_nft::test_utils::NftBuilder;
use pallet_sft::test_utils::SftBuilder;
use seed_pallet_common::test_prelude::*;
use sp_runtime::ArithmeticError;

// Helper function to create a MultiChainTokenId from a standard TRN TokenId
fn create_mc_token_id(token_id: TokenId) -> MultiChainTokenId<mock::MaxByteLength> {
	MultiChainTokenId {
		chain_id: ChainId::get(),
		collection_id: GenericCollectionId::U32(token_id.0),
		serial_number: GenericSerialNumber::U32(token_id.1),
	}
}

mod set_relayer {
	use super::*;

	#[test]
	fn set_relayer_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Set relayer to Alice
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), alice()));
			assert_eq!(Relayer::<Test>::get(), Some(alice()));

			// Check event
			System::assert_last_event(MockEvent::Nfi(crate::Event::RelayerSet {
				account: alice(),
			}));

			// Set relayer to Bob
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), bob()));
			assert_eq!(Relayer::<Test>::get(), Some(bob()));
		});
	}

	#[test]
	fn set_relayer_not_root_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Not sudo should fail
			assert_noop!(Nfi::set_relayer(RawOrigin::Signed(alice()).into(), alice()), BadOrigin);
		});
	}
}

mod set_fee_to {
	use super::*;

	#[test]
	fn set_fee_to_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Change fee_to account
			let new_fee_to = create_account(10);
			assert_ok!(Nfi::set_fee_to(RawOrigin::Root.into(), Some(new_fee_to.clone())));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::FeeToSet {
				account: Some(new_fee_to),
			}));
			// Storage updated
			assert_eq!(FeeTo::<Test>::get().unwrap(), new_fee_to);
		});
	}

	#[test]
	fn set_fee_to_not_root_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Change fee_to account from not sudo should fail
			let new_fee_to = create_account(10);
			assert_noop!(
				Nfi::set_fee_to(Some(create_account(11)).into(), Some(new_fee_to)),
				BadOrigin
			);
		});
	}
}

mod set_fee_details {
	use super::*;

	#[test]
	fn set_fee_details_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let new_fee = FeeDetails { asset_id: 123, amount: 400, receiver: bob() };
			let sub_type = NFISubType::NFI;
			assert_ok!(Nfi::set_fee_details(
				RawOrigin::Root.into(),
				sub_type,
				Some(new_fee.clone())
			));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::FeeDetailsSet {
				sub_type: NFISubType::NFI,
				fee_details: Some(new_fee.clone()),
			}));

			// Storage updated
			assert_eq!(MintFee::<Test>::get(sub_type).unwrap(), new_fee);
		});
	}

	#[test]
	fn set_fee_details_removes_storage_if_none() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Set initial value
			let new_fee = FeeDetails { asset_id: 123, amount: 400, receiver: bob() };
			let sub_type = NFISubType::NFI;
			assert_ok!(Nfi::set_fee_details(
				RawOrigin::Root.into(),
				sub_type,
				Some(new_fee.clone())
			));
			// Sanity check
			assert_eq!(MintFee::<Test>::get(sub_type).unwrap(), new_fee);

			// Set to none
			assert_ok!(Nfi::set_fee_details(RawOrigin::Root.into(), sub_type, None));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::FeeDetailsSet {
				sub_type: NFISubType::NFI,
				fee_details: None,
			}));

			// Storage updated
			assert_eq!(MintFee::<Test>::get(sub_type), None);
		});
	}

	#[test]
	fn set_fee_details_zero_amount_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let new_fee = FeeDetails { asset_id: 123, amount: 0, receiver: bob() };
			let sub_type = NFISubType::NFI;
			assert_noop!(
				Nfi::set_fee_details(RawOrigin::Root.into(), sub_type, Some(new_fee.clone())),
				Error::<Test>::InvalidMintFee
			);
		});
	}

	#[test]
	fn set_fee_details_not_root_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let new_fee = FeeDetails { asset_id: 123, amount: 400, receiver: bob() };
			let sub_type = NFISubType::NFI;
			assert_noop!(
				Nfi::set_fee_details(
					RawOrigin::Signed(alice()).into(),
					sub_type,
					Some(new_fee.clone())
				),
				BadOrigin
			);
		});
	}
}

mod enable_nfi {
	use super::*;

	#[test]
	fn enable_nfi_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let collection_id = NftBuilder::<Test>::new(collection_owner).build();
			let chain_id = ChainId::get();

			// Sanity check
			assert!(!NfiEnabled::<Test>::get(
				(chain_id, GenericCollectionId::U32(collection_id)),
				sub_type
			));

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::NfiEnabled {
				sub_type,
				collection_id: GenericCollectionId::U32(collection_id),
			}));

			// Storage updated
			assert!(NfiEnabled::<Test>::get(
				(chain_id, GenericCollectionId::U32(collection_id)),
				sub_type
			));
		});
	}

	#[test]
	fn enable_nfi_not_collection_owner_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let collection_id = NftBuilder::<Test>::new(collection_owner).build();

			// Enable NFI should fail
			assert_noop!(
				Nfi::enable_nfi_for_trn_collection(
					RawOrigin::Signed(bob()).into(),
					collection_id,
					sub_type
				),
				Error::<Test>::NotCollectionOwner
			);

			// Still disabled
			let collection_id = (ChainId::get(), GenericCollectionId::U32(collection_id));
			assert!(!NfiEnabled::<Test>::get(collection_id, sub_type));
		});
	}
}

mod manual_data_request {
	use super::*;

	#[test]
	fn manual_data_request_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let token_owner = bob();
			let collection_id = NftBuilder::<Test>::new(collection_owner)
				.token_owner(token_owner)
				.initial_issuance(1)
				.build();
			let token_id = create_mc_token_id((collection_id, 0));

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			// Request data
			assert_ok!(Nfi::manual_data_request(
				RawOrigin::Signed(token_owner.clone()).into(),
				token_id.clone(),
				sub_type
			));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataRequestNew {
				caller: token_owner,
				sub_type,
				token_id,
			}));
		});
	}

	#[test]
	fn manual_data_request_overwrite_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let token_owner = bob();
			let collection_id = NftBuilder::<Test>::new(collection_owner)
				.token_owner(token_owner)
				.initial_issuance(1)
				.build();
			let token_id = create_mc_token_id((collection_id, 0));
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), bob()));

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			// Request data
			assert_ok!(Nfi::manual_data_request(
				RawOrigin::Signed(token_owner.clone()).into(),
				token_id.clone(),
				sub_type
			));

			// Event thrown for requesting new data
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataRequestNew {
				caller: token_owner,
				sub_type,
				token_id: token_id.clone(),
			}));


			// Submit data
			let data_item = NFIDataType::NFI(NFIMatrix {
				metadata_link: BoundedVec::truncate_from(b"https://example.com".to_vec()),
				verification_hash: H256::from_low_u64_be(123),
			});
			assert_ok!(Nfi::submit_nfi_data(
				RawOrigin::Signed(bob()).into(),
				token_id.clone(),
				data_item.clone()
			));


			// Request data again
			assert_ok!(Nfi::manual_data_request(
				RawOrigin::Signed(token_owner.clone()).into(),
				token_id.clone(),
				sub_type
			));

			// Event thrown for requesting data on pre-existing token
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataRequestExisting {
				caller: token_owner,
				sub_type,
				token_id: token_id.clone(),
			}));


			// Submit data is successful
			let data_item = NFIDataType::NFI(NFIMatrix {
				metadata_link: BoundedVec::truncate_from(b"https://example2.com".to_vec()),
				verification_hash: H256::from_low_u64_be(124),
			});
			assert_ok!(Nfi::submit_nfi_data(
				RawOrigin::Signed(bob()).into(),
				token_id.clone(),
				data_item.clone()
			));

			// verify data
			assert_eq!(NfiData::<Test>::get(token_id, sub_type).unwrap(), data_item);
		});
	}

	#[test]
	fn manual_data_request_collection_owner_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let token_owner = bob();
			let collection_id = NftBuilder::<Test>::new(collection_owner)
				.token_owner(token_owner)
				.initial_issuance(1)
				.build();
			let token_id = create_mc_token_id((collection_id, 0));

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			// Request data
			assert_ok!(Nfi::manual_data_request(
				RawOrigin::Signed(collection_owner.clone()).into(),
				token_id.clone(),
				sub_type
			));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataRequestNew {
				caller: collection_owner,
				sub_type,
				token_id,
			}));
		});
	}

	#[test]
	fn manual_data_request_works_for_sft() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let token_id = SftBuilder::<Test>::new(collection_owner).build();

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				token_id.0,
				sub_type
			));

			let token_id = create_mc_token_id(token_id);

			// Request data
			assert_ok!(Nfi::manual_data_request(
				RawOrigin::Signed(collection_owner.clone()).into(),
				token_id.clone(),
				sub_type
			));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataRequestNew {
				caller: collection_owner,
				sub_type,
				token_id,
			}));
		});
	}

	#[test]
	fn manual_data_request_not_token_owner_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let token_owner = bob();
			let collection_id = NftBuilder::<Test>::new(collection_owner)
				.token_owner(token_owner)
				.initial_issuance(1)
				.build();
			let token_id = create_mc_token_id((collection_id, 0));

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			// Request data should fail
			assert_noop!(
				Nfi::manual_data_request(
					RawOrigin::Signed(create_account(123)).into(),
					token_id,
					sub_type
				),
				Error::<Test>::NotTokenOwner
			);
		});
	}

	#[test]
	fn manual_data_request_not_token_owner_fails_for_sft() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let token_id = SftBuilder::<Test>::new(collection_owner).build();

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				token_id.0,
				sub_type
			));

			let token_id = create_mc_token_id(token_id);
			// Request data should fail
			assert_noop!(
				Nfi::manual_data_request(
					RawOrigin::Signed(create_account(123)).into(),
					token_id,
					sub_type
				),
				Error::<Test>::NotTokenOwner
			);
		});
	}

	#[test]
	fn manual_data_request_pays_fees() {
		let collection_owner = alice();
		let mint_fee = 400;

		TestExt::<Test>::default()
			.with_balances(&[(collection_owner, mint_fee)])
			.build()
			.execute_with(|| {
				let sub_type = NFISubType::NFI;
				let collection_id =
					NftBuilder::<Test>::new(collection_owner).initial_issuance(1).build();
				let token_id = create_mc_token_id((collection_id, 0));

				// Enable NFI
				assert_ok!(Nfi::enable_nfi_for_trn_collection(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					sub_type
				));

				// Set fee details to 400 ROOT
				let fee = FeeDetails {
					asset_id: NativeAssetId::get(),
					amount: mint_fee,
					receiver: bob(),
				};
				assert_ok!(Nfi::set_fee_details(
					RawOrigin::Root.into(),
					sub_type,
					Some(fee.clone())
				));

				// Request data
				assert_ok!(Nfi::manual_data_request(
					RawOrigin::Signed(collection_owner).into(),
					token_id,
					sub_type
				));

				// Check fees paid
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &bob()), mint_fee);
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &collection_owner), 0);
			});
	}

	#[test]
	fn manual_data_request_pays_fees_xrp() {
		let collection_owner = alice();
		let mint_fee = 400;

		TestExt::<Test>::default()
			.with_xrp_balances(&[(collection_owner, mint_fee)])
			.build()
			.execute_with(|| {
				let sub_type = NFISubType::NFI;
				let collection_id =
					NftBuilder::<Test>::new(collection_owner).initial_issuance(1).build();
				let token_id = create_mc_token_id((collection_id, 0));

				// Enable NFI
				assert_ok!(Nfi::enable_nfi_for_trn_collection(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					sub_type
				));

				// Set fee details to 400 XRP
				let fee = FeeDetails { asset_id: XRP_ASSET_ID, amount: mint_fee, receiver: bob() };
				assert_ok!(Nfi::set_fee_details(
					RawOrigin::Root.into(),
					sub_type,
					Some(fee.clone())
				));

				// Request data
				assert_ok!(Nfi::manual_data_request(
					RawOrigin::Signed(collection_owner).into(),
					token_id,
					sub_type
				));

				// Check fees paid
				assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &bob()), mint_fee);
				assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &collection_owner), 0);
			});
	}

	#[test]
	fn manual_data_request_pays_network_fee() {
		let collection_owner = alice();
		let mint_fee = 1000;

		TestExt::<Test>::default()
			.with_balances(&[(collection_owner, mint_fee)])
			.build()
			.execute_with(|| {
				let sub_type = NFISubType::NFI;
				let collection_id =
					NftBuilder::<Test>::new(collection_owner).initial_issuance(1).build();
				let token_id = create_mc_token_id((collection_id, 0));

				// Enable NFI
				assert_ok!(Nfi::enable_nfi_for_trn_collection(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					sub_type
				));

				// Set fee details to 1000 ROOT
				let fee = FeeDetails {
					asset_id: NativeAssetId::get(),
					amount: mint_fee,
					receiver: bob(),
				};
				assert_ok!(Nfi::set_fee_details(
					RawOrigin::Root.into(),
					sub_type,
					Some(fee.clone())
				));

				// Set FeeTo address
				let fee_to = charlie();
				assert_ok!(Nfi::set_fee_to(RawOrigin::Root.into(), Some(fee_to.clone())));

				// Request data
				assert_ok!(Nfi::manual_data_request(
					RawOrigin::Signed(collection_owner).into(),
					token_id,
					sub_type
				));

				// Check fees paid
				let network_fee = NFINetworkFeePercentage::get().mul(mint_fee);
				assert_eq!(
					AssetsExt::balance(NativeAssetId::get(), &bob()),
					mint_fee - network_fee
				);
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_to), network_fee);
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &collection_owner), 0);
			});
	}

	#[test]
	fn manual_data_request_low_balance_fails() {
		let collection_owner = alice();
		let mint_fee = 400;

		TestExt::<Test>::default()
			.with_balances(&[(collection_owner, mint_fee - 1)])
			.build()
			.execute_with(|| {
				let sub_type = NFISubType::NFI;
				let collection_id =
					NftBuilder::<Test>::new(collection_owner).initial_issuance(1).build();
				let token_id = create_mc_token_id((collection_id, 0));

				// Enable NFI
				assert_ok!(Nfi::enable_nfi_for_trn_collection(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					sub_type
				));

				// Set fee details to 400 ROOT
				let fee = FeeDetails {
					asset_id: NativeAssetId::get(),
					amount: mint_fee,
					receiver: bob(),
				};
				assert_ok!(Nfi::set_fee_details(
					RawOrigin::Root.into(),
					sub_type,
					Some(fee.clone())
				));

				// Request data fails due to low balance
				assert_noop!(
					Nfi::manual_data_request(
						RawOrigin::Signed(collection_owner).into(),
						token_id,
						sub_type
					),
					ArithmeticError::Underflow
				);
			});
	}

	#[test]
	fn manual_data_request_invalid_trn_collection_format_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let token_owner = bob();
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get(),
				collection_id: GenericCollectionId::U64(123),
				serial_number: GenericSerialNumber::U32(2),
			};

			// Manually Enable NFI
			NfiEnabled::<Test>::insert(
				(ChainId::get(), GenericCollectionId::U64(123)),
				sub_type,
				true,
			);

			// Request data
			assert_noop!(
				Nfi::manual_data_request(
					RawOrigin::Signed(token_owner.clone()).into(),
					token_id.clone(),
					sub_type
				),
				Error::<Test>::InvalidTokenFormat
			);
		});
	}

	#[test]
	fn manual_data_request_invalid_trn_serial_format_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let token_owner = bob();
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get(),
				collection_id: GenericCollectionId::U32(123),
				serial_number: GenericSerialNumber::U64(2),
			};

			// Manually Enable NFI
			NfiEnabled::<Test>::insert(
				(ChainId::get(), GenericCollectionId::U32(123)),
				sub_type,
				true,
			);

			// Request data
			assert_noop!(
				Nfi::manual_data_request(
					RawOrigin::Signed(token_owner.clone()).into(),
					token_id.clone(),
					sub_type
				),
				Error::<Test>::InvalidTokenFormat
			);
		});
	}

	#[test]
	fn manual_data_request_non_trn_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let token_owner = bob();

			// Request data U64
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get() + 1, // Not TRN chain Id will bypass checks
				collection_id: GenericCollectionId::U64(123),
				serial_number: GenericSerialNumber::U64(2),
			};
			assert_ok!(Nfi::manual_data_request(
				RawOrigin::Signed(token_owner.clone()).into(),
				token_id.clone(),
				sub_type
			));
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataRequestNew {
				caller: token_owner,
				sub_type,
				token_id,
			}));

			// Request data U128
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get() + 1, // Not TRN chain Id will bypass checks
				collection_id: GenericCollectionId::U128(123),
				serial_number: GenericSerialNumber::U128(2),
			};
			assert_ok!(Nfi::manual_data_request(
				RawOrigin::Signed(token_owner.clone()).into(),
				token_id.clone(),
				sub_type
			));
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataRequestNew {
				caller: token_owner,
				sub_type,
				token_id,
			}));

			// Request data H160
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get() + 1, // Not TRN chain Id will bypass checks
				collection_id: GenericCollectionId::H160(H160::zero()),
				serial_number: GenericSerialNumber::U32(2),
			};
			assert_ok!(Nfi::manual_data_request(
				RawOrigin::Signed(token_owner.clone()).into(),
				token_id.clone(),
				sub_type
			));
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataRequestNew {
				caller: token_owner,
				sub_type,
				token_id,
			}));

			// Request data H256
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get() + 1, // Not TRN chain Id will bypass checks
				collection_id: GenericCollectionId::H256(H256::zero()),
				serial_number: GenericSerialNumber::U32(2),
			};
			assert_ok!(Nfi::manual_data_request(
				RawOrigin::Signed(token_owner.clone()).into(),
				token_id.clone(),
				sub_type
			));
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataRequestNew {
				caller: token_owner,
				sub_type,
				token_id,
			}));

			// Request data Bytes
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get() + 1, // Not TRN chain Id will bypass checks
				collection_id: GenericCollectionId::Bytes(BoundedVec::truncate_from(
					b"123".to_vec(),
				)),
				serial_number: GenericSerialNumber::Bytes(BoundedVec::truncate_from(b"2".to_vec())),
			};
			assert_ok!(Nfi::manual_data_request(
				RawOrigin::Signed(token_owner.clone()).into(),
				token_id.clone(),
				sub_type
			));
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataRequestNew {
				caller: token_owner,
				sub_type,
				token_id,
			}));

			// Request data Empty
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get() + 1, // Not TRN chain Id will bypass checks
				collection_id: GenericCollectionId::Empty,
				serial_number: GenericSerialNumber::U32(2),
			};
			assert_ok!(Nfi::manual_data_request(
				RawOrigin::Signed(token_owner.clone()).into(),
				token_id.clone(),
				sub_type
			));
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataRequestNew {
				caller: token_owner,
				sub_type,
				token_id,
			}));
		});
	}

	#[test]
	fn manual_data_request_non_trn_pays_mint_fee() {
		let token_owner = alice();
		let mint_fee = 1000;

		TestExt::<Test>::default()
			.with_balances(&[(token_owner, mint_fee)])
			.build()
			.execute_with(|| {
				let sub_type = NFISubType::NFI;

				// Set fee details to 1000 ROOT
				let fee = FeeDetails {
					asset_id: NativeAssetId::get(),
					amount: mint_fee,
					receiver: bob(),
				};
				assert_ok!(Nfi::set_fee_details(
					RawOrigin::Root.into(),
					sub_type,
					Some(fee.clone())
				));

				// Set FeeTo address
				let fee_to = charlie();
				assert_ok!(Nfi::set_fee_to(RawOrigin::Root.into(), Some(fee_to.clone())));

				// Request data U64
				let token_id = MultiChainTokenId {
					chain_id: ChainId::get() + 1, // Not TRN chain Id will bypass checks
					collection_id: GenericCollectionId::U64(123),
					serial_number: GenericSerialNumber::U64(2),
				};
				assert_ok!(Nfi::manual_data_request(
					RawOrigin::Signed(token_owner.clone()).into(),
					token_id.clone(),
					sub_type
				));
				System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataRequestNew {
					caller: token_owner,
					sub_type,
					token_id,
				}));

				// Check fees paid
				let network_fee = NFINetworkFeePercentage::get().mul(mint_fee);
				assert_eq!(
					AssetsExt::balance(NativeAssetId::get(), &bob()),
					mint_fee - network_fee
				);
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_to), network_fee);
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &token_owner), 0);
			});
	}
}

mod submit_nfi_data {
	use super::*;

	#[test]
	fn submit_nfi_data_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let collection_id =
				NftBuilder::<Test>::new(collection_owner).initial_issuance(1).build();
			let token_id = create_mc_token_id((collection_id, 0));

			// Set relayer to bob
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), bob()));

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			let data_item = NFIDataType::NFI(NFIMatrix {
				metadata_link: BoundedVec::truncate_from(b"https://example.com".to_vec()),
				verification_hash: H256::from_low_u64_be(123),
			});

			// Submit data
			assert_ok!(Nfi::submit_nfi_data(
				RawOrigin::Signed(bob()).into(),
				token_id.clone(),
				data_item.clone()
			));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataSet {
				sub_type,
				token_id: token_id.clone(),
				data_item: data_item.clone(),
			}));

			// Storage updated
			assert_eq!(NfiData::<Test>::get(token_id, sub_type), Some(data_item));
		});
	}

	#[test]
	fn submit_nfi_data_works_for_sft() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let token_id = SftBuilder::<Test>::new(collection_owner).build();

			// Set relayer to bob
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), bob()));

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				token_id.0,
				sub_type
			));

			let token_id = create_mc_token_id(token_id);
			let data_item = NFIDataType::NFI(NFIMatrix {
				metadata_link: BoundedVec::truncate_from(b"https://example.com".to_vec()),
				verification_hash: H256::from_low_u64_be(123),
			});

			// Submit data
			assert_ok!(Nfi::submit_nfi_data(
				RawOrigin::Signed(bob()).into(),
				token_id.clone(),
				data_item.clone()
			));

			// Event thrown
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataSet {
				sub_type,
				token_id,
				data_item,
			}));
		});
	}

	#[test]
	fn submit_nfi_data_not_relayer_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let collection_id =
				NftBuilder::<Test>::new(collection_owner).initial_issuance(1).build();
			let token_id = create_mc_token_id((collection_id, 0));

			// Set relayer to charlie
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), charlie()));

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			// Submit data should fail
			assert_noop!(
				Nfi::submit_nfi_data(
					RawOrigin::Signed(collection_owner).into(),
					token_id,
					NFIDataType::NFI(NFIMatrix {
						metadata_link: BoundedVec::truncate_from(b"https://example.com".to_vec()),
						verification_hash: H256::from_low_u64_be(123),
					})
				),
				Error::<Test>::NotRelayer
			);
		});
	}

	#[test]
	fn submit_nfi_data_not_enabled_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let collection_owner = alice();
			let collection_id =
				NftBuilder::<Test>::new(collection_owner).initial_issuance(1).build();
			let token_id = create_mc_token_id((collection_id, 0));

			// Set relayer to bob
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), bob()));

			// Submit data should fail
			assert_noop!(
				Nfi::submit_nfi_data(
					RawOrigin::Signed(bob()).into(),
					token_id,
					NFIDataType::NFI(NFIMatrix {
						metadata_link: BoundedVec::truncate_from(b"https://example.com".to_vec()),
						verification_hash: H256::from_low_u64_be(123),
					})
				),
				Error::<Test>::NotEnabled
			);
		});
	}

	#[test]
	fn submit_nfi_data_no_token_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let collection_id =
				NftBuilder::<Test>::new(collection_owner).initial_issuance(1).build();

			// Set relayer to bob
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), bob()));

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			// Submit data should fail
			let token_id = create_mc_token_id((collection_id, 1));
			assert_noop!(
				Nfi::submit_nfi_data(
					RawOrigin::Signed(bob()).into(),
					token_id, // Token does not exist
					NFIDataType::NFI(NFIMatrix {
						metadata_link: BoundedVec::truncate_from(b"https://example.com".to_vec()),
						verification_hash: H256::from_low_u64_be(123),
					})
				),
				Error::<Test>::NoToken
			);
		});
	}

	#[test]
	fn submit_nfi_data_no_token_fails_sft() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let (collection_id, _) = SftBuilder::<Test>::new(collection_owner).build();

			// Set relayer to bob
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), bob()));

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			let token_id = create_mc_token_id((collection_id, 1));
			// Submit data should fail
			assert_noop!(
				Nfi::submit_nfi_data(
					RawOrigin::Signed(bob()).into(),
					token_id, // Token does not exist
					NFIDataType::NFI(NFIMatrix {
						metadata_link: BoundedVec::truncate_from(b"https://example.com".to_vec()),
						verification_hash: H256::from_low_u64_be(123),
					})
				),
				Error::<Test>::NoToken
			);
		});
	}

	#[test]
	fn submit_data_request_invalid_trn_collection_format_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get(),
				collection_id: GenericCollectionId::U64(123),
				serial_number: GenericSerialNumber::U32(2),
			};
			let data = NFIDataType::NFI(NFIMatrix {
				metadata_link: BoundedVec::truncate_from(b"https://example.com".to_vec()),
				verification_hash: H256::from_low_u64_be(123),
			});

			// Set relayer to alice
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), alice()));

			// Manually Enable NFI
			NfiEnabled::<Test>::insert(
				(ChainId::get(), GenericCollectionId::U64(123)),
				sub_type,
				true,
			);

			// Request data
			assert_noop!(
				Nfi::submit_nfi_data(RawOrigin::Signed(alice()).into(), token_id.clone(), data),
				Error::<Test>::InvalidTokenFormat
			);
		});
	}

	#[test]
	fn submit_data_request_invalid_trn_serial_format_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get(),
				collection_id: GenericCollectionId::U32(123),
				serial_number: GenericSerialNumber::U64(2),
			};
			let data = NFIDataType::NFI(NFIMatrix {
				metadata_link: BoundedVec::truncate_from(b"https://example.com".to_vec()),
				verification_hash: H256::from_low_u64_be(123),
			});

			// Set relayer to alice
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), alice()));

			// Manually Enable NFI
			NfiEnabled::<Test>::insert(
				(ChainId::get(), GenericCollectionId::U32(123)),
				sub_type,
				true,
			);

			// Request data
			assert_noop!(
				Nfi::submit_nfi_data(RawOrigin::Signed(alice()).into(), token_id.clone(), data),
				Error::<Test>::InvalidTokenFormat
			);
		});
	}

	#[test]
	fn submit_data_request_non_trn_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let data = NFIDataType::NFI(NFIMatrix {
				metadata_link: BoundedVec::truncate_from(b"https://example.com".to_vec()),
				verification_hash: H256::from_low_u64_be(123),
			});

			// Set relayer to alice
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), alice()));

			// Request data U64
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get() + 1, // Not TRN chain id will bypass checks
				collection_id: GenericCollectionId::U64(123),
				serial_number: GenericSerialNumber::U64(2),
			};
			assert_ok!(Nfi::submit_nfi_data(
				RawOrigin::Signed(alice()).into(),
				token_id.clone(),
				data.clone()
			));
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataSet {
				sub_type,
				token_id: token_id.clone(),
				data_item: data.clone(),
			}));
			assert_eq!(NfiData::<Test>::get(token_id, sub_type), Some(data.clone()));

			// Request data U128
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get() + 1, // Not TRN chain id will bypass checks
				collection_id: GenericCollectionId::U128(123),
				serial_number: GenericSerialNumber::U128(2),
			};
			assert_ok!(Nfi::submit_nfi_data(
				RawOrigin::Signed(alice()).into(),
				token_id.clone(),
				data.clone()
			));
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataSet {
				sub_type,
				token_id: token_id.clone(),
				data_item: data.clone(),
			}));
			assert_eq!(NfiData::<Test>::get(token_id, sub_type), Some(data.clone()));

			// Request data H160
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get() + 1, // Not TRN chain id will bypass checks
				collection_id: GenericCollectionId::H160(H160::zero()),
				serial_number: GenericSerialNumber::U32(2),
			};
			assert_ok!(Nfi::submit_nfi_data(
				RawOrigin::Signed(alice()).into(),
				token_id.clone(),
				data.clone()
			));
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataSet {
				sub_type,
				token_id: token_id.clone(),
				data_item: data.clone(),
			}));
			assert_eq!(NfiData::<Test>::get(token_id, sub_type), Some(data.clone()));

			// Request data H256
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get() + 1, // Not TRN chain id will bypass checks
				collection_id: GenericCollectionId::H256(H256::zero()),
				serial_number: GenericSerialNumber::U32(2),
			};
			assert_ok!(Nfi::submit_nfi_data(
				RawOrigin::Signed(alice()).into(),
				token_id.clone(),
				data.clone()
			));
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataSet {
				sub_type,
				token_id: token_id.clone(),
				data_item: data.clone(),
			}));
			assert_eq!(NfiData::<Test>::get(token_id, sub_type), Some(data.clone()));

			// Request data Bytes
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get() + 1, // Not TRN chain id will bypass checks
				collection_id: GenericCollectionId::Bytes(BoundedVec::truncate_from(
					b"123".to_vec(),
				)),
				serial_number: GenericSerialNumber::Bytes(BoundedVec::truncate_from(b"2".to_vec())),
			};
			assert_ok!(Nfi::submit_nfi_data(
				RawOrigin::Signed(alice()).into(),
				token_id.clone(),
				data.clone()
			));
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataSet {
				sub_type,
				token_id: token_id.clone(),
				data_item: data.clone(),
			}));
			assert_eq!(NfiData::<Test>::get(token_id, sub_type), Some(data.clone()));

			// Request data Empty
			let token_id = MultiChainTokenId {
				chain_id: ChainId::get() + 1, // Not TRN chain id will bypass checks
				collection_id: GenericCollectionId::Empty,
				serial_number: GenericSerialNumber::U32(2),
			};
			assert_ok!(Nfi::submit_nfi_data(
				RawOrigin::Signed(alice()).into(),
				token_id.clone(),
				data.clone()
			));
			System::assert_last_event(MockEvent::Nfi(Event::<Test>::DataSet {
				sub_type,
				token_id: token_id.clone(),
				data_item: data.clone(),
			}));
			assert_eq!(NfiData::<Test>::get(token_id, sub_type), Some(data.clone()));
		});
	}
}

mod nft_mint {
	use super::*;

	#[test]
	fn mint_nft_requests_data_automatically() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let token_owner = bob();
			let collection_id = NftBuilder::<Test>::new(collection_owner).build();

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			// Mint NFT to token_owner
			assert_ok!(Nft::mint(
				RawOrigin::Signed(collection_owner.clone()).into(),
				collection_id,
				1,
				Some(token_owner),
			));

			let token_id = create_mc_token_id((collection_id, 0));
			// Event thrown
			System::assert_has_event(MockEvent::Nfi(Event::<Test>::DataRequestNew {
				caller: collection_owner,
				sub_type,
				token_id,
			}));
		});
	}

	#[test]
	fn mint_nft_pays_mint_fee() {
		let collection_owner = alice();
		let mint_fee = 400;

		TestExt::<Test>::default()
			.with_balances(&[(collection_owner, mint_fee)])
			.build()
			.execute_with(|| {
				let sub_type = NFISubType::NFI;
				let collection_id = NftBuilder::<Test>::new(collection_owner).build();

				// Enable NFI
				assert_ok!(Nfi::enable_nfi_for_trn_collection(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					sub_type
				));

				// Set fee details to 400 ROOT
				let fee = FeeDetails {
					asset_id: NativeAssetId::get(),
					amount: mint_fee,
					receiver: bob(),
				};
				assert_ok!(Nfi::set_fee_details(
					RawOrigin::Root.into(),
					sub_type,
					Some(fee.clone())
				));

				// Mint NFT
				assert_ok!(Nft::mint(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					1,
					Some(bob()),
				));

				// Check fees paid
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &bob()), mint_fee);
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &collection_owner), 0);
			});
	}

	#[test]
	fn mint_nft_pays_network_fee() {
		let collection_owner = alice();
		let mint_fee = 400;

		TestExt::<Test>::default()
			.with_balances(&[(collection_owner, mint_fee)])
			.build()
			.execute_with(|| {
				let sub_type = NFISubType::NFI;
				let collection_id = NftBuilder::<Test>::new(collection_owner).build();

				// Enable NFI
				assert_ok!(Nfi::enable_nfi_for_trn_collection(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					sub_type
				));

				// Set fee details to 400 ROOT
				let fee = FeeDetails {
					asset_id: NativeAssetId::get(),
					amount: mint_fee,
					receiver: bob(),
				};
				assert_ok!(Nfi::set_fee_details(
					RawOrigin::Root.into(),
					sub_type,
					Some(fee.clone())
				));

				// Set FeeTo address
				let fee_to = charlie();
				assert_ok!(Nfi::set_fee_to(RawOrigin::Root.into(), Some(fee_to.clone())));

				// Mint NFT
				assert_ok!(Nft::mint(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					1,
					Some(bob()),
				));

				// Check fees paid
				let network_fee = NFINetworkFeePercentage::get().mul(mint_fee);
				assert_eq!(
					AssetsExt::balance(NativeAssetId::get(), &bob()),
					mint_fee - network_fee
				);
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_to), network_fee);
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &collection_owner), 0);
			});
	}

	#[test]
	fn mint_nft_insufficient_balance_fails() {
		let collection_owner = alice();
		let mint_fee = 400;

		TestExt::<Test>::default()
			.with_balances(&[(collection_owner, mint_fee - 1)])
			.build()
			.execute_with(|| {
				let sub_type = NFISubType::NFI;
				let collection_id = NftBuilder::<Test>::new(collection_owner).build();

				// Enable NFI
				assert_ok!(Nfi::enable_nfi_for_trn_collection(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					sub_type
				));

				// Set fee details to 400 ROOT
				let fee = FeeDetails {
					asset_id: NativeAssetId::get(),
					amount: mint_fee,
					receiver: bob(),
				};
				assert_ok!(Nfi::set_fee_details(
					RawOrigin::Root.into(),
					sub_type,
					Some(fee.clone())
				));

				// Mint NFT fails due to low balance
				assert_noop!(
					Nft::mint(
						RawOrigin::Signed(collection_owner).into(),
						collection_id,
						1,
						Some(bob())
					),
					ArithmeticError::Underflow
				);
			});
	}
}

mod nft_burn {
	use super::*;

	#[test]
	fn burn_nft_clears_nfi_data() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let token_owner = bob();
			let relayer = charlie();
			let collection_id = NftBuilder::<Test>::new(collection_owner)
				.initial_issuance(2)
				.token_owner(token_owner)
				.build();

			// Set Relayer
			assert_ok!(Nfi::set_relayer(RawOrigin::Root.into(), relayer));

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			let token_id_1 = create_mc_token_id((collection_id, 0));
			let token_id_2 = create_mc_token_id((collection_id, 1));
			// Submit some fake data
			let data = NFIDataType::NFI(NFIMatrix {
				metadata_link: BoundedVec::truncate_from(b"https://example.com".to_vec()),
				verification_hash: H256::from_low_u64_be(123),
			});
			// Data for token we will burn
			assert_ok!(Nfi::submit_nfi_data(
				RawOrigin::Signed(relayer).into(),
				token_id_1.clone(),
				data.clone()
			));
			// Data for token we will keep
			assert_ok!(Nfi::submit_nfi_data(
				RawOrigin::Signed(relayer).into(),
				token_id_2.clone(),
				data.clone()
			));
			assert_eq!(NfiData::<Test>::get(token_id_1.clone(), sub_type), Some(data.clone()));
			assert_eq!(NfiData::<Test>::get(token_id_2.clone(), sub_type), Some(data.clone()));

			// Burn NFT
			assert_ok!(Nft::burn(RawOrigin::Signed(token_owner).into(), (collection_id, 0)));

			// Check event thrown
			System::assert_has_event(MockEvent::Nfi(Event::<Test>::DataRemoved {
				token_id: token_id_1.clone(),
			}));

			// Check data cleared
			assert_eq!(NfiData::<Test>::get(token_id_1, sub_type), None);
			// Data for token we kept should still be there
			assert_eq!(NfiData::<Test>::get(token_id_2, sub_type), Some(data.clone()));
		});
	}
}

mod sft_create_token {
	use super::*;

	#[test]
	fn create_sft_token_requests_data_automatically() {
		TestExt::<Test>::default().build().execute_with(|| {
			let sub_type = NFISubType::NFI;
			let collection_owner = alice();
			let (collection_id, _) = SftBuilder::<Test>::new(collection_owner).build();

			// Enable NFI
			assert_ok!(Nfi::enable_nfi_for_trn_collection(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				sub_type
			));

			// Create new SFT token
			assert_ok!(Sft::create_token(
				RawOrigin::Signed(collection_owner.clone()).into(),
				collection_id,
				BoundedVec::truncate_from(b"SFT Token".to_vec()),
				0,
				None,
				None,
			));

			let token_id = create_mc_token_id((collection_id, 1));
			// Event thrown
			System::assert_has_event(MockEvent::Nfi(Event::<Test>::DataRequestNew {
				caller: collection_owner,
				sub_type,
				token_id,
			}));
		});
	}

	#[test]
	fn create_sft_token_pays_mint_fee() {
		let collection_owner = alice();
		let mint_fee = 400;

		TestExt::<Test>::default()
			.with_balances(&[(collection_owner, mint_fee)])
			.build()
			.execute_with(|| {
				let sub_type = NFISubType::NFI;
				let token_id = SftBuilder::<Test>::new(collection_owner).build();

				// Enable NFI
				assert_ok!(Nfi::enable_nfi_for_trn_collection(
					RawOrigin::Signed(collection_owner).into(),
					token_id.0,
					sub_type
				));

				// Set fee details to 400 ROOT
				let fee = FeeDetails {
					asset_id: NativeAssetId::get(),
					amount: mint_fee,
					receiver: bob(),
				};
				assert_ok!(Nfi::set_fee_details(
					RawOrigin::Root.into(),
					sub_type,
					Some(fee.clone())
				));

				// Create new SFT token
				assert_ok!(Sft::create_token(
					RawOrigin::Signed(collection_owner).into(),
					token_id.0,
					BoundedVec::truncate_from(b"SFT Token".to_vec()),
					0,
					None,
					None,
				));

				// Check fees paid
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &bob()), mint_fee);
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &collection_owner), 0);
			});
	}

	#[test]
	fn create_sft_token_pays_network_fee() {
		let collection_owner = alice();
		let mint_fee = 400;

		TestExt::<Test>::default()
			.with_balances(&[(collection_owner, mint_fee)])
			.build()
			.execute_with(|| {
				let sub_type = NFISubType::NFI;
				let token_id = SftBuilder::<Test>::new(collection_owner).build();

				// Enable NFI
				assert_ok!(Nfi::enable_nfi_for_trn_collection(
					RawOrigin::Signed(collection_owner).into(),
					token_id.0,
					sub_type
				));

				// Set fee details to 400 ROOT
				let fee = FeeDetails {
					asset_id: NativeAssetId::get(),
					amount: mint_fee,
					receiver: bob(),
				};
				assert_ok!(Nfi::set_fee_details(
					RawOrigin::Root.into(),
					sub_type,
					Some(fee.clone())
				));

				// Set FeeTo address
				let fee_to = charlie();
				assert_ok!(Nfi::set_fee_to(RawOrigin::Root.into(), Some(fee_to.clone())));

				// Create new SFT token
				assert_ok!(Sft::create_token(
					RawOrigin::Signed(collection_owner).into(),
					token_id.0,
					BoundedVec::truncate_from(b"SFT Token".to_vec()),
					0,
					None,
					None,
				));

				// Check fees paid
				let network_fee = NFINetworkFeePercentage::get().mul(mint_fee);
				assert_eq!(
					AssetsExt::balance(NativeAssetId::get(), &bob()),
					mint_fee - network_fee
				);
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_to), network_fee);
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &collection_owner), 0);
			});
	}

	#[test]
	fn create_sft_token_insufficient_balance_fails() {
		let collection_owner = alice();
		let mint_fee = 400;

		TestExt::<Test>::default()
			.with_balances(&[(collection_owner, mint_fee - 1)])
			.build()
			.execute_with(|| {
				let sub_type = NFISubType::NFI;
				let token_id = SftBuilder::<Test>::new(collection_owner).build();

				// Enable NFI
				assert_ok!(Nfi::enable_nfi_for_trn_collection(
					RawOrigin::Signed(collection_owner).into(),
					token_id.0,
					sub_type
				));

				// Set fee details to 400 ROOT
				let fee = FeeDetails {
					asset_id: NativeAssetId::get(),
					amount: mint_fee,
					receiver: bob(),
				};
				assert_ok!(Nfi::set_fee_details(
					RawOrigin::Root.into(),
					sub_type,
					Some(fee.clone())
				));

				// Create new SFT token fails due to low balance
				assert_noop!(
					Sft::create_token(
						RawOrigin::Signed(collection_owner).into(),
						token_id.0,
						BoundedVec::truncate_from(b"SFT Token".to_vec()),
						0,
						None,
						None,
					),
					ArithmeticError::Underflow
				);
			});
	}
}
