// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use codec::Encode;
use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup, ChainType, Properties};
use seed_runtime::{
	constants::{
		ONE_ROOT, ONE_XRP, ROOT_ASSET_ID, ROOT_DECIMALS, ROOT_MINIMUM_BALANCE, ROOT_NAME,
		ROOT_SYMBOL, XRP_ASSET_ID, XRP_DECIMALS, XRP_MINIMUM_BALANCE, XRP_NAME, XRP_SYMBOL,
	},
	keys::*,
	AccountId, AssetMetadata, AssetRegistryConfig, AssetsConfig, Balance, BalancesConfig,
	CustomMetadata, EthBridgeConfig, GenesisConfig, SessionConfig, SessionKeys, Signature,
	StakerStatus, StakingConfig, SudoConfig, SystemConfig, XRPLBridgeConfig, EXISTENTIAL_DEPOSIT,
};
use serde::{Deserialize, Serialize};
use sp_core::{Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};
use xcm::latest::prelude::*;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
	/// The relay chain of the Parachain.
	pub relay_chain: String,
	/// The id of the Parachain.
	pub para_id: u32,
}

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

#[allow(dead_code)]
type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
#[allow(dead_code)]
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

pub type AuthorityKeys = (AccountId, AuraId, EthBridgeId);
pub fn authority_keys_from_seed(s: &str, account_id: AccountId) -> AuthorityKeys {
	(account_id, get_from_seed::<AuraId>(s), get_from_seed::<EthBridgeId>(s))
}

fn properties() -> Properties {
	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), ROOT_SYMBOL.into());
	properties.insert("tokenDecimals".into(), ROOT_DECIMALS.into());
	properties
}

const PARA_ID: u32 = 2000;

