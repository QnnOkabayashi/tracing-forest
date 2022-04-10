use std::error::Error;
use std::fmt;

/// Error returned by [`Tree::event`][event].
///
/// [event]: crate::tree::Tree::event
#[derive(Debug)]
pub struct ExpectedEventError;

/// Error returned by [`Tree::span`][span].
///
/// [span]: crate::tree::Tree::span
#[derive(Debug)]
pub struct ExpectedSpanError;

impl Error for ExpectedEventError {}

impl Error for ExpectedSpanError {}

impl fmt::Display for ExpectedEventError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        "Expected an event, found a span".fmt(f)
    }
}

impl fmt::Display for ExpectedSpanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        "Expected a span, found an event".fmt(f)
    }
}
