// Copyright 2020-2021 Parity Technologies (UK) Ltd. and Centrality Investments Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Runtime API definition required by Ethy Gadget

#![cfg_attr(not(feature = "std"), no_std)]

use seed_primitives::ethy::{crypto::AuthorityId, ValidatorSet};
use sp_std::prelude::*;

sp_api::decl_runtime_apis! {
	/// Runtime API for validators.
	pub trait ValidatorSetApi
	{
		/// Return the validator set responsible for Ethereum Bridge(i.e Secp256k1 public keys of the active validator set)
		fn eth_validator_set() -> ValidatorSet<AuthorityId>;
		/// Return the validator set responsible for Xrpl Bridge(i.e Secp256k1 public keys of the active validator set)
		/// This is a subset of active validator set
		fn xrpl_validator_set() -> ValidatorSet<AuthorityId>;
	}
}
