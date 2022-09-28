use sp_core::H160;
use seed_primitives::validators::validator::EventProofId;
use sp_runtime::DispatchError;

/// Interface for an Xrpl event bridge
/// Generates proof of events for the remote
/// chain
pub trait XrplBridge {
	/// Send an event via the bridge for relaying to Xrpl
	///
	/// `source` the (pseudo) address of the pallet that submitted the event
	/// `destination` address on Xrpl
	/// `message` data
	///
	/// Returns a unique event proofId on success
	fn send_event(
		source: &H160,
		destination: &H160,
		message: &[u8],
	) -> Result<EventProofId, DispatchError>;
}
/// Verifies correctness of state on Ethereum i.e. by issuing `eth_call`s
pub trait XrplCallOracle {
	/// EVM address type
	type Address;
	/// Identifies call requests
	type CallId;
	/// Performs an `eth_call` on address `target` with `input` at (or near) `block_hint`
	///
	/// Returns a call Id for subscribers (impl `EthCallOracleSubscriber`)
	fn checked_eth_call(
		target: &Self::Address,
		input: &[u8],
		timestamp: u64,
		block_hint: u64,
		max_block_look_behind: u64,
	) -> Self::CallId;
}

impl XrplCallOracle for () {
	type Address = H160;
	type CallId = u64;
	fn checked_eth_call(
		_target: &Self::Address,
		_input: &[u8],
		_timestamp: u64,
		_block_hint: u64,
		_max_block_look_behind: u64,
	) -> Self::CallId {
		0_u64
	}
}
