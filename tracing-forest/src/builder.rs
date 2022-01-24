//! Build the [`TreeLayer`] and [`Subscriber`] with custom configuration values.
//!
//! To start, call [`builder`] to create a [`LayerBuilder`], which configures
//! the [`TreeLayer`] by chaining methods.
//!
//! After the the layer is configured, call [`async_layer`] or
//! [`blocking_layer`] on the [`LayerBuilder`] to get a [`SubscriberBuilder`].
//! At this point, other [`Layer`]s can be composed onto it by chaining calls to
//! the [`with`] method.
//!
//! Finally, code can be run in the context of the [`Subscriber`] by calling
//! either [`on_future`] or [`on_closure`], depending on the type of
//! [`TreeLayer`] that was created.
//!
//! # Note
//!
//! If you don't need advanced configuration options, see
//! [`#[tracing_forest::test]`][crate::test] and
//! [`#[tracing_forest::main]`][crate::main].
//!
//! # Examples
//! Running asynchronously with a custom tag, writing to stderr, formatting with
//! pretty, and filtering out some logs.
//! ```
//! # use tracing_forest::prelude::*;
//! tracing_forest::declare_tags! {
//!     use tracing_forest::Tag;
//!
//!     #[derive(Tag)]
//!     pub(crate) enum BearTag {
//!         #[tag(lvl = "info", msg = "brown.bear", macro = "brown_bear")]
//!         BrownBear,
//!         #[tag(lvl = "warn", msg = "black.bear", macro = "black_bear")]
//!         BlackBear,
//!         #[tag(lvl = "error", msg = "polar.bear", macro = "polar_bear")]
//!         PolarBear
//!     }
//! }
//!
//! # #[test]
//! # fn test_builder() {
//! tracing_forest::builder()
//!     .pretty()
//!     .with_writer(std::io::stderr)
//!     .with_tag::<BearTag>()
//!     .blocking_layer()
//!     .with(LevelFilter::WARN)
//!     .on_closure(|| {
//!         brown_bear!("if it's brown get down");
//!         black_bear!("if it's black fight back");
//!         polar_bear!("if it's white good night");
//!     })
//! # }
//! ```
//! ```log
//! WARN     🚧 [black.bear]: if it's black fight back
//! ERROR    🚨 [polar.bear]: if it's white good night
//! ```
//!
//! See function level documentation of [`LayerBuilder`] and
//! [`SubscriberBuilder`] for details on the various configuration settings.
//!
//! [`async_layer`]: LayerBuilder::async_layer
//! [`blocking_layer`]: LayerBuilder::blocking_layer
//! [`with`]: SubscriberBuilder::with
//! [`on_future`]: SubscriberBuilder::on_future
//! [`on_closure`]: SubscriberBuilder::on_closure
use crate::formatter::{Formatter, Pretty};
use crate::processor::{BlockingProcessor, Processor};
use crate::tag::{NoTag, Tag};
use crate::{cfg_json, cfg_sync, TreeLayer};
cfg_json! {
    use crate::formatter::Json;
}
cfg_sync! {
    use crate::processor::AsyncProcessor;
    use std::future::Future;
    use std::io::Write;
}
use std::marker::PhantomData;
use tracing::Subscriber;
use tracing_subscriber::fmt::{MakeWriter, TestWriter};
use tracing_subscriber::layer::Layered;
use tracing_subscriber::{Layer, Registry};

#[cfg(feature = "env-filter")]
use tracing_subscriber::filter::EnvFilter;

/// Creates a new [`LayerBuilder`] to configure a [`Subscriber`] with a
/// [`TreeLayer`]. This is the prefered method for using a [`TreeLayer`].
///
/// See the [module level documentation] for details on using [`builder`].
///
/// [module level documentation]: self
pub fn builder() -> LayerBuilder<Pretty, fn() -> std::io::Stdout, NoTag> {
    LayerBuilder {
        formatter: Pretty::new(),
        make_writer: std::io::stdout,
        tag: PhantomData,
    }
}

/// A type for configuring [`TreeLayer`]s.
///
/// See the [module level documentation] for details on using [`LayerBuilder`].
///
/// [module level documentation]: self
pub struct LayerBuilder<F, W, T> {
    formatter: F,
    make_writer: W,
    tag: PhantomData<fn(T)>,
}

