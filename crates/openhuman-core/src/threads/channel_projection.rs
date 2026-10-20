//! Durable ownership of provider conversations by OpenHuman threads.
//!
//! Channel events can arrive without an active turn and after a restart. This
//! index records the exact trusted channel/account and conversation address
//! observed during ingress, so later receipts and reactions can be projected
//! without guessing from a sender, recent message, or current session.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tinystoragedrivers::{CollectionSpec, ErrorKind, Precondition, Scope};
use tinychannels_bus::{ChannelRef, ConversationRef};

use crate::core::runtime::{DispatchCtx, Tenant, tenant::tenant_key};
use crate::storage::{self, StorageBackend, documents::Repo};

const DOMAIN: &str = "threads::channel_projection";
const COLLECTION: &str = "thread_channel_conversations";
const LOG_PREFIX: &str = "[threads:channel-projection]";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationBindingKey {
    tenant_key: String,
    channel: ChannelRef,
    conversation: ConversationRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationBinding {
    key: ConversationBindingKey,
    thread_id: String,
}

fn collections() -> Vec<CollectionSpec> {
    vec![CollectionSpec::new(COLLECTION)]
}

fn doc_id(key: &ConversationBindingKey) -> Result<String> {
    let encoded = serde_json::to_vec(key).context("serialize channel conversation key")?;
    Ok(hex::encode(Sha256::digest(encoded)))
}

fn tenant_key_for(ctx: &DispatchCtx) -> Result<(Tenant, String)> {
    let tenant = ctx.tenant().map_err(|error| anyhow!(error.to_string()))?;
    let key = tenant_key(&tenant, "");
    Ok((tenant, key))
}

fn repo_for(ctx: &DispatchCtx, tenant: &Tenant) -> Result<Repo> {
    let core = ctx
        .resolved()
        .ok_or_else(|| anyhow!("no trusted core context for channel projection"))?;
    let workspace = core
        .workspace_dir()
        .map_err(|error| anyhow!("resolve projection workspace: {error}"))?;
    let backend = storage::local::backend_for(&workspace)
        .context("open channel projection storage backend")?;
    // Conversation records are shared by the agents of one profile, like
    // OpenHuman thread records. The injective tenant key below still narrows
    // event correlation to the exact agent that installed the binding.
    let scope = storage::scope_from(
        tenant.profile.as_deref(),
        None,
        crate::core::runtime::mode::is_saas(),
    )?;
    Repo::on(&backend, &scope, DOMAIN, collections).context("open channel projection collection")
}

/// Bind a provider conversation to the canonical OpenHuman thread after
/// trusted inbound preparation has selected both identities.
///
/// Repeated binding to the same thread is idempotent. A conflicting binding
/// fails closed instead of silently rerouting later receipts or reactions.
pub(crate) fn bind_conversation(
    ctx: &DispatchCtx,
    channel: &ChannelRef,
    conversation: &ConversationRef,
    openhuman_thread_id: &str,
) -> Result<(), String> {
    let result = (|| -> Result<()> {
        if channel.id.is_empty() || conversation.id.is_empty() || openhuman_thread_id.is_empty() {
            return Err(anyhow!("channel, conversation, and thread identities must be non-empty"));
        }
        let (tenant, key) = tenant_key_for(ctx)?;
        let repo = repo_for(ctx, &tenant)?;
        bind_in(
            &repo,
            ConversationBindingKey {
                tenant_key: key,
                channel: channel.clone(),
                conversation: conversation.clone(),
            },
            openhuman_thread_id,
        )
    })();
    result.map_err(|error| format!("{LOG_PREFIX} bind: {error:#}"))
}

/// Resolve only an exact, previously trusted channel conversation binding.
///
/// Missing records return `None`; the caller must not guess a thread.
pub(crate) fn resolve_conversation(
    ctx: &DispatchCtx,
    channel: &ChannelRef,
    conversation: &ConversationRef,
) -> Result<Option<String>, String> {
    let result = (|| -> Result<Option<String>> {
        let (tenant, key) = tenant_key_for(ctx)?;
        let repo = repo_for(ctx, &tenant)?;
        resolve_in(
            &repo,
            &ConversationBindingKey {
                tenant_key: key,
                channel: channel.clone(),
                conversation: conversation.clone(),
            },
        )
    })();
    result.map_err(|error| format!("{LOG_PREFIX} resolve: {error:#}"))
}

fn bind_in(repo: &Repo, key: ConversationBindingKey, thread_id: &str) -> Result<()> {
    let id = doc_id(&key)?;
    let binding = ConversationBinding {
        key: key.clone(),
        thread_id: thread_id.to_string(),
    };
    let body = serde_json::to_value(&binding).context("serialize channel conversation binding")?;
    repo.run(|docs| async move {
        match docs.put(COLLECTION, &id, body, Precondition::Absent).await {
            Ok(_) => Ok(()),
            Err(error) if error.kind() == ErrorKind::Conflict => {
                let stored = docs.get(COLLECTION, &id).await?;
                let Some(stored) = stored else {
                    return Err(error);
                };
                let existing: ConversationBinding = serde_json::from_value(stored.doc)
                    .map_err(|e| tinystoragedrivers::StorageError::invalid_input(e.to_string()))?;
                if existing.key == key && existing.thread_id == binding.thread_id {
                    Ok(())
                } else {
                    Err(tinystoragedrivers::StorageError::conflict(
                        "provider conversation is already bound to another OpenHuman thread",
                    ))
                }
            }
            Err(error) => Err(error),
        }
    })?;
    Ok(())
}

fn resolve_in(repo: &Repo, key: &ConversationBindingKey) -> Result<Option<String>> {
    let id = doc_id(key)?;
    let expected = key.clone();
    let stored = repo.run(|docs| async move { docs.get(COLLECTION, &id).await })?;
    let Some(stored) = stored else {
        return Ok(None);
    };
    let binding: ConversationBinding = serde_json::from_value(stored.doc)
        .context("decode channel conversation binding")?;
    // Verify the full values after digest lookup, so a collision or malformed
    // legacy row can never broaden the mapping.
    if binding.key != expected || binding.thread_id.is_empty() {
        return Ok(None);
    }
    Ok(Some(binding.thread_id))
}

#[cfg(test)]
#[path = "channel_projection_tests.rs"]
mod tests;
