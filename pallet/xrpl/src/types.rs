use alloc::{string::String, vec::Vec};
use codec::{Decode, Encode, MaxEncodedLen};
use libsecp256k1::Message;
use scale_info::TypeInfo;
use seed_primitives::AccountId20;
use sha2::Digest;
use sp_core::{ecdsa, ed25519, H160};
use sp_runtime::traits::Verify;
use xrpl_binary_codec::{
	deserializer::Deserializer,
	serializer::{field_id::TypeCode, field_info::field_info_lookup, Serializer},
};
use xrpl_types::{serialize::Serialize, types::TransactionCommon};

/// The memo type data to be hex encoded in XRPL transaction for extrinsic calls.
pub const MEMO_TYPE_EXTRINSIC: &str = "extrinsic";

/// Length of half a sha512 hash.
pub const SHA512_HASH_LENGTH: usize = 32;

/// The extracted extrinsic data from the XRPL transaction memos list.
#[derive(Debug)]
pub struct ExtrinsicMemoData {
	pub genesis_hash: [u8; 32],
	pub nonce: u32,
	pub max_block_number: u32,
	pub tip: u64,
	pub hashed_call: [u8; 32],
}

/// The prefix to be prepended to the signed message, for signature verification.
/// This is hex decoded string of the value `0x53545800`
pub const SIGNED_MESSAGE_PREFIX: [u8; 4] = [83, 84, 88, 0];

pub fn encode_for_signing(tx_common: &TransactionCommon) -> Result<Vec<u8>, &'static str> {
	let mut serializer = Serializer::new();
	tx_common
		.serialize(&mut serializer)
		.map_err(|_| "failed to serialize TransactionCommon")?;

	// re-construct serialized signed message with prefix
	let mut serialized_msg =
		serializer.into_bytes().map_err(|_| "failed to convert serializer to bytes")?;
	let mut message_with_prefix =
		Vec::with_capacity(SIGNED_MESSAGE_PREFIX.len() + serialized_msg.len());
	message_with_prefix.extend_from_slice(&SIGNED_MESSAGE_PREFIX);
	message_with_prefix.append(&mut serialized_msg);

	Ok(message_with_prefix)
}

/// Supported XRPL public key types:
/// https://xrpl.org/docs/concepts/accounts/cryptographic-keys#signing-algorithms
#[derive(Debug, Clone, Decode, Encode, MaxEncodedLen, PartialEq, TypeInfo)]
pub enum XrplPublicKey {
	ED25519(ed25519::Public),
	ECDSA(ecdsa::Public),
}

impl AsRef<[u8]> for XrplPublicKey {
	fn as_ref(&self) -> &[u8] {
		match self {
			XrplPublicKey::ED25519(public) => public.0.as_ref(),
			XrplPublicKey::ECDSA(public) => public.0.as_ref(),
		}
	}
}

impl TryInto<AccountId20> for XrplPublicKey {
	type Error = &'static str;
	fn try_into(self) -> Result<AccountId20, Self::Error> {
		match self {
			XrplPublicKey::ED25519(public) => Ok(public.try_into()?),
			XrplPublicKey::ECDSA(public) => Ok(public.try_into()?),
		}
	}
}

/// First half of sha512 hash of a message
pub fn sha512_first_half(message: &[u8]) -> [u8; SHA512_HASH_LENGTH] {
	let mut sha512 = sha2::Sha512::new();
	sha512.update(message);
	sha512.finalize()[..SHA512_HASH_LENGTH]
		.try_into()
		.expect("Incorrect byte length")
}

/// XRPL transaction memo field
/// Derived from: https://xrpl.org/transaction-common-fields.html#memos-field
#[derive(Debug, PartialEq, serde::Deserialize)]
pub struct Memo {
	#[serde(rename = "MemoData")]
	pub memo_data: String,
	#[serde(rename = "MemoType")]
	pub memo_type: String,
}

/// XRPL transaction memo field wrapper
#[derive(Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MemoElmRaw {
	pub memo: Memo,
}

