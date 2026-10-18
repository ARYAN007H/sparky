//! Maps TinyMemory failures to the existing memory RPC error envelope.

use serde_json::json;

use crate::core::StructuredRpcError;

impl From<tinymemory_api::Error> for StructuredRpcError {
    fn from(error: tinymemory_api::Error) -> Self {
        let code = error.code();
        let message = match error {
            tinymemory_api::Error::NotFound(message) | tinymemory_api::Error::Config(message) => {
                format!("invalid request: {message}")
            }
            tinymemory_api::Error::Conflict(message) => format!("engine error: {message}"),
            error => error.to_string(),
        };

        Self {
            message,
            data: Some(json!({ "kind": code, "code": code })),
            expected_user_state: code != "ENGINE",
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
