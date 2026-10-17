use super::*;
use crate::agent::subagent_host::types::SubagentCheckpointData;

pub(in crate::agent::subagent_host) struct OpenHumanPersistence {
    checkpoint_dir: std::path::PathBuf,
    host_outcome: Arc<Mutex<HashMap<SubagentTaskKey, SubagentRunOutcome>>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct TerminalRecord {
    outcome: SubagentOutcome,
    #[serde(default)]
    agent_id: String,
    #[serde(default)]
    elapsed_ms: u64,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum CompatibleTerminalRecord {
    Current(TerminalRecord),
    Legacy(SubagentOutcome),
}

impl OpenHumanPersistence {
    pub(in crate::agent::subagent_host) fn new(checkpoint_dir: std::path::PathBuf) -> Self {
        Self {
            checkpoint_dir,
            host_outcome: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(in crate::agent::subagent_host) fn with_host_outcome(
        mut self,
        host_outcome: Arc<Mutex<HashMap<SubagentTaskKey, SubagentRunOutcome>>>,
    ) -> Self {
        self.host_outcome = host_outcome;
        self
    }

    fn set_host_disposition(
        &self,
        key: &SubagentTaskKey,
        disposition: tinyagents_orchestration::subagent::SubagentPersistenceDisposition,
    ) {
        if let Some(outcome) = self
            .host_outcome
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get_mut(key)
        {
            outcome.persistence_disposition = disposition;
        }
    }

    fn key_component(key: &SubagentTaskKey) -> String {
        fn hex(value: &str) -> String {
            value
                .as_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect()
        }
        let encoded = format!(
            "{}-{}-{}-{}",
            hex(&key.root_run_id),
            hex(&key.parent_run_id),
            hex(key.thread_id.as_deref().unwrap_or("")),
            hex(&key.task_id),
        );
        // A task key normally contains short UUID-like identifiers, for which
        // the reversible encoding preserves existing checkpoint paths. A
        // background child may inherit a rendered parent identifier, however;
        // hex-encoding that value can exceed the filesystem filename limit.
        // Hash only that exceptional form while keeping all key fields in the
        // digest input so distinct lifecycle scopes cannot alias.
        if encoded.len() <= 180 {
            encoded
        } else {
            format!("sha256-{:x}", Sha256::digest(encoded.as_bytes()))
        }
    }

    pub(super) fn pause_path(&self, key: &SubagentTaskKey) -> std::path::PathBuf {
        self.checkpoint_dir
            .join("lifecycle")
            .join(format!("{}.pause.json", Self::key_component(key)))
    }

    fn terminal_path(&self, key: &SubagentTaskKey) -> std::path::PathBuf {
        self.checkpoint_dir
            .join("lifecycle")
            .join(format!("{}.terminal.json", Self::key_component(key)))
    }

    fn index_path(&self, task_id: &str) -> std::path::PathBuf {
        self.checkpoint_dir
            .join("lifecycle")
            .join(format!("task-{}.index.json", Self::task_component(task_id)))
    }

    fn lock_path(&self, key: &SubagentTaskKey) -> std::path::PathBuf {
        self.checkpoint_dir
            .join("lifecycle")
            .join(format!("{}.lock", Self::key_component(key)))
    }

    fn index_lock_path(&self, task_id: &str) -> std::path::PathBuf {
        self.checkpoint_dir
            .join("lifecycle")
            .join(format!("task-{}.index.lock", Self::task_component(task_id)))
    }

    fn task_component(task_id: &str) -> String {
        let encoded = task_id
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if encoded.len() <= 180 {
            encoded
        } else {
            format!("sha256-{:x}", Sha256::digest(encoded.as_bytes()))
        }
    }

    fn lock_file(path: &std::path::Path) -> Result<std::fs::File, SubagentError> {
        let parent = path.parent().expect("lifecycle lock path has a parent");
        std::fs::create_dir_all(parent)
            .map_err(|error| SubagentError::Persistence(error.to_string()))?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)
            .map_err(|error| SubagentError::Persistence(error.to_string()))?;
        file.lock_exclusive()
            .map_err(|error| SubagentError::Persistence(error.to_string()))?;
        Ok(file)
    }

    fn key_lock(&self, key: &SubagentTaskKey) -> Result<std::fs::File, SubagentError> {
        Self::lock_file(&self.lock_path(key))
    }

    fn index_lock(&self, task_id: &str) -> Result<std::fs::File, SubagentError> {
        Self::lock_file(&self.index_lock_path(task_id))
    }

    fn read_index_keys(&self, task_id: &str) -> Result<Vec<SubagentTaskKey>, SubagentError> {
        let raw = match std::fs::read_to_string(self.index_path(task_id)) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(SubagentError::Persistence(error.to_string())),
        };
        let value: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|error| SubagentError::Persistence(error.to_string()))?;
        // Accept the short-lived single-key format written by the first
        // lifecycle extraction, but immediately write the collision-safe
        // vector format on the next mutation.
        let keys = value
            .get("keys")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| SubagentError::Persistence(error.to_string()))?
            .or_else(|| {
                value
                    .get("key")
                    .cloned()
                    .and_then(|key| serde_json::from_value(key).ok())
                    .map(|key| vec![key])
            })
            .unwrap_or_default();
        Ok(keys)
    }