pub fn development_config() -> ChainSpec {
	ChainSpec::from_genesis(
		// Name
		"Seed Dev",
		// ID
		"seed_dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				// Sudo account Alith
				AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")),
				// initial collators
				vec![
					// Alice -> Alith
					authority_keys_from_seed(
						"Alith",
						AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")),
					),
					// Bob -> Baltathar
					authority_keys_from_seed(
						"Baltathar",
						AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0")),
					),
				],
				// Initial PoA authorities
				vec![
					AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")), // Alith
					AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0")), /* Baltathar */
					AccountId::from(hex!("798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc")), /* Charleth */
					AccountId::from(hex!("773539d4Ac0e786233D90A233654ccEE26a613D9")), /* Dorothy */
					AccountId::from(hex!("Ff64d3F6efE2317EE2807d223a0Bdc4c0c49dfDB")), // Ethan
					AccountId::from(hex!("C0F0f4ab324C46e55D02D0033343B4Be8A55532d")), // Faith
				],
				PARA_ID.into(),
				vec![AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"))],
				vec![get_from_seed::<EthBridgeId>("Alith")],
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		None,
		Some(properties()),
		// Extensions
		Extensions {
			relay_chain: "rococo-local".into(), // You MUST set this to the correct network!
			para_id: PARA_ID,
		},
	)
}

pub fn local_testnet_config() -> ChainSpec {
	ChainSpec::from_genesis(
		// Name
		"Seed Parachain Dev",
		// ID
		"seed_parachain_dev",
		ChainType::Local,
		move || {
			testnet_genesis(
				// Sudo account
				AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")),
				// initial collators
				vec![
					// Alice -> Alith
					authority_keys_from_seed(
						"Alice",
						AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")),
					),
					// Bob -> Baltathar
					authority_keys_from_seed(
						"Bob",
						AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0")),
					),
				],
				// Initial PoA authorities
				vec![
					AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")), // Alith
					AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0")), /* Baltathar */
					AccountId::from(hex!("798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc")), /* Charleth */
					AccountId::from(hex!("773539d4Ac0e786233D90A233654ccEE26a613D9")), /* Dorothy */
					AccountId::from(hex!("Ff64d3F6efE2317EE2807d223a0Bdc4c0c49dfDB")), // Ethan
					AccountId::from(hex!("C0F0f4ab324C46e55D02D0033343B4Be8A55532d")), // Faith
				],
				PARA_ID.into(),
				vec![AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"))],
				vec![get_from_seed::<EthBridgeId>("Alice")],
			)
		},
		// Bootnodes
		Vec::new(),
		// Telemetry
		None,
		// Protocol ID
		Some("seed"),
		None,
		Some(properties()),
		// Extensions
		Extensions {
			relay_chain: "rococo-local".into(), // You MUST set this to the correct network!
			para_id: PARA_ID,
		},
	)
}

fn testnet_genesis(
	sudo_key: AccountId,
	invulnerables: Vec<AuthorityKeys>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
	xrp_relayers: Vec<AccountId>,
	xrp_door_signers: Vec<EthBridgeId>,
) -> GenesisConfig {
	let metadata = vec![
		(
			ROOT_ASSET_ID,
			ROOT_NAME.as_bytes().to_vec(),
			ROOT_SYMBOL.as_bytes().to_vec(),
			ROOT_DECIMALS,
		),
		(XRP_ASSET_ID, XRP_NAME.as_bytes().to_vec(), XRP_SYMBOL.as_bytes().to_vec(), XRP_DECIMALS),
	];
	let assets = vec![
		(ROOT_ASSET_ID, sudo_key, true, ROOT_MINIMUM_BALANCE),
		(XRP_ASSET_ID, sudo_key, true, XRP_MINIMUM_BALANCE),
	];
	let mut endowed_assets = Vec::with_capacity(endowed_accounts.len());
	let mut endowed_balances = Vec::with_capacity(endowed_accounts.len());
	for account in endowed_accounts {
		endowed_assets.push((XRP_ASSET_ID, account, 1_000_000 * ONE_XRP));
		endowed_balances.push((account, 1_000_000 * ONE_ROOT));
	}
	const VALIDATOR_BOND: Balance = 100_000 * ONE_ROOT;

	GenesisConfig {
		// System
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: seed_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(sudo_key),
		},
		//  otherwise causes: Thread 'main' panicked at 'Authorities are already initialized!'
		assets: AssetsConfig { assets, accounts: endowed_assets, metadata },
		// Monetary
		balances: BalancesConfig { balances: endowed_balances },
		// transaction_payment: Default::default(), //TODO: fix genesis config issue

		// Consensus
		aura: Default::default(),
		aura_ext: Default::default(),
		assets_ext: Default::default(),
		nft: Default::default(),
		// NOTE(surangap): keeping xrpl stuff inside the eth bridge isn't elegant. Refactor this to
		// validator-set pallet in the future.
		eth_bridge: EthBridgeConfig { xrp_door_signers },
		parachain_info: seed_runtime::ParachainInfoConfig { parachain_id: id },
		collator_selection: seed_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _, _)| acc).collect(),
			candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
			..Default::default()
		},
		session: SessionConfig {
			keys: invulnerables
				.iter()
				.cloned()
				.map(|(acc, aura, _ethy)| {
					(
						acc.clone(), // validator stash id
						acc,         // validator controller id
						// SessionKeys { aura, ethy }, // session keys
						SessionKeys { aura }, // session keys
					)
				})
				.collect(),
		},
		parachain_system: Default::default(),
		polkadot_xcm: seed_runtime::PolkadotXcmConfig { safe_xcm_version: Some(SAFE_XCM_VERSION) },
		// transaction_payment: Default::default(),
		staking: StakingConfig {
			validator_count: 21,
			minimum_validator_count: 1,
			stakers: invulnerables
				.iter()
				// stash == controller
				.map(|x| (x.0.clone(), x.0.clone(), VALIDATOR_BOND, StakerStatus::Validator))
				.collect(),
			invulnerables: invulnerables.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			..Default::default()
		},
		ethereum: seed_runtime::EthereumConfig {},
		evm: seed_runtime::EVMConfig { accounts: Default::default() },
		xrpl_bridge: XRPLBridgeConfig { xrp_relayers },
		asset_registry: AssetRegistryConfig {
			assets: vec![(
				XRP_ASSET_ID,
				(AssetMetadata {
					decimals: 12,
					name: "XRP".as_bytes().to_vec(),
					symbol: "XRP".as_bytes().to_vec(),
					existential_deposit: 1,
					location: Some(
						MultiLocation {
							parents: 1,
							interior: X2(Parachain(2000), GeneralIndex(2)),
						}
						.into_versioned(),
					),
					additional: CustomMetadata { fee_per_second: 1 },
				})
				.encode(),
			)],
			last_asset_id: 2,
		},
	}
}
