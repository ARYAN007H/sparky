//! Native trace export for completed TinyAgents runs.
//!
//! OpenHuman owns product attribution, privacy gates, local export settings,
//! and the authenticated backend transport. TinyAgents owns the span
//! collector, journal projection, and OTLP payload format.

mod export;

use tinyagents_harness::observability::trace_export::ExportBrand;

/// Product identity stamped onto exported telemetry.
pub(crate) fn export_brand() -> ExportBrand<'static> {
    ExportBrand {
        product: "openhuman",
        version: env!("CARGO_PKG_VERSION"),
    }
}

pub(crate) use export::{export_run_trace_from_journal, export_subagent_journal_trace};
