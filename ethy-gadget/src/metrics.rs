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

//! ETHY Prometheus metrics definition

use substrate_prometheus_endpoint::{register, Counter, Gauge, PrometheusError, Registry, U64};

/// ETHY metrics exposed through Prometheus
pub(crate) struct Metrics {
	/// Current active validator set id
	pub ethy_validator_set_id: Gauge<U64>,
	/// Total number of votes sent by this node
	pub ethy_witness_sent: Counter<U64>,
}

impl Metrics {
	pub(crate) fn register(registry: &Registry) -> Result<Self, PrometheusError> {
		Ok(Self {
			ethy_validator_set_id: register(
				Gauge::new("ethy_validator_set_id", "Current ETHY active validator set id.")?,
				registry,
			)?,
			ethy_witness_sent: register(
				Counter::new("ethy_witness_sent", "Number of witnesses sent by this node")?,
				registry,
			)?,
		})
	}
}

// Note: we use the `format` macro to convert an expr into a `u64`. This will fail,
// if expr does not derive `Display`.
#[macro_export]
macro_rules! metric_set {
	($self:ident, $m:ident, $v:expr) => {{
		let val: u64 = format!("{}", $v).parse().unwrap();

		if let Some(metrics) = $self.metrics.as_ref() {
			metrics.$m.set(val);
		}
	}};
}

#[macro_export]
macro_rules! metric_inc {
	($self:ident, $m:ident) => {{
		if let Some(metrics) = $self.metrics.as_ref() {
			metrics.$m.inc();
		}
	}};
}
