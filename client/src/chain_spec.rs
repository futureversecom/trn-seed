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

use hex_literal::hex;
use pallet_transaction_payment::Multiplier;
use sc_service::ChainType;
use seed_runtime::{
	constants::{
		ONE_ROOT, ONE_XRP, ROOT_ASSET_ID, ROOT_DECIMALS, ROOT_MINIMUM_BALANCE, ROOT_NAME,
		ROOT_SYMBOL, VTX_ASSET_ID, VTX_DECIMALS, VTX_MINIMUM_BALANCE, VTX_NAME, VTX_SYMBOL,
		XRP_ASSET_ID, XRP_DECIMALS, XRP_MINIMUM_BALANCE, XRP_NAME, XRP_SYMBOL,
	},
	keys::*,
	AccountId, AssetsConfig, BabeConfig, Balance, BalancesConfig, CouncilConfig, DemocracyConfig,
	ElectionsConfig, EthBridgeConfig, RuntimeGenesisConfig, SessionConfig, SessionKeys, Signature,
	StakerStatus, StakingConfig, SudoConfig, SystemConfig,
	TransactionPaymentConfig, XRPLBridgeConfig, BABE_GENESIS_EPOCH_CONFIG, WASM_BINARY,
};
use sp_core::{ecdsa, Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig>;

/// Type alias for the stash, controller + session key types tuple used by validators
pub type AuthorityKeys = (AccountId, BabeId, ImOnlineId, GrandpaId, EthBridgeId);

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate a set of runtime session keys (stash/controller, babe, grandpa)
pub fn authority_keys_from_seed(s: &str) -> AuthorityKeys {
	(
		get_account_id_from_seed::<ecdsa::Public>(s),
		get_from_seed::<BabeId>(s),
		get_from_seed::<ImOnlineId>(s),
		get_from_seed::<GrandpaId>(s),
		get_from_seed::<EthBridgeId>(s),
	)
}

pub fn root_config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../../chain-spec/root.json")[..])
}

pub fn porcini_config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../../chain-spec/porcini.json")[..])
}

