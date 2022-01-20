//! A [`Formatter`] that formats logs as JSON objects.
//!
//! See [`Json`] for more details.

use crate::formatter::Formatter;
use crate::layer::Tree;
use std::io::{self, Write};

/// Format logs as JSON objects.
#[derive(Clone, Copy)]
pub struct Json<const IS_COMPACT: bool> {
    _priv: (),
}

impl Json<true> {
    /// Create a new [`Json`] formatter with compact formatting.
    ///
    /// # Examples
    /// ```
    /// # use tracing_forest::formatter::Json;
    /// # use tracing_forest::processor::{BlockingProcessor, Processor};
    /// let json_subscriber = BlockingProcessor::new(Json::compact(), std::io::stdout)
    ///     .into_layer()
    ///     .into_subscriber();
    /// ```
    pub const fn compact() -> Self {
        Json { _priv: () }
    }
}

impl Formatter for Json<true> {
    fn fmt(&self, tree: Tree, mut writer: &mut Vec<u8>) -> io::Result<()> {
        serde_json::to_writer(&mut writer, &tree)?;
        writeln!(writer)
    }
}

impl Json<false> {
    /// Create a new [`Json`] formatter with pretty formatting.
    ///
    /// # Examples
    /// ```
    /// # use tracing_forest::formatter::Json;
    /// # use tracing_forest::processor::{BlockingProcessor, Processor};
    /// let json_pretty_subscriber = BlockingProcessor::new(Json::pretty(), std::io::stdout)
    ///     .into_layer()
    ///     .into_subscriber();
    /// ```
    pub const fn pretty() -> Self {
        Json { _priv: () }
    }
}

impl Formatter for Json<false> {
    fn fmt(&self, tree: Tree, mut writer: &mut Vec<u8>) -> io::Result<()> {
        serde_json::to_writer_pretty(&mut writer, &tree)?;
        writeln!(writer)
    }
}
