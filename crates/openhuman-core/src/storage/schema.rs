//! The host's composed v2 owner collection declarations.
//!
//! Owner crates expose pure `CollectionSpec` values. The host prepares those
//! declarations on each known scope before installing module grants; the
//! owner remains responsible for its own record shapes and migrations.

use std::collections::BTreeMap;

use tinystoragedrivers::CollectionSpec;

use super::StorageError;

/// Every collection this build links, with duplicate declarations merged and
/// validated before any backend is touched.
pub fn owner_collections() -> Result<Vec<CollectionSpec>, StorageError> {
    let mut collections = BTreeMap::<String, CollectionSpec>::new();
    for spec in declared_collections() {
        if let Some(existing) = collections.get(&spec.name) {
            let merged = existing.merge(&spec)?;
            collections.insert(spec.name.clone(), merged);
        } else {
            spec.validate()?;
            collections.insert(spec.name.clone(), spec);
        }
    }
    Ok(collections.into_values().collect())
}

fn declared_collections() -> Vec<CollectionSpec> {
    let mut collections = Vec::new();
    collections.extend(tinyagents_harness::storage::collections());
    collections.extend(tinyagents_session::port::collections());
    collections.extend(tinyagents_graph::storage::collections());
    collections.extend(tinyagents_tasks::collections());
    collections.extend(crate::security::approval::collection_specs());
    collections.extend(crate::security::devices::collection_specs());
    collections.extend(crate::desktop::notifications::collection_specs());
    collections.extend(crate::integrations::task_sources::collection_specs());
    collections.extend(crate::artifacts::collection_specs());
    collections.extend(crate::platform::cost::collection_specs());
    collections.extend(crate::config::schema::config_storage_collections());
    collections.extend(crate::profiles::collection_specs());
    collections.extend(crate::threads::store::collection_specs());
    collections.extend(crate::agent::orchestration::subagent_sessions::collection_specs());
    collections.extend(super::agents::collection_specs());

    #[cfg(feature = "skills")]
    collections.extend(tinyskills_drivers::collections());
    #[cfg(feature = "modules")]
    {
        collections.extend(tinymcp_bus::storage::collection_specs());
        collections.extend(tinychannels_bus::storage::collections());
        collections.extend(tinyconnectors_bus::storage::collections());
        collections.extend([
            CollectionSpec::new("box.boxes"),
            CollectionSpec::new("box.jail_index"),
        ]);
        collections.extend(tinyjuice_bus::storage::collections());
    }
    #[cfg(feature = "web3")]
    collections.extend(tinywallet_bus::storage::collections());
    #[cfg(feature = "tinymemes")]
    collections.extend(tinymemes_bus::storage::collections());
    #[cfg(feature = "inbound-queues")]
    collections.extend(tinyeventqueues_bus::storage::collection_specs());

    collections
}

/// Collections declared by one loadable module owner.
pub fn module_collections(owner: &str) -> Vec<CollectionSpec> {
    #[cfg(feature = "modules")]
    match owner {
        "mcp" => return tinymcp_bus::storage::collection_specs(),
        "channels" => return tinychannels_bus::storage::collections(),
        "connectors" => return tinyconnectors_bus::storage::collections(),
        "box" => {
            return vec![
                CollectionSpec::new("box.boxes"),
                CollectionSpec::new("box.jail_index"),
            ];
        }
        "juice" => return tinyjuice_bus::storage::collections(),
        _ => {}
    }
    #[cfg(feature = "web3")]
    if owner == "wallet" {
        return tinywallet_bus::storage::collections();
    }
    #[cfg(feature = "tinymemes")]
    if owner == "memes" {
        return tinymemes_bus::storage::collections();
    }
    #[cfg(feature = "inbound-queues")]
    if owner == "teq" {
        return tinyeventqueues_bus::storage::collection_specs();
    }
    let _ = owner;
    Vec::new()
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
