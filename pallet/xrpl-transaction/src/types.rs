use alloc::{string::String, vec::Vec};
use libsecp256k1::Message;
use seed_primitives::AccountId20;
use sha2::Digest;
use sp_core::{ecdsa::Public, H160};
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
	pub nonce: u32,
	pub max_block_number: u32,
	pub call: Vec<u8>,
}

/// First half of sha512 hash of a message
pub fn sha512_first_half(message: &[u8]) -> [u8; SHA512_HASH_LENGTH] {
	let mut sha512 = sha2::Sha512::new();
	sha512.update(message);
	sha512.finalize()[..SHA512_HASH_LENGTH].try_into().unwrap()
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

#[derive(Debug, PartialEq, serde::Deserialize)]
pub struct Memo {
	#[serde(rename = "MemoData")]
	pub memo_data: String,
	#[serde(rename = "MemoType")]
	pub memo_type: String,
}
#[derive(Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MemoElmRaw {
	pub memo: Memo,
}
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
	pub fn get_public_key(&self) -> Result<[u8; 33], &'static str> {
		let pub_key: [u8; 33] = hex::decode(&self.signing_pub_key)
			.map_err(|_| "Error decoding hex string")?
			.try_into()
			.map_err(|_| "Invalid length of decoded bytes")?;
		Ok(pub_key)
	}

	/// Derives the account (H160) from the `signing_pub_key` in the XRPL transaction.
	pub fn get_account(&self) -> Result<H160, &'static str> {
		let account_bytes = self.get_public_key()?;
		let public = Public::from_raw(account_bytes);
		let account: AccountId20 = public.try_into()?;
		Ok(account.into())
	}

	/// Extracts the extrinsic data from the memos in the XRPL transaction.
	/// Finds the memos in the list where the memo type is `extrinsic`; and
	/// extracts the `nonce`, `max_block_number` and call from the memo data.
	/// The memo data is hex encoded with the following format (string):
	/// `nonce<u32>:max_block_number<u32>:call<Vec<u8>>`
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
				// split string by `:`, parse each string to extract nonce, max_block_number and
				// call data
				let mut split = hex_decoded_data_str.split(":");
				let nonce = split.next().ok_or("failed to get nonce from memo_data")?;
				let nonce = nonce.parse::<u32>().map_err(|_| "failed to parse string as u32")?;
				let max_block_number =
					split.next().ok_or("failed to get max_block_number from memo_data")?;
				let max_block_number =
					max_block_number.parse::<u32>().map_err(|_| "failed to parse string as u32")?;
				let call = split.next().ok_or("failed to get call from memo_data")?;
				let call = hex::decode(call).map_err(|_| "failed to decode call as hex")?;
				return Ok(ExtrinsicMemoData { nonce, max_block_number, call })
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

		let hashed_msg: Message =
			libsecp256k1::Message::parse(&sha512_first_half(&encoded_message));
		let pub_key_bytes: [u8; 33] = hex::decode(&self.signing_pub_key)
			.map_err(|e| {
				log::warn!("⛔️ failed to decode signing_pub_key as hex: {:?}", e);
				"failed to decode signing_pub_key as hex"
			})?
			.try_into()
			.map_err(|e| {
				log::warn!("⛔️ failed to convert signing_pub_key to bytes: {:?}", e);
				"failed to convert signing_pub_key to bytes"
			})?;
		let pub_key = libsecp256k1::PublicKey::parse_compressed(&pub_key_bytes).map_err(|e| {
			log::warn!("⛔️ failed to parse public key: {:?}", e);
			"failed to parse public key"
		})?;
		let signature = libsecp256k1::Signature::parse_der(&signature).map_err(|e| {
			log::warn!("⛔️ failed to parse signature: {:?}", e);
			"failed to parse signature"
		})?;
		let success = libsecp256k1::verify(&hashed_msg, &signature, &pub_key);
		Ok(success)
	}
}
impl TryFrom<&[u8]> for XRPLTransaction {
	type Error = &'static str;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		let deserializer = &mut Deserializer::new(value.clone().to_vec(), field_info_lookup());
		let tx_json = deserializer
			.to_json(&TypeCode::Object, &value)
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
					memo_data: xrpl_types::Blob::from_hex(&memo_data)
						.map_err(|_| "failed to convert memo_data to Blob")?,
					memo_type: xrpl_types::Blob::from_hex(&memo_type)
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
	fn verification() {
		let signature = "3045022100FB7583772B8F348F4789620C5571146B6517887AC231B38E29D7688D73F9D2510220615DC87698A2BA64DF2CA83BD9A214002F74C2D615CA20E328AC4AB5E4CDE8BC";
		let tx = XRPLTransaction {
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
		let result = tx.verify_transaction(&hex::decode(signature).unwrap());
		assert_ok!(result);
		assert!(result.unwrap());
	}

	#[test]
	fn xrpl_public_key() {
		let tx = XRPLTransaction {
			signing_pub_key: "02A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A"
				.into(),
			..Default::default()
		};
		let pub_key: [u8; 33] = tx.get_public_key().unwrap();
		assert_eq!(
			[
				2, 166, 147, 78, 135, 152, 132, 102, 185, 139, 81, 242, 235, 9, 229, 188, 76, 9,
				228, 110, 181, 241, 254, 8, 114, 61, 248, 173, 35, 213, 187, 156, 106
			],
			pub_key,
		);
		assert_eq!(
			"02A6934E87988466B98B51F2EB09E5BC4C09E46EB5F1FE08723DF8AD23D5BB9C6A".to_string(),
			hex::encode(pub_key).to_uppercase(),
		);
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
		assert_eq!("failed to parse string as u32", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo { memo_type: hex::encode("extrinsic"), memo_data: hex::encode("0:") },
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("failed to parse string as u32", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo { memo_type: hex::encode("extrinsic"), memo_data: hex::encode("0:0") },
			}],
			..Default::default()
		};
		assert!(tx.get_extrinsic_data().is_err());
		assert_eq!("failed to get call from memo_data", tx.get_extrinsic_data().unwrap_err());

		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo { memo_type: hex::encode("extrinsic"), memo_data: hex::encode("0:0:") },
			}],
			..Default::default()
		};
		assert_ok!(tx.get_extrinsic_data());
	}

	#[test]
	fn get_account_nonce_from_extrinsic_data() {
		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode("-1:0:"), // negative nonce
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
					memo_data: hex::encode("4294967296:0:"), // u32::MAX + 1
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
					memo_data: hex::encode("4294967295:0:"), // u32::MAX
				},
			}],
			..Default::default()
		};
		assert_ok!(tx.get_extrinsic_data());
		assert_eq!(tx.get_extrinsic_data().unwrap().nonce, u32::MAX);
	}

	#[test]
	fn get_max_block_number_from_extrinsic_data() {
		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo {
					memo_type: hex::encode("extrinsic"),
					memo_data: hex::encode("0:-1:"), // negative nonce
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
					memo_data: hex::encode("0:4294967296:"), // u32::MAX + 1
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
					memo_data: hex::encode("0:4294967295:"), // u32::MAX
				},
			}],
			..Default::default()
		};
		assert_ok!(tx.get_extrinsic_data());
		assert_eq!(tx.get_extrinsic_data().unwrap().max_block_number, u32::MAX);
	}

	#[test]
	fn try_into_transaction_common() {
		let tx = XRPLTransaction {
			memos: vec![MemoElmRaw {
				memo: Memo { memo_type: hex::encode("extrinsic"), memo_data: hex::encode("0:0:") },
			}],
			..Default::default()
		};
		let tx_common_result: Result<TransactionCommon, &'static str> = (&tx).try_into();
		assert!(tx_common_result.is_err());

		let tx = XRPLTransaction {
			account_txn_id: "16969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580"
				.into(),
			memos: vec![MemoElmRaw {
				memo: Memo { memo_type: hex::encode("extrinsic"), memo_data: hex::encode("0:0:") },
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
				memo: Memo { memo_type: hex::encode("extrinsic"), memo_data: hex::encode("0:0:") },
			}],
			..Default::default()
		};
		let tx_common_result: Result<TransactionCommon, &'static str> = (&tx).try_into();
		assert_ok!(tx_common_result);
	}
}
