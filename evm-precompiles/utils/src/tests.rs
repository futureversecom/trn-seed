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

use crate::{prelude::*, revert::Backtrace};
use hex_literal::hex;
use pallet_evm::Context;
use sp_core::{H160, H256, U256};

fn u256_repeat_byte(byte: u8) -> U256 {
	let value = H256::repeat_byte(byte);

	U256::from_big_endian(value.as_bytes())
}

// When debugging it is useful to display data in chunks of 32 bytes.
#[allow(dead_code)]
fn display_bytes(bytes: &[u8]) {
	bytes
		.chunks_exact(32)
		.map(H256::from_slice)
		.for_each(|hash| println!("{:?}", hash));
}

#[test]
fn write_bool() {
	let value = true;

	let writer_output = EvmDataWriter::new().write(value).build();

	let mut expected_output = [0u8; 32];
	expected_output[31] = 1;

	assert_eq!(writer_output, expected_output);
}

#[test]
fn read_bool() {
	let value = true;

	let writer_output = EvmDataWriter::new().write(value).build();

	let mut reader = EvmDataReader::new(&writer_output);
	let parsed: bool = reader.read().expect("to correctly parse bool");

	assert_eq!(value, parsed);
}

#[test]
fn write_u64() {
	let value = 42u64;

	let writer_output = EvmDataWriter::new().write(value).build();

	let mut expected_output = [0u8; 32];
	expected_output[24..].copy_from_slice(&value.to_be_bytes());

	assert_eq!(writer_output, expected_output);
}

#[test]
fn read_u64() {
	let value = 42u64;
	let writer_output = EvmDataWriter::new().write(value).build();

	let mut reader = EvmDataReader::new(&writer_output);
	let parsed: u64 = reader.read().expect("to correctly parse u64");

	assert_eq!(value, parsed);
}

#[test]
fn write_u128() {
	let value = 42u128;

	let writer_output = EvmDataWriter::new().write(value).build();

	let mut expected_output = [0u8; 32];
	expected_output[16..].copy_from_slice(&value.to_be_bytes());

	assert_eq!(writer_output, expected_output);
}

#[test]
fn read_u128() {
	let value = 42u128;
	let writer_output = EvmDataWriter::new().write(value).build();

	let mut reader = EvmDataReader::new(&writer_output);
	let parsed: u128 = reader.read().expect("to correctly parse u128");

	assert_eq!(value, parsed);
}

#[test]
fn write_u256() {
	let value = U256::from(42);

	let writer_output = EvmDataWriter::new().write(value).build();

	let mut expected_output = [0u8; 32];
	value.to_big_endian(&mut expected_output);

	assert_eq!(writer_output, expected_output);
}

#[test]
fn read_u256() {
	let value = U256::from(42);
	let writer_output = EvmDataWriter::new().write(value).build();

	let mut reader = EvmDataReader::new(&writer_output);
	let parsed: U256 = reader.read().expect("to correctly parse U256");

	assert_eq!(value, parsed);
}

#[test]
fn read_selector() {
	use sha3::{Digest, Keccak256};

	#[precompile_utils_macro::generate_function_selector]
	#[derive(Debug, PartialEq)]
	enum FakeAction {
		Action1 = "action1()",
	}

	let selector = &Keccak256::digest(b"action1()")[0..4];

	let parsed_selector =
		EvmDataReader::read_selector::<FakeAction>(selector).expect("there is a selector");
	EvmDataReader::new_skip_selector(selector).expect("there is a selector");

	assert_eq!(parsed_selector, FakeAction::Action1)
}

#[test]
fn read_u256_too_short() {
	let value = U256::from(42);
	let writer_output = EvmDataWriter::new().write(value).build();

	let mut reader = EvmDataReader::new(&writer_output[0..31]);
	let result: Result<U256, _> = reader.read().map_err(|_| "to correctly parse U256");
	assert_eq!(result.unwrap_err().to_string(), "to correctly parse U256");
}

