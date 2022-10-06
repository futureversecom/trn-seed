//! Integration tests for staking related payouts

use frame_support::{
	assert_ok,
	dispatch::RawOrigin,
	traits::{fungible::Inspect, Get, OffchainWorker, OnFinalize, OnInitialize},
};
use sp_runtime::traits::Zero;
use sp_staking::{EraIndex, SessionIndex};

use seed_client::chain_spec::authority_keys_from_seed;
use seed_pallet_common::FinalSessionTracker;
use seed_primitives::{Balance, BlockNumber};

use crate::{
	constants::{MILLISECS_PER_BLOCK, ONE_XRP},
	Balances, Call, CheckedExtrinsic, ElectionProviderMultiPhase, EpochDuration, EthBridge,
	Executive, Runtime, Session, SessionKeys, SessionsPerEra, Staking, System, Timestamp, TxFeePot,
	XrpCurrency,
};

use super::{alice, bob, charlie, sign_xt, signed_extra, ExtBuilder, INIT_TIMESTAMP};

// the following helpers are copied from substrate `pallet-staking/src/mock.rs`
/// Progress to the given block, triggering session and era changes as we progress.
///
/// This will finalize the previous block, initialize up to the given block, essentially simulating
/// a block import/propose process where we first initialize the block, then execute some stuff (not
/// in the function), and then finalize the block.
fn run_to_block(n: BlockNumber) {
	println!("call run to block: {:?}", n);
	Staking::on_finalize(System::block_number());
	for b in (System::block_number() + 1)..=n {
		println!(
			"start block: {:?}, era: {:?}, session: {:?}",
			b,
			active_era(),
			Session::current_index()
		);
		System::set_block_number(b);
		Timestamp::set_timestamp(
			INIT_TIMESTAMP + (System::block_number() * MILLISECS_PER_BLOCK as u32) as u64,
		);
		<pallet_babe::CurrentSlot<Runtime>>::put(sp_consensus_babe::Slot::from(b as u64));
		Session::on_initialize(b);
		Staking::on_initialize(b);
		ElectionProviderMultiPhase::on_initialize(b);
		ElectionProviderMultiPhase::offchain_worker(b);
		if b != n {
			Staking::on_finalize(System::block_number());
		}
	}
}

/// Convenient getter for current era aka (scheduled active after session delay)
fn current_era() -> EraIndex {
	Staking::current_era().expect("current era is set")
}

/// Convenient getter for active era
fn active_era() -> EraIndex {
	Staking::active_era().expect("active era is set").index
}

/// Progresses from the current block number (whatever that may be) to the `epoch duration *
/// session_index + 1`.
fn start_session(session_index: SessionIndex) {
	let end = session_index * EpochDuration::get() as u32;
	run_to_block(end);
	// session must have progressed properly.
	assert_eq!(
		Session::current_index(),
		session_index,
		"current session index = {}, expected = {}",
		Session::current_index(),
		session_index,
	);
}

/// Rotate to the next session
fn advance_session() {
	start_session(Session::current_index() + 1)
}

/// Progress until the given era.
fn start_active_era(era_index: EraIndex) {
	start_session((era_index * <SessionsPerEra as Get<u32>>::get()).into());
	assert_eq!(active_era(), era_index);
	// One way or another, current_era must have changed before the active era
	assert_eq!(current_era(), active_era());
}

#[test]
fn era_payout_redistributes_era_tx_fees() {
	ExtBuilder::default().build().execute_with(|| {
		let genesis_root_issuance = Balances::total_issuance();
		let genesis_xrp_issuance = XrpCurrency::total_issuance();
		// send some transactions to accrue fees
		let xt = sign_xt(CheckedExtrinsic {
			signed: fp_self_contained::CheckedSignature::Signed(
				charlie(),
				signed_extra(0, 5 * ONE_XRP),
			),
			function: Call::System(frame_system::Call::remark { remark: b"hello chain".to_vec() }),
		});
		let alice_era0_balance = XrpCurrency::balance(&alice());
		let bob_era0_balance = XrpCurrency::balance(&bob());
		let charlie_initial_balance = XrpCurrency::balance(&charlie());

		// Send transaction from 'Charlie'
		assert_ok!(Executive::apply_extrinsic(xt));

		// Tx fees are taken from the user and added to the 'tx fee pot'
		let tx_fee_pot_era0_balance = TxFeePot::era_pot_balance();
		assert!(
			tx_fee_pot_era0_balance > 0 &&
				XrpCurrency::balance(&charlie()) + tx_fee_pot_era0_balance ==
					charlie_initial_balance
		);
		// after tx fee paid, issuance ok
		assert_eq!(genesis_xrp_issuance, XrpCurrency::total_issuance());
		assert_eq!(genesis_root_issuance, Balances::total_issuance());

		// allocate 50/50 block authoring points to alice & bob in era 0
		Staking::reward_by_ids([(alice(), 50), (bob(), 50)]);
		// end era 0
		start_active_era(1);

		// trigger payout for validator 'Alice' in era 0
		assert_ok!(Staking::payout_stakers(RawOrigin::Signed(alice()).into(), alice(), 0));
		assert_ok!(Staking::payout_stakers(RawOrigin::Signed(bob()).into(), bob(), 0));

		println!("tx pot start era 1 bob payout: {:?}", TxFeePot::era_pot_balance());
		println!("{:?}", XrpCurrency::balance(&alice()));

		assert_eq!(
			alice_era0_balance + tx_fee_pot_era0_balance / 2,
			XrpCurrency::balance(&alice()),
		);
		assert_eq!(bob_era0_balance + tx_fee_pot_era0_balance / 2, XrpCurrency::balance(&bob()),);

		// all rewards claimed
		assert!(TxFeePot::total_pot_balance().is_zero());

		// after payout, issuance ok
		assert_eq!(genesis_xrp_issuance, XrpCurrency::total_issuance());
		assert_eq!(genesis_root_issuance, Balances::total_issuance());
	});
}

