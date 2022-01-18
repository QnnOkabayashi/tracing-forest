//! A [`Formatter`] that formats logs as JSON objects.
//!
//! See [`Json`] for more details.

use crate::formatter::Formatter;
use crate::layer::Tree;
use std::io::{self, Write};

/// Format logs as JSON objects.
pub struct Json<F> {
    _marker: F,
}

/// A flag for compact formatting.
///
/// See [`Json::compact`] for details.
pub struct Compact(());

impl Json<Compact> {
    /// Create a new [`Json`] formatter with compact formatting.
    ///
    /// # Example
    /// ```
    /// # use tracing_forest::{blocking, Json, Processor};
    /// let _guard = tracing::subscriber::set_default({
    ///     blocking(Json::compact(), std::io::stdout)
    ///         .into_layer()
    ///         .into_subscriber()
    /// });
    ///
    /// tracing::info!(answer = 42, "my event");
    /// ```
    /// ```json
    /// {"level":"INFO","kind":{"Event":{"tag":null,"message":"my event","fields":{"answer":"42"}}}}
    /// ```
    pub const fn compact() -> Self {
        Json {
            _marker: Compact(()),
        }
    }
}

impl Formatter for Json<Compact> {
    fn fmt(&self, tree: Tree, mut writer: &mut Vec<u8>) -> io::Result<()> {
        serde_json::to_writer(&mut writer, &tree)?;
        writeln!(writer)
    }
}

/// A flag for pretty formatting.
///
/// See [`Json::pretty`] for details.
pub struct Pretty(());

impl Json<Pretty> {
    /// Create a new [`Json`] formatter with pretty formatting.
    ///
    /// # Example
    /// ```
    /// # use tracing_forest::{blocking, Json, Processor};
    /// let _guard = tracing::subscriber::set_default({
    ///     blocking(Json::pretty(), std::io::stdout)
    ///         .into_layer()
    ///         .into_subscriber()
    /// });
    ///
    /// tracing::info!(answer = 42, "my event");
    /// ```
    /// ```json
    /// {
    ///   "level": "INFO",
    ///   "kind": {
    ///     "Event": {
    ///       "tag": null,
    ///       "message": "my event",
    ///       "fields": {
    ///         "answer": "42"
    ///       }
    ///     }
    ///   }
    /// }
    /// ```
    pub const fn pretty() -> Self {
        Json {
            _marker: Pretty(()),
        }
    }
}

impl Formatter for Json<Pretty> {
    fn fmt(&self, tree: Tree, mut writer: &mut Vec<u8>) -> io::Result<()> {
        serde_json::to_writer_pretty(&mut writer, &tree)?;
        writeln!(writer)
    }
}
