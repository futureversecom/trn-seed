use sc_service::ChainType;
use sp_core::{ecdsa, Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};

use seed_primitives::Balance;
use seed_runtime::{
	constants::{
		MYCL_ASSET_ID, MYCL_DECIMALS, MYCL_MINIMUM_BALANCE, MYCL_NAME, MYCL_SYMBOL, ONE_MYCL,
		ONE_XRP, XRP_ASSET_ID, XRP_DECIMALS, XRP_MINIMUM_BALANCE, XRP_NAME, XRP_SYMBOL,
	},
	keys::*,
	AccountId, AssetsConfig, BalancesConfig, Forcing, GenesisConfig, SessionConfig, SessionKeys,
	Signature, StakerStatus, StakingConfig, SudoConfig, SystemConfig, WASM_BINARY,
};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Type alias for the stash, controller + session key types tuple used by validators
pub type AuthorityKeys = (AccountId, AuraId, ImOnlineId, GrandpaId);

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

/// Generate a set of runtime session keys (stash/controller, aura, grandpa)
pub fn authority_keys_from_seed(s: &str) -> AuthorityKeys {
	(
		get_account_id_from_seed::<ecdsa::Public>(s),
		get_from_seed::<AuraId>(s),
		get_from_seed::<ImOnlineId>(s),
		get_from_seed::<GrandpaId>(s),
	)
}

pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = sc_service::Properties::new();
	properties.insert("tokenSymbol".into(), XRP_SYMBOL.into());
	properties.insert("tokenDecimals".into(), XRP_DECIMALS.into());

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
				get_account_id_from_seed::<ecdsa::Public>("Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<ecdsa::Public>("Alice"),
					get_account_id_from_seed::<ecdsa::Public>("Bob"),
					get_account_id_from_seed::<ecdsa::Public>("Alice//stash"),
					get_account_id_from_seed::<ecdsa::Public>("Bob//stash"),
				],
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

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;
	let mut properties = sc_service::Properties::new();
	properties.insert("tokenSymbol".into(), XRP_SYMBOL.into());
	properties.insert("tokenDecimals".into(), XRP_DECIMALS.into());

	Ok(ChainSpec::from_genesis(
		// Name
		"Seed Local",
		// ID
		"seed_local",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
				// Sudo account
				get_account_id_from_seed::<ecdsa::Public>("Alice"),
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
				],
				false,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,
		Some(properties),
		// Extensions
		None,
	))
}

pub fn porcini_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;
	let mut properties = sc_service::Properties::new();
	properties.insert("tokenSymbol".into(), XRP_SYMBOL.into());
	properties.insert("tokenDecimals".into(), XRP_DECIMALS.into());

	Ok(ChainSpec::from_genesis(
		// Name
		"Porcini",
		// ID
		"porcini",
		ChainType::Live,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![
					authority_keys_from_seed("Alice"),
					authority_keys_from_seed("Bob"),
					authority_keys_from_seed("Charlie"),
					authority_keys_from_seed("Dave"),
					authority_keys_from_seed("Eve"),
				],
				// Sudo account
				get_account_id_from_seed::<ecdsa::Public>("Alice"),
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
				],
				false,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,
		Some(properties),
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AccountId, AuraId, ImOnlineId, GrandpaId)>,
	root_key: AccountId,
	accounts_to_fund: Vec<AccountId>,
	_enable_println: bool,
) -> GenesisConfig {
	let metadata = vec![
		(
			MYCL_ASSET_ID,
			MYCL_NAME.as_bytes().to_vec(),
			MYCL_SYMBOL.as_bytes().to_vec(),
			MYCL_DECIMALS,
		),
		(XRP_ASSET_ID, XRP_NAME.as_bytes().to_vec(), XRP_SYMBOL.as_bytes().to_vec(), XRP_DECIMALS),
	];
	let assets = vec![
		(MYCL_ASSET_ID, root_key, true, MYCL_MINIMUM_BALANCE),
		(XRP_ASSET_ID, root_key, true, XRP_MINIMUM_BALANCE),
	];
	let mut endowed_assets = Vec::with_capacity(accounts_to_fund.len());
	let mut endowed_balances = Vec::with_capacity(accounts_to_fund.len());
	for account in accounts_to_fund {
		endowed_assets.push((MYCL_ASSET_ID, account, 1_000_000 * ONE_MYCL));
		endowed_balances.push((account, 1_000_000 * ONE_XRP));
	}
	const VALIDATOR_BOND: Balance = 100_000 * ONE_MYCL;

	GenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
		},
		balances: BalancesConfig { balances: endowed_balances },
		// aura & grandpa initialization handled by session
		//  otherwise causes: Thread 'main' panicked at 'Authorities are already initialized!'
		aura: Default::default(),
		assets: AssetsConfig { assets, accounts: endowed_assets, metadata },
		assets_ext: Default::default(),
		grandpa: Default::default(),
		im_online: Default::default(),
		nft: Default::default(),
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.cloned()
				.map(|(acc, aura, im_online, grandpa)| {
					(
						acc.clone(),                              // validator stash id
						acc,                                      // validator controller id
						SessionKeys { aura, im_online, grandpa }, // session keys
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
				.map(|x| (x.0.clone(), x.0.clone(), VALIDATOR_BOND, StakerStatus::Validator))
				.collect(),
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			force_era: Forcing::ForceNone,
			slash_reward_fraction: Perbill::from_percent(10),
			..Default::default()
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		base_fee: Default::default(),
		ethereum: seed_runtime::EthereumConfig {},
		evm: seed_runtime::EVMConfig { accounts: Default::default() },
	}
}
