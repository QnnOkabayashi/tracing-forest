//! A [`Processor`] that sends logs to another task to be processed.
//!
//! See [`AsyncProcessor`] for more details.

use crate::formatter::Formatter;
use crate::layer::Tree;
use crate::processor::Processor;
use std::io::Write;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing_subscriber::fmt::MakeWriter;

/// A [`Processor`] that sends logs to another async task for processing.
///
/// To initialize a new [`AsyncProcessor`], see [`async_spawn`].
///
/// # Examples
///
/// In the case where a processing task has already been initialized,
/// an [`AsyncProcessor`] can also be constructed manually from a
/// [`tokio::sync::mpsc::UnboundedSender`]:
///
/// ```
/// # use tokio::sync::mpsc;
/// # use tracing_forest::layer::{Tree, TreeLayer};
/// # use tracing_forest::processor::sync::AsyncProcessor;
/// # use tracing_forest::Processor;
/// fn build_async_layer(tx: mpsc::UnboundedSender<Tree>) -> TreeLayer<AsyncProcessor> {
///     AsyncProcessor::from(tx).into_layer()
/// }
/// ```
pub struct AsyncProcessor {
    tx: mpsc::UnboundedSender<Tree>,
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

/// Initialize a new [`AsyncProcessor`] and spawns a processing task,
/// returning the processor and a [`JoinHandle`] for the task.
///
/// Since dropped handle are never necessarily run to completion, it's
/// important that the handle is awaited if the function doesn't run
/// indefinitely, otherwise some logs may not be processed.
///
/// ## Note
///
/// This function is usually called by the [`tracing_forest::main`][crate::main]
/// and [`tracing_forest::test`][crate::test] attribute macros, which are the
/// prefered method for creating [`AsyncProcessor`]s.
///
/// ## Examples
///
/// In a function that runs indefinitely
///
/// ```
/// # use tracing_forest::{async_spawn, Pretty, Processor};
/// # async fn start_server() {}
/// #[tokio::main(flavor = "current_thread")]
/// async fn main() {
///     let (processor, _) = async_spawn(Pretty::new(), std::io::stdout);
///     tracing::subscriber::set_global_default({
///         processor
///             .into_layer()
///             .into_subscriber()
///     }).unwrap();
///     
///     start_server().await;
/// }
/// ```
///
/// In a function that terminates
///
/// ```
/// # use tracing_forest::{async_spawn, Pretty, Processor};
/// #[tokio::test]
/// async fn my_short_test() {
///     let (guard, handle) = {
///         let (processor, handle) = async_spawn(Pretty::new(), std::io::stdout);
///     
///         let guard = tracing::subscriber::set_default({
///             processor
///                 .into_layer()
///                 .into_subscriber()
///         });
///     
///         (guard, handle)
///     };
///
///     tracing::info!(satisfied = true, "it works!");
///     
///     // drop subscriber to close all senders
///     drop(guard);
///     handle.await.unwrap();
/// }
/// ```
///
/// ## Panics
///
/// Follows same panic semantics as [`tokio::spawn`], which panics if called
/// from **outside** of the Tokio runtime.
pub fn async_spawn<F, W>(formatter: F, make_writer: W) -> (AsyncProcessor, JoinHandle<()>)
where
    F: 'static + Formatter + Send,
    W: 'static + for<'a> MakeWriter<'a> + Send,
{
    let (tx, mut rx) = mpsc::unbounded_channel();

    let handle = tokio::spawn(async move {
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
    });

    let processor = AsyncProcessor::from(tx);

    (processor, handle)
}
