//! shared pallet common utilities

// Maximum value that fits into 22 bits
const MAX_U22: u32 = (1 << 22) - 1;
// Maximum value that fits into 10 bits
const MAX_U10: u32 = (1 << 10) - 1;

/// Combines the incrementing next_id with the parachain_id
///
/// Useful for NFT collections and asset_id creation
///
/// The first 22 bits are dedicated to the unique ID
/// The last 10 bits are dedicated to the parachain_id
/// |    22 next_id bits   | 10 parachain_id bits |
/// |          1           |   100   |
/// 0b000000000000000000001_0001100100
pub fn next_asset_uuid(next_id: u32, parachain_id: u32) -> Option<u32> {
	// Overflow check
	if next_id.checked_add(1_u32).is_none() {
		return None
	}
	// Check ids fit within limited bit sizes
	// next_id max 22 bits, parachain_id max 10 bits
	if next_id + 1_u32 > MAX_U22 || parachain_id > MAX_U10 {
		return None
	}

	// next_id is the first 22 bits, parachain_id is the last 10 bits
	let next_global_uuid: u32 = (next_id << 10) | parachain_id;
	Some(next_global_uuid)
}
