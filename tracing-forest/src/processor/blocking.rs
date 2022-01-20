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
/// This type is usually created and used by the [`LayerBuilder`] type.
///
/// [`LayerBuilder`]: crate::builder::LayerBuilder
pub struct BlockingProcessor<F, W> {
    formatter: F,
    make_writer: W,
}

impl<F, W> BlockingProcessor<F, W>
where
    F: 'static + Formatter,
    W: 'static + for<'a> MakeWriter<'a>,
{
    /// Create a new [`BlockingProcessor`].
    ///
    /// # Examples
    /// ```
    /// # use tracing_forest::formatter::Pretty;
    /// # use tracing_forest::processor::{BlockingProcessor, Processor};
    /// fn main() {
    ///     let _guard = tracing::subscriber::set_default({
    ///         BlockingProcessor::new(Pretty::new(), std::io::stdout)
    ///             .into_layer()
    ///             .into_subscriber()
    ///     });
    ///     
    ///     tracing::info!("blocking the thread to process this log >:)");
    /// }
    /// ```
    pub fn new(formatter: F, make_writer: W) -> Self {
        BlockingProcessor {
            formatter,
            make_writer,
        }
    }
}

impl<F, W> Processor for BlockingProcessor<F, W>
where
    F: 'static + Formatter,
    W: 'static + for<'a> MakeWriter<'a>,
{
    fn process(&self, tree: Tree) {
        let mut buf = Vec::with_capacity(0);

        #[allow(clippy::expect_used)]
        {
            self.formatter
                .fmt(tree, &mut buf)
                .expect("formatting failed");
            self.make_writer
                .make_writer()
                .write_all(&buf[..])
                .expect("writing failed");
        }
    }
}
