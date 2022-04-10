//! Trait for processing logs of a span after it is closed.
//!
//! See [`Processor`] for more details.
use crate::cfg_tokio;
use crate::printer::{MakeStderr, MakeStdout, Pretty, Printer};
use crate::tree::Tree;
use std::error::Error;
use std::fmt;
use std::sync::mpsc::{Sender, SyncSender};
use std::sync::Arc;

/// An [`Error`] type for when a the sender half of a channel fails to send a
/// `Tree` across a channel for processing.
#[derive(Debug)]
pub struct ChannelClosedError;

impl fmt::Display for ChannelClosedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        "Sending on a closed channel, this is likely caused by sending from a dangling thread/task. If this is intentional, try adding a fallback with `Processor::or_stderr`.".fmt(f)
    }
}

impl Error for ChannelClosedError {}

/// The result type of [`Processor::process`].
pub type Result = std::result::Result<(), (Tree, Box<dyn Error + Send + Sync>)>;

/// A type that can process [trace trees].
///
/// `Processor`s are responsible for both formatting and writing logs to their
/// intended destinations. This is typically implemented using
/// [`Formatter`], [`MakeWriter`], and [`io::Write`].
///
/// [trace trees]: crate::tree::Tree
/// [`Formatter`]: crate::printer::Formatter
/// [`MakeWriter`]: tracing_subscriber::fmt::MakeWriter
/// [`io::Write`]: std::io::Write
pub trait Processor: 'static + Sized {
    /// Process a [`Tree`]. This can mean many things, such as writing to
    /// stdout or a file, sending over a network, storing in memory, ignoring,
    /// or anything else.
    ///
    /// # Errors
    ///
    /// If the `Tree` cannot be processed, then it is returned along with a
    /// `Box<dyn Error + Send + Sync>`. If the processor is configured with a
    /// fallback processor from [`Processor::or`], then the `Tree` is deferred
    /// to that processor.
    fn process(&self, tree: Tree) -> Result;

    /// Returns a `Processor` that first attempts processing with `self`, and
    /// resorts to processing with `fallback` on failure.
    ///
    /// Note that [`or_stdout`], [`or_stderr`], and [`or_none`] can be used as
    /// shortcuts for pretty printing or dropping the `Tree` entirely.
    ///
    /// [`or_stdout`]: Processor::or_stdout
    /// [`or_stderr`]: Processor::or_stderr
    /// [`or_none`]: Processor::or_none
    fn or<P: Processor>(self, fallback: P) -> WithFallback<Self, P> {
        WithFallback {
            primary: self,
            fallback,
        }
    }

    /// Returns a `Processor` that first attempts processing with `self`, and
    /// resorts to pretty-printing to stdout on failure.
    fn or_stdout(self) -> WithFallback<Self, Printer<Pretty, MakeStdout>> {
        self.or(Printer::new().writer(MakeStdout))
    }

    /// Returns a `Processor` that first attempts processing with `self`, and
    /// resorts to pretty-printing to stderr on failure.
    fn or_stderr(self) -> WithFallback<Self, Printer<Pretty, MakeStderr>> {
        self.or(Printer::new().writer(MakeStderr))
    }

    /// Returns a `Processor` that first attempts processing with `self`, otherwise
    /// silently fails.
    fn or_none(self) -> WithFallback<Self, Sink> {
        self.or(Sink)
    }
}

/// A [`Processor`] composed of a primary and a fallback `Processor`.
///
/// This type is returned by [`Processor::or`].
#[derive(Debug)]
pub struct WithFallback<P, F> {
    primary: P,
    fallback: F,
}

impl<P, F> Processor for WithFallback<P, F>
where
    P: Processor,
    F: Processor,
{
    fn process(&self, tree: Tree) -> Result {
        self.primary.process(tree).or_else(|(tree, err)| {
            eprintln!("{}, using fallback processor...", err);
            self.fallback.process(tree)
        })
    }
}

/// A [`Processor`] that ignores any incoming logs.
///
/// This processor cannot fail.
#[derive(Debug)]
pub struct Sink;

impl Processor for Sink {
    fn process(&self, _tree: Tree) -> Result {
        Ok(())
    }
}

impl<P: Processor> Processor for Box<P> {
    fn process(&self, tree: Tree) -> Result {
        self.as_ref().process(tree)
    }
}

impl<P: Processor> Processor for Arc<P> {
    fn process(&self, tree: Tree) -> Result {
        self.as_ref().process(tree)
    }
}

impl Processor for Sender<Tree> {
    fn process(&self, tree: Tree) -> Result {
        self.send(tree)
            .map_err(|err| (err.0, ChannelClosedError.into()))
    }
}

impl Processor for SyncSender<Tree> {
    fn process(&self, tree: Tree) -> Result {
        self.send(tree)
            .map_err(|err| (err.0, ChannelClosedError.into()))
    }
}

cfg_tokio! {
    use tokio::sync::mpsc::UnboundedSender;

    impl Processor for UnboundedSender<Tree> {
        fn process(&self, tree: Tree) -> Result {
            self.send(tree).map_err(|err| (err.0, ChannelClosedError.into()))
        }
    }
}