#[test]
fn write_h256() {
	let mut raw = [0u8; 32];
	raw[0] = 42;
	raw[12] = 43;
	raw[31] = 44;

	let value = H256::from(raw);

	let output = EvmDataWriter::new().write(value).build();

	assert_eq!(&output, &raw);
}

#[test]
fn tmp() {
	let u = U256::from(1_000_000_000);
	println!("U256={:?}", u.0);
}

#[test]
fn read_h256() {
	let mut raw = [0u8; 32];
	raw[0] = 42;
	raw[12] = 43;
	raw[31] = 44;
	let value = H256::from(raw);
	let writer_output = EvmDataWriter::new().write(value).build();

	let mut reader = EvmDataReader::new(&writer_output);
	let parsed: H256 = reader.read().expect("to correctly parse H256");

	assert_eq!(value, parsed);
}

#[test]
fn read_h256_too_short() {
	let mut raw = [0u8; 32];
	raw[0] = 42;
	raw[12] = 43;
	raw[31] = 44;
	let value = H256::from(raw);
	let writer_output = EvmDataWriter::new().write(value).build();

	let mut reader = EvmDataReader::new(&writer_output[0..31]);
	let result: Result<H256, _> = reader.read().map_err(|_| "to correctly parse H256");
	assert_eq!(result.unwrap_err().to_string(), "to correctly parse H256");
}

#[test]
fn write_address() {
	let value = H160::repeat_byte(0xAA);

	let output = EvmDataWriter::new().write(Address(value)).build();

	assert_eq!(output.len(), 32);
	assert_eq!(&output[12..32], value.as_bytes());
}

#[test]
fn read_address() {
	let value = H160::repeat_byte(0xAA);
	let writer_output = EvmDataWriter::new().write(Address(value)).build();

	let mut reader = EvmDataReader::new(&writer_output);
	let parsed: Address = reader.read().expect("to correctly parse Address");

	assert_eq!(value, parsed.0);
}

#[test]
fn write_h256_array() {
	let array = vec![
		H256::repeat_byte(0x11),
		H256::repeat_byte(0x22),
		H256::repeat_byte(0x33),
		H256::repeat_byte(0x44),
		H256::repeat_byte(0x55),
	];
	let writer_output = EvmDataWriter::new().write(array.clone()).build();
	assert_eq!(writer_output.len(), 0xE0);

	// We can read this "manualy" using simpler functions since arrays are 32-byte aligned.
	let mut reader = EvmDataReader::new(&writer_output);

	assert_eq!(reader.read::<U256>().expect("read offset"), 32.into());
	assert_eq!(reader.read::<U256>().expect("read size"), 5.into());
	assert_eq!(reader.read::<H256>().expect("read 1st"), array[0]);
	assert_eq!(reader.read::<H256>().expect("read 2nd"), array[1]);
	assert_eq!(reader.read::<H256>().expect("read 3rd"), array[2]);
	assert_eq!(reader.read::<H256>().expect("read 4th"), array[3]);
	assert_eq!(reader.read::<H256>().expect("read 5th"), array[4]);
}

#[test]
fn read_h256_array() {
	let array = vec![
		H256::repeat_byte(0x11),
		H256::repeat_byte(0x22),
		H256::repeat_byte(0x33),
		H256::repeat_byte(0x44),
		H256::repeat_byte(0x55),
	];
	let writer_output = EvmDataWriter::new().write(array.clone()).build();

	let mut reader = EvmDataReader::new(&writer_output);
	let parsed: Vec<H256> = reader.read().expect("to correctly parse Vec<H256>");

	assert_eq!(array, parsed);
}