pub fn dev_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = sc_service::Properties::new();
	properties.insert("tokenSymbol".into(), ROOT_SYMBOL.into());
	properties.insert("tokenDecimals".into(), ROOT_DECIMALS.into());
	Ok(ChainSpec::from_genesis(
		// Name
		"Seed Dev",
		// ID
		"seed_dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice")],
				// Sudo account
				AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<ecdsa::Public>("Alice"),
					get_account_id_from_seed::<ecdsa::Public>("Bob"),
					get_account_id_from_seed::<ecdsa::Public>("Charlie"),
					get_account_id_from_seed::<ecdsa::Public>("Dave"),
					get_account_id_from_seed::<ecdsa::Public>("Eve"),
					get_account_id_from_seed::<ecdsa::Public>("Ferdie"),
					get_account_id_from_seed::<ecdsa::Public>("Alice//stash"),
					get_account_id_from_seed::<ecdsa::Public>("Bob//stash"),
					get_account_id_from_seed::<ecdsa::Public>("Charlie//stash"),
					get_account_id_from_seed::<ecdsa::Public>("Dave//stash"),
					get_account_id_from_seed::<ecdsa::Public>("Eve//stash"),
					get_account_id_from_seed::<ecdsa::Public>("Ferdie//stash"),
					AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")), // Alith
					AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0")), // Baltathar
					AccountId::from(hex!("798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc")), // Charleth
					AccountId::from(hex!("773539d4Ac0e786233D90A233654ccEE26a613D9")), // Dorothy
					AccountId::from(hex!("Ff64d3F6efE2317EE2807d223a0Bdc4c0c49dfDB")), // Ethan
					AccountId::from(hex!("C0F0f4ab324C46e55D02D0033343B4Be8A55532d")), // Faith
				],
				vec![AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"))],
				vec![authority_keys_from_seed("Alice").4],
				false,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		None,
		// Properties
		Some(properties),
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<AuthorityKeys>,
	root_key: AccountId,
	accounts_to_fund: Vec<AccountId>,
	xrp_relayers: Vec<AccountId>,
	xrp_door_signers: Vec<EthBridgeId>,
	_enable_println: bool,
) -> RuntimeGenesisConfig {
	let metadata = vec![
		(
			ROOT_ASSET_ID,
			ROOT_NAME.as_bytes().to_vec(),
			ROOT_SYMBOL.as_bytes().to_vec(),
			ROOT_DECIMALS,
		),
		(XRP_ASSET_ID, XRP_NAME.as_bytes().to_vec(), XRP_SYMBOL.as_bytes().to_vec(), XRP_DECIMALS),
		(VTX_ASSET_ID, VTX_NAME.as_bytes().to_vec(), VTX_SYMBOL.as_bytes().to_vec(), VTX_DECIMALS),
	];
	let assets = vec![
		(ROOT_ASSET_ID, root_key, true, ROOT_MINIMUM_BALANCE),
		(XRP_ASSET_ID, root_key, true, XRP_MINIMUM_BALANCE),
		(VTX_ASSET_ID, root_key, true, VTX_MINIMUM_BALANCE),
	];
	let endowed_accounts = accounts_to_fund.clone();
	let mut endowed_assets = Vec::with_capacity(accounts_to_fund.len());
	let mut endowed_balances = Vec::with_capacity(accounts_to_fund.len());
	for account in accounts_to_fund {
		endowed_assets.push((XRP_ASSET_ID, account, 1_000_000 * ONE_XRP));
		endowed_balances.push((account, 1_000_000 * ONE_ROOT));
	}
	const VALIDATOR_BOND: Balance = 100_000 * ONE_ROOT;
	let multiplier: Multiplier = Multiplier::from_rational(1_u128, 1_000_000_000_u128);
	let election_stake = 100_000 * ONE_ROOT;

	RuntimeGenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			..Default::default()
		},
		babe: BabeConfig {
			authorities: vec![],
			epoch_config: Some(BABE_GENESIS_EPOCH_CONFIG),
			..Default::default()
		},
		balances: BalancesConfig { balances: endowed_balances },
		// babe & grandpa initialization handled by session
		//  otherwise causes: Thread 'main' panicked at 'Authorities are already initialized!'
		assets: AssetsConfig { assets, accounts: endowed_assets, metadata },
		assets_ext: Default::default(),
		grandpa: Default::default(),
		im_online: Default::default(),
		nft: Default::default(),
		marketplace: Default::default(),
		transaction_payment: TransactionPaymentConfig { multiplier, ..Default::default() },
		// NOTE(surangap): keeping xrpl stuff inside the eth bridge isn't elegant. Refactor this to
		// validator-set pallet in the future.
		eth_bridge: EthBridgeConfig { xrp_door_signers },
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.cloned()
				.map(|(acc, babe, im_online, grandpa, ethy)| {
					(
						acc,                                            // validator stash id
						acc,                                            // validator controller id
						SessionKeys { babe, im_online, grandpa, ethy }, // session keys
					)
				})
				.collect(),
		},
		staking: StakingConfig {
			validator_count: 21,
			minimum_validator_count: 1,
			stakers: initial_authorities
				.iter()
				// stash == controller
				.map(|x| (x.0, x.0, VALIDATOR_BOND, StakerStatus::Validator))
				.collect(),
			invulnerables: initial_authorities.iter().map(|x| x.0).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			..Default::default()
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		ethereum: seed_runtime::EthereumConfig { ..Default::default() },
		evm: seed_runtime::EVMConfig { ..Default::default() },
		xrpl_bridge: XRPLBridgeConfig { xrp_relayers },
		council: CouncilConfig::default(),
		elections: ElectionsConfig {
			members: endowed_accounts
				.iter()
				.skip(12)
				.take(6)
				.cloned()
				.map(|member| (member, election_stake))
				.collect(),
		},
		democracy: DemocracyConfig::default(),
	}
}
