use primitive_types::H160;
use sp_runtime::DispatchError;
use seed_primitives::validators::validator::EventProofId;

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