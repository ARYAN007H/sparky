//! Helpers the host domains' `storage_schema()` declarations and their v1 -> v2
//! steps share: owner construction, time conversion and id construction.
//!
//! A step is a pure function of its input (schema-v2 section 4), so none of
//! these read a clock or the environment.

use serde_json::{Map, Value};
use tinystoragedrivers::schema::Owner;
use tinystoragedrivers::{Key, StorageError};

/// The owner named `name`.
///
/// # Panics
///
/// When `name` is not a valid owner. Every caller passes a constant that
/// `storage_schema_tests` composes into the registry.
#[must_use]
pub fn owner(name: &str) -> Owner {
    #[allow(clippy::expect_used)]
    Owner::new(name).expect("a constant owner name satisfies the grammar")
}

/// Unix epoch milliseconds of an RFC 3339 timestamp, `None` when it does not
/// parse.
#[must_use]
pub fn rfc3339_ms(text: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|at| at.timestamp_millis())
}

/// The time held by `body[field]`, as epoch milliseconds: an RFC 3339 string.
#[must_use]
pub fn time_ms(body: &Value, field: &str) -> Option<i64> {
    body.get(field).and_then(Value::as_str).and_then(rfc3339_ms)
}

/// The document id for a single-part natural key.
///
/// # Errors
///
/// When `part` is empty.
pub fn key_of(part: &str) -> Result<String, StorageError> {
    Key::of([part]).map(|key| key.as_str().to_owned())
}

/// A body object builder that drops absent values, so a v2 body never carries
/// `null` for a field that was never set.
#[derive(Default)]
pub struct Body(Map<String, Value>);

impl Body {
    /// An empty body.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `field` to `value`.
    #[must_use]
    pub fn set(mut self, field: &str, value: impl Into<Value>) -> Self {
        self.0.insert(field.to_owned(), value.into());
        self
    }

    /// Sets `field` when `value` is present.
    #[must_use]
    pub fn set_some<V: Into<Value>>(self, field: &str, value: Option<V>) -> Self {
        match value {
            Some(value) => self.set(field, value),
            None => self,
        }
    }

    /// Copies `field` from `from` when it is present and not null.
    #[must_use]
    pub fn copy(self, from: &Value, field: &str) -> Self {
        match from.get(field) {
            Some(value) if !value.is_null() => self.set(field, value.clone()),
            _ => self,
        }
    }

    /// Copies `from_field` of `from` to `field` when present and not null.
    #[must_use]
    pub fn copy_as(self, from: &Value, from_field: &str, field: &str) -> Self {
        match from.get(from_field) {
            Some(value) if !value.is_null() => self.set(field, value.clone()),
            _ => self,
        }
    }

    /// The finished body.
    #[must_use]
    pub fn build(self) -> Value {
        Value::Object(self.0)
    }
}

#[cfg(test)]
#[path = "schema_support_tests.rs"]
mod tests;
