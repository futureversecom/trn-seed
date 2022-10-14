use crate::{*, mock::*};
use hex_literal::hex;
use frame_support::{assert_ok, traits::Hooks, assert_err};

use pallet_nft::{CollectionInformation, MetadataScheme};

// #[test]
// fn event_handler_decodes_correctly() {
//     let source = H160::zero();
//     let designated_function = 1;
//     let token_address_source = hex!["d9145cce52d386f254917e481eb44e9943f39138"];
//     let destination_source = hex!["5b38da6a701c568545dcfcb03fcb875f56beddc4"];
//     let token_address = H160::from(token_address_source);
//     let destination = H160::from(destination_source);
//     let inner_token_id = U256::from(1);

//         // NFT bridge data encoded
//     let data = ethabi::encode(&[
//         Token::Uint(U256::from(designated_function)),
//         Token::Array(
//             vec![Token::Address(token_address)]
//         ),
//         Token::Array(vec![
//             Token::Array(vec![
//                 Token::Uint(inner_token_id)
//             ])
//         ]),
//         Token::Address(destination)
//     ]);
//     assert_ok!(Pallet::<Test>::on_event(&source, &data));
// }

// #[test]
// fn decoded_nft_bridge_events_schedule_a_mint() {
//     ExtBuilder::default().build().execute_with(|| {
//         let token_address_source = hex!["d9145cce52d386f254917e481eb44e9943f39138"];
//         let destination_source = hex!["5b38da6a701c568545dcfcb03fcb875f56beddc4"];

//         // Test vals
//         let source_address = H160::zero();
//         let designated_function = 1;
//         let token_address = H160::from(token_address_source);
//         let inner_token_id = U256::from(1);
//         let destination = H160::from(destination_source);

//         let mint_delay_length = 6;

// 		// NFT bridge data encoded
// 		let data = ethabi::encode(&[
// 			// Token::Uint(U256::from(designated_function)),
// 			Token::Array(
//                 vec![Token::Address(token_address)]
//             ),
// 			Token::Array(vec![
//                 Token::Array(vec![
//                     Token::Uint(inner_token_id)
//                 ])
//             ]),
// 			Token::Address(destination)
// 		]);

//         // Event is sent
//         assert_ok!(Pallet::<Test>::decode_deposit_event(&source_address, &data)); 
//         // Mint of bridged tokens are scheduled for some configured point in the future
//         assert_eq!(
//             DelayedMints::<Test>::contains_key(mint_delay_length),
//             true
//         );
//     })
// }

#[test]
fn scheduled_mint_events_create_nfts() {
    ExtBuilder::default().build().execute_with(|| {
        let token_address_source = hex!["d9145cce52d386f254917e481eb44e9943f39138"];
        let destination_source = hex!["5b38da6a701c568545dcfcb03fcb875f56beddc4"];

        // Test vals
        let source_address = H160::zero();
        let designated_function = 1;
        let token_address = H160::from(token_address_source);
        let destination = H160::from(destination_source);
        let empty_name = "".encode();

        let peg_info = PeggedNftInfo::<Test> {
            source: source_address,
            token_addresses: BoundedVec::<H160, MaxAddresses>::try_from(vec![token_address]).unwrap(),
            token_ids: BoundedVec::<BoundedVec<U256, MaxIdsPerMultipleMint>, MaxAddresses>::try_from(vec![
                BoundedVec::<U256, MaxIdsPerMultipleMint>::try_from(vec![U256::from(designated_function)]).unwrap()
            ]).unwrap(),
            destination: destination,
        };

        DelayedMints::<Test>::insert(0,peg_info);

        let collection_id = Nft::next_collection_uuid().unwrap();

        // Simulate a wait period for the mint operation
        NftPeg::on_initialize(0);

        assert_eq!(Nft::next_collection_uuid().unwrap(), 1124);
        assert_eq!(
			Nft::collection_info(collection_id).unwrap(),
			CollectionInformation {
                owner: <Test as pallet_nft::Config>::PalletId::get().into_account_truncating(),
				name: empty_name,
				metadata_scheme: MetadataScheme::Ethereum(H160::zero()),
				royalties_schedule: None,
				max_issuance: None,
				source_chain: OriginChain::Ethereum
			}
		);
    })
}


#[test]
fn scheduled_mint_events_cannot_mint_existing_token() {
    ExtBuilder::default().build().execute_with(|| {

        let source = H160::zero();
        let token_addresses = vec![H160::from(hex!["d9145cce52d386f254917e481eb44e9943f39138"])];
        let token_ids: Vec<BoundedVec<U256, MaxIdsPerMultipleMint>> = vec![BoundedVec::try_from(vec![U256::from(1)]).unwrap()];
        let destination = H160::from(hex!["5b38da6a701c568545dcfcb03fcb875f56beddc4"]);

        assert_ok!(Pallet::<Test>::do_deposit(
            &source,
            BoundedVec::try_from(token_addresses.clone()).unwrap(),
            BoundedVec::try_from(token_ids.clone()).unwrap(),
            destination
        ));

        let new_token_ids: Vec<BoundedVec<U256, MaxIdsPerMultipleMint>> = vec![BoundedVec::try_from(vec![U256::from(2)]).unwrap()];

        assert_ok!(
            Pallet::<Test>::do_deposit(
                &source,
                BoundedVec::try_from(token_addresses.clone()).unwrap(),
                BoundedVec::try_from(new_token_ids.clone()).unwrap(),
                destination
            ));

            // let collection_owner = Pallet::<Test>::mapped_collections(token_addresses[0]).unwrap();
            let collection_owner = token_addresses[0];
            let serial_number = new_token_ids[0][0].as_u32();

        // Attempt to set new values through a mint did not occur
        assert_eq!(Nft::token_owner(100, serial_number), None);
    })
}