#[test]
fn write_u256_array() {
	let array = vec![
		u256_repeat_byte(0x11),
		u256_repeat_byte(0x22),
		u256_repeat_byte(0x33),
		u256_repeat_byte(0x44),
		u256_repeat_byte(0x55),
	];
	let writer_output = EvmDataWriter::new().write(array.clone()).build();
	assert_eq!(writer_output.len(), 0xE0);

	// We can read this "manualy" using simpler functions since arrays are 32-byte aligned.
	let mut reader = EvmDataReader::new(&writer_output);

	assert_eq!(reader.read::<U256>().expect("read offset"), 32.into());
	assert_eq!(reader.read::<U256>().expect("read size"), 5.into());
	assert_eq!(reader.read::<U256>().expect("read 1st"), array[0]);
	assert_eq!(reader.read::<U256>().expect("read 2nd"), array[1]);
	assert_eq!(reader.read::<U256>().expect("read 3rd"), array[2]);
	assert_eq!(reader.read::<U256>().expect("read 4th"), array[3]);
	assert_eq!(reader.read::<U256>().expect("read 5th"), array[4]);
}

#[test]
fn read_u256_array() {
	let array = vec![
		u256_repeat_byte(0x11),
		u256_repeat_byte(0x22),
		u256_repeat_byte(0x33),
		u256_repeat_byte(0x44),
		u256_repeat_byte(0x55),
	];
	let writer_output = EvmDataWriter::new().write(array.clone()).build();

	let mut reader = EvmDataReader::new(&writer_output);
	let parsed: Vec<U256> = reader.read().expect("to correctly parse Vec<H256>");

	assert_eq!(array, parsed);
}

#[test]
fn write_address_array() {
	let array = vec![
		Address(H160::repeat_byte(0x11)),
		Address(H160::repeat_byte(0x22)),
		Address(H160::repeat_byte(0x33)),
		Address(H160::repeat_byte(0x44)),
		Address(H160::repeat_byte(0x55)),
	];
	let writer_output = EvmDataWriter::new().write(array.clone()).build();

	// We can read this "manualy" using simpler functions since arrays are 32-byte aligned.
	let mut reader = EvmDataReader::new(&writer_output);

	assert_eq!(reader.read::<U256>().expect("read offset"), 32.into());
	assert_eq!(reader.read::<U256>().expect("read size"), 5.into());
	assert_eq!(reader.read::<Address>().expect("read 1st"), array[0]);
	assert_eq!(reader.read::<Address>().expect("read 2nd"), array[1]);
	assert_eq!(reader.read::<Address>().expect("read 3rd"), array[2]);
	assert_eq!(reader.read::<Address>().expect("read 4th"), array[3]);
	assert_eq!(reader.read::<Address>().expect("read 5th"), array[4]);
}

#[test]
fn read_address_array() {
	let array = vec![
		Address(H160::repeat_byte(0x11)),
		Address(H160::repeat_byte(0x22)),
		Address(H160::repeat_byte(0x33)),
		Address(H160::repeat_byte(0x44)),
		Address(H160::repeat_byte(0x55)),
	];
	let writer_output = EvmDataWriter::new().write(array.clone()).build();

	let mut reader = EvmDataReader::new(&writer_output);
	let parsed: Vec<Address> = reader.read().expect("to correctly parse Vec<H256>");

	assert_eq!(array, parsed);
}

#[test]
fn read_address_array_size_too_big() {
	let array = vec![
		Address(H160::repeat_byte(0x11)),
		Address(H160::repeat_byte(0x22)),
		Address(H160::repeat_byte(0x33)),
		Address(H160::repeat_byte(0x44)),
		Address(H160::repeat_byte(0x55)),
	];
	let mut writer_output = EvmDataWriter::new().write(array).build();

	U256::from(6u32).to_big_endian(&mut writer_output[0x20..0x40]);

	let mut reader = EvmDataReader::new(&writer_output);

	match reader.read::<Vec<Address>>().in_field("field") {
		Ok(_) => panic!("should not parse correctly"),
		Err(err) => {
			assert_eq!(err.to_string(), "field[5]: Tried to read address out of bounds")
		},
	}
}

