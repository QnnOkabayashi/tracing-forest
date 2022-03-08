use crate::processor::{ProcessReport, Processor};
use crate::tree::Tree;
use std::error::Error;
use std::io::{self, Write};
use tracing_subscriber::fmt::MakeWriter;

mod pretty;
pub use pretty::Pretty;

/// Write a [`Tree`] as a `String`.
///
/// # Examples
///
/// This trait implements all `Fn(&Tree) -> Result<String, E>` types, where `E: Debug`.
/// If the `serde` feature is enabled, functions like [`serde_json::to_string_pretty`]
/// can be used wherever a `StringifyTree` type is required.
/// ```
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// tracing_forest::new()
///     .map_receiver(|rx| {
///         // `Printer::fmt` expects some `StringifyTree` type.
///         rx.fmt(serde_json::to_string_pretty)
///     })
///     .on_registry()
///     .on(async {
///         info!("write this as json");
///     })
///     .await
/// # }
/// ```
/// Produces the following result:
/// ```json
/// {
///   "Event": {
///     "shared": {
///       "uuid": "00000000-0000-0000-0000-000000000000",
///       "timestamp": "2022-03-06T01:18:12.255209+00:00",
///       "level": "INFO"
///     },
///     "message": "write this as json",
///     "tag": "info",
///     "fields": {}
///   }
/// }
/// ```
pub trait StringifyTree {
    /// The error type if the `Tree` cannot be stringified.
    type Error: Error + Send + Sync;

    /// Stringifies the `Tree`, or returns an error.
    fn fmt(&self, tree: &Tree) -> Result<String, Self::Error>;
}

impl<F, E> StringifyTree for F
where
    F: Fn(&Tree) -> Result<String, E>,
    E: Error + Send + Sync,
{
    type Error = E;

    #[inline(always)]
    fn fmt(&self, tree: &Tree) -> Result<String, E> {
        self(tree)
    }
}

/// Returns a [`Printer`] with the default configuration.
pub fn printer() -> Printer<Pretty, fn() -> io::Stdout> {
    Printer::new(Pretty, io::stdout)
}

/// A [`Processor`] that formats and writes logs.
#[derive(Clone, Debug)]
pub struct Printer<S, W> {
    to_string: S,
    make_writer: W,
}

impl<S, W> Printer<S, W>
where
    S: 'static + StringifyTree,
    W: 'static + for<'a> MakeWriter<'a>,
{
    /// Returns a new [`Printer`].
    pub fn new(to_string: S, make_writer: W) -> Self {
        Printer {
            to_string,
            make_writer,
        }
    }

    /// Set the formatter.
    pub fn fmt<S2>(self, to_string: S2) -> Printer<S2, W>
    where
        S2: 'static + StringifyTree,
    {
        Printer::new(to_string, self.make_writer)
    }

    /// Set the writer.
    pub fn write<W2>(self, make_writer: W2) -> Printer<S, W2>
    where
        W2: 'static + for<'a> MakeWriter<'a>,
    {
        Printer::new(self.to_string, make_writer)
    }
}

impl Default for Printer<Pretty, fn() -> io::Stdout> {
    fn default() -> Self {
        printer()
    }
}

impl<F, W> Processor for Printer<F, W>
where
    F: 'static + StringifyTree,
    W: 'static + for<'a> MakeWriter<'a>,
{
    fn process(&self, tree: Tree) -> Result<(), ProcessReport> {
        // Since both formatting and writing can error, we need to be able to
        // generate a process report for each. Since both are recoverable, they
        // should both return ownership to the payload. However, we can't have
        // a `ProcessReport` for each because then two reports need ownership.
        //
        // It turns out we can avoid cloning by chaining results together and
        // propagating errors to the very end so we only have to create one closure
        // taking ownership of the tree.
        self.to_string
            .fmt(&tree)
            .map_err(Into::into)
            .and_then(|buf| {
                self.make_writer
                    .make_writer()
                    .write_all(buf.as_bytes())
                    .map_err(Into::into)
            })
            .map_err(|err| ProcessReport::new(Some(tree), err))
    }
}
