// Copyright 2019-2022 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

use sha3::{digest::generic_array::GenericArray, Digest, Keccak256};
use typenum::U32;

#[precompile_utils_macro::generate_function_selector]
pub enum Action {
	Toto = "toto()",
	Tata = "tata()",
}

#[test]
fn test_keccak256() {
	assert_eq!(
		&precompile_utils_macro::keccak256!(""),
		<GenericArray<u8, U32> as AsRef<[u8]>>::as_ref(&Keccak256::digest(b"")),
	);
	assert_eq!(
		&precompile_utils_macro::keccak256!("toto()"),
		<GenericArray<u8, U32> as AsRef<[u8]>>::as_ref(&Keccak256::digest(b"toto()")),
	);
	assert_ne!(
		&precompile_utils_macro::keccak256!("toto()"),
		<GenericArray<u8, U32> as AsRef<[u8]>>::as_ref(&Keccak256::digest(b"tata()")),
	);
}

#[test]
fn test_generate_function_selector() {
	assert_eq!(&(Action::Toto as u32).to_be_bytes()[..], &Keccak256::digest(b"toto()")[0..4],);
	assert_eq!(&(Action::Tata as u32).to_be_bytes()[..], &Keccak256::digest(b"tata()")[0..4],);
	assert_ne!(Action::Toto as u32, Action::Tata as u32);
}