impl<F, W, T> LayerBuilder<F, W, T>
where
    F: 'static + Formatter + Send,
    W: 'static + for<'a> MakeWriter<'a> + Send,
    T: Tag,
{
    /// Applies a writer that is suitable for test environments.
    ///
    /// Configuration methods can be chained on the return value.
    ///
    /// # Examples
    /// ```
    /// tracing_forest::builder()
    ///     .with_test_writer()
    ///     .blocking_layer()
    ///     .on_closure(|| {
    ///         tracing::info!("Hello, world!");
    ///     })
    /// ```
    pub fn with_test_writer(self) -> LayerBuilder<F, TestWriter, T> {
        self.with_writer(TestWriter::new())
    }

    /// Applies the specified [`MakeWriter`].
    ///
    /// Configuration methods can be chained on the return value.
    ///
    /// # Examples
    /// ```
    /// tracing_forest::builder()
    ///     .with_writer(std::io::stderr)
    ///     .blocking_layer()
    ///     .on_closure(|| {
    ///         tracing::info!("Hello, world!");
    ///     })
    /// ```
    pub fn with_writer<W2>(self, make_writer: W2) -> LayerBuilder<F, W2, T>
    where
        W2: for<'a> MakeWriter<'a>,
    {
        LayerBuilder {
            formatter: self.formatter,
            make_writer,
            tag: self.tag,
        }
    }

    cfg_json! {
        /// Applies compact JSON formatting.
        ///
        /// Configuration methods can be chained on the return value.
        ///
        /// # Examples
        /// ```
        /// tracing_forest::builder()
        ///     .json()
        ///     .blocking_layer()
        ///     .on_closure(|| {
        ///         tracing::info!("Hello, world!");
        ///     })
        /// ```
        /// ```log
        /// {"level":"INFO","kind":{"Event":{"tag":null,"message":"Hello, world!","fields":{}}}}
        /// ```
        pub fn json(self) -> LayerBuilder<Json<true>, W, T> {
            self.with_formatter(Json::compact())
        }

        /// Applies pretty JSON formatting.
        ///
        /// Configuration methods can be chained on the return value.
        ///
        /// # Examples
        /// ```
        /// tracing_forest::builder()
        ///     .json_pretty()
        ///     .blocking_layer()
        ///     .on_closure(|| {
        ///         tracing::info!("Hello, world!");
        ///     })
        /// ```
        /// ```log
        /// {
        ///   "level": "INFO",
        ///   "kind": {
        ///     "Event": {
        ///       "tag": null,
        ///       "message": "Hello, world!",
        ///       "fields": {}
        ///     }
        ///   }
        /// }
        /// ```
        pub fn json_pretty(self) -> LayerBuilder<Json<false>, W, T> {
            self.with_formatter(Json::pretty())
        }
    }

    /// Applies pretty formatting.
    ///
    /// Configuration methods can be chained on the return value.
    ///
    /// # Examples
    /// ```
    /// tracing_forest::builder()
    ///     .json_pretty()
    ///     .blocking_layer()
    ///     .on_closure(|| {
    ///         tracing::info!("Hello, world!");
    ///     })
    /// ```
    /// ```log
    /// INFO     💬 [info]: Hello, world!
    /// ```
    pub fn pretty(self) -> LayerBuilder<Pretty, W, T> {
        self.with_formatter(Pretty::new())
    }

    /// Applies a custom [`Formatter`].
    ///
    /// Configuration methods can be chained on the return value.
    ///
    /// # Examples
    /// ```
    /// # use tracing_forest::formatter::Formatter;
    /// # use tracing_forest::layer::Tree;
    /// # use std::io::{self, Write};
    /// struct UselessFormatter;
    ///
    /// impl Formatter for UselessFormatter {
    ///     fn fmt(&self, _tree: Tree, writer: &mut Vec<u8>) -> io::Result<()> {
    ///         writeln!(writer, "I am useless")
    ///     }
    /// }
    ///
    /// tracing_forest::builder()
    ///     .with_formatter(UselessFormatter)
    ///     .blocking_layer()
    ///     .on_closure(|| {
    ///         tracing::info!("Hello, world!");
    ///     })
    /// ```
    /// ```log
    /// I am useless
    /// ```
    pub fn with_formatter<F2>(self, formatter: F2) -> LayerBuilder<F2, W, T> {
        LayerBuilder {
            formatter,
            make_writer: self.make_writer,
            tag: self.tag,
        }
    }

    /// Applies a custom [`Tag`].
    ///
    /// Configuration methods can be chained on the return value.
    ///
    /// # Examples
    /// ```
    /// tracing_forest::declare_tags! {
    ///     use tracing_forest::Tag;
    ///
    ///     #[derive(Tag)]
    ///     pub(crate) enum MyTag {
    ///         #[tag(lvl = "info", msg = "greeting", macro = "greeting")]
    ///         Greeting,
    ///     }
    /// }
    ///
    /// #[test]
    /// # fn test_with_tag() {
    /// tracing_forest::builder()
    ///     .with_tag::<MyTag>()
    ///     .blocking_layer()
    ///     .on_closure(|| {
    ///         greeting!("Hello, world!");
    ///     })
    /// # }
    /// ```
    /// ```log
    /// INFO     💬 [greeting]: Hello, world!
    /// ```
    pub fn with_tag<T2>(self) -> LayerBuilder<F, W, T2>
    where
        T2: Tag,
    {
        LayerBuilder {
            formatter: self.formatter,
            make_writer: self.make_writer,
            tag: PhantomData,
        }
    }

    /// Finalizes the layer to run with an [`AsyncProcessor`].
    ///
    /// # Examples
    /// ```
    /// # #[tokio::test]
    /// # async fn test_doc_write_async() {
    /// tracing_forest::builder()
    ///     .async_layer()
    ///     .on_future(async {
    ///         tracing::info!("Hello from Tokio");
    ///     })
    ///     .await
    /// # }
    /// ```
    #[cfg(feature = "sync")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sync")))]
    pub fn async_layer(
        self,
    ) -> SubscriberBuilder<
        Layered<TreeLayer<AsyncProcessor>, Registry>,
        AsyncExtensions<impl Future<Output = ()>>,
    > {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let processor = AsyncProcessor::from(tx);

        let handle = async move {
            while let Some(tree) = rx.recv().await {
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
        };

        let subscriber = processor.into_layer().tag::<T>().into_subscriber();

        SubscriberBuilder {
            subscriber,
            extensions: AsyncExtensions(handle),
        }
    }

    /// Finalizes the layer to run with a [`BlockingProcessor`].
    ///
    /// # Examples
    /// ```
    /// # fn main() {
    /// tracing_forest::builder()
    ///     .blocking_layer()
    ///     .on_closure(|| {
    ///         tracing::info!("Hello from the current thread");
    ///     })
    /// # }
    /// ```
    pub fn blocking_layer(
        self,
    ) -> SubscriberBuilder<Layered<TreeLayer<BlockingProcessor<F, W>>, Registry>, BlockingExtensions>
    {
        let processor = BlockingProcessor::new(self.formatter, self.make_writer);
        let subscriber = processor.into_layer().tag::<T>().into_subscriber();

        SubscriberBuilder {
            subscriber,
            extensions: BlockingExtensions,
        }
    }
}

/// Extensions for [`AsyncProcessor`].
pub struct AsyncExtensions<E>(E);

/// Extensions for [`BlockingProcessor`].
pub struct BlockingExtensions;

/// A type for building [`Subscriber`]s by composing many [`Layer`]s, while also
/// holding extension data necessary for some [`TreeLayer`] types.
pub struct SubscriberBuilder<S, E> {
    subscriber: S,
    extensions: E,
}

impl<S, E> SubscriberBuilder<S, E>
where
    S: Subscriber,
{
    /// Wraps the inner subscriber with the provided `layer`.
    ///
    /// # Examples
    /// ```
    /// # use tracing_forest::prelude::*;
    /// tracing_forest::builder()
    ///     .blocking_layer()
    ///     .with(LevelFilter::INFO)
    ///     .on_closure(|| {
    ///         // do stuff here...
    ///     })
    /// ```
    pub fn with<L>(self, layer: L) -> SubscriberBuilder<Layered<L, S>, E>
    where
        L: Layer<S>,
    {
        SubscriberBuilder {
            subscriber: layer.with_subscriber(self.subscriber),
            extensions: self.extensions,
        }
    }

    /// Adds an environment filter to this subscriber. This is based on the
    /// [`tracing_subscriber`] [`EnvFilter`] and uses the same `RUST_LOG` syntax.
    ///
    /// If the `RUST_LOG` environment value is not found, this will default to `info`.
    ///
    /// # Examples
    /// ```
    /// # use tracing_forest::prelude::*;
    /// tracing_forest::builder()
    ///     .blocking_layer()
    ///     .with_env_filter()
    ///     .on_closure(|| {
    ///         // do stuff here...
    ///     })
    /// ```
    #[cfg(feature = "env-filter")]
    #[cfg_attr(docsrs, doc(cfg(feature = "env-filter")))]
    pub fn with_env_filter(self) -> SubscriberBuilder<Layered<EnvFilter, S>, E>
    {
        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new("info"))
            .expect("Failed to construct envfilter - this is a bug!");
        self.with(filter_layer)
    }
}

#[cfg(feature = "sync")]
#[cfg_attr(docsrs, doc(cfg(feature = "sync")))]
impl<S, E> SubscriberBuilder<S, AsyncExtensions<E>>
where
    S: Subscriber + Send + Sync,
    E: 'static + Future<Output = ()> + Send,
{
    /// Runs the provided future in the context of a subscriber layered with a
    /// [`TreeLayer`] and a [`tokio`] runtime.
    ///
    /// [`TreeLayer`]: crate::layer::TreeLayer
    pub async fn on_future(self, future: impl Future<Output = ()>) {
        let handle = tokio::spawn(self.extensions.0);
        let guard = tracing::subscriber::set_default(self.subscriber);
        future.await;
        drop(guard);
        handle.await.expect("failed closing the writing task");
    }
}

impl<S> SubscriberBuilder<S, BlockingExtensions>
where
    S: Subscriber + Send + Sync,
{
    /// Runs the provided closure in the context of a subscriber layered with a
    /// [`TreeLayer`].
    ///
    /// [`TreeLayer`]: crate::layer::TreeLayer
    pub fn on_closure<R>(self, closure: impl FnOnce() -> R) -> R {
        let _guard = tracing::subscriber::set_default(self.subscriber);
        closure()
    }
}