    fn write_index_keys(
        &self,
        task_id: &str,
        keys: &[SubagentTaskKey],
    ) -> Result<(), SubagentError> {
        let path = self.index_path(task_id);
        if keys.is_empty() {
            match std::fs::remove_file(path) {
                Ok(()) => return Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(SubagentError::Persistence(error.to_string())),
            }
        }
        Self::replace_file(
            &path,
            serde_json::to_vec(&serde_json::json!({"keys": keys}))
                .map_err(|error| SubagentError::Persistence(error.to_string()))?,
        )
    }

    fn add_index_key(&self, key: &SubagentTaskKey) -> Result<(), SubagentError> {
        let _index_lock = self.index_lock(&key.task_id)?;
        let mut keys = self.read_index_keys(&key.task_id)?;
        if !keys.contains(key) {
            keys.push(key.clone());
            self.write_index_keys(&key.task_id, &keys)?;
        }
        Ok(())
    }

    fn remove_index_key(&self, key: &SubagentTaskKey) -> Result<(), SubagentError> {
        let _index_lock = self.index_lock(&key.task_id)?;
        let mut keys = self.read_index_keys(&key.task_id)?;
        keys.retain(|candidate| candidate != key);
        self.write_index_keys(&key.task_id, &keys)
    }

    fn replace_file(path: &std::path::Path, bytes: Vec<u8>) -> Result<(), SubagentError> {
        let parent = path.parent().expect("lifecycle state path has a parent");
        std::fs::create_dir_all(parent)
            .map_err(|error| SubagentError::Persistence(error.to_string()))?;
        let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        std::fs::write(&temporary, bytes)
            .map_err(|error| SubagentError::Persistence(error.to_string()))?;
        match std::fs::rename(&temporary, path) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = std::fs::remove_file(&temporary);
                Err(SubagentError::Persistence(error.to_string()))
            }
        }
    }

    fn insert_file(path: &std::path::Path, bytes: Vec<u8>) -> Result<bool, SubagentError> {
        let parent = path.parent().expect("lifecycle state path has a parent");
        std::fs::create_dir_all(parent)
            .map_err(|error| SubagentError::Persistence(error.to_string()))?;
        let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        std::fs::write(&temporary, bytes)
            .map_err(|error| SubagentError::Persistence(error.to_string()))?;
        match std::fs::hard_link(&temporary, path) {
            Ok(()) => {
                let _ = std::fs::remove_file(&temporary);
                Ok(true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let _ = std::fs::remove_file(&temporary);
                Ok(false)
            }
            Err(error) => {
                let _ = std::fs::remove_file(&temporary);
                Err(SubagentError::Persistence(error.to_string()))
            }
        }
    }

    fn read_pause_checkpoint(
        &self,
        key: &SubagentTaskKey,
    ) -> Result<Option<SubagentCheckpointData>, SubagentError> {
        let raw = match std::fs::read_to_string(self.pause_path(key)) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(SubagentError::Persistence(error.to_string())),
        };
        let checkpoint = serde_json::from_str::<SubagentCheckpointData>(&raw)
            .map_err(|error| SubagentError::Persistence(error.to_string()))?;
        if checkpoint.task_key.as_ref() != Some(key) {
            return Err(SubagentError::Persistence("checkpoint key mismatch".into()));
        }
        Ok(Some(checkpoint))
    }

    fn read_terminal_record(
        &self,
        key: &SubagentTaskKey,
    ) -> Result<Option<TerminalRecord>, SubagentError> {
        let raw = match std::fs::read_to_string(self.terminal_path(key)) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(SubagentError::Persistence(error.to_string())),
        };
        match serde_json::from_str(&raw)
            .map_err(|error| SubagentError::Persistence(error.to_string()))?
        {
            CompatibleTerminalRecord::Current(record) => Ok(Some(record)),
            CompatibleTerminalRecord::Legacy(outcome) => Ok(Some(TerminalRecord {
                outcome,
                agent_id: String::new(),
                elapsed_ms: 0,
            })),
        }
    }

    pub(super) fn terminal_outcome(
        &self,
        key: &SubagentTaskKey,
    ) -> Result<Option<SubagentOutcome>, SubagentError> {
        let _key_lock = self.key_lock(key)?;
        Ok(self.read_terminal_record(key)?.map(|record| record.outcome))
    }

    pub(super) fn durable_host_metadata(
        &self,
        key: &SubagentTaskKey,
    ) -> Result<Option<(String, u64)>, SubagentError> {
        let _key_lock = self.key_lock(key)?;
        if let Some(record) = self.read_terminal_record(key)? {
            return Ok(
                (!record.agent_id.is_empty()).then_some((record.agent_id, record.elapsed_ms))
            );
        }
        Ok(self.read_pause_checkpoint(key)?.and_then(|checkpoint| {
            (!checkpoint.agent_id.is_empty())
                .then_some((checkpoint.agent_id, checkpoint.elapsed_ms))
        }))
    }

    pub(crate) fn checkpoint_resume(checkpoint: &SubagentCheckpointData) -> SubagentResume {
        SubagentResume {
            history: checkpoint
                .history
                .iter()
                .map(crate::agent::message_convert::chat_message_to_message)
                .collect(),
            checkpoint: checkpoint.resume_checkpoint.clone(),
            metadata: checkpoint.resume_metadata.clone(),
        }
    }

    fn checkpoint_outcome(checkpoint: SubagentCheckpointData) -> SubagentOutcome {
        let resume = Self::checkpoint_resume(&checkpoint);
        SubagentOutcome {
            task_id: checkpoint.task_id,
            output: checkpoint.output,
            history: checkpoint
                .history
                .iter()
                .map(crate::agent::message_convert::chat_message_to_message)
                .collect(),
            status: SubagentOutcomeKind::AwaitingInput(SubagentPause {
                reason: checkpoint.question,
                resume,
            }),
            usage: UsageTotals {
                calls: checkpoint.iterations as u64,
                usage: Usage {
                    input_tokens: checkpoint.usage.input_tokens,
                    output_tokens: checkpoint.usage.output_tokens,
                    total_tokens: checkpoint
                        .usage
                        .input_tokens
                        .saturating_add(checkpoint.usage.output_tokens),
                    cache_read_tokens: checkpoint.usage.cached_input_tokens,
                    // An unknown cost crosses as no charge at all, never as $0.
                    charged_amount: checkpoint
                        .usage
                        .cost()
                        .usd()
                        .map(|usd| ChargedAmount::usd_micros((usd * 1_000_000.0).round() as i64)),
                    ..Usage::default()
                },
            },
            artifacts: checkpoint
                .artifact_paths
                .into_iter()
                .map(|id| ArtifactReference {
                    id,
                    ..ArtifactReference::default()
                })
                .collect(),
            ..SubagentOutcome::cancelled(String::new())
        }
    }

    fn live_checkpoint(
        &self,
        key: &SubagentTaskKey,
    ) -> Result<Option<SubagentCheckpointData>, SubagentError> {
        let _key_lock = self.key_lock(key)?;
        if self.terminal_path(key).exists() {
            return Ok(None);
        }
        self.read_pause_checkpoint(key)
    }

    pub(super) fn checkpoint_for_task(
        &self,
        task_id: &str,
    ) -> Result<SubagentCheckpointData, SubagentRunError> {
        if !crate::agent::subagent_host::ops::is_safe_task_id(task_id) {
            return Err(SubagentRunError::Provider(anyhow::anyhow!(
                "unsafe task id"
            )));
        }
        let keys = {
            let _index_lock = self.index_lock(task_id).map_err(map_lifecycle_error)?;
            self.read_index_keys(task_id).map_err(map_lifecycle_error)?
        };
        let mut live = Vec::new();
        for key in keys {
            if let Some(checkpoint) = self.live_checkpoint(&key).map_err(map_lifecycle_error)? {
                live.push(checkpoint);
            }
        }
        match live.len() {
            1 => Ok(live.remove(0)),
            0 => Err(SubagentRunError::Provider(anyhow::anyhow!(
                "no live subagent pause checkpoint for task id"
            ))),
            _ => Err(SubagentRunError::Provider(anyhow::anyhow!(
                "ambiguous subagent task id; select a scoped lifecycle key"
            ))),
        }
    }

    pub(crate) fn recover_key(
        checkpoint_dir: &std::path::Path,
        task_id: &str,
    ) -> Result<SubagentTaskKey, SubagentRunError> {
        let persistence = Self::new(checkpoint_dir.to_path_buf());
        let checkpoint = persistence.checkpoint_for_task(task_id)?;
        checkpoint.task_key.ok_or_else(|| {
            SubagentRunError::Provider(anyhow::anyhow!("checkpoint omitted original task key"))
        })
    }
}

