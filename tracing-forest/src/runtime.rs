//! Run asynchronous code in the context of a `tracing-forest` subscriber.
//! 
//! This module provides useful abstractions for executing async code:
//! [`worker_task`] for `main` functions, and [`capture`] for unit tests,
//! both of which return a configurable [`Builder`] object.
//! 
//! # Nonblocking log processing with `worker_task`
//! 
//! `tracing-forest` collects trace data into trees, and can sometimes
//! produce large trees that need to be processed. To avoid blocking the main
//! task in these cases, a common strategy is to send this data to a worker
//! task for formatting and writing.
//! 
//! The [`worker_task`] function provides this behavior as a first-class feature of this
//! crate, and handles the configuration, initialization, and graceful shutdown
//! of a subscriber with an associated worker task for formatting and writing.
//! 
//! Unlike [`tracing-appender`] which uses a writer thread for formatted logs,
//! this module allows for log trees to be sent to a worker task before formatting,
//! allowing more log-related work to be offloaded to the worker task.
//! 
//! [`tracing-appender`]: https://crates.io/crates/tracing-appender
//! 
//! ## Examples
//! 
//! ```
//! use tracing::{info, info_span};
//! 
//! #[tokio::main]
//! async fn main() {
//!     tracing_forest::worker_task()
//!         .build()
//!         .on(async {
//!             info!("Hello, world!");
//!
//!             info_span!("my_span").in_scope(|| {
//!                 info!("Relevant information");
//!             })
//!         })
//!         .await;
//! }
//! ```
//! Produces the output:
//! ```log
//! INFO     ｉ [info]: Hello, world!
//! INFO     my_span [ 26.0µs | 100.000% ]
//! INFO     ┕━ ｉ [info]: Relevant information
//! ```
//! 
//! For full configuration options, see the [`Builder`] documentation.
//! 
//! # Inspecting trace data in unit tests with `capture`
//! 
//! The [`capture`] function offers the ability to programmatically inspect log
//! trees generated by `tracing-forest`. It is the unit testing analog of
//! [`worker_task`], except it returns `Vec<Tree>` after the future is completed,
//! which can be then be inspected.
//! 
//! ## Examples
//! 
//! ```
//! use tracing_forest::tree::{Tree, Event, Span};
//! use tracing::{info, info_span};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let logs: Vec<Tree> = tracing_forest::capture()
//!         .build()
//!         .on(async {
//!             info!("Hello, world!");
//!
//!             info_span!("my_span").in_scope(|| {
//!                 info!("Relevant information");
//!             })
//!         })
//!         .await;
//!     
//!     // There is one event and one span at the root level
//!     assert!(logs.len() == 2);
//!     
//!     // Inspect the first event
//!     let hello_world: &Event = logs[0].event()?;
//!     assert!(hello_world.message() == Some("Hello, world!"));
//!
//!     // Inspect the span
//!     let my_span: &Span = logs[1].span()?;
//!     assert!(my_span.name() == "my_span");
//! 
//!     // Only the `info` event is recorded
//!     assert!(my_span.nodes().len() == 1);
//! 
//!     let relevant_info: &Event = my_span.nodes()[0].event()?;
//! 
//!     assert!(relevant_info.message() == Some("Relevant information"));
//! 
//!     Ok(())
//! }
//! ```
//! 
//! Additional options for tree inspection can be found in the
//! [`tree` module-level documentation](crate::tree)
//! 
//! For full configuration options, see the [`Builder`] documentation.
use crate::layer::ForestLayer;
use crate::printer::PrettyPrinter;
use crate::tree::Tree;
use crate::fail;
use crate::tag::{TagParser, NoTag};
use crate::processor::{self, Processor, WithFallback};
use std::future::Future;
use std::iter;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::sync::oneshot;
use tracing::Subscriber;
use tracing_subscriber::Registry;
use tracing_subscriber::layer::{Layered, SubscriberExt as _};

