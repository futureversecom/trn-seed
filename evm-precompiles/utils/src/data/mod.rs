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

pub use affix::paste;

use crate::revert::{InjectBacktrace, MayRevert, RevertReason};
use alloc::borrow::ToOwned;
use core::{any::type_name, marker::PhantomData, ops::Range};
use frame_support::traits::{ConstU32, Get};
use impl_trait_for_tuples::impl_for_tuples;
use sp_core::{H160, H256, U256};
use sp_std::{convert::TryInto, vec, vec::Vec};

type ConstU32Max = ConstU32<{ u32::MAX }>;

/// The `address` type of Solidity.
/// H160 could represent 2 types of data (bytes20 and address) that are not encoded the same way.
/// To avoid issues writing H160 is thus not supported.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Address(pub H160);

impl From<H160> for Address {
	fn from(a: H160) -> Address {
		Address(a)
	}
}

impl From<Address> for H160 {
	fn from(a: Address) -> H160 {
		a.0
	}
}

/// The `bytes`/`string` type of Solidity.
/// It is different from `Vec<u8>` which will be serialized with padding for each `u8` element
/// of the array, while `Bytes` is tightly packed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bytes(pub Vec<u8>);

impl Bytes {
	/// Interpret as `bytes`.
	pub fn as_bytes(&self) -> &[u8] {
		&self.0
	}

	/// Interpret as `string`.
	/// Can fail if the string is not valid UTF8.
	pub fn as_str(&self) -> Result<&str, sp_std::str::Utf8Error> {
		sp_std::str::from_utf8(&self.0)
	}
}

impl From<&[u8]> for Bytes {
	fn from(a: &[u8]) -> Self {
		Self(a.to_owned())
	}
}

impl From<&str> for Bytes {
	fn from(a: &str) -> Self {
		a.as_bytes().into()
	}
}

impl From<Bytes> for Vec<u8> {
	fn from(value: Bytes) -> Vec<u8> {
		value.0
	}
}

/// Same as `Bytes` but with an additional length bound on read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedBytes<S> {
	pub inner: Vec<u8>,
	_phantom: PhantomData<S>,
}

impl<S> BoundedBytes<S> {
	pub fn into_vec(self) -> Vec<u8> {
		self.inner
	}
}

/// The `bytes<X>` type of Solidity. X can be any number less than 32
/// This will post pad the value with zeros upto a total of 32 bytes
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bytes32PostPad(pub Vec<u8>);

impl From<&[u8]> for Bytes32PostPad {
	fn from(a: &[u8]) -> Self {
		Self(a.to_owned())
	}
}

/// Wrapper around an EVM input slice, helping to parse it.
/// Provide functions to parse common types.
#[derive(Clone, Copy, Debug)]
pub struct EvmDataReader<'a> {
	input: &'a [u8],
	cursor: usize,
}

impl<'a> EvmDataReader<'a> {
	/// Create a new input parser.
	pub fn new(input: &'a [u8]) -> Self {
		Self { input, cursor: 0 }
	}

