use clap::Parser;
use ethy_gadget::{get_digest, verify_secp256k1_signature};
use hex::ToHex;
use sc_cli::{Error, SubstrateCli};
use seed_primitives::ethy::EthyChainId;

#[derive(Debug, clap::Subcommand)]
pub enum VerifyProofSigSubCommand {
	/// verify proof signatures for XRPL
	Xrpl(XrplVefiryCommand),
}

impl VerifyProofSigSubCommand {
	pub fn run<C: SubstrateCli>(&self, _cli: &C) -> Result<(), Error> {
		match self {
			VerifyProofSigSubCommand::Xrpl(cmd) => cmd.run(),
		}
	}
}

#[derive(Debug, Clone, Parser)]
pub struct XrplVefiryCommand {
	#[clap(long)]
	pub signature: String,
	#[clap(long)]
	pub public_key: String,
	#[clap(long)]
	pub message: String,
}

impl XrplVefiryCommand {
	pub fn run(&self) -> Result<(), Error> {
		let data = hex::decode(&self.message).expect("Hex decoding failed");
		let signature = hex::decode(&self.signature).expect("Hex decoding failed");
		let pubkey = hex::decode(&self.public_key).expect("Hex decoding failed");

		let digest = get_digest(
			EthyChainId::Xrpl,
			data.to_vec(),
			pubkey.clone().try_into().expect("Incorrect Public key"),
		)
		.unwrap();
		println!("\ndigest: {:?}", digest.clone().encode_hex::<String>());

		// verify
		let result = verify_secp256k1_signature(
			signature,
			pubkey.try_into().expect("Incorrect Signature"),
			digest,
		);
		println!("result: {:?}", result);

		Ok(())
	}
}