#[test]
fn write_address_nested_array() {
	let array = vec![
		vec![
			Address(H160::repeat_byte(0x11)),
			Address(H160::repeat_byte(0x22)),
			Address(H160::repeat_byte(0x33)),
		],
		vec![Address(H160::repeat_byte(0x44)), Address(H160::repeat_byte(0x55))],
	];
	let writer_output = EvmDataWriter::new().write(array.clone()).build();
	assert_eq!(writer_output.len(), 0x160);

	// We can read this "manualy" using simpler functions since arrays are 32-byte aligned.
	let mut reader = EvmDataReader::new(&writer_output);

	assert_eq!(reader.read::<U256>().expect("read offset"), 0x20.into()); // 0x00
	assert_eq!(reader.read::<U256>().expect("read size"), 2.into()); // 0x20
	assert_eq!(reader.read::<U256>().expect("read 1st offset"), 0x40.into()); // 0x40
	assert_eq!(reader.read::<U256>().expect("read 2st offset"), 0xc0.into()); // 0x60
	assert_eq!(reader.read::<U256>().expect("read 1st size"), 3.into()); // 0x80
	assert_eq!(reader.read::<Address>().expect("read 1-1"), array[0][0]); // 0xA0
	assert_eq!(reader.read::<Address>().expect("read 1-2"), array[0][1]); // 0xC0
	assert_eq!(reader.read::<Address>().expect("read 1-3"), array[0][2]); // 0xE0
	assert_eq!(reader.read::<U256>().expect("read 2nd size"), 2.into()); // 0x100
	assert_eq!(reader.read::<Address>().expect("read 2-1"), array[1][0]); // 0x120
	assert_eq!(reader.read::<Address>().expect("read 2-2"), array[1][1]); // 0x140
}

#[test]
fn read_address_nested_array() {
	let array = vec![
		vec![
			Address(H160::repeat_byte(0x11)),
			Address(H160::repeat_byte(0x22)),
			Address(H160::repeat_byte(0x33)),
		],
		vec![Address(H160::repeat_byte(0x44)), Address(H160::repeat_byte(0x55))],
	];
	let writer_output = EvmDataWriter::new().write(array.clone()).build();

	let mut reader = EvmDataReader::new(&writer_output);
	let parsed: Vec<Vec<Address>> = reader.read().expect("to correctly parse Vec<Vec<Address>>");

	assert_eq!(array, parsed);
}

#[test]

fn write_multiple_arrays() {
	let array1 = vec![
		Address(H160::repeat_byte(0x11)),
		Address(H160::repeat_byte(0x22)),
		Address(H160::repeat_byte(0x33)),
	];

	let array2 = vec![H256::repeat_byte(0x44), H256::repeat_byte(0x55)];

	let writer_output = EvmDataWriter::new().write(array1.clone()).write(array2.clone()).build();

	assert_eq!(writer_output.len(), 0x120);

	// We can read this "manualy" using simpler functions since arrays are 32-byte aligned.
	let mut reader = EvmDataReader::new(&writer_output);

	assert_eq!(reader.read::<U256>().expect("read 1st offset"), 0x40.into()); // 0x00
	assert_eq!(reader.read::<U256>().expect("read 2nd offset"), 0xc0.into()); // 0x20
	assert_eq!(reader.read::<U256>().expect("read 1st size"), 3.into()); // 0x40
	assert_eq!(reader.read::<Address>().expect("read 1-1"), array1[0]); // 0x60
	assert_eq!(reader.read::<Address>().expect("read 1-2"), array1[1]); // 0x80
	assert_eq!(reader.read::<Address>().expect("read 1-3"), array1[2]); // 0xA0
	assert_eq!(reader.read::<U256>().expect("read 2nd size"), 2.into()); // 0xC0
	assert_eq!(reader.read::<H256>().expect("read 2-1"), array2[0]); // 0xE0
	assert_eq!(reader.read::<H256>().expect("read 2-2"), array2[1]); // 0x100
}

