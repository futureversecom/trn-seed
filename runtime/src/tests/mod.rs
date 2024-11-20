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

//! Integration runtime mock storage
//! Defines mock genesis state for the real seed runtime config

mod evm_fees;
mod evm_gas_costs;
mod evm_tests;
mod maintenance_mode;
mod multiplier;
mod staker_payouts;

use frame_support::traits::{
	fungibles::Inspect as _,
	tokens::{Fortitude, Preservation},
	Get,
};
use sp_core::{
	ecdsa,
	offchain::{testing, OffchainDbExt, OffchainWorkerExt, TransactionPoolExt},
	Encode, Pair,
};
use sp_runtime::{generic::Era, Perbill};

use crate::{
	constants::*, AssetsExt, Balances, CheckedExtrinsic, EVMChainId, FeeControl, Runtime,
	RuntimeOrigin, SessionKeys, SignedExtra, StakerStatus, System, Timestamp, TransactionAction,
	UncheckedExtrinsic, H256, U256,
};
use seed_client::chain_spec::{authority_keys_from_seed, get_account_id_from_seed, AuthorityKeys};
use seed_primitives::{AccountId, AccountId20, Balance, Nonce};
use sp_runtime::BuildStorage;

/// Base gas used for an EVM transaction
pub const BASE_TX_GAS_COST: u128 = 21000;
pub const MINIMUM_XRP_TX_COST: u128 = 157_500;

/// Default gas params in ethers
pub const MAX_PRIORITY_FEE_PER_GAS: u128 = 1_500_000_000;

/// The genesis block timestamp
pub const INIT_TIMESTAMP: u64 = 0;
/// A genesis block hash for the first mock block, useful for extrinsic signatures
pub(crate) const GENESIS_HASH: [u8; 32] = [69u8; 32];
/// The default validator staked amount
pub const VALIDATOR_BOND: Balance = 100_000 * ONE_XRP;
/// The default XRP balance of mock accounts
pub const INITIAL_XRP_BALANCE: Balance = 1_000_000 * ONE_XRP;
/// The default ROOT balance of mock accounts
pub const INITIAL_ROOT_BALANCE: Balance = INITIAL_XRP_BALANCE;

pub struct ExtBuilder {
	/// Extra accounts to initialize with XRP and ROOT balances (default: Alice-Charlie),
	/// authorities are always funded
	accounts_to_fund: Vec<AccountId>,
	// The initial authority set (default: {Alice, Bob})
	initial_authorities: Vec<AuthorityKeys>,
	/// Whether to make stakers invulnerable
	invulnerable: bool,
	/// Initial sudo account
	root_account: AccountId,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		let dev_uris = ["Alice", "Bob", "Charlie"];
		let initial_authorities: Vec<AuthorityKeys> =
			dev_uris.iter().map(|s| authority_keys_from_seed(s)).collect();
		Self {
			// fund Alice-Ferdie
			accounts_to_fund: initial_authorities.iter().map(|x| x.0).collect(),
			// Alice
			root_account: initial_authorities[0].0,
			// Alice & Bob
			initial_authorities: initial_authorities.into_iter().take(2).collect(),
			invulnerable: true,
		}
	}
}

impl ExtBuilder {
	// set invulnerables off (it's on by default)
	#[allow(dead_code)]
	pub fn invulnerables_off(mut self) -> Self {
		self.invulnerable = false;
		self
	}
	#[allow(dead_code)]
	pub fn initial_authorities(mut self, initial_authorities: &[AuthorityKeys]) -> Self {
		self.initial_authorities = initial_authorities.to_vec();
		self
	}
	#[allow(dead_code)]
	pub fn accounts_to_fund(mut self, accounts: &[AccountId]) -> Self {
		self.accounts_to_fund = accounts.to_vec();
		self
	}
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

		// balances + asset setup
		let metadata = vec![
			(
				ROOT_ASSET_ID,
				ROOT_NAME.as_bytes().to_vec(),
				ROOT_SYMBOL.as_bytes().to_vec(),
				ROOT_DECIMALS,
			),
			(
				XRP_ASSET_ID,
				XRP_NAME.as_bytes().to_vec(),
				XRP_SYMBOL.as_bytes().to_vec(),
				XRP_DECIMALS,
			),
		];
		let assets = vec![
			(ROOT_ASSET_ID, self.root_account, true, ROOT_MINIMUM_BALANCE),
			(XRP_ASSET_ID, self.root_account, true, XRP_MINIMUM_BALANCE),
		];

		let stashes: Vec<AccountId> = self.initial_authorities.iter().map(|x| x.0).collect();
		// ensure stashes will be funded too, ignore duplicates
		let mut accounts_to_fund = self.accounts_to_fund.clone();
		for s in stashes.iter() {
			if !accounts_to_fund.iter().any(|acc| acc == s) {
				accounts_to_fund.push(*s);
			}
		}

