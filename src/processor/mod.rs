//! Trait for processing logs of a span after it is closed.
//!
//! See [`Processor`] for more details.

use crate::layer::{Tree, TreeLayer};

pub mod blocking;

#[cfg(feature = "sync")]
pub mod sync;

/// A type that can process [trace trees][crate::layer::Tree].
///
/// `Processor`s are responsible for both formatting and writing logs to their
/// intended destinations. This is typically implemented using
/// [`Formatter`][crate::formatter::Formatter],
/// [`tracing_subscriber::fmt::MakeWriter`], and [`std::io::Write`].
///
/// This trait is already implemented for
/// [`BlockingProcessor`][blocking::BlockingProcessor] and
/// [`AsyncProcessor`][sync::AsyncProcessor].
pub trait Processor: 'static + Sized {
    /// Converts the [`Processor`] into a [`TreeLayer`].
    ///
    /// This is the same as `TreeLayer::new(processor)`.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use tracing_forest::{blocking, formatter::pretty::Pretty, Processor};
    /// let _guard = tracing::subscriber::set_default({
    ///     blocking(Pretty::new(), std::io::stdout)
    ///         .into_layer()
    ///         .into_subscriber()
    /// });
    /// ```
    fn into_layer(self) -> TreeLayer<Self> {
        TreeLayer::new(self)
    }

    /// Processes the [`Tree`] of logs. Implementors of this trait are free to
    /// define what this means, such as:
    /// * Writing to a stdout or a file
    /// * Sending over a network
    /// * Storing in memory for later access
    /// * Ignoring
    fn process(&self, tree: Tree);
}
