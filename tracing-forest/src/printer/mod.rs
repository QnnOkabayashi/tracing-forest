//! Utilities for formatting and writing trace trees.
use crate::processor::{ProcessReport, Processor};
use crate::tree::Tree;
use std::error::Error;
use std::io::{self, Write};
use tracing_subscriber::fmt::MakeWriter;

mod pretty;
pub use pretty::Pretty;

/// Format a [`Tree`] into a `String`.
///
/// # Examples
///
/// This trait implements all `Fn(&Tree) -> Result<String, E>` types, where `E: Debug`.
/// If the `serde` feature is enabled, functions like `serde_json::to_string_pretty`
/// can be used wherever a `Formatter` is required.
/// ```
/// # use tracing::info;
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// tracing_forest::worker_task()
///     .map_receiver(|receiver| {
///         receiver.set_formatter(serde_json::to_string_pretty)
///     })
///     .build()
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
///     "uuid": "00000000-0000-0000-0000-000000000000",
///     "timestamp": "2022-03-24T16:08:17.761149+00:00",
///     "level": "INFO",
///     "message": "write this as json",
///     "tag": "info",
///     "fields": {}
///   }
/// }
/// ```
pub trait Formatter {
    /// The error type if the `Tree` cannot be stringified.
    type Error: Error + Send + Sync;

    /// Stringifies the `Tree`, or returns an error.
    ///
    /// # Errors
    /// If the `Tree` cannot be formatted to a string, an error is returned.
    fn fmt(&self, tree: &Tree) -> Result<String, Self::Error>;
}

impl<F, E> Formatter for F
where
    F: Fn(&Tree) -> Result<String, E>,
    E: Error + Send + Sync,
{
    type Error = E;

    #[inline]
    fn fmt(&self, tree: &Tree) -> Result<String, E> {
        self(tree)
    }
}

/// A [`Processor`] that formats and writes logs.
#[derive(Clone, Debug)]
pub struct Printer<S, W> {
    formatter: S,
    make_writer: W,
}

pub type StdoutPrinter = Printer<Pretty, fn() -> io::Stdout>;

pub type StderrPrinter = Printer<Pretty, fn() -> io::Stderr>;

impl<F, W> Printer<F, W>
where
    F: 'static + Formatter,
    W: 'static + for<'a> MakeWriter<'a>,
{
    /// Returns a new [`Printer`].
    pub fn new(formatter: F, make_writer: W) -> Self {
        Printer {
            formatter,
            make_writer,
        }
    }

    /// Set the formatter.
    pub fn set_formatter<F2>(self, formatter: F2) -> Printer<F2, W>
    where
        F2: 'static + Formatter,
    {
        Printer::new(formatter, self.make_writer)
    }

    /// Set the writer.
    pub fn set_writer<W2>(self, make_writer: W2) -> Printer<F, W2>
    where
        W2: 'static + for<'a> MakeWriter<'a>,
    {
        Printer::new(self.formatter, make_writer)
    }
}

impl<F> Printer<F, fn() -> io::Stdout>
where
    F: 'static + Formatter,
{
    /// Returns a new [`Printer`] from a [`Formatter`], defaulting to writing to
    /// stdout.
    pub fn from_formatter(formatter: F) -> Self {
        Printer::new(formatter, io::stdout)
    }
}

impl<W> Printer<Pretty, W>
where
    W: 'static + for<'a> MakeWriter<'a>,
{
    /// Returns a new [`Printer`] from a [`MakeWriter`], defaulting to pretty
    /// printing.
    pub fn from_make_writer(make_writer: W) -> Self {
        Printer::new(Pretty::default(), make_writer)
    }
}

impl Default for Printer<Pretty, fn() -> io::Stdout> {
    fn default() -> Self {
        Printer::new(Pretty::default(), io::stdout)
    }
}

impl<F, W> Processor for Printer<F, W>
where
    F: 'static + Formatter,
    W: 'static + for<'a> MakeWriter<'a>,
{
    fn process(&self, tree: Tree) -> Result<(), ProcessReport> {
        let buf = match self.formatter.fmt(&tree) {
            Ok(buf) => buf,
            Err(e) => return Err(ProcessReport::new(Some(tree), e.into())),
        };

        match self.make_writer.make_writer().write_all(buf.as_bytes()) {
            Ok(()) => Ok(()),
            Err(e) => Err(ProcessReport::new(Some(tree), e.into())),
        }
    }
}
