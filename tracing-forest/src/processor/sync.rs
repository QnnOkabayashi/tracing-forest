//! A [`Processor`] that sends logs to another task to be processed.
//!
//! See [`AsyncProcessor`] for more details.

use crate::formatter::Formatter;
use crate::layer::Tree;
use crate::processor::Processor;
use std::future::Future;
use std::io::Write;
use tokio::sync::mpsc;
use tracing_subscriber::fmt::MakeWriter;

/// A [`Processor`] that sends logs to another [`tokio`] task for processing.
///
/// This type is usually created and used by the [`LayerBuilder`] type.
///
/// [`LayerBuilder`]: crate::builder::LayerBuilder
pub struct AsyncProcessor {
    tx: mpsc::UnboundedSender<Tree>,
}

impl AsyncProcessor {
    /// Create a new [`AsyncProcessor`] and [`Future`] for processing, returning
    /// the processor and the future.
    ///
    /// # Examples
    ///
    /// In a function that runs indefinitely, where the writing thread
    /// doesn't need to explicitly be awaited:
    /// ```
    /// # use tracing_forest::formatter::Pretty;
    /// # use tracing_forest::processor::{Processor, AsyncProcessor};
    /// # async fn start_server() {}
    /// #[tokio::main(flavor = "current_thread")]
    /// async fn main() {
    ///     let (processor, fut) = AsyncProcessor::spawn(Pretty::new(), std::io::stdout);
    ///     tokio::spawn(fut);
    ///     tracing::subscriber::set_global_default({
    ///         processor
    ///             .into_layer()
    ///             .into_subscriber()
    ///     }).unwrap();
    ///     
    ///     start_server().await;
    /// }
    /// ```
    /// In a function that terminates, where the writing thread needs to be
    /// explicitly awaited before going out of scope:
    /// ```
    /// # use tracing_forest::formatter::Pretty;
    /// # use tracing_forest::processor::{AsyncProcessor, Processor};
    /// #[tokio::test]
    /// async fn my_short_test() {
    ///     let (processor, fut) = AsyncProcessor::spawn(Pretty::new(), std::io::stdout);
    ///     let handle = tokio::spawn(fut);
    ///     
    ///     let guard = tracing::subscriber::set_default({
    ///         processor
    ///             .into_layer()
    ///             .into_subscriber()
    ///     });
    ///
    ///     tracing::info!(satisfied = true, "it works!");
    ///     
    ///     // drop subscriber to close all senders
    ///     drop(guard);
    ///     handle.await.unwrap();
    /// }
    /// ```
    pub fn spawn<F, W>(formatter: F, make_writer: W) -> (Self, impl Future<Output = ()>)
    where
        F: 'static + Formatter + Send,
        W: 'static + for<'a> MakeWriter<'a> + Send,
    {
        let (tx, mut rx) = mpsc::unbounded_channel();

        let handle = async move {
            while let Some(tree) = rx.recv().await {
                let mut buf = Vec::with_capacity(0);

                #[allow(clippy::expect_used)]
                {
                    formatter.fmt(tree, &mut buf).expect("formatting failed");
                    make_writer
                        .make_writer()
                        .write_all(&buf[..])
                        .expect("writing failed");
                }
            }
        };

        let processor = AsyncProcessor::from(tx);

        (processor, handle)
    }
}

impl From<mpsc::UnboundedSender<Tree>> for AsyncProcessor {
    fn from(tx: mpsc::UnboundedSender<Tree>) -> Self {
        AsyncProcessor { tx }
    }
}

impl Processor for AsyncProcessor {
    fn process(&self, tree: Tree) {
        #[allow(clippy::expect_used)]
        self.tx
            .send(tree)
            .expect("failed to send logs to processing thread");
    }
}
