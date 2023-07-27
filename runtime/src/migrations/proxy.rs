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

use crate::{impls::ProxyType, migrations::Map, Runtime, Weight};
use frame_support::traits::OnRuntimeUpgrade;
use pallet_futurepass::Holders;
use pallet_proxy::Proxies;
use seed_primitives::AccountId20;

pub struct Upgrade;
impl OnRuntimeUpgrade for Upgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		Ok(v1::pre_upgrade()?)
	}

	fn on_runtime_upgrade() -> Weight {
		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads_writes(2, 0);
		log::info!(target: "Migration", "Starting Proxy migration");
		weight += v1::migrate::<Runtime>();
		log::info!(target: "Migration", "Proxy: Migration successfully finished.");
		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		Ok(v1::post_upgrade()?)
	}
}

#[allow(dead_code)]
pub mod v1 {
	use super::*;
	#[cfg(feature = "try-runtime")]
	use pallet_futurepass::ProxyProvider;

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Proxy: Upgrade to v1 Pre Upgrade.");

		// validate first futurepass account should not have an owner
		if let Some((_owner, first_futurepass)) = Holders::<Runtime>::iter().next() {
			assert_eq!(
				<Runtime as pallet_futurepass::Config>::Proxy::owner(&first_futurepass),
				None,
			);
		}

		Ok(())
	}

	pub fn migrate<T: pallet_proxy::Config + pallet_futurepass::Config>() -> Weight
	where
		<T as frame_system::Config>::AccountId: From<sp_core::H160>,
	{
		let mut weight = 0;

		Map::iter::<Holders<Runtime>, _, _>().iter().for_each(|(owner, fp)| {
			// 2 reads; 1 read for key-value pair in Holders and 1 read for key-value pair in
			// Proxies
			weight += <Runtime as frame_system::Config>::DbWeight::get().reads(2);

			Proxies::<Runtime>::mutate(fp, |(ref mut proxies, _)| {
				for proxy_def in proxies.iter_mut() {
					weight += <Runtime as frame_system::Config>::DbWeight::get().reads(1); // 1 read for each proxies iteration
					if Into::<AccountId20>::into(proxy_def.delegate) == *owner {
						proxy_def.proxy_type = ProxyType::Owner;
						weight += <Runtime as frame_system::Config>::DbWeight::get().writes(1); // 1 write for each proxy_def
						break
					}
				}
			});
		});

		weight
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Proxy: Sanity checking futurepasses");
		Proxies::<Runtime>::iter().for_each(|(fp, (delegates, _))| {
			log::info!(target: "Migration", "Proxy: Sanity checking futurepass: {:?}", fp);
			let owner = delegates
				.iter()
				.filter(|delegate| matches!(delegate.proxy_type, ProxyType::Owner))
				.map(|delegate| delegate.delegate.clone())
				.next();

			if owner == None {
				log::error!(
					"There was an error migrating Proxy delegates: {:?} does not have an owner",
					fp
				);
			}
		});

		log::info!(target: "Migration", "Proxy: Upgrade to v1 Post Upgrade.");
		Ok(())
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;
		use pallet_futurepass::ProxyProvider;
		use pallet_proxy::ProxyDefinition;

		#[test]
		fn migration_test() {
			new_test_ext().execute_with(|| {
				let alice = seed_primitives::AccountId20([1; 20]);
				let alice_futurepass = seed_primitives::AccountId20([255; 20]);

				let bob = seed_primitives::AccountId20([2; 20]);
				let bob_futurepass = seed_primitives::AccountId20([254; 20]);

				pallet_futurepass::Holders::<Runtime>::insert(alice, alice_futurepass);
				pallet_futurepass::Holders::<Runtime>::insert(bob, bob_futurepass);

				pallet_proxy::Proxies::<Runtime>::insert::<_, (sp_runtime::BoundedVec<_, _>, _)>(
					alice_futurepass,
					(
						vec![
							ProxyDefinition {
								delegate: alice,
								proxy_type: ProxyType::Any,
								delay: 0,
							},
							ProxyDefinition { delegate: bob, proxy_type: ProxyType::Any, delay: 0 },
						]
						.try_into()
						.unwrap(),
						0,
					),
				);
				pallet_proxy::Proxies::<Runtime>::insert::<_, (sp_runtime::BoundedVec<_, _>, _)>(
					bob_futurepass,
					(
						vec![
							ProxyDefinition {
								delegate: alice,
								proxy_type: ProxyType::Any,
								delay: 0,
							},
							ProxyDefinition { delegate: bob, proxy_type: ProxyType::Any, delay: 0 },
						]
						.try_into()
						.unwrap(),
						0,
					),
				);

				// validate no owner before upgrade
				assert_eq!(
					<Runtime as pallet_futurepass::Config>::Proxy::owner(&alice_futurepass),
					None,
				);
				assert_eq!(
					<Runtime as pallet_futurepass::Config>::Proxy::owner(&bob_futurepass),
					None,
				);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();

				// validate futurepass ownership after upgrade
				assert_eq!(
					<Runtime as pallet_futurepass::Config>::Proxy::owner(&alice_futurepass)
						.unwrap(),
					alice,
				);
				// validate alice is owner, while bob remains delegate on alice's futurepass proxy
				assert_eq!(
					pallet_proxy::Pallet::<Runtime>::proxies(alice_futurepass)
						.0
						.iter()
						.find(|pd| pd.delegate == alice)
						.unwrap()
						.proxy_type,
					ProxyType::Owner
				);
				assert_eq!(
					pallet_proxy::Pallet::<Runtime>::proxies(alice_futurepass)
						.0
						.iter()
						.find(|pd| pd.delegate == bob)
						.unwrap()
						.proxy_type,
					ProxyType::Any
				);

				// validate bob is owner, while alice remains delegate on bob's futurepass proxy
				assert_eq!(
					<Runtime as pallet_futurepass::Config>::Proxy::owner(&bob_futurepass).unwrap(),
					bob,
				);
				assert_eq!(
					pallet_proxy::Pallet::<Runtime>::proxies(bob_futurepass)
						.0
						.iter()
						.find(|pd| pd.delegate == alice)
						.unwrap()
						.proxy_type,
					ProxyType::Any
				);
				assert_eq!(
					pallet_proxy::Pallet::<Runtime>::proxies(bob_futurepass)
						.0
						.iter()
						.find(|pd| pd.delegate == bob)
						.unwrap()
						.proxy_type,
					ProxyType::Owner
				);
			});
		}
	}
}
