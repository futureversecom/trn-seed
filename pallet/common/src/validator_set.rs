//! shared pallet types and traits
#![cfg_attr(not(feature = "std"), no_std)]

use seed_primitives::ethy::ValidatorSetId;
use sp_std::{fmt::Debug, vec::Vec};

pub trait ValidatorSetChangeHandler<EthyId> {
	fn validator_set_change_in_progress(info: ValidatorSetChangeInfo<EthyId>);
	fn validator_set_change_finalized(info: ValidatorSetChangeInfo<EthyId>);
}

#[derive(Debug, Clone)]
pub struct ValidatorSetChangeInfo<EthyId> {
	pub current_validator_set_id: ValidatorSetId,
	pub current_validator_set: Vec<EthyId>,
	pub next_validator_set_id: ValidatorSetId,
	pub next_validator_set: Vec<EthyId>,
}

impl<EthyId> Default for ValidatorSetChangeInfo<EthyId> {
	fn default() -> Self {
		ValidatorSetChangeInfo::<EthyId> {
			current_validator_set_id: ValidatorSetId::default(),
			current_validator_set: Default::default(),
			next_validator_set_id: Default::default(),
			next_validator_set: Default::default(),
		}
	}
}

pub trait ValidatorSetInterface<EthyId> {
	fn get_validator_set_id() -> ValidatorSetId;
	fn get_validator_set() -> Vec<EthyId>;
	fn get_next_validator_set() -> Vec<EthyId>;
	fn get_xrpl_validator_set() -> Vec<EthyId>;
	fn get_xrpl_door_signers() -> Vec<EthyId>;
	fn get_xrpl_notary_keys(validator_list: &Vec<EthyId>) -> Vec<EthyId>;
}