#[test]
fn read_multiple_arrays() {
	let array1 = vec![
		Address(H160::repeat_byte(0x11)),
		Address(H160::repeat_byte(0x22)),
		Address(H160::repeat_byte(0x33)),
	];

	let array2 = vec![H256::repeat_byte(0x44), H256::repeat_byte(0x55)];

	let writer_output = EvmDataWriter::new().write(array1.clone()).write(array2.clone()).build();

	// offset 0x20
	// offset 0x40
	// size 0x60
	// 3 addresses 0xC0
	// size 0xE0
	// 2 H256 0x120
	assert_eq!(writer_output.len(), 0x120);

	let mut reader = EvmDataReader::new(&writer_output);

	let parsed: Vec<Address> = reader.read().expect("to correctly parse Vec<Address>");
	assert_eq!(array1, parsed);

	let parsed: Vec<H256> = reader.read().expect("to correctly parse Vec<H256>");
	assert_eq!(array2, parsed);
}

#[test]
fn read_bytes() {
	let data = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod\
	tempor incididunt ut labore et dolore magna aliqua.";
	let writer_output = EvmDataWriter::new().write(Bytes::from(&data[..])).build();

	let mut reader = EvmDataReader::new(&writer_output);
	let parsed: Bytes = reader.read().expect("to correctly parse Bytes");

	assert_eq!(data, parsed.as_bytes());
}

#[test]
fn write_bytes() {
	let data = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod\
	tempor incididunt ut labore et dolore magna aliqua.";

	let writer_output = EvmDataWriter::new().write(Bytes::from(&data[..])).build();

	// We can read this "manualy" using simpler functions.
	let mut reader = EvmDataReader::new(&writer_output);

	// We pad data to a multiple of 32 bytes.
	let mut padded = data.to_vec();
	assert!(data.len() < 0x80);
	padded.resize(0x80, 0);

	assert_eq!(reader.read::<U256>().expect("read offset"), 32.into());
	assert_eq!(reader.read::<U256>().expect("read size"), data.len().into());
	let mut read = |e| reader.read::<H256>().expect(e); // shorthand
	assert_eq!(read("read part 1"), H256::from_slice(&padded[0x00..0x20]));
	assert_eq!(read("read part 2"), H256::from_slice(&padded[0x20..0x40]));
	assert_eq!(read("read part 3"), H256::from_slice(&padded[0x40..0x60]));
	assert_eq!(read("read part 4"), H256::from_slice(&padded[0x60..0x80]));
}

#[test]
fn read_string() {
	let data = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod\
	tempor incididunt ut labore et dolore magna aliqua.";
	let writer_output = EvmDataWriter::new().write(Bytes::from(data)).build();

	let mut reader = EvmDataReader::new(&writer_output);
	let parsed: Bytes = reader.read().expect("to correctly parse Bytes");

	assert_eq!(data, parsed.as_str().expect("valid utf8"));
}

#[test]
fn write_string() {
	let data = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod\
	tempor incididunt ut labore et dolore magna aliqua.";

	let writer_output = EvmDataWriter::new().write(Bytes::from(data)).build();

	// We can read this "manualy" using simpler functions.
	let mut reader = EvmDataReader::new(&writer_output);

	// We pad data to next multiple of 32 bytes.
	let mut padded = data.as_bytes().to_vec();
	assert!(data.len() < 0x80);
	padded.resize(0x80, 0);

	assert_eq!(reader.read::<U256>().expect("read offset"), 32.into());
	assert_eq!(reader.read::<U256>().expect("read size"), data.len().into());
	let mut read = |e| reader.read::<H256>().expect(e); // shorthand
	assert_eq!(read("read part 1"), H256::from_slice(&padded[0x00..0x20]));
	assert_eq!(read("read part 2"), H256::from_slice(&padded[0x20..0x40]));
	assert_eq!(read("read part 3"), H256::from_slice(&padded[0x40..0x60]));
	assert_eq!(read("read part 4"), H256::from_slice(&padded[0x60..0x80]));
}

