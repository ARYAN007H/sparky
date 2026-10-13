use std::path::PathBuf;

use super::types::{DurableSubagentSession, SubagentSessionStore};

impl SubagentSessionStore {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }

    pub fn load(&self) -> Result<Vec<DurableSubagentSession>, String> {
        super::store_documents::current(&self.workspace_dir)
            .and_then(|docs| docs.load())
            .map_err(|err| format!("{err:#}"))
    }

    pub fn save(&self, sessions: &[DurableSubagentSession]) -> Result<(), String> {
        super::store_documents::current(&self.workspace_dir)
            .and_then(|docs| docs.save(sessions))
            .map_err(|err| format!("{err:#}"))
    }
}