	/// Create a new input parser from a selector-initial input.
	pub fn read_selector<T>(input: &'a [u8]) -> MayRevert<T>
	where
		T: num_enum::TryFromPrimitive<Primitive = u32>,
	{
		if input.len() < 4 {
			return Err(RevertReason::read_out_of_bounds("selector").into());
		}

		let mut buffer = [0u8; 4];
		buffer.copy_from_slice(&input[0..4]);
		let selector = T::try_from_primitive(u32::from_be_bytes(buffer)).map_err(|_| {
			log::trace!(
				target: "precompile-utils",
				"Failed to match function selector for {}",
				type_name::<T>()
			);
			RevertReason::UnknownSelector
		})?;

		Ok(selector)
	}

	/// Create a new input parser from a selector-initial input.
	pub fn new_skip_selector(input: &'a [u8]) -> MayRevert<Self> {
		if input.len() < 4 {
			return Err(RevertReason::read_out_of_bounds("selector").into());
		}

		Ok(Self::new(&input[4..]))
	}

	/// Check the input has at least the correct amount of arguments before the end (32 bytes
	/// values).
	pub fn expect_arguments(&self, args: usize) -> MayRevert<()> {
		if self.input.len() >= self.cursor + args * 32 {
			Ok(())
		} else {
			Err(RevertReason::ExpectedAtLeastNArguments(args).into())
		}
	}

	/// Read data from the input.
	pub fn read<T: EvmData>(&mut self) -> MayRevert<T> {
		T::read(self)
	}

	/// Read raw bytes from the input.
	/// Doesn't handle any alignment checks, prefer using `read` instead of possible.
	/// Returns an error if trying to parse out of bounds.
	pub fn read_raw_bytes(&mut self, len: usize) -> MayRevert<&[u8]> {
		let range = self.move_cursor(len)?;

		let data = self
			.input
			.get(range)
			.ok_or_else(|| RevertReason::read_out_of_bounds("raw bytes"))?;

		Ok(data)
	}

	/// Reads a pointer, returning a reader targetting the pointed location.
	pub fn read_pointer(&mut self) -> MayRevert<Self> {
		let offset: usize = self
			.read::<U256>()
			.map_err(|_| RevertReason::read_out_of_bounds("pointer"))?
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("pointer"))?;

		if offset >= self.input.len() {
			return Err(RevertReason::PointerToOutofBound.into());
		}

		Ok(Self { input: &self.input[offset..], cursor: 0 })
	}

	/// Read remaining bytes
	pub fn read_till_end(&mut self) -> MayRevert<&[u8]> {
		let range = self.move_cursor(self.input.len() - self.cursor)?;

		let data = self
			.input
			.get(range)
			.ok_or_else(|| RevertReason::read_out_of_bounds("raw bytes"))?;

		Ok(data)
	}

	/// Move the reading cursor with provided length, and return a range from the previous cursor
	/// location to the new one.
	/// Checks cursor overflows.
	fn move_cursor(&mut self, len: usize) -> MayRevert<Range<usize>> {
		let start = self.cursor;
		let end = self.cursor.checked_add(len).ok_or(RevertReason::CursorOverflow)?;

		self.cursor = end;

		Ok(start..end)
	}
}

/// Help build an EVM input/output data.
///
/// Functions takes `self` to allow chaining all calls like
/// `EvmDataWriter::new().write(...).write(...).build()`.
/// While it could be more ergonomic to take &mut self, this would
/// prevent to have a `build` function that don't clone the output.
#[derive(Clone, Debug)]
pub struct EvmDataWriter {
	pub(crate) data: Vec<u8>,
	offset_data: Vec<OffsetDatum>,
	selector: Option<u32>,
}

#[derive(Clone, Debug)]
struct OffsetDatum {
	// Offset location in the container data.
	offset_position: usize,
	// Data pointed by the offset that must be inserted at the end of container data.
	data: Vec<u8>,
	// Inside of arrays, the offset is not from the start of array data (length), but from the
	// start of the item. This shift allows us to correct this.
	offset_shift: usize,
}

impl EvmDataWriter {
	/// Creates a new empty output builder (without selector).
	pub fn new() -> Self {
		Self { data: vec![], offset_data: vec![], selector: None }
	}

	/// Creates a new empty output builder with provided selector.
	/// Selector will only be appended before the data when calling
	/// `build` to not mess with the offsets.
	pub fn new_with_selector(selector: impl Into<u32>) -> Self {
		Self { data: vec![], offset_data: vec![], selector: Some(selector.into()) }
	}

	/// Return the built data.
	pub fn build(mut self) -> Vec<u8> {
		Self::bake_offsets(&mut self.data, self.offset_data);

		if let Some(selector) = self.selector {
			let mut output = selector.to_be_bytes().to_vec();
			output.append(&mut self.data);
			output
		} else {
			self.data
		}
	}

	/// Add offseted data at the end of this writer's data, updating the offsets.
	fn bake_offsets(output: &mut Vec<u8>, offsets: Vec<OffsetDatum>) {
		for mut offset_datum in offsets {
			let offset_position = offset_datum.offset_position;
			let offset_position_end = offset_position + 32;

			// The offset is the distance between the start of the data and the
			// start of the pointed data (start of a struct, length of an array).
			// Offsets in inner data are relative to the start of their respective "container".
			// However in arrays the "container" is actually the item itself instead of the whole
			// array, which is corrected by `offset_shift`.
			let free_space_offset = output.len() - offset_datum.offset_shift;

			// Override dummy offset to the offset it will be in the final output.
			U256::from(free_space_offset)
				.to_big_endian(&mut output[offset_position..offset_position_end]);

			// Append this data at the end of the current output.
			output.append(&mut offset_datum.data);
		}
	}

