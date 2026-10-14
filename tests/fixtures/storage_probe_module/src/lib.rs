//! A module that relays one raw call to the host's storage interface and
//! reports the answer, so a test can see exactly what the host decided for a
//! real loaded module.
//!
//! `Call` takes `[object, selector, request, confidential]` and returns
//! `{"ok": <reply>}` or `{"err": {"name": .., "message": ..}}`. `PutBlob`
//! takes `[key, len]` and sends an inline blob of `len` bytes the same way. The request is
//! forwarded untouched: the probe adds no scope, no module id and no grant,
//! because the host must not take any from a request.
use async_trait::async_trait;
use serde_json::{Value, json};
use tinybus::{CallContext, Connection, Error, Interface, InterfaceName, MemberName, Result};

const STORAGE: &str = "ai.tinyhumans.tinystorage.Storage";
const PROBE: &str = "ai.tinyhumans.test.StorageProbe";

struct Probe {
    connection: Connection,
}

#[async_trait]
impl Interface for Probe {
    fn name(&self) -> InterfaceName {
        InterfaceName::new(PROBE).expect("a fixed name")
    }

    fn members(&self) -> Vec<MemberName> {
        ["Call", "PutBlob"]
            .map(|member| MemberName::new(member).expect("a fixed name"))
            .to_vec()
    }

    async fn call(&self, member: &MemberName, args: Value) -> Result<Value> {
        let (object, selector, request, confidential): (String, String, Value, bool) =
            if member.as_str() == "PutBlob" {
                // `[key, len]`: the body is built here, because a body that
                // size cannot be sent to the probe over one bus frame either.
                let (key, len): (String, usize) = serde_json::from_value(args)?;
                (
                    "Blobs".into(),
                    "Put".into(),
                    json!({ "key": key, "bytes": vec![255u8; len] }),
                    false,
                )
            } else {
                serde_json::from_value(args)?
            };
        // Short, so a call the bus drops (an oversized frame) is an answer
        // here well inside the caller's own timeout.
        let proxy = self
            .connection
            .proxy(
                STORAGE,
                format!("/ai/tinyhumans/tinystorage/{object}"),
                STORAGE,
            )?
            .with_timeout(std::time::Duration::from_secs(6));
        let reply: Result<Value> = if confidential {
            proxy.call_confidential(&selector, (request,)).await
        } else {
            proxy.call(&selector, (request,)).await
        };
        Ok(match reply {
            Ok(value) => json!({ "ok": value }),
            Err(Error::MethodFailed { name, message }) => {
                json!({ "err": { "name": name, "message": message } })
            }
            Err(other) => json!({ "err": { "name": "transport", "message": other.to_string() } }),
        })
    }

    async fn call_with_context(
        &self,
        member: &MemberName,
        args: Value,
        _context: &CallContext,
    ) -> Result<Value> {
        self.call(member, args).await
    }
}

async fn setup(connection: Connection) -> Result<()> {
    connection
        .serve_at(
            "/probe".parse()?,
            Probe {
                connection: connection.clone(),
            },
        )
        .await
}

// The canonical SDK owns the native ABI; nothing unsafe is written here.
tinybus_module::module_export! {
    setup = setup,
    worker_threads = 1,
    provides = ["ai.tinyhumans.test.StorageProbe"],
    methods = ["Call", "PutBlob"],
    signals = [],
    requires = [],
    optional = [],
    lazy = false,
}