/// XRPL transaction minimal fields required for verifying extrinsic and embedding memo data
/// Derived from: https://xrpl.org/transaction-common-fields.html#transaction-common-fields
#[derive(Debug, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct XRPLTransaction {
	pub account: String,
	#[serde(rename = "AccountTxnID")]
	pub account_txn_id: String,
	pub memos: Vec<MemoElmRaw>,
	#[serde(rename = "SigningPubKey")]
	pub signing_pub_key: String,
}
impl XRPLTransaction {
	/// Retrieves the public key from the `signing_pub_key` in the XRPL transaction.
	pub fn get_public_key(&self) -> Result<XrplPublicKey, &'static str> {
		let pub_key =
			hex::decode(&self.signing_pub_key).map_err(|_| "Error decoding hex string")?;
		// check prefix (ignore-case "ED") to determine the public key type
		// uppercase first 2 characters and compare with "ED"
		let key_prefix = &self.signing_pub_key[0..2].to_uppercase();
		match key_prefix.as_str() {
			"ED" => {
				// remove the first byte (prefix) from the public key
				let pub_key: [u8; 32] =
					pub_key[1..].try_into().map_err(|_| "Invalid length of decoded bytes")?;
				let public = ed25519::Public::from_raw(pub_key);
				Ok(XrplPublicKey::ED25519(public))
			},
			"02" | "03" => {
				let pub_key: [u8; 33] =
					pub_key.try_into().map_err(|_| "Invalid length of decoded bytes")?;
				let public = ecdsa::Public::from_raw(pub_key);
				Ok(XrplPublicKey::ECDSA(public))
			},
			_ => Err("Unsupported public key type"),
		}
	}

	/// Derives the account (H160) from the `signing_pub_key` in the XRPL transaction.
	pub fn get_account(&self) -> Result<H160, &'static str> {
		let public = self.get_public_key()?;
		let account: AccountId20 = public.try_into()?;
		Ok(account.into())
	}

	/// Extracts the extrinsic data from the memos in the XRPL transaction.
	/// Finds the memos in the list where the memo type is `extrinsic`; and
	/// extracts the `nonce`, `max_block_number` and call from the memo data.
	/// The memo data is hex encoded with the following format (string):
	/// `genesis_hash<[u8;32]>:nonce<u32>:max_block_number<u32>:tip<u64>:hashed_call<[u8;32]>`
	pub fn get_extrinsic_data(&self) -> Result<ExtrinsicMemoData, &'static str> {
		for memo_elm in &self.memos {
			// convert `memo_elm.memo.memo_type` hex string to utf8 string for comparison
			let hex_decoded_type = hex::decode(&memo_elm.memo.memo_type)
				.map_err(|_| "failed to decode memo_type as hex")?;
			let memo_type = String::from_utf8(hex_decoded_type)
				.map_err(|_| "failed to convert memo_type to utf8")?;
			if memo_type.eq_ignore_ascii_case(MEMO_TYPE_EXTRINSIC) {
				let hex_decoded_data = hex::decode(&memo_elm.memo.memo_data)
					.map_err(|_| "failed to decode memo_data as hex")?;
				let hex_decoded_data_str = String::from_utf8(hex_decoded_data)
					.map_err(|_| "failed to convert memo_data to utf8")?;
				// split string by `:`, parse each string to extract chainId, nonce,
				// max_block_number, tip and call data
				let mut split = hex_decoded_data_str.split(":");
				let genesis_hash = split.next().ok_or("failed to get chain_id from memo_data")?;
				let genesis_hash = hex::decode(genesis_hash)
					.map_err(|_| "failed to decode genesis_hash as hex")?
					.try_into()
					.map_err(|_| "failed to convert genesis_hash to 32 bytes")?;
				let nonce = split
					.next()
					.ok_or("failed to get nonce from memo_data")?
					.parse::<u32>()
					.map_err(|_| "failed to parse string as u32")?;
				let max_block_number = split
					.next()
					.ok_or("failed to get max_block_number from memo_data")?
					.parse::<u32>()
					.map_err(|_| "failed to parse string as u32")?;
				let tip = split
					.next()
					.ok_or("failed to get tip from memo_data")?
					.parse::<u64>()
					.map_err(|_| "failed to parse string as u64")?;
				let hashed_call = split.next().ok_or("failed to get hashed_call from memo_data")?;
				let hashed_call: [u8; 32] = hex::decode(hashed_call)
					.map_err(|_| "failed to decode hashed_call as hex")?
					.try_into()
					.map_err(|_| "failed to convert hashed_call to 32 bytes")?;
				return Ok(ExtrinsicMemoData {
					genesis_hash,
					nonce,
					max_block_number,
					tip,
					hashed_call,
				});
			}
		}
		Err("no extrinsic call found in memos")
	}

	/// Converts the `XRPLTransaction` (self) to `TransactionCommon` (from SDK), re-serializes and
	/// encodes the transaction with hex prefix to be verified by `libsecp256k1`, given the
	/// signature.
	pub fn verify_transaction(&self, signature: &[u8]) -> Result<bool, &'static str> {
		let tx_common: TransactionCommon = self.try_into()?;
		let encoded_message = encode_for_signing(&tx_common)?;

		let pub_key = self.get_public_key()?;
		match pub_key {
			XrplPublicKey::ED25519(public) => {
				let signature = ed25519::Signature::from_raw(
					signature.try_into().map_err(|_| "Invalid length of decoded bytes")?,
				);
				let success = signature.verify(&*encoded_message, &public);
				Ok(success)
			},
			XrplPublicKey::ECDSA(public) => {
				let hashed_msg: Message =
					libsecp256k1::Message::parse(&sha512_first_half(&encoded_message));
				let pub_key =
					libsecp256k1::PublicKey::parse_compressed(&public.0).map_err(|e| {
						log::warn!("⛔️ failed to parse public key: {:?}", e);
						"failed to parse public key"
					})?;
				let signature = libsecp256k1::Signature::parse_der(signature).map_err(|e| {
					log::warn!("⛔️ failed to parse signature: {:?}", e);
					"failed to parse signature"
				})?;
				let success = libsecp256k1::verify(&hashed_msg, &signature, &pub_key);
				Ok(success)
			},
		}
	}
}
impl TryFrom<&[u8]> for XRPLTransaction {
	type Error = &'static str;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		let deserializer = &mut Deserializer::new(value.to_vec(), field_info_lookup());
		let tx_json = deserializer
			.to_json(&TypeCode::Object, value)
			.map_err(|_e| "failed to convert encoded_msg to json value")?;
		let tx: XRPLTransaction = serde_json::from_value(tx_json)
			.map_err(|_e| "failed to deserialize json value of encoded_msg")?;
		Ok(tx)
	}
}
impl TryInto<TransactionCommon> for &XRPLTransaction {
	type Error = &'static str;

