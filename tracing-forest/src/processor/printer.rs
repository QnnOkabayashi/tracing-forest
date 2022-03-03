//! A [`Processor`] that formats and writes logs on the current thread.
//!
//! See [`Printer`] for more details.

use crate::builder::MakeStdout;
use crate::formatter::{Formatter, Pretty};
use crate::layer::Tree;
use crate::processor::Processor;
use std::io::Write;
use tracing_subscriber::fmt::MakeWriter;

use super::ProcessingError;

/// A [`Processor`] that blocks the current thread to format and write logs on
/// arrival.
pub struct Printer<F, W> {
    formatter: F,
    make_writer: W,
}

impl<F, W> Printer<F, W>
where
    F: 'static + Formatter,
    W: 'static + for<'a> MakeWriter<'a>,
{
    /// Create a new [`Printer`] with the provided [`Formatter`] and [`MakeWriter`].
    ///
    /// # Examples
    /// ```
    /// # use tracing_subscriber::{Layer, Registry};
    /// # use tracing_forest::layer::TreeLayer;
    /// # use tracing_forest::formatter::Pretty;
    /// # use tracing_forest::processor::{Printer, Processor};
    /// # fn main() {
    /// let _guard = tracing::subscriber::set_default({
    ///     let processor = Printer::new(Pretty::new(), std::io::stdout);
    ///     let layer = TreeLayer::new(processor);
    ///
    ///     layer.with_subscriber(Registry::default())
    /// });
    ///
    /// tracing::info!("blocking the thread to process this log >:)");
    /// # }
    /// ```
    pub fn new(formatter: F, make_writer: W) -> Self {
        Printer {
            formatter,
            make_writer,
        }
    }

    pub fn with_formatter<F2>(self, formatter: F2) -> Printer<F2, W>
    where
        F2: 'static + Formatter,
    {
        Printer::new(formatter, self.make_writer)
    }

    pub fn with_writer<W2>(self, make_writer: W2) -> Printer<F, W2>
    where
        W2: 'static + for<'a> MakeWriter<'a>,
    {
        Printer::new(self.formatter, make_writer)
    }
}

impl Default for Printer<Pretty, MakeStdout> {
    fn default() -> Self {
        Self {
            formatter: Pretty::new(),
            make_writer: std::io::stdout,
        }
    }
}

impl<F, W> Processor for Printer<F, W>
where
    F: 'static + Formatter,
    W: 'static + for<'a> MakeWriter<'a>,
{
    fn process(&self, tree: Tree) -> Result<(), ProcessingError> {
        let mut buf = Vec::with_capacity(0);

        self.formatter
            .fmt(tree, &mut buf)
            .expect("formatting failed");
        self.make_writer
            .make_writer()
            .write_all(&buf[..])
            .expect("writing failed");

        Ok(())
    }
}
