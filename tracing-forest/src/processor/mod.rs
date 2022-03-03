//! Trait for processing logs of a span after it is closed.
//!
//! See [`Processor`] for more details.

use crate::cfg_sync;
use crate::formatter::Pretty;
use crate::layer::Tree;
use std::sync::mpsc::{Sender, SyncSender};
use std::sync::Arc;

mod printer;
pub use printer::Printer;

/// A type that can process [trace trees][crate::layer::Tree].
///
/// `Processor`s are responsible for both formatting and writing logs to their
/// intended destinations. This is typically implemented using
/// [`Formatter`][crate::formatter::Formatter],
/// [`tracing_subscriber::fmt::MakeWriter`], and [`std::io::Write`].
pub trait Processor: 'static + Sized {
    /// Processes the [`Tree`] of logs. This can mean many things, such as writing
    /// to stdout or a file, sending over a network, storing in memory, ignoring,
    /// or anything else.
    fn process(&self, tree: Tree) -> Result<(), ProcessingError>;

    /// Returns a `Processor` that first attempts to process logs with the `self`
    /// processor, and in case of failure, attempts to process logs with the
    /// fallback processor.
    // TODO(Quinn): write an example
    fn with_fallback<F>(self, fallback: F) -> WithFallback<Self, F>
    where
        F: Processor,
    {
        WithFallback {
            primary: self,
            fallback,
        }
    }

    fn with_stdout_fallback(self) -> WithFallback<Self, Printer<Pretty, fn() -> std::io::Stdout>> {
        let fallback = Printer::new(Pretty::new(), std::io::stdout as _);
        self.with_fallback(fallback)
    }

    fn with_stderr_fallback(self) -> WithFallback<Self, Printer<Pretty, fn() -> std::io::Stderr>> {
        let fallback = Printer::new(Pretty::new(), std::io::stderr as _);
        self.with_fallback(fallback)
    }

    fn with_ignore_fallback(self) -> WithFallback<Self, NoProcessor> {
        self.with_fallback(NoProcessor::new())
    }
}

/// A [`Processor`] processor wrapping a primary and a fallback processor. If the
/// primary processor fails, then the fallback processor is attempted.
///
/// This type is returned by [`Processor::with_fallback`].
pub struct WithFallback<P, F> {
    primary: P,
    fallback: F,
}

impl<P, F> Processor for WithFallback<P, F>
where
    P: Processor,
    F: Processor,
{
    fn process(&self, tree: Tree) -> Result<(), ProcessingError> {
        self.primary
            .process(tree)
            .or_else(|err| self.fallback.process(err.0))
    }
}

/// A [`Processor`] that ignores any incoming logs.
pub struct NoProcessor {
    _priv: (),
}

impl NoProcessor {
    /// Returns a new [`NoProcessor`].
    pub const fn new() -> Self {
        NoProcessor { _priv: () }
    }
}

impl Processor for NoProcessor {
    fn process(&self, tree: Tree) -> Result<(), ProcessingError> {
        let _ = tree;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ProcessingError(pub Tree);

impl<P: Processor> Processor for Box<P> {
    fn process(&self, tree: Tree) -> Result<(), ProcessingError> {
        self.as_ref().process(tree)
    }
}

impl<P: Processor> Processor for Arc<P> {
    fn process(&self, tree: Tree) -> Result<(), ProcessingError> {
        self.as_ref().process(tree)
    }
}

impl Processor for Sender<Tree> {
    fn process(&self, tree: Tree) -> Result<(), ProcessingError> {
        self.send(tree).map_err(|err| ProcessingError(err.0))
    }
}

impl Processor for SyncSender<Tree> {
    fn process(&self, tree: Tree) -> Result<(), ProcessingError> {
        self.send(tree).map_err(|err| ProcessingError(err.0))
    }
}

cfg_sync! {
    use tokio::sync::mpsc::UnboundedSender;

    impl Processor for UnboundedSender<Tree> {
        fn process(&self, tree: Tree) -> Result<(), ProcessingError> {
            self.send(tree).map_err(|err| ProcessingError(err.0))
        }
    }
}
