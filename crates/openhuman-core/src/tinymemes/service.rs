//! The bus half of the `tinymemes` module: the host callbacks it is served
//! (`Host` interface) and the client call that starts a remix.

use async_trait::async_trait;
use tinybus::{CallContext, Connection, InterfaceName, MemberName, ObjectPath};
use tinymemes_bus::{
    host_methods, remix_methods, RemixRefusal, RemixRequest, RemixResponse, RemixResult,
    HOST_INTERFACE, HOST_METHODS, HOST_PATH, REMIX_INTERFACE, REMIX_PATH, STATE_INTERFACE,
};

use super::callbacks;
use crate::config::Config;

/// Registry id of the module the callbacks serve.
pub(crate) const MODULE_ID: &str = "tinymemes";

struct HostService {
    connection: Connection,
}

#[async_trait]
impl tinybus::Interface for HostService {
    fn name(&self) -> InterfaceName {
        InterfaceName::new(HOST_INTERFACE).expect("contract interface name is well formed")
    }

    fn members(&self) -> Vec<MemberName> {
        HOST_METHODS
            .iter()
            .filter_map(|member| MemberName::new(*member).ok())
            .collect()
    }

    async fn call(
        &self,
        _member: &MemberName,
        _args: serde_json::Value,
    ) -> tinybus::Result<serde_json::Value> {
        // Without the broker's context there is no caller to authorize.
        Err(tinybus::Error::failed("unauthenticated tinymemes caller"))
    }

    async fn call_with_context(
        &self,
        member: &MemberName,
        args: serde_json::Value,
        context: &CallContext,
    ) -> tinybus::Result<serde_json::Value> {
        // Only the loaded `tinymemes` module may ask: its bus name's owner
        // must be the authenticated sender.
        let Some(sender) = context.authenticated_sender() else {
            return Err(tinybus::Error::failed("unauthenticated tinymemes caller"));
        };
        let owner = self
            .connection
            .name_owner(tinymemes_bus::STATE_INTERFACE)
            .await
            .ok()
            .flatten();
        if owner.as_ref() != Some(sender) {
            log::warn!("[tinymemes] refused a host callback from a peer that is not the module");
            return Err(tinybus::Error::failed("unknown tinymemes caller"));
        }
        // The module is the only caller, but it serves many remixes: the
        // correlation id says which workspace this callback answers for.
        let correlation = callbacks::correlation_of(&args).map(str::to_owned);
        if member.as_str() == host_methods::END {
            // Forgetting an unknown id is harmless, so `End` needs no session.
            if let Some(correlation) = &correlation {
                callbacks::end(correlation);
            }
            return serde_json::to_value(tinymemes_bus::HostResult::<()>::Ok(()))
                .map_err(|_| tinybus::Error::failed("invalid tinymemes host response"));
        }
        let Some(session) = correlation.as_deref().and_then(callbacks::session) else {
            log::debug!("[tinymemes] host callback with no live remix for its correlation id");
            return Err(tinybus::Error::failed("no such tinymemes remix"));
        };
        callbacks::dispatch(&session, member.as_str(), args).await
    }
}

/// Serve the module's host callbacks on the module bus.
pub(crate) async fn install(connection: &Connection) -> tinybus::Result<()> {
    connection
        .serve_at(
            ObjectPath::new(HOST_PATH)?,
            HostService {
                connection: connection.clone(),
            },
        )
        .await
}

/// Remix one reply through the loaded module, serving its callbacks for
/// `config`'s workspace meanwhile. `request.correlation` is set here.
pub(crate) async fn remix(
    config: &Config,
    mut request: RemixRequest,
) -> Result<RemixResponse, String> {
    if !config.components.memes {
        return Err("the memes component is off".to_owned());
    }
    crate::modules::ensure_loaded(config, MODULE_ID).await?;
    let runtime = crate::modules::host::runtime()
        .await
        .map_err(|error| format!("the module bus is not running: {error}"))?;
    let proxy = runtime
        .connection()
        .proxy(STATE_INTERFACE, REMIX_PATH, REMIX_INTERFACE)
        .map_err(|error| error.to_string())?;
    let registration = callbacks::register(config);
    request.correlation = registration.correlation().to_owned();
    let outcome: tinybus::Result<RemixResult> =
        proxy.call(remix_methods::REMIX, (request,)).await;
    match outcome {
        Ok(Ok(response)) => {
            // Background research may still call back; the module's `End`
            // (or the age sweep) retires the entry.
            registration.keep_for_background();
            Ok(response)
        }
        Ok(Err(refusal)) => Err(match refusal {
            RemixRefusal::Closing => "the tinymemes module is closing".to_owned(),
            RemixRefusal::Unauthenticated => "the tinymemes module refused the caller".to_owned(),
        }),
        Err(error) => Err(format!("tinymemes remix call failed: {error}")),
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
