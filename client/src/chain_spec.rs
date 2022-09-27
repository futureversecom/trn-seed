use hex_literal::hex;
use sc_service::ChainType;
use seed_runtime::{
	constants::{
		ONE_ROOT, ONE_XRP, ROOT_ASSET_ID, ROOT_DECIMALS, ROOT_MINIMUM_BALANCE, ROOT_NAME,
		ROOT_SYMBOL, XRP_ASSET_ID, XRP_DECIMALS, XRP_MINIMUM_BALANCE, XRP_NAME, XRP_SYMBOL,
	},
	keys::*,
	AccountId, AssetsConfig, BabeConfig, Balance, BalancesConfig, Forcing, GenesisConfig,
	SessionConfig, SessionKeys, Signature, StakerStatus, StakingConfig, SudoConfig, SystemConfig,
	XRPLBridgeConfig, BABE_GENESIS_EPOCH_CONFIG, WASM_BINARY,
};
use sp_core::{ecdsa, Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

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
				vec![AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"))],
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
				vec![AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"))],
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
				vec![AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"))],
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
	initial_authorities: Vec<AuthorityKeys>,
	root_key: AccountId,
	accounts_to_fund: Vec<AccountId>,
	xrp_relayers: Vec<AccountId>,
	_enable_println: bool,
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
		(ROOT_ASSET_ID, root_key, true, ROOT_MINIMUM_BALANCE),
		(XRP_ASSET_ID, root_key, true, XRP_MINIMUM_BALANCE),
	];
	let mut endowed_assets = Vec::with_capacity(accounts_to_fund.len());
	let mut endowed_balances = Vec::with_capacity(accounts_to_fund.len());
	for account in accounts_to_fund {
		endowed_assets.push((ROOT_ASSET_ID, account, 1_000_000 * ONE_ROOT));
		endowed_balances.push((account, 1_000_000 * ONE_XRP));
	}
	const VALIDATOR_BOND: Balance = 100_000 * ONE_ROOT;

	GenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
		},
		babe: BabeConfig { authorities: vec![], epoch_config: Some(BABE_GENESIS_EPOCH_CONFIG) },
		balances: BalancesConfig { balances: endowed_balances },
		// babe & grandpa initialization handled by session
		//  otherwise causes: Thread 'main' panicked at 'Authorities are already initialized!'
		assets: AssetsConfig { assets, accounts: endowed_assets, metadata },
		assets_ext: Default::default(),
		grandpa: Default::default(),
		im_online: Default::default(),
		nft: Default::default(),
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.cloned()
				.map(|(acc, babe, im_online, grandpa, ethy)| {
					(
						acc.clone(),                                    // validator stash id
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
		xrpl_bridge: XRPLBridgeConfig { xrp_relayers },
	}
}