#[test]
fn write_vec_bytes() {
	let data = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod\
	tempor incididunt ut labore et dolore magna aliqua.";

	let writer_output = EvmDataWriter::new()
		.write(vec![Bytes::from(&data[..]), Bytes::from(&data[..])])
		.build();

	writer_output
		.chunks_exact(32)
		.map(H256::from_slice)
		.for_each(|hash| println!("{:?}", hash));

	// We pad data to a multiple of 32 bytes.
	let mut padded = data.to_vec();
	assert!(data.len() < 0x80);
	padded.resize(0x80, 0);

	let mut reader = EvmDataReader::new(&writer_output);

	// Offset of vec
	assert_eq!(reader.read::<U256>().expect("read offset"), 32.into());

	// Length of vec
	assert_eq!(reader.read::<U256>().expect("read offset"), 2.into());

	// Relative offset of first bytgmes object
	assert_eq!(reader.read::<U256>().expect("read offset"), 0x40.into());
	// Relative offset of second bytes object
	assert_eq!(reader.read::<U256>().expect("read offset"), 0xe0.into());

	// Length of first bytes object
	assert_eq!(reader.read::<U256>().expect("read size"), data.len().into());

	// First byte objects data
	let mut read = |e| reader.read::<H256>().expect(e); // shorthand
	assert_eq!(read("read part 1"), H256::from_slice(&padded[0x00..0x20]));
	assert_eq!(read("read part 2"), H256::from_slice(&padded[0x20..0x40]));
	assert_eq!(read("read part 3"), H256::from_slice(&padded[0x40..0x60]));
	assert_eq!(read("read part 4"), H256::from_slice(&padded[0x60..0x80]));

	// Length of second bytes object
	assert_eq!(reader.read::<U256>().expect("read size"), data.len().into());

	// Second byte objects data
	let mut read = |e| reader.read::<H256>().expect(e); // shorthand
	assert_eq!(read("read part 1"), H256::from_slice(&padded[0x00..0x20]));
	assert_eq!(read("read part 2"), H256::from_slice(&padded[0x20..0x40]));
	assert_eq!(read("read part 3"), H256::from_slice(&padded[0x40..0x60]));
	assert_eq!(read("read part 4"), H256::from_slice(&padded[0x60..0x80]));
}

#[test]
fn read_vec_of_bytes() {
	let data = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod\
	tempor incididunt ut labore et dolore magna aliqua.";

	let writer_output = EvmDataWriter::new()
		.write(vec![Bytes::from(&data[..]), Bytes::from(&data[..])])
		.build();

	writer_output
		.chunks_exact(32)
		.map(H256::from_slice)
		.for_each(|hash| println!("{:?}", hash));

	let mut reader = EvmDataReader::new(&writer_output);
	let parsed: Vec<Bytes> = reader.read().expect("to correctly parse Vec<u8>");

	assert_eq!(vec![Bytes::from(&data[..]), Bytes::from(&data[..])], parsed);
}

// The following test parses input data generated by web3 from a Solidity contract.
// This is important to test on external data since all the above tests can only test consistency
// between `EvmDataReader` and `EvmDataWriter`.
//
// It also provides an example on how to impl `EvmData` for Solidity structs.
//
// struct MultiLocation {
// 	   uint8 parents;
// 	   bytes [] interior;
// }
//
// function transfer(
//     address currency_address,
//     uint256 amount,
//     MultiLocation memory destination,
//     uint64 weight
// ) external;

#[derive(Clone, Debug, Eq, PartialEq)]
struct MultiLocation {
	parents: u8,
	interior: Vec<Bytes>,
}