/// Begins the configuration of a `ForestLayer` subscriber that sends log trees
/// to a processing task for formatting and writing.
/// 
/// For full configuration options, see [`Builder`].
/// 
/// For a high-level overview on usage, see the [module-level documentation][nonblocking-processing]
/// for more details.
/// 
/// # Note
/// 
/// The [`worker_task`] function defaults to setting the global subscriber, which is required
/// to detect logs in multithreading scenarios, but prevents setting other [`Subscriber`]s
/// globally afterwards. This can be disabled via the [`set_global`] method.
/// 
/// [nonblocking-processing]: crate::runtime#nonblocking-log-processing-with-worker_task
/// [`set_global`]: Builder::set_global
pub fn worker_task() -> Builder<InnerSender<impl Processor>, WorkerTask<PrettyPrinter>, NoTag> {
    worker_task_inner(WorkerTask(PrettyPrinter::new()), true)
}

/// Begins the configuration of a `ForestLayer` subscriber that sends log trees
/// to a buffer that can later be inspected programatically.
/// 
/// For full configuration options, see [`Builder`].
/// 
/// For a high-level overview on usage, see the [module-level documentation][inspecting-trace-data]
/// for more details.
/// 
/// # Note
/// 
/// The [`capture`] function defaults to not setting the global subscriber, which
/// allows multiple unit tests in the same file, but prevents trace data from other
/// threads to be collected. This can be enabled via the [`set_global`] method.
/// 
/// [inspecting-trace-data]: crate::runtime#inspecting-trace-data-in-unit-tests-with-capture
/// [`set_global`]: Builder::set_global
pub fn capture() -> Builder<InnerSender<impl Processor>, Capture, NoTag> {
    worker_task_inner(Capture(()), false)
}

fn worker_task_inner<P>(worker_processor: P, is_global: bool) -> Builder<InnerSender<impl Processor>, P, NoTag> {
    let (tx, rx) = mpsc::unbounded_channel();

    let sender_processor = processor::from_fn(move |tree| tx
        .send(tree)
        .map_err(|err| {
            let msg = err.to_string().into();
            processor::error(err.0, msg)
        })
    );

    Builder {
        sender_processor: InnerSender(sender_processor),
        worker_processor,
        receiver: rx,
        tag: NoTag,
        is_global,
    }
}

/// Return type of [`worker_task`] and [`capture`].
/// 
/// # Configuring a `Runtime`
/// 
/// `Builder` follows the [builder pattern][builder] to configure a [`Runtime`].
/// 
/// Configuration options include:
/// * Setting the [tag][set_tag].
/// * Installing [globally][set_global].
/// * Configuring the [internal sender][map_sender] with fallbacks.
/// * Configuring the [processor][map_receiver] in the worker task.
/// 
/// To finish the `Runtime`, call the [`build`] method to compose the configured
/// `ForestLayer` onto a [`Registry`]. Alternatively, the [`build_on`] method
/// can be used construct arbitrary `Subscriber`s from the configured `ForestLayer`,
/// which is used in the returned `Runtime`.
/// 
/// [builder]: https://rust-lang.github.io/api-guidelines/type-safety.html#builders-enable-construction-of-complex-values-c-builder
/// [set_tag]: Builder::set_tag
/// [set_global]: Builder::set_global
/// [map_sender]: Builder::map_sender
/// [map_receiver]: Builder::map_receiver
/// [`build`]: Builder::build
/// [`build_on`]: Builder::build_on
pub struct Builder<Tx, Rx, T> {
    sender_processor: Tx,
    worker_processor: Rx,
    receiver: UnboundedReceiver<Tree>,
    tag: T,
    is_global: bool,
}

/// A marker type indicating that trace data should be captured for later use.
pub struct Capture(());

/// A marker type indicating that trace data should be processed.
pub struct WorkerTask<P>(P);

/// The [`Processor`] used within a `tracing-forest` subscriber for sending logs
/// to a processing task.
/// 
/// This type cannot be constructed by downstream users.
#[derive(Debug)]
pub struct InnerSender<P>(P);

impl<P: Processor> Processor for InnerSender<P> {
    fn process(&self, tree: Tree) -> processor::Result {
        self.0.process(tree)
    }
}

