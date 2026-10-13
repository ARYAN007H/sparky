use super::*;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use tinybus::broker::Broker;
use tinybus::transport::memory::MemoryBus;
use tinybus::{BusName, Connection};
use tinystoragedrivers::secrets::{DerivedKeys, KeyProvider};
use tinystoragedrivers::{MemoryStorage, Scope, StorageError};
use tinystoragedrivers_bus::{SecretNameRequest, SecretValue, SetSecretRequest};
use zeroize::Zeroizing;

use super::super::storage_grants::StorageGrant;
use super::super::storage_host::{module_scope, CallerDirectory, Environment, StorageHost};
use crate::storage::StorageBackend;

/// The pending slot is process-wide, so tests that attach take turns.
static TURN: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const MODULE: &str = "tinymcp";

struct Directory(BusName);

#[async_trait]
impl CallerDirectory for Directory {
    async fn module_for(&self, sender: &BusName) -> Option<String> {
        (sender == &self.0).then(|| MODULE.to_string())
    }
}

struct Env(Arc<dyn StorageBackend>);

#[async_trait]
impl Environment for Env {
    fn backend(&self) -> Option<Arc<dyn StorageBackend>> {
        Some(Arc::clone(&self.0))
    }
    fn module_scope(&self, module_id: &str) -> Result<Scope, StorageError> {
        module_scope(None, &Scope::local(), module_id)
    }
    fn grant(&self, module_id: &str) -> Option<StorageGrant> {
        super::super::storage_grants::declared(module_id)
    }
    fn keys(&self) -> Result<Arc<dyn KeyProvider>, StorageError> {
        Ok(Arc::new(DerivedKeys::new(Zeroizing::new([9u8; 32]))))
    }
    async fn admit_write(&self, _scope: &Scope) -> Result<(), StorageError> {
        Ok(())
    }
}

struct Bus {
    caller: Connection,
    _modules: ModuleHost,
}

/// A bus whose storage host is the linked module, over `backend`, with one
/// caller that the directory knows as `tinymcp`.
async fn boot(backend: Arc<dyn StorageBackend>) -> Bus {
    let transport = MemoryBus::new();
    let broker = Broker::new();
    broker.spawn(transport.clone());
    let modules = ModuleHost::new(broker);
    let host_connection = Connection::connect(transport.connect().await.unwrap())
        .await
        .unwrap();
    let caller = Connection::connect(transport.connect().await.unwrap())
        .await
        .unwrap();
    let directory = Directory(caller.unique_name().unwrap());
    let host = Arc::new(StorageHost::new(
        Arc::new(directory),
        Arc::new(Env(backend)),
        Arc::new(NoAudit),
    ));
    attach(&modules, &host_connection, host).await.unwrap();
    Bus {
        caller,
        _modules: modules,
    }
}

struct NoAudit;
impl super::super::storage_host::AuditSink for NoAudit {
    fn record(&self, _record: &super::super::storage_host::AuditRecord) {}
}

fn object(name: &str) -> tinybus::Proxy {
    Bus::proxy(name)
}

impl Bus {
    fn proxy(_name: &str) -> tinybus::Proxy {
        unreachable!("replaced below")
    }
}
