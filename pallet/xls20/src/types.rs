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

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use seed_primitives::xrpl::Xls20TokenId;
use sp_core::H160;
use sp_runtime::Permill;
use sp_std::prelude::*;

#[derive(Debug, PartialEq)]
pub struct Xls20Token {
	pub flags: u16,
	pub transfer_fee: Permill,
	pub issuer: H160,
	pub taxon: u32,
	pub sequence: u32,
}

impl Xls20Token {
	/// Check if the token is burnable by comparing the flag
	/// https://xrpl.org/docs/references/protocol/data-types/nftoken#nftoken-flags
	pub fn is_burnable(&self) -> bool {
		self.flags & 0x0001 == 1
	}

	/// Unscramble the taxon using the sequence as the seed
	fn unscramble_taxon(taxon: u32, sequence: u32) -> u32 {
		const SEED: u64 = 384160001;
		const INCREMENT: u64 = 2459;
		const MAX: u64 = 4294967296;

		// perform scrambling calculations, there will be no overflow as max is u64
		// Max value would be u32::MAX * 384160001 which is less than u64::MAX
		// https://xrpl.org/docs/references/protocol/data-types/nftoken
		let mut scramble = SEED.saturating_mul(sequence as u64) % MAX;
		scramble = scramble.saturating_add(INCREMENT) % MAX;

		taxon ^ scramble as u32
	}
}

impl From<Xls20TokenId> for Xls20Token {
	fn from(xls20_token: Xls20TokenId) -> Self {
		//  000B 0C44 95F14B0E44F78A264E41713C64B5F89242540EE2 BC8B858E 00000D65
		// 	+--- +--- +--------------------------------------- +------- +-------
		// 	|    |    |                                        |        |
		// 	|    |    |                                        |        `---> Sequence: 3,429
		// 	|    |    |                                        |
		//  |    |    |                                        `---> Taxon: 146,999,694
		// 	|    |    |
		// 	|    |    `---> Issuer: rNCFjv8Ek5oDrNiMJ3pw6eLLFtMjZLJnf2
		// 	|    |
		//  |    `---> TransferFee: 314.0 bps or 3.140%
		// 	|
		//  `---> Flags: 12 -> lsfBurnable, lsfOnlyXRP and lsfTransferable

		let flags: u16 = u16::from_be_bytes(xls20_token[0..2].try_into().unwrap());
		let transfer_fee = u16::from_be_bytes(xls20_token[2..4].try_into().unwrap()) as u32;
		let transfer_fee = Permill::from_rational(transfer_fee, 100_000);
		let issuer_bytes: [u8; 20] = xls20_token[4..24].try_into().unwrap();
		let issuer: H160 = H160::from_slice(&issuer_bytes);
		let scrambled_taxon: u32 = u32::from_be_bytes(xls20_token[24..28].try_into().unwrap());
		let sequence: u32 = u32::from_be_bytes(xls20_token[28..].try_into().unwrap());
		let taxon = Self::unscramble_taxon(scrambled_taxon, sequence);
		let token = Xls20Token { flags, transfer_fee, issuer, taxon, sequence };
		token
	}
}

/// A collection mapping is a tuple of (issuer_address, taxon)
#[derive(Clone, Copy, MaxEncodedLen, PartialEq, Eq, Debug, Encode, Decode, TypeInfo)]
pub struct Xls20Collection {
	issuer_address: H160,
	taxon: u32,
}

impl Xls20Collection {
	pub fn new(issuer_address: H160, taxon: u32) -> Self {
		Self { issuer_address, taxon }
	}
}