	/// Write arbitrary bytes.
	/// Doesn't handle any alignement checks, prefer using `write` instead if possible.
	fn write_raw_bytes(mut self, value: &[u8]) -> Self {
		self.data.extend_from_slice(value);
		self
	}

	/// Write data of requested type.
	pub fn write<T: EvmData>(mut self, value: T) -> Self {
		T::write(&mut self, value);
		self
	}

	/// Writes a pointer to given data.
	/// The data will be appended when calling `build`.
	/// Initially write a dummy value as offset in this writer's data, which will be replaced by
	/// the correct offset once the pointed data is appended.
	///
	/// Takes `&mut self` since its goal is to be used inside `EvmData` impl and not in chains.
	pub fn write_pointer(&mut self, data: Vec<u8>) {
		let offset_position = self.data.len();
		H256::write(self, H256::repeat_byte(0xff));

		self.offset_data.push(OffsetDatum { offset_position, data, offset_shift: 0 });
	}
}

impl Default for EvmDataWriter {
	fn default() -> Self {
		Self::new()
	}
}

/// Data that can be converted from and to EVM data types.
pub trait EvmData: Sized {
	fn read(reader: &mut EvmDataReader) -> MayRevert<Self>;
	fn write(writer: &mut EvmDataWriter, value: Self);
	fn has_static_size() -> bool;
}

#[impl_for_tuples(1, 18)]
impl EvmData for Tuple {
	fn has_static_size() -> bool {
		for_tuples!(#( Tuple::has_static_size() )&*)
	}

	fn read(reader: &mut EvmDataReader) -> MayRevert<Self> {
		if !Self::has_static_size() {
			let reader = &mut reader.read_pointer()?;
			let mut index = 0;
			Ok(for_tuples!( ( #( {
				let elem = reader.read::<Tuple>().in_tuple(index)?;
				index +=1;
				elem
			} ),* ) ))
		} else {
			let mut index = 0;
			Ok(for_tuples!( ( #( {
				let elem = reader.read::<Tuple>().in_tuple(index)?;
				index +=1;
				elem
			} ),* ) ))
		}
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		if !Self::has_static_size() {
			let mut inner_writer = EvmDataWriter::new();
			for_tuples!( #( Tuple::write(&mut inner_writer, value.Tuple); )* );
			writer.write_pointer(inner_writer.build());
		} else {
			for_tuples!( #( Tuple::write(writer, value.Tuple); )* );
		}
	}
}

impl EvmData for H256 {
	fn read(reader: &mut EvmDataReader) -> MayRevert<Self> {
		let range = reader.move_cursor(32)?;

		let data = reader
			.input
			.get(range)
			.ok_or_else(|| RevertReason::read_out_of_bounds("bytes32"))?;

		Ok(H256::from_slice(data))
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		writer.data.extend_from_slice(value.as_bytes());
	}

	fn has_static_size() -> bool {
		true
	}
}

impl EvmData for Address {
	fn read(reader: &mut EvmDataReader) -> MayRevert<Self> {
		let range = reader.move_cursor(32)?;

		let data = reader
			.input
			.get(range)
			.ok_or_else(|| RevertReason::read_out_of_bounds("address"))?;

		Ok(H160::from_slice(&data[12..32]).into())
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		H256::write(writer, value.0.into());
	}

	fn has_static_size() -> bool {
		true
	}
}

impl EvmData for U256 {
	fn read(reader: &mut EvmDataReader) -> MayRevert<Self> {
		let range = reader.move_cursor(32)?;

		let data = reader
			.input
			.get(range)
			.ok_or_else(|| RevertReason::read_out_of_bounds("uint256"))?;

		Ok(U256::from_big_endian(data))
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		let mut buffer = [0u8; 32];
		value.to_big_endian(&mut buffer);
		writer.data.extend_from_slice(&buffer);
	}

