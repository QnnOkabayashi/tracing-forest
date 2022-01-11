//! Trait for formatting logs.
//!
//! See [`Formatter`] for more details.

use crate::layer::Tree;
use std::io;

pub mod pretty;

#[cfg(feature = "json")]
pub mod json;

/// A type that formats [`Tree`]s into a buffer.
/// 
/// [`Formatter`] types are typically used by [`Processor`]s in order to break 
/// down processing responsibilities into smaller, composable units.
/// 
/// If you're implementing a custom formatter, see the [layer module] 
/// documentation for internal representation details.
/// 
/// [`Processor`]: crate::processor::Processor
/// [layer module]: crate::layer
pub trait Formatter {
    /// Format a [`Tree`] into a buffer for writing.
    fn fmt(&self, tree: Tree, writer: &mut Vec<u8>) -> io::Result<()>;
}
