//! shared pallet types and traits
#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::legacy::byte_sized_error::DispatchError;
use seed_primitives::ethy::{EventProofId, ValidatorSetId};

pub trait ValidatorSetChangeHandler<EthyId> {
    fn validator_set_change_in_progress(info: ValidatorSetChangeInfo<EthyId>);
    fn validator_set_change_finalized(info: ValidatorSetChangeInfo<EthyId>);
}

#[derive(Debug, Clone)]
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

pub trait ValidatorSetInterface<EthyId> {
    fn get_validator_set_id() -> Result<ValidatorSetId, DispatchError>;
    fn get_validator_set() -> Result<Vec<EthyId>, DispatchError>;
    fn get_next_validator_set() -> Result<Vec<EthyId>, DispatchError>;
    fn get_xrpl_validator_set() -> Result<Vec<EthyId>, DispatchError>;
    fn get_xrpl_notary_keys(validator_list: &Vec<EthyId>) -> Result<Vec<EthyId>, DispatchError>;
}