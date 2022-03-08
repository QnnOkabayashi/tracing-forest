use crate::processor::Processor;
use crate::tree::Tree;
use std::error::Error;
use std::fmt;

/// Error returns by [`Processor::process`].
///
/// Sometimes the processor doesn't consume the [`Tree`], allowing fallback
/// `Processor`s to try processing the tree.
pub struct ProcessReport {
    payload: Option<Tree>,
    reason: Box<dyn Error + Send + Sync>,
}

impl ProcessReport {
    /// Returns a new [`ProcessReport`].
    ///
    /// If the [`Tree`] wasn't consumed, then it can be passed for fallback
    /// processors to use.
    pub fn new(payload: Option<Tree>, reason: Box<dyn Error + Send + Sync>) -> Self {
        ProcessReport { payload, reason }
    }

    pub(crate) fn try_fallback<P: Processor>(self, fallback: &P) -> Result<(), ProcessReport> {
        match self.payload {
            Some(tree) => {
                // TODO(Quinn): accumulate errors instead of only keeping the most recent
                fallback.process(tree)
            }
            _ => Err(self),
        }
    }
}

impl fmt::Debug for ProcessReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Seeing the whole tree is really unnecessary and unhelpful.
        // It would only make sense to show whether the payload were present.
        f.debug_struct("ProcessReport")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for ProcessReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.reason.fmt(f)
    }
}

impl Error for ProcessReport {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.reason.source()
    }
}

/// Error type when a sender processor fails to send a `Tree` across a channel.
///
/// Unlike the built-in `SendError` types that senders return when failing `send`,
/// this type doesn't carry the returned payload. This is because this type is
/// typically within a `ProcessReport` type, which would carry the payload.
#[derive(Debug)]
pub struct SendError;

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Sending on a closed channel. This is likely caused by dangling Tokio tasks. If this is intentional, try adding a fallback with `Processor::with_fallback`.".fmt(f)
    }
}

impl Error for SendError {}
