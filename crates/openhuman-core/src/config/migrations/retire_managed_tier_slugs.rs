//! Generic schema-version progression for the retired 12 → 13 model
//! normalization step. Provider-specific normalization runs through the host
//! registration independently of schema-version gates.

use crate::config::Config;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MigrationStats {
    pub rewritten: usize,
}

pub fn run(_config: &mut Config) -> anyhow::Result<MigrationStats> {
    Ok(MigrationStats::default())
}