impl EvmData for MultiLocation {
	fn read(reader: &mut EvmDataReader) -> MayRevert<Self> {
		let mut inner_reader = reader.read_pointer()?;
		let parents = inner_reader.read().in_field("parents")?;
		let interior = inner_reader.read().in_field("interior")?;

		Ok(MultiLocation { parents, interior })
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		EvmData::write(writer, (value.parents, value.interior));
	}

	fn has_static_size() -> bool {
		<(u8, Vec<Bytes>)>::has_static_size()
	}
}

#[generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	TransferMultiAsset = "transfer_multiasset((uint8,bytes[]),uint256,(uint8,bytes[]),uint64)",
}

#[test]
fn read_complex_solidity_function() {
	// Function call data generated by web3.
	let data = hex!(
		"b38c60fa
		0000000000000000000000000000000000000000000000000000000000000080
		0000000000000000000000000000000000000000000000000000000000000064
		00000000000000000000000000000000000000000000000000000000000001a0
		0000000000000000000000000000000000000000000000000000000000000064
		0000000000000000000000000000000000000000000000000000000000000001
		0000000000000000000000000000000000000000000000000000000000000040
		0000000000000000000000000000000000000000000000000000000000000002
		0000000000000000000000000000000000000000000000000000000000000040
		0000000000000000000000000000000000000000000000000000000000000080
		0000000000000000000000000000000000000000000000000000000000000005
		00000003e8000000000000000000000000000000000000000000000000000000
		0000000000000000000000000000000000000000000000000000000000000002
		0403000000000000000000000000000000000000000000000000000000000000
		0000000000000000000000000000000000000000000000000000000000000001
		0000000000000000000000000000000000000000000000000000000000000040
		0000000000000000000000000000000000000000000000000000000000000001
		0000000000000000000000000000000000000000000000000000000000000020
		0000000000000000000000000000000000000000000000000000000000000022
		0101010101010101010101010101010101010101010101010101010101010101
		0100000000000000000000000000000000000000000000000000000000000000"
	);

	let selector = EvmDataReader::read_selector::<Action>(&data).expect("to read selector");
	let mut reader = EvmDataReader::new_skip_selector(&data).expect("to read selector");

	assert_eq!(selector, Action::TransferMultiAsset);
	// asset
	assert_eq!(
		reader.read::<MultiLocation>().unwrap(),
		MultiLocation {
			parents: 1,
			interior: vec![Bytes::from(&hex!("00000003e8")[..]), Bytes::from(&hex!("0403")[..]),]
		}
	);

	// amount
	assert_eq!(reader.read::<U256>().unwrap(), 100u32.into());

	// destination
	assert_eq!(
		reader.read::<MultiLocation>().unwrap(),
		MultiLocation {
			parents: 1,
			interior: vec![Bytes::from(
				&hex!("01010101010101010101010101010101010101010101010101010101010101010100")[..]
			)],
		}
	);

	// weight
	assert_eq!(reader.read::<U256>().unwrap(), 100u32.into());
}

#[test]
fn test_check_function_modifier() {
	let context = |value: u32| Context {
		address: H160::zero(),
		caller: H160::zero(),
		apparent_value: U256::from(value),
	};

	let payable_error = || Revert::new(RevertReason::custom("Function is not payable"));
	let static_error =
		|| Revert::new(RevertReason::custom("Can't call non-static function in static context"));

	// Can't call non-static functions in static context.
	assert_eq!(
		check_function_modifier(&context(0), true, FunctionModifier::Payable),
		Err(static_error())
	);
	assert_eq!(
		check_function_modifier(&context(0), true, FunctionModifier::NonPayable),
		Err(static_error())
	);
	assert_eq!(check_function_modifier(&context(0), true, FunctionModifier::View), Ok(()));

	// Static check is performed before non-payable check.
	assert_eq!(
		check_function_modifier(&context(1), true, FunctionModifier::Payable),
		Err(static_error())
	);
	assert_eq!(
		check_function_modifier(&context(1), true, FunctionModifier::NonPayable),
		Err(static_error())
	);
	// FunctionModifier::View pass static check but fail for payable.
	assert_eq!(
		check_function_modifier(&context(1), true, FunctionModifier::View),
		Err(payable_error())
	);

	// Can't send funds to non payable function
	assert_eq!(check_function_modifier(&context(1), false, FunctionModifier::Payable), Ok(()));
	assert_eq!(
		check_function_modifier(&context(1), false, FunctionModifier::NonPayable),
		Err(payable_error())
	);
	assert_eq!(
		check_function_modifier(&context(1), false, FunctionModifier::View),
		Err(payable_error())
	);

	// Any function can be called without funds.
	assert_eq!(check_function_modifier(&context(0), false, FunctionModifier::Payable), Ok(()));
	assert_eq!(check_function_modifier(&context(0), false, FunctionModifier::NonPayable), Ok(()));
	assert_eq!(check_function_modifier(&context(0), false, FunctionModifier::View), Ok(()));
}

