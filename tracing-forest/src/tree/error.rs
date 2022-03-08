use std::fmt;

/// Error returned by [`Tree::event`][event].
///
/// [event]: crate::tree::Tree::event
#[derive(Debug)]
pub struct ExpectedEventError(pub(super) ());

/// Error returned by [`Tree::span`][span].
///
/// [span]: crate::tree::Tree::span
#[derive(Debug)]
pub struct ExpectedSpanError(pub(super) ());

impl std::error::Error for ExpectedEventError {}

impl std::error::Error for ExpectedSpanError {}

impl fmt::Display for ExpectedEventError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("Expected an event, found a span")
    }
}

impl fmt::Display for ExpectedSpanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("Expected a span, found an event")
    }
}