	fn has_static_size() -> bool {
		true
	}
}

macro_rules! impl_evmdata_for_uints {
	($($uint:ty, )*) => {
		$(
			impl EvmData for $uint {
				fn read(reader: &mut EvmDataReader) -> MayRevert<Self> {
					let value256: U256 = reader.read()
					.map_err(|_| RevertReason::read_out_of_bounds(
						alloc::format!("uint{}", core::mem::size_of::<Self>() * 8)
					))?;

					value256
						.try_into()
						.map_err(|_| RevertReason::value_is_too_large(
							alloc::format!("uint{}", core::mem::size_of::<Self>() * 8)
						).into())
				}

				fn write(writer: &mut EvmDataWriter, value: Self) {
					U256::write(writer, value.into());
				}

				fn has_static_size() -> bool {
					true
				}
			}
		)*
	};
}

impl_evmdata_for_uints!(u8, u16, u32, u64, u128,);

impl EvmData for bool {
	fn read(reader: &mut EvmDataReader) -> MayRevert<Self> {
		let h256 = H256::read(reader).map_err(|_| RevertReason::read_out_of_bounds("bool"))?;

		Ok(!h256.is_zero())
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		let mut buffer = [0u8; 32];
		if value {
			buffer[31] = 1;
		}

		writer.data.extend_from_slice(&buffer);
	}

	fn has_static_size() -> bool {
		true
	}
}

impl EvmData for Bytes {
	fn read(reader: &mut EvmDataReader) -> MayRevert<Self> {
		Ok(Bytes(BoundedBytes::<ConstU32Max>::read(reader)?.into_vec()))
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		BoundedBytes::<ConstU32Max>::write(
			writer,
			BoundedBytes { inner: value.0, _phantom: PhantomData },
		);
	}

	fn has_static_size() -> bool {
		false
	}
}

impl<S: Get<u32>> EvmData for BoundedBytes<S> {
	fn read(reader: &mut EvmDataReader) -> MayRevert<Self> {
		let mut inner_reader = reader.read_pointer()?;

		// Read bytes/string size.
		let array_size: usize = inner_reader
			.read::<U256>()
			.map_err(|_| RevertReason::read_out_of_bounds("length"))?
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("length"))?;

		if array_size > S::get() as usize {
			return Err(RevertReason::value_is_too_large("length").into());
		}

		// Get valid range over the bytes data.
		let range = inner_reader.move_cursor(array_size)?;

		let data = inner_reader
			.input
			.get(range)
			.ok_or_else(|| RevertReason::read_out_of_bounds("bytes/string"))?;

		let bytes = Self { inner: data.to_owned(), _phantom: PhantomData };

		Ok(bytes)
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		let value = value.into_vec();
		let length = value.len();

		// Pad the data.
		// Leave it as is if a multiple of 32, otherwise pad to next
		// multiple or 32.
		let chunks = length / 32;
		let padded_size = match length % 32 {
			0 => chunks * 32,
			_ => (chunks + 1) * 32,
		};

		let mut value = value.to_vec();
		value.resize(padded_size, 0);

		writer.write_pointer(
			EvmDataWriter::new().write(U256::from(length)).write_raw_bytes(&value).build(),
		);
	}

	fn has_static_size() -> bool {
		false
	}
}

impl EvmData for Bytes32PostPad {
	fn read(reader: &mut EvmDataReader) -> MayRevert<Self> {
		let range = reader.move_cursor(32)?;
		let data = reader
			.input
			.get(range)
			.ok_or_else(|| RevertReason::read_out_of_bounds("bytes32"))?;

		Ok(Bytes32PostPad::from(data))
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		let mut value = value.0;
		value.resize(32, 0_u8);
		writer.data.extend_from_slice(value.as_slice());
	}

	fn has_static_size() -> bool {
		true
	}
}

impl<T: EvmData> EvmData for Vec<T> {
	fn read(reader: &mut EvmDataReader) -> MayRevert<Self> {
		BoundedVec::<T, ConstU32Max>::read(reader).map(|x| x.into_vec())
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		BoundedVec::<T, ConstU32Max>::write(
			writer,
			BoundedVec { inner: value, _phantom: PhantomData },
		)
	}

