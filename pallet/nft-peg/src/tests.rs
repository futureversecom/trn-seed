use crate::{*, mock::*, mock::Event as MockEvent};
use hex_literal::hex;
use frame_support::{traits::Hooks, assert_ok};

use pallet_nft::{CollectionInformation, MetadataScheme};

#[test]
fn event_handler_decodes_correctly() {
    ExtBuilder::default().build().execute_with(|| {

        let source = H160::zero();
        let designated_function = 1;
        let token_address_source = hex!["d9145cce52d386f254917e481eb44e9943f39138"];
        let destination_source = hex!["5b38da6a701c568545dcfcb03fcb875f56beddc4"];
        let token_address = H160::from(token_address_source);
        let destination = H160::from(destination_source);
        let inner_token_id = U256::from(1);
    
        // NFT bridge data encoded
        let data = ethabi::encode(&[
            Token::Uint(U256::from(designated_function)),
            Token::Array(
                vec![Token::Address(token_address)]
            ),
            Token::Array(vec![
                Token::Array(vec![
                    Token::Uint(inner_token_id)
                ])
            ]),
            Token::Address(destination)
        ]);

        assert_ok!(Pallet::<Test>::on_event(&source, &data));
    });
}

#[test]
fn deposit_bridge_events_schedule_a_mint() {
    ExtBuilder::default().build().execute_with(|| {
        let token_address_source = hex!["d9145cce52d386f254917e481eb44e9943f39138"];
        let destination_source = hex!["5b38da6a701c568545dcfcb03fcb875f56beddc4"];

        // Test vals
        let source_address = H160::zero();
        let designated_function = 1;
        let token_address = H160::from(token_address_source);
        let inner_token_id = U256::from(1);
        let destination = H160::from(destination_source);

        let mint_delay_length = 6;

		// NFT bridge data encoded
		let data = ethabi::encode(&[
			Token::Uint(U256::from(designated_function)),
			Token::Array(
                vec![Token::Address(token_address)]
            ),
			Token::Array(vec![
                Token::Array(vec![
                    Token::Uint(inner_token_id)
                ])
            ]),
			Token::Address(destination)
		]);

        // Event is sent
        assert_ok!(Pallet::<Test>::decode_deposit_event(&source_address, &data)); 
        // Mint of bridged tokens are scheduled for some configured point in the future
        assert_eq!(
            DelayedMints::<Test>::contains_key(mint_delay_length),
            true
        );
    })
}

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


        /////
        let token_ids = vec![vec![1]];
    
        
        assert_ok!(Pallet::<Test>::do_withdraw(
            destination,
            vec![collection_id].clone(),
            token_ids,
            source_address
        ));

        let token_ids = BoundedVec::<BoundedVec<U256, MaxIdsPerMultipleMint>, MaxAddresses>::try_from(vec![
            BoundedVec::<U256, MaxIdsPerMultipleMint>::try_from(vec![U256::from(1)]).unwrap()
        ]).unwrap();

        let token_addresses = [collection_id].iter().map(|k| {
            Pallet::<Test>::root_to_eth_nft(k).unwrap()
        }).collect();

        let token_ids:Vec<Vec<U256>> = token_ids.clone().into_iter().map(|i| {
            i.into_inner()
        }).collect();

        System::assert_last_event(MockEvent::NftPeg(crate::Event::EthErc721Withdrawal{
            token_addresses,
            token_ids,
            destination: source_address
        }));




    })
}

// #[test]
// fn withdraw_emits_event() {
//     ExtBuilder::default().build().execute_with(|| {
//         let token_address_source = hex!["d9145cce52d386f254917e481eb44e9943f39138"];
//         let destination_source = hex!["5b38da6a701c568545dcfcb03fcb875f56beddc4"];

//         let source_address = H160::zero();
//         let token_address = H160::from(token_address_source);
//         let destination = H160::from(destination_source);

        // let token_ids = BoundedVec::<BoundedVec<U256, MaxIdsPerMultipleMint>, MaxAddresses>::try_from(vec![
        //     BoundedVec::<U256, MaxIdsPerMultipleMint>::try_from(vec![U256::from(1)]).unwrap()
        // ]).unwrap();

//         let collection_ids = vec![Nft::next_collection_uuid().unwrap()];

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
// 			Token::Uint(U256::from(designated_function)),
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

//         // Simulate a wait period for the mint operation
//         NftPeg::on_initialize(mint_delay_length);

//         assert_ok!(Pallet::<Test>::do_withdraw(
//             destination,
//             collection_ids.clone(),
//             vec![vec![1]],
//             source_address
//         ));

//         let token_addresses = collection_ids.iter().map(|k| {
//             Pallet::<Test>::root_to_eth_nft(k).unwrap()
//         }).collect();

//         let token_ids:Vec<Vec<U256>> = token_ids.clone().into_iter().map(|i| {
//             i.into_inner()
//         }).collect();

//         System::assert_last_event(MockEvent::NftPeg(crate::Event::EthErc721Withdrawal{
//             token_addresses,
//             token_ids,
//             destination: source_address
//         }));


//     })
// }