		let mut endowed_assets = Vec::with_capacity(accounts_to_fund.len());
		let mut endowed_balances = Vec::with_capacity(accounts_to_fund.len());
		for account in accounts_to_fund {
			endowed_balances.push((account, INITIAL_XRP_BALANCE));
			endowed_assets.push((XRP_ASSET_ID, account, INITIAL_ROOT_BALANCE));
		}
		pallet_balances::GenesisConfig::<Runtime> { balances: endowed_balances }
			.assimilate_storage(&mut t)
			.unwrap();

		pallet_assets::GenesisConfig::<Runtime> { assets, accounts: endowed_assets, metadata }
			.assimilate_storage(&mut t)
			.unwrap();

		pallet_sudo::GenesisConfig::<Runtime> { key: Some(self.root_account) }
			.assimilate_storage(&mut t)
			.unwrap();

		// staking setup
		let invulnerables = if self.invulnerable { stashes } else { vec![] };
		pallet_staking::GenesisConfig::<Runtime> {
			minimum_validator_count: 1,
			validator_count: self.initial_authorities.len() as u32,
			stakers: self
				.initial_authorities
				.clone()
				.iter()
				.map(|x| (x.0, x.0, VALIDATOR_BOND, StakerStatus::Validator))
				.collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			invulnerables,
			..Default::default()
		}
		.assimilate_storage(&mut t)
		.unwrap();

		pallet_session::GenesisConfig::<Runtime> {
			keys: self
				.initial_authorities
				.into_iter()
				.map(|(stash, babe, im_online, grandpa, ethy)| {
					(
						stash,
						stash, // use as controller too
						SessionKeys { babe, im_online, grandpa, ethy },
					)
				})
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| {
			// Ensure a test genesis hash exists in storage.
			// This allows signed extrinsics to validate.
			System::set_parent_hash(GENESIS_HASH.into());
			Timestamp::set_timestamp(INIT_TIMESTAMP);
		});

		// OCW setup
		// setup offchain worker for staking election and offence reports
		let (offchain, _state) = testing::TestOffchainExt::new();
		ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
		ext.register_extension(OffchainDbExt::new(offchain));
		let (pool, _state) = testing::TestTransactionPoolExt::new();
		ext.register_extension(TransactionPoolExt::new(pool));

		ext
	}
}

pub fn alice() -> AccountId {
	get_account_id_from_seed::<ecdsa::Public>("Alice")
}

pub fn bob() -> AccountId {
	get_account_id_from_seed::<ecdsa::Public>("Bob")
}

pub fn charlie() -> AccountId {
	get_account_id_from_seed::<ecdsa::Public>("Charlie")
}

/// Constructs transaction `SignedExtra` payload.
pub fn signed_extra(nonce: Nonce, tip: Balance) -> SignedExtra {
	(
		frame_system::CheckNonZeroSender::new(),
		frame_system::CheckSpecVersion::new(),
		frame_system::CheckTxVersion::new(),
		frame_system::CheckGenesis::new(),
		frame_system::CheckEra::from(Era::Immortal),
		frame_system::CheckNonce::from(nonce),
		frame_system::CheckWeight::new(),
		pallet_maintenance_mode::MaintenanceChecker::<Runtime>::new(),
		pallet_transaction_payment::ChargeTransactionPayment::from(tip),
	)
}

// TODO: re-use ethy-keystore for this
/// Sign `xt` with chain metadata and its embedded signer (provided keyring knows about it)
pub fn sign_xt(xt: CheckedExtrinsic) -> UncheckedExtrinsic {
	let func = xt.clone().function;
	match xt.signed {
		fp_self_contained::CheckedSignature::Signed(signed, extra) => {
			let pair = get_pair_from_signer(&signed);
			let raw_payload =
				sp_runtime::generic::SignedPayload::new(xt.function, extra.clone()).unwrap();
			let signature: ecdsa::Signature = raw_payload.using_encoded(|b| {
				// b is SCALE encoded payload or blake2(encoded_payload)
				// Ethereum signature scheme equivalence requires keccak256 hashing all transaction
				// data for signing
				let msg = &sp_io::hashing::keccak_256(b);
				pair.sign_prehashed(msg)
			});

			fp_self_contained::UncheckedExtrinsic::new_signed(func, signed, signature.into(), extra)
		},
		fp_self_contained::CheckedSignature::Unsigned => {
			fp_self_contained::UncheckedExtrinsic::new_unsigned(xt.function)
		},
		_ => unimplemented!(),
	}
}