	fn try_into(self) -> Result<TransactionCommon, Self::Error> {
		let memos = self
			.memos
			.iter()
			.map(|MemoElmRaw { memo: Memo { memo_data, memo_type } }| {
				Ok::<xrpl_types::Memo, &str>(xrpl_types::Memo {
					memo_data: xrpl_types::Blob::from_hex(memo_data)
						.map_err(|_| "failed to convert memo_data to Blob")?,
					memo_type: xrpl_types::Blob::from_hex(memo_type)
						.map_err(|_| "failed to convert memo_type to Blob")?,
					memo_format: None,
				})
			})
			.collect::<Result<Vec<_>, _>>()?;

		Ok(TransactionCommon {
			account_txn_id: Some(
				xrpl_types::Hash256::from_hex(&self.account_txn_id)
					.map_err(|_| "failed to convert account_txn_id to Hash256")?,
			),
			signing_pub_key: Some(
				xrpl_types::Blob::from_hex(&self.signing_pub_key)
					.map_err(|_| "failed to convert signing_pub_key to PublicKey")?,
			),
			account: xrpl_types::AccountId::from_address(&self.account)
				.map_err(|_| "failed to convert account to AccountId")?,
			memos,
			..Default::default()
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::assert_ok;
	use sp_core::hexdisplay::AsBytesRef;

	#[test]
	fn decode_xrpl_transaction_blob() {
		let tx_blob = "5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A74473045022100FB7583772B8F348F4789620C5571146B6517887AC231B38E29D7688D73F9D2510220615DC87698A2BA64DF2CA83BD9A214002F74C2D615CA20E328AC4AB5E4CDE8BC811424A53BB5CAAD40A961836FEF648E8424846EC75AF9EA7C1F687474703A2F2F6578616D706C652E636F6D2F6D656D6F2F67656E657269637D0472656E74E1F1";
		let tx_blob_bytes = hex::decode(tx_blob).unwrap();
		let tx_want = XRPLTransaction {
			account_txn_id: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580"
				.into(),
			signing_pub_key: "02A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A"
				.into(),
			account: "rhLmGWkHr59h9ffYgPEAqZnqiQZMGb71yo".into(),
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: "687474703A2F2F6578616D706C652E636F6D2F6D656D6F2F67656E65726963"
						.into(),
					memo_data: "72656E74".into(),
				},
			}],
		};
		let tx_got = XRPLTransaction::try_from(tx_blob_bytes.as_bytes_ref()).unwrap();
		assert_eq!(tx_want, tx_got);
	}