mod sealed {
    pub trait Sealed {}
}

impl<P> sealed::Sealed for InnerSender<P> {}

impl<S: sealed::Sealed, P> sealed::Sealed for WithFallback<S, P> {}

impl<Tx, P, T> Builder<Tx, WorkerTask<P>, T>
where
    P: Processor,
{
    /// Configure the processor on the receiving end of the log channel.
    /// This is particularly useful for adding fallbacks.
    /// 
    /// This method accepts a closure that accepts the current [`Processor`] on the
    /// worker task, and maps it to another [`Processor`].
    /// 
    /// # Note
    /// 
    /// This method is only available if called after [`worker_task`].
    /// 
    /// # Examples
    ///
    /// Configuring the writing task to write to a file, or else fall back to stderr.
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() {
    /// use tracing_forest::traits::*;
    /// use std::fs::File;
    /// 
    /// let out = File::create("out.log").unwrap();
    /// 
    /// tracing_forest::worker_task()
    ///     .map_receiver(|printer| printer
    ///         .writer(out)
    ///         .or_stderr()
    ///     )
    ///     .build()
    ///     .on(async {
    ///         // ...
    ///     })
    ///     .await;
    /// # }
    /// ```
    pub fn map_receiver<F, P2>(self, f: F) -> Builder<Tx, WorkerTask<P2>, T>
    where
        F: FnOnce(P) -> P2,
        P2: Processor,
    {
        Builder {
            sender_processor: self.sender_processor,
            worker_processor: WorkerTask(f(self.worker_processor.0)),
            receiver: self.receiver,
            tag: self.tag,
            is_global: self.is_global,
        }
    }
}

impl<Tx, Rx, T> Builder<Tx, Rx, T>
where
    Tx: Processor + sealed::Sealed,
    T: TagParser,
{
    /// Configure the processer within the subscriber that sends log trees to
    /// a processing task. This allows for dangling tasks to still generate trace
    /// data, even after the worker task closes.
    /// 
    /// # Examples
    ///
    /// Allowing the subscriber to defer to stderr if the worker task finished.
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() {
    /// use tracing_forest::traits::*;
    /// 
    /// tracing_forest::worker_task()
    ///     .map_sender(|sender| sender.or_stderr())
    ///     .build()
    ///     .on(async {
    /// #       mod tokio {
    /// #          pub async fn spawn<T>(_: T) {}
    /// #          pub mod signal {
    /// #              pub async fn ctrl_c() -> Result<(), ()> { Ok(()) }
    /// #          }
    /// #       }
    ///         // The handle is immediately dropped, leaving the task dangling
    ///         tokio::spawn(async {
    ///             // Some unending task
    ///         });
    /// 
    ///         // Wait until the user stops the application
    ///         tokio::signal::ctrl_c().await.expect("Failed to listen for CTRL-C");
    ///     })
    ///     .await;
    ///     // The worker task is completed and the channel is closed at this point.
    ///     // Any new trace data generated by the dangling task at this point
    ///     // is deferred to stderr because of the added fallback.
    /// # }
    /// ```
    /// 
    /// Since dropping the sender half would make the receiver task useless, this
    /// method uses traits to enforce at compile time that the function returns
    /// some derivation of the sender. Currently, the only accepted wrapping is
    /// through adding a fallback.
    /// ```compile_fail
    /// use tracing_forest::PrettyPrinter;
    /// 
    /// # #[tokio::main]
    /// # async fn main() {
    /// tracing_forest::worker_task()
    ///     .map_sender(|_sender| {
    ///         // Some variation of the sender isn't returned, so this won't compile.
    ///         PrettyPrinter::new()
    ///     })
    ///     .build()
    ///     .on(async {
    ///         // ...
    ///     })
    ///     .await;
    /// # }
    /// ```
    pub fn map_sender<F, Tx2>(self, f: F) -> Builder<Tx2, Rx, T>
    where
        F: FnOnce(Tx) -> Tx2,
        Tx2: Processor + sealed::Sealed,
    {
        Builder {
            sender_processor: f(self.sender_processor),
            worker_processor: self.worker_processor,
            receiver: self.receiver,
            tag: self.tag,
            is_global: self.is_global,

        }
    }

    /// Set the [`TagParser`].
    /// 
    /// # Examples
    /// 
    /// ```
    /// use tracing_forest::{util::*, Tag};
    /// 
    /// fn simple_tag(event: &Event) -> Option<Tag> {
    ///     // -- snip --
    ///     # None
    /// }
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     tracing_forest::worker_task()
    ///         .set_tag(simple_tag)
    ///         .build()
    ///         .on(async {
    ///             // ...
    ///         })
    ///         .await;
    /// }
    /// ```
    pub fn set_tag<T2>(self, tag: T2) -> Builder<Tx, Rx, T2>
    where
        T2: TagParser,
    {
        Builder {
            sender_processor: self.sender_processor,
            worker_processor: self.worker_processor,
            receiver: self.receiver,
            tag,
            is_global: self.is_global,
        }
    }

    /// Set whether or not the subscriber should be set globally.
    /// 
    /// Setting the subscriber globally is intended for `main` functions, since
    /// it allows logs to be be collected across multithreaded environments. Not
    /// setting globally is intended for test functions, which need to set a new
    /// subscriber multiple times in the same program.
    /// 
    /// # Examples
    /// 
    /// For multithreaded tests, `set_global` can be used so that the subscriber
    /// applies to all the threads. However, each function that sets a global
    /// subscriber must be in its own compilation unit, like an integration test,
    /// otherwise the global subscriber will carry over across tests.
    /// ```
    /// #[tokio::test(flavor = "multi_thread")]
    /// async fn test_multithreading() {
    ///     let logs = tracing_forest::capture()
    ///         .set_global(true)
    ///         .build()
    ///         .on(async {
    ///             // spawn some tasks
    ///         })
    ///         .await;
    ///     
    ///     // inspect logs...
    /// }
    /// ```
    pub fn set_global(mut self, is_global: bool) -> Self {
        self.is_global = is_global;
        self
    }

    /// Finishes the `ForestLayer` by composing it into a [`Registry`], and
    /// returns it as a [`Runtime`].
    /// 
    /// This method is useful for a basic configuration of a `Subscriber`. For
    /// a more advanced configuration, see the [`build_on`] and [`build_with`]
    /// methods.
    /// 
    /// [`build_on`]: Builder::build_on
    /// [`build_with`]: Builder::build_with
    /// 
    /// # Examples
    /// 
    /// ```
    /// #[tokio::main]
    /// async fn main() {
    ///     tracing_forest::worker_task()
    ///         .build()
    ///         .on(async {
    ///             // ...
    ///         })
    ///         .await;
    /// }
    /// ```
    pub fn build(self) -> Runtime<Layered<ForestLayer<Tx, T>, Registry>, Rx> {
        self.build_on(|x| x)
    }

    /// Finishes the `ForestLayer` by calling a function to build a `Subscriber`,
    /// and returns in as a [`Runtime`].
    /// 
    /// Unlike [`build_with`], this method composes the layer onto a [`Registry`]
    /// prior to passing it into the function. This makes it more convenient for
    /// the majority of use cases.
    ///
    /// This method is useful for advanced configuration of `Subscriber`s as
    /// defined in [`tracing-subscriber`s documentation]. For a basic configuration,
    /// see the [`build`] method.
    /// 
    /// [`build_with`]: Builder::build_with
    /// [`tracing-subscriber`s documentation]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/index.html#composing-layers
    /// [`build`]: Builder::build
    /// 
    /// # Examples
    /// 
    /// Composing a `Subscriber` with multiple layers:
    /// ```
    /// use tracing_forest::{traits::*, util::*};
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     tracing_forest::worker_task()
    ///         .build_on(|subscriber| subscriber.with(LevelFilter::INFO))
    ///         .on(async {
    ///             // ...
    ///         })
    ///         .await;
    /// }
    /// ```
    pub fn build_on<F, S>(self, f: F) -> Runtime<S, Rx>
    where
        F: FnOnce(Layered<ForestLayer<Tx, T>, Registry>) -> S,
        S: Subscriber,
    {
        self.build_with(|layer| f(Registry::default().with(layer)))
    }

    /// Finishes the `ForestLayer` by calling a function to build a `Subscriber`,
    /// and returns it as a [`Runtime`].
    ///
    /// Unlike [`build_on`], this method passes the `ForestLayer` to the function
    /// without presupposing a [`Registry`] base. This makes it the most flexible
    /// option for construction.
    /// 
    /// This method is useful for advanced configuration of `Subscriber`s as
    /// defined in [`tracing-subscriber`s documentation]. For a basic configuration,
    /// see the [`build`] method.
    /// 
    /// [`build_on`]: Builder::build_on
    /// [`tracing-subscriber`s documentation]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/index.html#composing-layers
    /// [`build`]: Builder::build
    /// 
    /// # Examples
    /// 
    /// Composing a `Subscriber` with multiple layers:
    /// ```
    /// use tracing_subscriber::Registry;
    /// use tracing_forest::{traits::*, util::*};
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     tracing_forest::worker_task()
    ///         .build_with(|layer: ForestLayer<_, _>| {
    ///             Registry::default()
    ///                 .with(layer)
    ///                 .with(LevelFilter::INFO)
    ///         })
    ///         .on(async {
    ///             // ...
    ///         })
    ///         .await;
    /// }
    /// ```
    pub fn build_with<F, S>(self, f: F) -> Runtime<S, Rx>
    where
        F: FnOnce(ForestLayer<Tx, T>) -> S,
        S: Subscriber,
    {
        let layer = ForestLayer::new(self.sender_processor, self.tag);
        let subscriber = f(layer);

        Runtime {
            subscriber,
            worker_processor: self.worker_processor,
            receiver: self.receiver,
            is_global: self.is_global,
        }
    }
}

/// Execute a `Future` in the context of a subscriber with a `ForestLayer`.
/// 
/// This type is returned by [`Builder::build`] and [`Builder::build_with`].
pub struct Runtime<S, P> {
    subscriber: S,
    worker_processor: P, // either `Process<_>` or `Capture`
    receiver: UnboundedReceiver<Tree>,
    is_global: bool,
}

impl<S, P> Runtime<S, WorkerTask<P>>
where
    S: Subscriber + Send + Sync,
    P: Processor + Send,
{
    /// Execute a future in the context of the configured subscriber.
    pub async fn on<F: Future>(self, f: F) -> F::Output {
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let processor = self.worker_processor.0;
        let mut receiver = self.receiver;

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(tree) = receiver.recv() => processor.process(tree).expect(fail::PROCESSING_ERROR),
                    Ok(()) = &mut shutdown_rx => break,
                    else => break,
                }
            }

            receiver.close();

            // Drain any remaining logs in the channel buffer.
            while let Ok(tree) = receiver.try_recv() {
                processor.process(tree).expect(fail::PROCESSING_ERROR);
            }
        });

        let output = {
            let _guard = if self.is_global {
                tracing::subscriber::set_global_default(self.subscriber)
                    .expect("global default already set");
                None
            } else {
                Some(tracing::subscriber::set_default(self.subscriber))
            };

            f.await
        };

        shutdown_tx.send(()).expect("Shutdown signal couldn't send, this is a bug");

        handle.await.expect("Failed to join the writing task, this is a bug");

        output
    }
}

impl<S> Runtime<S, Capture>
where
    S: Subscriber + Send + Sync,
{
    /// Execute a future in the context of the configured subscriber, and return
    /// a `Vec<Tree>` of generated logs.
    pub async fn on(self, f: impl Future<Output = ()>) -> Vec<Tree> {
        {
            let _guard = if self.is_global {
                tracing::subscriber::set_global_default(self.subscriber)
                    .expect("global default already set");
                None
            } else {
                Some(tracing::subscriber::set_default(self.subscriber))
            };

            f.await;
        }

        let mut receiver = self.receiver;

        receiver.close();

        iter::from_fn(|| receiver.try_recv().ok()).collect()
    }
}