	fn has_static_size() -> bool {
		false
	}
}

/// Wrapper around a Vec that provides a max length bound on read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedVec<T, S> {
	inner: Vec<T>,
	_phantom: PhantomData<S>,
}

impl<T, S: Get<u32>> BoundedVec<T, S> {
	pub fn into_vec(self) -> Vec<T> {
		self.inner
	}
}

impl<T: EvmData, S: Get<u32>> EvmData for BoundedVec<T, S> {
	fn read(reader: &mut EvmDataReader) -> MayRevert<Self> {
		let mut inner_reader = reader.read_pointer()?;

		let array_size: usize = inner_reader
			.read::<U256>()
			.map_err(|_| RevertReason::read_out_of_bounds("length"))?
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("length"))?;

		if array_size > S::get() as usize {
			return Err(RevertReason::value_is_too_large("length").into());
		}

		let mut array = vec![];

		let mut item_reader = EvmDataReader {
			input: inner_reader
				.input
				.get(32..)
				.ok_or_else(|| RevertReason::read_out_of_bounds("array content"))?,
			cursor: 0,
		};

		for i in 0..array_size {
			array.push(item_reader.read().in_array(i)?);
		}

		Ok(BoundedVec { inner: array, _phantom: PhantomData })
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		let value = value.into_vec();
		let mut inner_writer = EvmDataWriter::new().write(U256::from(value.len()));

		for inner in value {
			// Any offset in items are relative to the start of the item instead of the
			// start of the array. However if there is offseted data it must but appended after
			// all items (offsets) are written. We thus need to rely on `compute_offsets` to do
			// that, and must store a "shift" to correct the offsets.
			let shift = inner_writer.data.len();
			let item_writer = EvmDataWriter::new().write(inner);

			inner_writer = inner_writer.write_raw_bytes(&item_writer.data);
			for mut offset_datum in item_writer.offset_data {
				offset_datum.offset_shift += 32;
				offset_datum.offset_position += shift;
				inner_writer.offset_data.push(offset_datum);
			}
		}

		writer.write_pointer(inner_writer.build());
	}

	fn has_static_size() -> bool {
		false
	}
}

/// Helper to write `EvmData` impl for Solidity structs.
/// Identifiers used should match Solidity ones.
/// Types are infered from context, which should always be
/// possible when parsing input to build a Rust struct.
///
/// ```rust,ignore
/// impl EvmData for Currency {
///     fn read(reader: &mut EvmDataReader) -> MayRevert<Self> {
///         read_struct!(reader, (address, amount));
///         Ok(Currency { address, amount })
///     }
///
///     fn write(writer: &mut EvmDataWriter, value: Self) {
///         EvmData::write(writer, (value.address, value.amount));
///     }
///
///     fn has_static_size() -> bool {
///         <(Address, U256)>::has_static_size()
///     }
/// }
/// ```
#[macro_export]
macro_rules! read_struct {
	($reader:ident, {$($field:ident: $type:ty),+}) => {
		use $crate::revert::BacktraceExt as _;
		let ($($field),*): ($($type),*) = $reader
			.read()
			.map_in_tuple_to_field(&[$(stringify!($field)),*])?;
	};
}

/// Helper to read arguments of a Solidity function.
/// Arguments are read in the provided order using the provided types.
/// Those types should match the ones in the Solidity file,
/// and identifiers used should match Solidity ones.
///
/// Identifiers written in Rust in snake_case are converted to
/// camelCase to match Solidity conventions.
///
/// ```rust,ignore
/// // Reading Solidity function `f(address ownner, uint256 accountIndex)`.
/// read_args!(handle, {owner: Address, account_index: U256});
/// let owner: H160 = owner.into();
/// ```
#[macro_export]
macro_rules! read_args {
	(@count) => (0usize);
	(@count $x:ident $($xs:ident)* ) => (1usize + read_args!(@count $($xs)*));
	($handle:ident, {$($field:ident: $type:ty),*}) => {
		$crate::data::paste! {
			let mut input = $handle.read_after_selector()?;
			input.expect_arguments(read_args!(@count $($field)*))?;
			$(
				let $field: $type = input.read().in_field(
					stringify!([<$field:camel>])
				)?;
			)*
		}
	};
}
