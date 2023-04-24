// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use sc_utils::notification::{NotificationSender, NotificationStream, TracingKeyStr};

use seed_primitives::ethy::VersionedEventProof;

/// The sending half of the event proof channel(s).
///
/// Used to send notifications about event proofs generated after a majority of validators have
/// witnessed the event
pub type EthyEventProofSender = NotificationSender<VersionedEventProof>;

/// The receiving half of the event proof channel.
///
/// Used to receive notifications about event proofs generated at the end of a ETHY round.
pub type EthyEventProofStream = NotificationStream<VersionedEventProof, EthyEventProofTracingKey>;

/// Provides tracing key for ETHY event proof stream.
#[derive(Clone)]
pub struct EthyEventProofTracingKey;
impl TracingKeyStr for EthyEventProofTracingKey {
	const TRACING_KEY: &'static str = "mpsc_ethy_event_proof_notification_stream";
}
