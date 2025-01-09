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

use clap::Parser;
use ethy_gadget::data_to_digest;
use hex::ToHex;
use libsecp256k1::{Message, PublicKey, Signature};
use sc_cli::{Error, SubstrateCli};
use seed_primitives::ethy::EthyChainId;

#[derive(Debug, clap::Subcommand)]
pub enum VerifyProofSigSubCommand {
	/// verify proof signatures for XRPL
	Xrpl(XrplVerifyCommand),
}

impl VerifyProofSigSubCommand {
	pub fn run<C: SubstrateCli>(&self, _cli: &C) -> Result<(), Error> {
		match self {
			VerifyProofSigSubCommand::Xrpl(cmd) => cmd.run(),
		}
	}
}

#[derive(Debug, Clone, Parser)]
pub struct XrplVerifyCommand {
	#[clap(long)]
	pub signature: String,
	#[clap(long)]
	pub public_key: String,
	#[clap(long)]
	pub message: String,
}

impl XrplVerifyCommand {
	pub fn run(&self) -> Result<(), Error> {
		let data = hex::decode(&self.message).expect("Hex decoding failed");
		let signature = hex::decode(&self.signature).expect("Hex decoding failed");
		let pub_key = hex::decode(&self.public_key).expect("Hex decoding failed");

		let digest = data_to_digest(
			EthyChainId::Xrpl,
			data.to_vec(),
			pub_key.clone().try_into().expect("Incorrect Public key"),
		)
		.expect("Data failed to digest");
		println!("\ndigest: {:?}", digest.clone().encode_hex::<String>());

		// verify
		let result = libsecp256k1::verify(
			&Message::parse(&digest),
			&Signature::parse_der(&signature).unwrap(),
			&PublicKey::parse_compressed(&pub_key.try_into().expect("Incorrect Public key"))
				.expect("Failed to parse compressed public key"),
		);

		println!("result: {:?}", result);
		Ok(())
	}
}
