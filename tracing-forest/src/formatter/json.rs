//! A [`Formatter`] that formats logs as JSON objects.
//!
//! See [`Json`] for more details.

use crate::formatter::Formatter;
use crate::layer::Tree;
use std::io::{self, Write};

/// Format logs as JSON objects.
///
/// # Examples
///
/// ```json
/// {
///   "level": "TRACE",
///   "kind": {
///     "Span": {
///       "name": "first",
///       "nanos_total": 104667,
///       "nanos_nested": 13917,
///       "children": [
///         {
///           "level": "TRACE",
///           "kind": {
///             "Span": {
///               "name": "second",
///               "nanos_total": 13917,
///               "nanos_nested": 0,
///               "children": []
///             }
///           }
///         }
///       ]
///     }
///   }
/// }
/// ```
pub struct Json {
    /// Whether or not the logs should have compact formatting.
    compact: bool,
    #[doc(hidden)]
    _priv: (),
}

impl Json {
    /// Construct a new [`Json`] formatter.
    pub const fn new(compact: bool) -> Self {
        Json { compact, _priv: () }
    }
}

impl Formatter for Json {
    fn fmt(&self, tree: Tree, mut writer: &mut Vec<u8>) -> io::Result<()> {
        if self.compact {
            serde_json::to_writer(&mut writer, &tree)?;
        } else {
            serde_json::to_writer_pretty(&mut writer, &tree)?;
        }
        writeln!(writer)
    }
}