// quick and dirty get ecdsa keypair matching some known accounts
fn get_pair_from_signer(signer: &AccountId20) -> ecdsa::Pair {
	let alice = alice();
	let bob = bob();
	let charlie = charlie();
	if signer == &alice {
		ecdsa::Pair::from_string("//Alice", None).unwrap()
	} else if signer == &bob {
		ecdsa::Pair::from_string("//Bob", None).unwrap()
	} else if signer == &charlie {
		ecdsa::Pair::from_string("//Charlie", None).unwrap()
	} else {
		unimplemented!("unknown signer, add to keyring");
	}
}

#[test]
fn fund_authorities_and_accounts() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice, Bob, Charlie funded
		assert_eq!(Balances::total_issuance(), INITIAL_ROOT_BALANCE * 3);
		assert_eq!(AssetsExt::total_issuance(XRP_ASSET_ID), INITIAL_XRP_BALANCE * 3);

		assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &alice()), INITIAL_XRP_BALANCE);
		assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &bob()), INITIAL_XRP_BALANCE);
		assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &charlie()), INITIAL_XRP_BALANCE);

		assert_eq!(AssetsExt::balance(ROOT_ASSET_ID, &alice()), INITIAL_ROOT_BALANCE);
		assert_eq!(AssetsExt::balance(ROOT_ASSET_ID, &bob()), INITIAL_ROOT_BALANCE);
		assert_eq!(AssetsExt::balance(ROOT_ASSET_ID, &charlie()), INITIAL_ROOT_BALANCE);

		// Alice, Bob staked
		assert_eq!(
			AssetsExt::reducible_balance(
				ROOT_ASSET_ID,
				&alice(),
				Preservation::Preserve,
				Fortitude::Polite
			),
			INITIAL_ROOT_BALANCE - VALIDATOR_BOND
		);
		assert_eq!(
			AssetsExt::reducible_balance(
				ROOT_ASSET_ID,
				&bob(),
				Preservation::Preserve,
				Fortitude::Polite
			),
			INITIAL_ROOT_BALANCE - VALIDATOR_BOND
		);
	});
}

// Simple Transaction builder
pub struct TxBuilder {
	transaction: ethereum::EIP1559Transaction,
	origin: RuntimeOrigin,
}

impl TxBuilder {
	pub fn default() -> Self {
		let action = ethereum::TransactionAction::Call(bob().into());
		let transaction = ethereum::EIP1559Transaction {
			chain_id: EVMChainId::get(),
			nonce: U256::zero(),
			max_priority_fee_per_gas: U256::zero(),
			max_fee_per_gas: FeeControl::base_fee_per_gas(),
			gas_limit: U256::from(BASE_TX_GAS_COST),
			action,
			value: U256::zero(),
			input: vec![],
			access_list: vec![],
			odd_y_parity: false,
			r: H256::zero(),
			s: H256::zero(),
		};
		let origin =
			RuntimeOrigin::from(pallet_ethereum::RawOrigin::EthereumTransaction(bob().into()));

		Self { transaction, origin }
	}

	pub fn ethers_default_gas() -> Self {
		let action = ethereum::TransactionAction::Call(bob().into());
		let transaction = ethereum::EIP1559Transaction {
			chain_id: EVMChainId::get(),
			nonce: U256::zero(),
			max_priority_fee_per_gas: U256::from(MAX_PRIORITY_FEE_PER_GAS),
			max_fee_per_gas: FeeControl::base_fee_per_gas() * 2
				+ U256::from(MAX_PRIORITY_FEE_PER_GAS),
			gas_limit: U256::from(BASE_TX_GAS_COST),
			action,
			value: U256::zero(),
			input: vec![],
			access_list: vec![],
			odd_y_parity: false,
			r: H256::zero(),
			s: H256::zero(),
		};
		let origin =
			RuntimeOrigin::from(pallet_ethereum::RawOrigin::EthereumTransaction(bob().into()));

		Self { transaction, origin }
	}

	#[allow(dead_code)]
	pub fn action(&mut self, value: TransactionAction) -> &mut Self {
		self.transaction.action = value;
		self
	}

	pub fn origin(&mut self, value: AccountId) -> &mut Self {
		self.origin =
			RuntimeOrigin::from(pallet_ethereum::RawOrigin::EthereumTransaction(value.into()));
		self
	}

	pub fn value(&mut self, value: U256) -> &mut Self {
		self.transaction.value = value;
		self
	}

	pub fn build(&self) -> (RuntimeOrigin, pallet_ethereum::Transaction) {
		let tx = pallet_ethereum::Transaction::EIP1559(self.transaction.clone());
		(self.origin.clone(), tx)
	}
}