	#[test]
	fn verification_ed25519() {
		let signature = "352ED4ABB4A9D5D2AAD34BF4DBDEB788F880FAD473952BDAABBBFDCFDD57075281DAB12E6BEE18E0873AD5C276753869CC6A4A40606438E7D903C72497E9F708";
		let tx = XRPLTransaction {
			account_txn_id: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580"
				.into(),
			signing_pub_key: "edfb2a3a850b43e24d2700532ef1f9ccb2475dff4f62b634b0c58845f23c263965"
				.into(),
			account: "r3PkESDrGaZHHPNLzJP1Uhki1yq94XTBSr".into(),
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: "65787472696e736963"
						.into(),
					memo_data: "636438353462323135363766646531636362306466306532313035306436633934633131313534373134616230313263623638353334376235666535643434393a303a313230373a303a35633933633236383339613137636235616366323765383961616330306639646433663531643161316161346234383266363930663634333633396665383732".into(),
				},
			}],
		};

		// validate encoded message matches with the expected
		let tx_common: TransactionCommon = (&tx).try_into().unwrap();
		let encoded_message = encode_for_signing(&tx_common).unwrap();
		let prefix = hex::encode(SIGNED_MESSAGE_PREFIX).to_uppercase();
		assert_eq!(
			prefix + "5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C25807321EDFB2A3A850B43E24D2700532EF1F9CCB2475DFF4F62B634B0C58845F23C26396581145116224CEF7355137BEBBA8E277A9BE18E0596E7F9EA7C0965787472696E7369637D8A636438353462323135363766646531636362306466306532313035306436633934633131313534373134616230313263623638353334376235666535643434393A303A313230373A303A35633933633236383339613137636235616366323765383961616330306639646433663531643161316161346234383266363930663634333633396665383732E1F1",
			hex::encode(encoded_message).to_uppercase(),
		);

		// verify
		let result = tx.verify_transaction(&hex::decode(signature).unwrap());
		assert_ok!(result);
		assert!(result.unwrap());
	}

	#[test]
	fn verification_ecdsa() {
		let signature = "304402202C747A5F169432FADD15C48D56FAD7AC05C06441FC56B20ABDB1A0984429224602200969D2FFF2D39986CFBAD81842B83FCACBEF8EB89005C6EB22C07F238A3586E7";
		let tx = XRPLTransaction {
			account_txn_id: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580"
				.into(),
			signing_pub_key: "035c080e3218faef37ffd21f7cd2eff0e574d0fdce703e15a590ae84de42c5bb5f"
				.into(),
			account: "rnJjAa6uJqwkfztFeyxTvxj1NdsCyCNuD7".into(),
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: "65787472696e736963"
						.into(),
					memo_data: "636438353462323135363766646531636362306466306532313035306436633934633131313534373134616230313263623638353334376235666535643434393a303a313033313a303a35633933633236383339613137636235616366323765383961616330306639646433663531643161316161346234383266363930663634333633396665383732".into(),
				},
			}],
		};

		// validate encoded message matches with the expected
		let tx_common: TransactionCommon = (&tx).try_into().unwrap();
		let encoded_message = encode_for_signing(&tx_common).unwrap();
		let prefix = hex::encode(SIGNED_MESSAGE_PREFIX).to_uppercase();
		assert_eq!(
			prefix + "5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C25807321035C080E3218FAEF37FFD21F7CD2EFF0E574D0FDCE703E15A590AE84DE42C5BB5F81142F3B69B564A3465ACF3D164E66FD642EA7A41E51F9EA7C0965787472696E7369637D8A636438353462323135363766646531636362306466306532313035306436633934633131313534373134616230313263623638353334376235666535643434393A303A313033313A303A35633933633236383339613137636235616366323765383961616330306639646433663531643161316161346234383266363930663634333633396665383732E1F1",
			hex::encode(encoded_message).to_uppercase(),
		);

		// verify
		let result = tx.verify_transaction(&hex::decode(signature).unwrap());
		assert_ok!(result);
		assert!(result.unwrap());
	}

	#[test]
	fn xrpl_invalid_public_key() {
		let tx = XRPLTransaction {
			signing_pub_key: "some unsupported public key".into(),
			..Default::default()
		};
		assert_eq!("Error decoding hex string", tx.get_public_key().unwrap_err());
	}

	#[test]
	fn xrpl_public_key_ed25519() {
		let tx = XRPLTransaction {
			signing_pub_key: "EDFB2A3A850B43E24D2700532EF1F9CCB2475DFF4F62B634B0C58845F23C263965"
				.into(),
			..Default::default()
		};
		let pub_key = tx.get_public_key().unwrap();
		assert_eq!(
			[
				251, 42, 58, 133, 11, 67, 226, 77, 39, 0, 83, 46, 241, 249, 204, 178, 71, 93, 255,
				79, 98, 182, 52, 176, 197, 136, 69, 242, 60, 38, 57, 101
			],
			pub_key.as_ref(),
		);
		assert_eq!(
			"FB2A3A850B43E24D2700532EF1F9CCB2475DFF4F62B634B0C58845F23C263965".to_string(),
			hex::encode(pub_key.as_ref()).to_uppercase(),
		);
	}

	#[test]
	fn xrpl_public_key_ecdsa() {
		let tx = XRPLTransaction {
			signing_pub_key: "02A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A"
				.into(),
			..Default::default()
		};
		let pub_key = tx.get_public_key().unwrap();
		assert_eq!(
			[
				2, 166, 147, 78, 135, 152, 132, 102, 185, 139, 81, 242, 235, 9, 229, 188, 76, 9,
				228, 110, 181, 241, 254, 8, 114, 61, 248, 173, 35, 213, 187, 156, 106
			],
			pub_key.as_ref(),
		);
		assert_eq!(
			"02A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A".to_string(),
			hex::encode(pub_key.as_ref()).to_uppercase(),
		);
	}

	#[test]
	fn xrpl_unsupported_public_key() {
		// 0x04 prefix is not supported
		let tx = XRPLTransaction {
			signing_pub_key: "04A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A"
				.into(),
			..Default::default()
		};
		assert_eq!("Unsupported public key type", tx.get_public_key().unwrap_err());
	}

	#[test]
	fn xrpl_public_key_to_ethereum_account() {
		let tx = XRPLTransaction {
			signing_pub_key: "02A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A"
				.into(),
			..Default::default()
		};
		let eth_account: H160 = tx.get_account().unwrap();
		assert_eq!("0xa2ea…dc8a".to_string(), eth_account.to_string());
	}

	#[test]
	fn get_extrinsic_data_failure_cases() {
		let tx = XRPLTransaction::default();
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("no extrinsic call found in memos", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: "some unsupported type".into(),
					memo_data: "some unsupported data".into(),
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("failed to decode memo_type as hex", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: "extrinsic".into(),
					memo_data: "some unsupported data".into(),
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("failed to decode memo_type as hex", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: "some unsupported data".into(),
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("failed to decode memo_data as hex", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode("some unsupported data"),
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("failed to decode genesis_hash as hex", tx.get_extrinsic_data().unwrap_err());

		let empty_buf = hex::encode([0u8; 32]);
		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!("{empty_buf}:")),
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("failed to parse string as u32", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!("{empty_buf}:0")),
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!(
			"failed to get max_block_number from memo_data",
			tx.get_extrinsic_data().unwrap_err()
		);

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!("{empty_buf}:0:0")),
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("failed to get tip from memo_data", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!("{empty_buf}:0:0:0")),
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!(
			"failed to get hashed_call from memo_data",
			tx.get_extrinsic_data().unwrap_err()
		);

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!("{empty_buf}:0:0:0:")),
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!(
			"failed to convert hashed_call to 32 bytes",
			tx.get_extrinsic_data().unwrap_err()
		);

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!("{empty_buf}:0:0:0:{empty_buf}")),
				},
			}],
			..Default::default()
		};
		assert_ok!(tx.get_extrinsic_data());
	}

	#[test]
	fn get_account_nonce_from_extrinsic_data() {
		let empty_buf = hex::encode([0u8; 32]);
		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!("{empty_buf}:-1:0:0:{empty_buf}")), /* negative nonce */
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("failed to parse string as u32", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!(
						"{empty_buf}:{}:0:{empty_buf}",
						Into::<u64>::into(u32::MAX) + 1, // u32::MAX + 1
					)),
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("failed to parse string as u32", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!("{empty_buf}:{}:0:0:{empty_buf}", u32::MAX)), /* u32::MAX */
				},
			}],
			..Default::default()
		};
		assert_ok!(tx.get_extrinsic_data());
		assert_eq!(tx.get_extrinsic_data().unwrap().nonce, u32::MAX);
	}

	#[test]
	fn get_max_block_number_from_extrinsic_data() {
		let empty_buf = hex::encode([0u8; 32]);
		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!("{empty_buf}:0:-1:0:{empty_buf}")), /* negative max_block_number */
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("failed to parse string as u32", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!(
						"{empty_buf}:0:{}:0:{empty_buf}",
						Into::<u64>::into(u32::MAX) + 1,
					)), // u32::MAX + 1
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("failed to parse string as u32", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!("{empty_buf}:0:{}:0:{empty_buf}", u32::MAX)), /* u32::MAX */
				},
			}],
			..Default::default()
		};
		assert_ok!(tx.get_extrinsic_data());
		assert_eq!(tx.get_extrinsic_data().unwrap().max_block_number, u32::MAX);
	}

	#[test]
	fn get_tip_from_extrinsic_data() {
		let empty_buf = hex::encode([0u8; 32]);
		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!("{empty_buf}:0:0:-1:{empty_buf}")), /* negative tip */
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("failed to parse string as u64", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!(
						"{empty_buf}:0:0:{}:{empty_buf}",
						Into::<u128>::into(u64::MAX) + 1,
					)), // u32::MAX + 1
				},
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("failed to parse string as u64", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode(format!("{empty_buf}:0:0:{}:{empty_buf}", u64::MAX)), /* u64::MAX */
				},
			}],
			..Default::default()
		};
		assert_ok!(tx.get_extrinsic_data());
		assert_eq!(tx.get_extrinsic_data().unwrap().tip, u64::MAX);
	}

	#[test]
	fn try_into_transaction_common() {
		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode("0:0:0:"),
				},
			}],
			..Default::default()
		};
		let tx_common_result: Result<TransactionCommon, &'static str> = (&tx).try_into();
		assert!(tx_common_result.is_err());

		let tx = XRPLTransaction {
			account_txn_id: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580"
				.into(),
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode("0:0:0:"),
				},
			}],
			..Default::default()
		};
		let tx_common_result: Result<TransactionCommon, &'static str> = (&tx).try_into();
		assert!(tx_common_result.is_err());

		let tx = XRPLTransaction {
			account_txn_id: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580"
				.into(),
			account: "rhLmGWkHr59h9ffYgPEAqZnqiQZMGb71yo".into(),
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode("0:0:0:"),
				},
			}],
			..Default::default()
		};
		let tx_common_result: Result<TransactionCommon, &'static str> = (&tx).try_into();
		assert_ok!(tx_common_result);
	}
}
