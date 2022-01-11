//! A [`Processor`] that formats and writes logs on the current thread.
//!
//! See [`BlockingProcessor`] for more details.

use crate::formatter::Formatter;
use crate::layer::Tree;
use crate::processor::Processor;
use std::io::Write;
use tracing_subscriber::fmt::MakeWriter;

/// A [`Processor`] that blocks the current thread to format and write logs on
/// arrival.
///
/// To initialize a new [`BlockingProcessor`], see [`blocking`].
pub struct BlockingProcessor<F, W> {
    formatter: F,
    make_writer: W,
}

impl<F, W> Processor for BlockingProcessor<F, W>
where
    F: 'static + Formatter,
    W: 'static + for<'a> MakeWriter<'a>,
{
    fn process(&self, tree: Tree) {
        let mut buf = Vec::with_capacity(0);

        #[allow(clippy::expect_used)]
        self.formatter.fmt(tree, &mut buf).expect("formatting failed");
        #[allow(clippy::unwrap_used)]
        self.make_writer.make_writer().write_all(&buf[..]).unwrap();
    }
}

/// Initialize a new [`BlockingProcessor`].
///
/// ## Note
///
/// This function is usually called by the [`tracing_forest::main`][crate::main]
/// and [`tracing_forest::test`][crate::test] attribute macros, which are the
/// prefered method for creating [`BlockingProcessor`]s.
///
/// ## Examples
///
/// ```
/// # use tracing_forest::{blocking, formatter::pretty::Pretty, Processor};
/// fn main() {
///     tracing::subscriber::set_global_default({
///         blocking(Pretty::new(), std::io::stdout)
///             .into_layer()
///             .into_subscriber()
///     }).unwrap();
///     
///     tracing::info!("blocking the thread to process this log >:)");
/// }
/// ```
pub fn blocking<F, W>(formatter: F, make_writer: W) -> BlockingProcessor<F, W>
where
    F: 'static + Formatter + Send,
    W: 'static + for<'a> MakeWriter<'a> + Send,
{
    BlockingProcessor {
        formatter,
        make_writer,
    }
}