#[async_trait]
impl tinyagents_orchestration::subagent::SubagentPersistence for OpenHumanPersistence {
    async fn load_terminal(
        &self,
        key: &SubagentTaskKey,
    ) -> Result<Option<SubagentOutcome>, SubagentError> {
        let _key_lock = self.key_lock(key)?;
        Ok(self.read_terminal_record(key)?.map(|record| record.outcome))
    }
    async fn load(&self, key: &SubagentTaskKey) -> Result<Option<SubagentResume>, SubagentError> {
        let _key_lock = self.key_lock(key)?;
        if self.terminal_path(key).exists() {
            return Ok(None);
        }
        Ok(self
            .read_pause_checkpoint(key)?
            .as_ref()
            .map(Self::checkpoint_resume))
    }

    async fn load_pause(
        &self,
        key: &SubagentTaskKey,
    ) -> Result<Option<SubagentOutcome>, SubagentError> {
        let _key_lock = self.key_lock(key)?;
        if self.terminal_path(key).exists() {
            return Ok(None);
        }
        Ok(self
            .read_pause_checkpoint(key)?
            .map(Self::checkpoint_outcome))
    }

    async fn save_pause(
        &self,
        pause: tinyagents_orchestration::subagent::PersistedSubagentPause,
    ) -> Result<SubagentPausePersistenceDisposition, SubagentError> {
        let _key_lock = self.key_lock(&pause.key)?;
        if self.terminal_path(&pause.key).exists() {
            self.set_host_disposition(
                &pause.key,
                tinyagents_orchestration::subagent::SubagentPersistenceDisposition::TerminalExisting,
            );
            return Ok(SubagentPausePersistenceDisposition::TerminalExisting);
        }
        let paused = match &pause.outcome.status {
            SubagentOutcomeKind::AwaitingInput(paused) => paused,
            _ => {
                return Err(SubagentError::Persistence(
                    "attempted to persist a non-paused subagent outcome as a pause".into(),
                ));
            }
        };
        let metadata = &paused.resume.metadata;
        let optional_metadata = |name: &str| {
            metadata
                .get(name)
                .filter(|value| !value.is_empty())
                .cloned()
        };
        let checkpoint = SubagentCheckpointData {
            task_key: Some(pause.key.clone()),
            task_id: pause.key.task_id.clone(),
            agent_id: optional_metadata("agent_id").unwrap_or_default(),
            worker_thread_id: optional_metadata("worker_thread_id"),
            history: pause
                .outcome
                .history
                .iter()
                .filter_map(tinyagents_runtime::transcript::message_to_native_transcript)
                .collect(),
            question: paused.reason.clone(),
            options: None,
            skill_filter_override: optional_metadata("skill_filter_override"),
            model_override: optional_metadata("model_override"),
            created_at: chrono::Utc::now().to_rfc3339(),
            resume_checkpoint: paused.resume.checkpoint.clone(),
            resume_metadata: metadata.clone(),
            output: pause.outcome.output.clone(),
            iterations: pause.outcome.usage.calls as usize,
            usage: SubagentUsage {
                input_tokens: pause.outcome.usage.usage.input_tokens,
                output_tokens: pause.outcome.usage.usage.output_tokens,
                cached_input_tokens: pause.outcome.usage.usage.cache_read_tokens,
                charged_amount_usd: pause
                    .outcome
                    .usage
                    .usage
                    .charged_amount
                    .map(|amount| amount.micros as f64 / 1_000_000.0)
                    .unwrap_or_default(),
                cost_source: cost_source_of(pause.outcome.usage.usage.charged_amount.as_ref()),
            },
            artifact_paths: pause
                .outcome
                .artifacts
                .iter()
                .map(|artifact| artifact.id.clone())
                .collect(),
            elapsed_ms: self
                .host_outcome
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(&pause.key)
                .map(|outcome| outcome.elapsed.as_millis() as u64)
                .unwrap_or_default(),
        };
        let json = serde_json::to_vec_pretty(&checkpoint)
            .map_err(|error| SubagentError::Persistence(error.to_string()))?;
        let path = self.pause_path(&pause.key);
        let existing = self.read_pause_checkpoint(&pause.key)?;
        let disposition = match (pause.replaces.as_ref(), existing) {
            (None, None) => {
                // Index first: a failed data write can leave a stale index,
                // which readers filter by the live pause record. A successful
                // pause can never be unresumable through a missing index.
                self.add_index_key(&pause.key)?;
                if Self::insert_file(&path, json)? {
                    Ok(SubagentPausePersistenceDisposition::Inserted)
                } else {
                    Ok(SubagentPausePersistenceDisposition::Existing)
                }
            }
            (Some(expected), Some(current)) if Self::checkpoint_resume(&current) == *expected => {
                self.add_index_key(&pause.key)?;
                Self::replace_file(&path, json)?;
                Ok(SubagentPausePersistenceDisposition::Replaced)
            }
            _ => Ok(SubagentPausePersistenceDisposition::Existing),
        }?;
        let host_disposition = match disposition {
            SubagentPausePersistenceDisposition::Inserted => {
                tinyagents_orchestration::subagent::SubagentPersistenceDisposition::PauseCommitted
            }
            SubagentPausePersistenceDisposition::Replaced => {
                tinyagents_orchestration::subagent::SubagentPersistenceDisposition::PauseReplaced
            }
            SubagentPausePersistenceDisposition::Existing => {
                tinyagents_orchestration::subagent::SubagentPersistenceDisposition::PauseExisting
            }
            SubagentPausePersistenceDisposition::TerminalExisting => {
                tinyagents_orchestration::subagent::SubagentPersistenceDisposition::TerminalExisting
            }
        };
        self.set_host_disposition(&pause.key, host_disposition);
        Ok(disposition)
    }

