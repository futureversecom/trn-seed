use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{ecdsa, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

use root_primitives::Balance;
use root_runtime::{
	AccountId, AssetsConfig, AuraConfig, BalancesConfig, GenesisConfig, GrandpaConfig,
	SessionConfig, SessionKeys, Signature, SudoConfig, SystemConfig,
	WASM_BINARY,
	constants::{
		MYCL_ASSET_ID, MYCL_DECIMALS, MYCL_MINIMUM_BALANCE, ONE_MYCL, XRP_ASSET_ID, XRP_DECIMALS, XRP_MINIMUM_BALANCE,
	},
};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

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
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AuraId, GrandpaId) {
	(get_account_id_from_seed::<ecdsa::Public>(s), get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
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
				true,
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
		None,
		// Extensions
		None,
	))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
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
				true,
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
		None,
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AccountId, AuraId, GrandpaId)>,
	root_key: AccountId,
	accounts_to_fund: Vec<AccountId>,
	_enable_println: bool,
) -> GenesisConfig {
	let metadata = vec![
		(MYCL_ASSET_ID, b"Mycelium".to_vec(), b"MYCL".to_vec(), MYCL_DECIMALS),
		(XRP_ASSET_ID, b"XRP".to_vec(), b"XRP".to_vec(), XRP_DECIMALS),
	];
	let assets = vec![
		(MYCL_ASSET_ID, root_key, true, MYCL_MINIMUM_BALANCE),
		(XRP_ASSET_ID, root_key, true, XRP_MINIMUM_BALANCE),
	];
	let mut endowed_assets = Vec::with_capacity(accounts_to_fund.len());
	let mut endowed_balances = Vec::with_capacity(accounts_to_fund.len());
	for account in accounts_to_fund {
		endowed_assets.push((
			XRP_ASSET_ID,
			account,
			1_000_000 * 10_u32.pow(XRP_DECIMALS as u32) as Balance,
		));
		endowed_balances.push((account, 1_000_000 * ONE_MYCL));
	}

	GenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
		},
		balances: BalancesConfig { balances: endowed_balances },
		aura: AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone())).collect(),
		},
		assets: AssetsConfig { assets, accounts: endowed_assets, metadata },
		grandpa: GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.2.clone(), 1)).collect(),
		},
		nft: Default::default(),
		session: SessionConfig {
			keys: initial_authorities
				.into_iter()
				.map(|(acc, aura, grandpa)| {
					(
						acc.clone(),                   // validator stash id
						acc,                           // validator controller id
						SessionKeys { aura, grandpa }, // session keys
					)
				})
				.collect(),
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
	}
}
