//! Types relating to the core tree structure of `tracing-forest`.
//!
use crate::tag::Tag;
use crate::{cfg_chrono, cfg_uuid};
#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
#[cfg(feature = "serde")]
use serde::Serialize;
use std::time::Duration;
use tracing::Level;
#[cfg(feature = "uuid")]
use uuid::Uuid;
#[cfg(feature = "serde")]
mod ser;

mod error;
use error::{ExpectedEventError, ExpectedSpanError};

mod field;
pub use field::Field;
pub(crate) use field::FieldSet;

/// The core tree structure of `tracing-forest`.
///
///
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Tree {
    Event(Event),
    Span(Span),
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Event {
    /// Shared fields between events and spans.
    pub(crate) shared: Shared,

    /// The message associated with the event.
    pub(crate) message: Option<String>,

    /// The tag that the event was collected with.
    pub(crate) tag: Tag,

    /// Key-value data.
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::fields"))]
    pub(crate) fields: FieldSet,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Span {
    /// Shared fields between events and spans.
    pub(crate) shared: Shared,

    /// The name of the span.
    pub(crate) name: &'static str,

    /// Events and spans collected while the span was open.
    pub(crate) children: Vec<Tree>,

    /// The total duration the span was open for.
    #[cfg_attr(
        feature = "serde",
        serde(rename = "nanos_total", serialize_with = "ser::nanos")
    )]
    pub(crate) total_duration: Duration,

    /// The total duration inner spans were open for.
    #[cfg_attr(
        feature = "serde",
        serde(rename = "nanos_nested", serialize_with = "ser::nanos")
    )]
    pub(crate) inner_duration: Duration,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub(crate) struct Shared {
    /// The ID of the event or span.
    #[cfg(feature = "uuid")]
    pub(crate) uuid: Uuid,

    /// When the event occurred or when the span opened.
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::timestamp"))]
    pub(crate) timestamp: DateTime<Utc>,

    /// The level the event or span occurred at.
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::level"))]
    pub(crate) level: Level,
}

impl Tree {
    /// Returns the inner [`Event`] if the tree is an event.
    ///
    /// # Examples
    ///
    /// Inspecting a `Tree` returned from [`capture`]:
    /// ```
    /// # use tracing::{info, info_span};
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let logs = tracing_forest::capture()
    ///     .on_registry()
    ///     .on(async {
    ///         info!("inside the span");
    ///     })
    ///     .await;
    ///
    /// assert!(logs.len() == 1);
    ///
    /// let event = logs[0].event()?;
    /// assert!(my_span.message() == Some("inside the span"));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`capture`]: crate::builder::capture
    pub fn event(&self) -> Result<&Event, ExpectedEventError> {
        match self {
            Tree::Event(event) => Ok(event),
            Tree::Span(_) => Err(ExpectedEventError(())),
        }
    }

    /// Returns the inner [`Span`] if the tree is a span.
    ///
    /// # Examples
    ///
    /// Inspecting a `Tree` returned from [`capture`]:
    /// ```
    /// # use tracing::{info, info_span};
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let logs = tracing_forest::capture()
    ///     .on_registry()
    ///     .on(async {
    ///         info_span!("my_span").in_scope(|| {
    ///             info!("inside the span");
    ///         });
    ///     })
    ///     .await;
    ///
    /// assert!(logs.len() == 1);
    ///
    /// let my_span = logs[0].span()?;
    /// assert!(my_span.name() == "my_span");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`capture`]: crate::builder::capture
    pub fn span(&self) -> Result<&Span, ExpectedSpanError> {
        match self {
            Tree::Event(_) => Err(ExpectedSpanError(())),
            Tree::Span(span) => Ok(span),
        }
    }
}

impl Event {
    cfg_uuid! {
        /// Returns the events [`Uuid`].
        pub fn uuid(&self) -> Uuid {
            self.shared.uuid
        }
    }

    cfg_chrono! {
        /// Returns the [`DateTime`] that the event occurred at.
        pub fn timestamp(&self) -> DateTime<Utc> {
            self.shared.timestamp
        }
    }

    /// Returns the events [`Level`].
    pub fn level(&self) -> Level {
        self.shared.level
    }

    /// Returns the events message.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Returns the events [`Tag`].
    ///
    /// If no tag was provided during construction, the event will hold a default
    /// tag associated with its level.
    pub fn tag(&self) -> &Tag {
        &self.tag
    }

    /// Returns the events fields as an slice of key-value pairs.
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }
}

impl Span {
    cfg_uuid! {
        /// Returns the spans [`Uuid`].
        pub fn uuid(&self) -> Uuid {
            self.shared.uuid
        }
    }

    cfg_chrono! {
        /// Returns the [`DateTime`] that the span occurred at.
        pub fn timestamp(&self) -> DateTime<Utc> {
            self.shared.timestamp
        }
    }

    /// Returns the spans [`Level`].
    pub fn level(&self) -> Level {
        self.shared.level
    }

    /// Returns the events message.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn children(&self) -> &[Tree] {
        &self.children
    }

    pub fn total_duration(&self) -> Duration {
        self.total_duration
    }

    pub fn inner_duration(&self) -> Duration {
        self.inner_duration
    }

    pub fn base_duration(&self) -> Duration {
        self.total_duration - self.inner_duration
    }
}
