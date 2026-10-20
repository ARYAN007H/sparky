//! Generic schema-version progression for the retired 7 → 8 provider
//! normalization step. Provider-specific normalization is registered by the
//! host and runs independently of schema-version gates.

use crate::config::Config;

#[derive(Debug, Default, Clone)]
pub struct MigrationStats;

pub fn run(_config: &mut Config) -> anyhow::Result<MigrationStats> {
    Ok(MigrationStats)
}