    async fn record_terminal(
        &self,
        key: &SubagentTaskKey,
        outcome: &SubagentOutcome,
        replaces: Option<&SubagentResume>,
    ) -> Result<SubagentTerminalPersistenceDisposition, SubagentError> {
        let _key_lock = self.key_lock(key)?;
        if self.terminal_path(key).exists() {
            self.set_host_disposition(
                key,
                tinyagents_orchestration::subagent::SubagentPersistenceDisposition::TerminalExisting,
            );
            return Ok(SubagentTerminalPersistenceDisposition::Existing);
        }
        if let Some(current) = self.read_pause_checkpoint(key)? {
            let can_consume_pause =
                replaces.is_some_and(|expected| Self::checkpoint_resume(&current) == *expected);
            if !can_consume_pause {
                self.set_host_disposition(
                    key,
                    tinyagents_orchestration::subagent::SubagentPersistenceDisposition::PauseExisting,
                );
                return Ok(SubagentTerminalPersistenceDisposition::PauseExisting);
            }
        } else if replaces.is_some() {
            return Err(SubagentError::Persistence(
                "terminal continuation lost its expected durable pause".into(),
            ));
        }
        let path = self.terminal_path(key);
        let host = self
            .host_outcome
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(key)
            .cloned();
        let record = serde_json::to_vec(&TerminalRecord {
            outcome: outcome.clone(),
            agent_id: host
                .as_ref()
                .map(|outcome| outcome.agent_id.clone())
                .unwrap_or_default(),
            elapsed_ms: host
                .as_ref()
                .map(|outcome| outcome.elapsed.as_millis() as u64)
                .unwrap_or_default(),
        })
        .map_err(|error| SubagentError::Persistence(error.to_string()))?;
        let disposition = match Self::insert_file(&path, record)? {
            true => {
                // Terminal is the durable winner. Retiring a matching pause
                // afterwards cannot reopen the lifecycle because `load` first
                // observes this terminal marker.
                let _ = std::fs::remove_file(self.pause_path(key));
                // A stale task-id index is harmless because recovery filters
                // terminal keys. Never turn a durable terminal success into an
                // error merely because cleanup of a derived index failed.
                if let Err(error) = self.remove_index_key(key) {
                    tracing::warn!(task_id = %key.task_id, error = %error, "[subagent] terminal committed but task-id index cleanup failed");
                }
                Ok(SubagentTerminalPersistenceDisposition::Inserted)
            }
            false => Ok(SubagentTerminalPersistenceDisposition::Existing),
        }?;
        let host_disposition = match disposition {
            SubagentTerminalPersistenceDisposition::Inserted => {
                tinyagents_orchestration::subagent::SubagentPersistenceDisposition::TerminalInserted
            }
            SubagentTerminalPersistenceDisposition::Existing => {
                tinyagents_orchestration::subagent::SubagentPersistenceDisposition::TerminalExisting
            }
            SubagentTerminalPersistenceDisposition::PauseExisting => {
                tinyagents_orchestration::subagent::SubagentPersistenceDisposition::PauseExisting
            }
        };
        self.set_host_disposition(key, host_disposition);
        Ok(disposition)
    }
}
