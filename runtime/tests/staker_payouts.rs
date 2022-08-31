//! Integration tests for staking related payouts

use frame_support::{
	assert_ok,
	dispatch::RawOrigin,
	traits::{fungible::Inspect, Get, OffchainWorker, OnFinalize, OnInitialize},
};
use sp_runtime::traits::{AccountIdConversion, Zero};
use sp_staking::{EraIndex, SessionIndex};

use seed_primitives::BlockNumber;
use seed_runtime::{
	constants::{MILLISECS_PER_BLOCK, ONE_XRP},
	Balances, Call, CheckedExtrinsic, ElectionProviderMultiPhase, Executive, Session,
	SessionLength as Period, SessionsPerEra, Staking, System, Timestamp, TxFeePotId,
};

mod mock;
use mock::{alice, bob, charlie, sign_xt, signed_extra, ExtBuilder, INIT_TIMESTAMP};

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
		Session::on_initialize(b);
		Staking::on_initialize(b);
		ElectionProviderMultiPhase::on_initialize(b);
		ElectionProviderMultiPhase::offchain_worker(b);
		Timestamp::set_timestamp(
			INIT_TIMESTAMP + (System::block_number() * MILLISECS_PER_BLOCK as u32) as u64,
		);
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
	let end = session_index * Period::get();
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

/// Progress until the given era.
fn start_active_era(era_index: EraIndex) {
	start_session((era_index * <SessionsPerEra as Get<u32>>::get()).into());
	assert_eq!(active_era(), era_index);
	// One way or another, current_era must have changed before the active era
	assert_eq!(current_era(), active_era());
}

#[test]
fn era_payout_redistributes_era_tx_fees() {
	// setup stakers ✅
	ExtBuilder::default().build().execute_with(|| {
		let genesis_issuance = Balances::total_issuance();
		// send some transactions to accrue fees
		let xt = sign_xt(CheckedExtrinsic {
			signed: fp_self_contained::CheckedSignature::Signed(
				charlie(),
				signed_extra(0, 5 * ONE_XRP),
			),
			function: Call::System(frame_system::Call::remark { remark: b"hello chain".to_vec() }),
		});
		let alice_era0_balance = Balances::balance(&alice());
		let bob_era0_balance = Balances::balance(&bob());
		let charlie_initial_balance = Balances::balance(&charlie());

		// Send transaction from 'Charlie'
		assert_ok!(Executive::apply_extrinsic(xt));

		// Tx fees are taken from the user and added to the 'tx fee pot'
		let tx_fee_pot_era0_balance =
			Balances::balance(&TxFeePotId::get().into_account_truncating());
		assert!(
			tx_fee_pot_era0_balance > 0 &&
				Balances::balance(&charlie()) + tx_fee_pot_era0_balance ==
					charlie_initial_balance
		);
		// after tx fee paid, issuance ok
		assert_eq!(genesis_issuance, Balances::total_issuance());

		// allocate 50/50 block authoring points to alice & bob in era 0
		Staking::reward_by_ids([(alice(), 50), (bob(), 50)]);
		// end era 0
		start_active_era(1);

		// trigger payout for validator 'Alice' in era 0
		assert_ok!(Staking::payout_stakers(RawOrigin::Signed(alice()).into(), alice(), 0));
		assert_ok!(Staking::payout_stakers(RawOrigin::Signed(bob()).into(), bob(), 0));

		assert_eq!(alice_era0_balance + tx_fee_pot_era0_balance / 2, Balances::balance(&alice()),);
		assert_eq!(bob_era0_balance + tx_fee_pot_era0_balance / 2, Balances::balance(&bob()),);

		// all rewards claimed
		assert!(Balances::balance(&TxFeePotId::get().into_account_truncating()).is_zero());

		// after payout, issuance ok
		assert_eq!(genesis_issuance, Balances::total_issuance());
	});
}

#[test]
fn era_payouts_are_independent() {
	// ensure previous era payouts don't affect subsequent eras
}

#[test]
fn applied_slash_stores_stake() {
	// slash validator
	// check stake stored
}