#[test]
fn era_payout_does_not_carry_over() {
	ExtBuilder::default().build().execute_with(|| {
		let genesis_root_issuance = Balances::total_issuance();
		let genesis_xrp_issuance = XrpCurrency::total_issuance();

		// run through eras 0, 1, 2, create a tx and accrue fees
		let mut era_payouts = Vec::<Balance>::default();
		for next_era_index in 1_u32..=3 {
			let charlie_nonce = next_era_index - 1; // nonce starts at 0
			let xt = sign_xt(CheckedExtrinsic {
				signed: fp_self_contained::CheckedSignature::Signed(
					charlie(),
					signed_extra(charlie_nonce, 5 * ONE_XRP),
				),
				function: Call::System(frame_system::Call::remark {
					remark: b"hello chain".to_vec(),
				}),
			});
			assert_ok!(Executive::apply_extrinsic(xt));

			era_payouts.push(TxFeePot::era_pot_balance());
			// all block author points to alice
			Staking::reward_by_ids([(alice(), 100)]);
			start_active_era(next_era_index);
		}

		let mut alice_balance = XrpCurrency::balance(&alice());
		for (era_index, era_payout) in era_payouts.iter().enumerate() {
			assert_ok!(Staking::payout_stakers(
				RawOrigin::Signed(alice()).into(),
				alice(),
				era_index as u32
			));
			assert_eq!(alice_balance + era_payout, XrpCurrency::balance(&alice()));
			alice_balance += era_payout;
		}

		// all fees paid out, pot is at zero again
		assert!(TxFeePot::total_pot_balance().is_zero());

		// after payout, issuance ok
		assert_eq!(genesis_root_issuance, Balances::total_issuance());
		assert_eq!(genesis_xrp_issuance, XrpCurrency::total_issuance());
	});
}

#[test]
fn staking_final_session_tracking_ethy() {
	ExtBuilder::default().build().execute_with(|| {
		// session 0,1,2 complete
		start_active_era(1);
		// in session 3
		assert!(!<Runtime as pallet_ethy::Config>::FinalSessionTracker::is_active_session_final());

		advance_session();
		// in session 4
		assert!(!<Runtime as pallet_ethy::Config>::FinalSessionTracker::is_active_session_final());

		// Queue some new keys for alice validator
		let (_, babe, im_online, grandpa, ethy) = authority_keys_from_seed("Alice2.0");
		let new_keys = SessionKeys { babe, grandpa, im_online, ethy };
		assert_ok!(Session::set_keys(RawOrigin::Signed(alice()).into(), new_keys.clone(), vec![]));

		advance_session();
		// in session 5
		assert!(<Runtime as pallet_ethy::Config>::FinalSessionTracker::is_active_session_final());

		advance_session(); // era 2 starts and keys contain the updated key
		assert!(EthBridge::notary_keys().into_iter().find(|x| x == &new_keys.ethy).is_some());

		// Forcing era, marks active session final, sets keys
		let (_, babe, im_online, grandpa, ethy) = authority_keys_from_seed("Alice3.0");
		let new_keys = SessionKeys { babe, grandpa, im_online, ethy };
		assert_ok!(Session::set_keys(RawOrigin::Signed(alice()).into(), new_keys.clone(), vec![]));
		advance_session();
		assert_ok!(Staking::force_new_era(RawOrigin::Root.into()));
		assert!(<Runtime as pallet_ethy::Config>::FinalSessionTracker::is_active_session_final());

		advance_session(); // era 3 starts (forced) and keys contain the updated key
		assert!(EthBridge::notary_keys().into_iter().find(|x| x == &new_keys.ethy).is_some());
	});
}