#[test]
fn read_static_size_tuple() {
	// (address, uint256) encoded by web3
	let data = hex!(
		"0000000000000000000000001111111111111111111111111111111111111111
		0000000000000000000000000000000000000000000000000000000000000001"
	);

	let mut reader = EvmDataReader::new(&data);

	assert_eq!(
		reader.read::<(Address, U256)>().unwrap(),
		(Address(H160::repeat_byte(0x11)), U256::from(1u8))
	);
}

#[test]
fn read_dynamic_size_tuple() {
	// (uint8, bytes[]) encoded by web3
	let data = hex!(
		"0000000000000000000000000000000000000000000000000000000000000020
		0000000000000000000000000000000000000000000000000000000000000001
		0000000000000000000000000000000000000000000000000000000000000040
		0000000000000000000000000000000000000000000000000000000000000001
		0000000000000000000000000000000000000000000000000000000000000020
		0000000000000000000000000000000000000000000000000000000000000001
		0100000000000000000000000000000000000000000000000000000000000000"
	);

	let mut reader = EvmDataReader::new(&data);

	assert_eq!(reader.read::<(u8, Vec<Bytes>)>().unwrap(), (1, vec![Bytes(vec![0x01])]));
}

#[test]
fn write_static_size_tuple() {
	let output = EvmDataWriter::new()
		.write((Address(H160::repeat_byte(0x11)), U256::from(1u8)))
		.build();

	// (address, uint256) encoded by web3
	let data = hex!(
		"0000000000000000000000001111111111111111111111111111111111111111
		0000000000000000000000000000000000000000000000000000000000000001"
	);

	assert_eq!(output, data);
}

#[test]
fn write_dynamic_size_tuple() {
	let output = EvmDataWriter::new().write((1u8, vec![Bytes(vec![0x01])])).build();

	// (uint8, bytes[]) encoded by web3
	let data = hex!(
		"0000000000000000000000000000000000000000000000000000000000000020
		0000000000000000000000000000000000000000000000000000000000000001
		0000000000000000000000000000000000000000000000000000000000000040
		0000000000000000000000000000000000000000000000000000000000000001
		0000000000000000000000000000000000000000000000000000000000000020
		0000000000000000000000000000000000000000000000000000000000000001
		0100000000000000000000000000000000000000000000000000000000000000"
	);

	assert_eq!(output, data);
}

#[test]
fn error_location_formatting() {
	assert_eq!(
		Backtrace::new()
			.in_field("foo")
			.in_array(2)
			.in_array(3)
			.in_field("bar")
			.in_field("fuz")
			.to_string(),
		"fuz.bar[3][2].foo"
	);
}

#[test]
fn error_formatting() {
	assert_eq!(
		Revert::new(RevertReason::custom("Test"))
			.in_field("foo")
			.in_array(2)
			.in_array(3)
			.in_field("bar")
			.in_field("fuz")
			.to_string(),
		"fuz.bar[3][2].foo: Test"
	);
}
