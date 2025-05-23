//! Utilities for formatting and writing trace trees.
use crate::processor::{self, Processor};
use crate::tree::{Event, Tree};
use std::error::Error;
use std::io::{self, Write};
use tracing_subscriber::fmt::MakeWriter;

mod pretty;
pub use pretty::Pretty;

/// Format a [`Tree`] into a `String`.
///
/// You can override this trait to implement completely custom rendering of `Tree`s, or you can use the other types in
/// this module to override only the parts you care about most.
///
/// # Examples
///
/// This trait implements all `Fn(&Tree) -> Result<String, E>` types, where `E: Error + Send + Sync`.
/// If the `serde` feature is enabled, functions like `serde_json::to_string_pretty`
/// can be used wherever a `Formatter` is required.
/// ```
/// # use tracing::info;
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// tracing_forest::worker_task()
///     .map_receiver(|receiver| {
///         receiver.formatter(serde_json::to_string_pretty)
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
    ///
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

/// Information about a level of indentation and where a log entry is within it.
///
/// For example, in a log that looks something like this:
///
/// ```yml
/// span 1:
/// - span a:
///   - entry i
///   - span ii:
///     - entry 1
///     - entry 2
/// - entry b
/// ```
///
/// - "span 1" would see `&[Level { index: 0, total: 1 }]`
/// - "entry i" would see `&[Level { index: 0, total: 1 }, Level { index: 0, total: 2 }, Level { index: 0, total: 2 }]`
/// - "entry 2" would see `&[Level { index: 0, total: 1 }, Level { index: 0, total: 2 }, Level { index: 1, total: 2 }, Level { index: 1, total: 2 }]`
/// - "entry b" would see `&[Level { index: 0, total: 1 }, Level { index: 1, total: 2 }]`
pub struct Indent {
    /// The index of this entry at this level of indentation. Zero-indexed.
    pub index: usize,
    /// The total number of entries at this level of indentation.
    pub total: usize,
    /// Where this level of indentation comes from.
    ///
    /// If `true`, it came from a `Span`, i.e. the level of indentation was caused by a span open as of this line
    /// being rendered. If `false`, it came from a multi-line `Entry`, i.e. the relevant [`FormatEntry`] returned a
    /// multi-line string.
    pub span: bool,
}

/// Format a single level of indentation into a `String`.
///
/// # Examples
///
/// ```rust
/// // TODO: Code sample showing off how this could work
/// /*
/// async {
///     debug!("here is a log line")
/// }.instrument(debug_span!("here is a span"))
/// */
/// ```
/// Produces the following result:
/// ```txt
/// DEBUG    here is a span [ 0µs | 0.00% / 100.00% ]
/// DEBUG      🐛 [debug]: An untagged debug log
/// ```
pub trait FormatIndents {
    /// The error type if the levels can't be formatted into indentation.
    type Error: Error + Send + Sync;

    /// Given a set of indentation levels, render them into a string.
    fn fmt_indent(&self, levels: &[Indent]) -> Result<String, Self::Error>;
}

/// Format a log event, including tag, message, and fields
pub trait FormatEvent {
    /// The error type if the event can't be formatted.
    type Error: Error + Send + Sync;

    /// Given a log event, format it into a string.
    ///
    /// This does **not** render the indentation, only the event itself. See [`FormatIndents`].
    ///
    /// The string can contain newlines. If it does, this event essentially gets treated as a span, for formatting,
    /// with each line in the output being rendered equivalently to a (single-line) event under a span. In other words,
    /// it should just work how you expect.
    fn fmt_event(&self, event: &Event) -> Result<String, Self::Error>;
}

/// A [`Processor`] that formats and writes logs.
#[derive(Clone, Debug)]
pub struct Printer<F, W> {
    formatter: F,
    make_writer: W,
}

/// A [`MakeWriter`] that writes to stdout.
///
/// This is functionally the same as using [`std::io::stdout`] as a `MakeWriter`,
/// except it has a named type and can therefore be used in type signatures.
#[derive(Debug)]
pub struct MakeStdout;

/// A [`MakeWriter`] that writes to stderr.
///
/// This is functionally the same as using [`std::io::stderr`] as a `MakeWriter`,
/// except it has a named type and can therefore be used in type signatures.
#[derive(Debug)]
pub struct MakeStderr;

impl<'a> MakeWriter<'a> for MakeStdout {
    type Writer = io::Stdout;

    fn make_writer(&self) -> Self::Writer {
        io::stdout()
    }
}

impl<'a> MakeWriter<'a> for MakeStderr {
    type Writer = io::Stderr;

    fn make_writer(&self) -> Self::Writer {
        io::stderr()
    }
}

/// A [`Processor`] that pretty-prints to stdout.
pub type PrettyPrinter = Printer<Pretty, MakeStdout>;

impl PrettyPrinter {
    /// Returns a new [`PrettyPrinter`] that pretty-prints to stdout.
    ///
    /// Use [`Printer::formatter`] and [`Printer::writer`] for custom configuration.
    pub const fn new() -> Self {
        Printer {
            formatter: Pretty,
            make_writer: MakeStdout,
        }
    }
}

impl<F, W> Printer<F, W>
where
    F: 'static + Formatter,
    W: 'static + for<'a> MakeWriter<'a>,
{
    /// Set the formatter.
    ///
    /// See the [`Formatter`] trait for details on possible inputs.
    pub fn formatter<F2>(self, formatter: F2) -> Printer<F2, W>
    where
        F2: 'static + Formatter,
    {
        Printer {
            formatter,
            make_writer: self.make_writer,
        }
    }

    /// Set the writer.
    pub fn writer<W2>(self, make_writer: W2) -> Printer<F, W2>
    where
        W2: 'static + for<'a> MakeWriter<'a>,
    {
        Printer {
            formatter: self.formatter,
            make_writer,
        }
    }
}

impl Default for PrettyPrinter {
    fn default() -> Self {
        PrettyPrinter::new()
    }
}

impl<F, W> Processor for Printer<F, W>
where
    F: 'static + Formatter,
    W: 'static + for<'a> MakeWriter<'a>,
{
    fn process(&self, tree: Tree) -> processor::Result {
        let string = match self.formatter.fmt(&tree) {
            Ok(s) => s,
            Err(e) => return Err(processor::error(tree, e.into())),
        };

        match self.make_writer.make_writer().write_all(string.as_bytes()) {
            Ok(()) => Ok(()),
            Err(e) => Err(processor::error(tree, e.into())),
        }
    }
}

/// A [`Processor`] that captures logs during tests and allows them to be presented
/// when --nocapture is used.
#[derive(Clone, Debug)]
pub struct TestCapturePrinter<F> {
    formatter: F,
}

impl TestCapturePrinter<Pretty> {
    /// Construct a new test capturing printer with the default `Pretty` formatter. This printer
    /// is intented for use in tests only as it works with the default rust stdout capture mechanism
    pub const fn new() -> Self {
        TestCapturePrinter { formatter: Pretty }
    }
}

impl<F> Processor for TestCapturePrinter<F>
where
    F: 'static + Formatter,
{
    fn process(&self, tree: Tree) -> processor::Result {
        let string = self
            .formatter
            .fmt(&tree)
            .map_err(|e| processor::error(tree, e.into()))?;

        print!("{}", string);
        Ok(())
    }
}
