//! shared pallet types and traits
#![cfg_attr(not(feature = "std"), no_std)]

use seed_primitives::ethy::ValidatorSetId;

pub trait ValidatorSetChangeHandler<EthyId> {
    fn validator_set_change_in_progress(info: ValidatorSetChangeInfo<EthyId>);
    fn validator_set_change_finalized(info: ValidatorSetChangeInfo<EthyId>);
}

pub struct ValidatorSetChangeInfo<EthyId> {
    pub current_validator_set_id : ValidatorSetId,
    pub current_validator_set: Vec<EthyId>,
    pub next_validator_set_id: ValidatorSetId,
    pub next_validator_set: Vec<EthyId>,
}

impl<EthyId> Default for ValidatorSetChangeInfo<EthyId> {
    fn default() -> Self {
        ValidatorSetChangeInfo {
            current_validator_set_id: Default::default(),
            current_validator_set: Default::default(),
            next_validator_set_id: Default::default(),
            next_validator_set: Default::default(),
        }
    }
}