use tracing::Subscriber;
use tracing_subscriber::{Registry, registry::LookupSpan};
use uuid::Uuid;
use crate::layer::OpenedSpan;
use crate::fail;

/// Gets the current [`Uuid`] of an entered span within a `tracing-forest`
/// subscriber.
///
/// # Examples
///
/// Passing in a `Uuid` to a span, and then retreiving it from within the span:
/// ```
/// # use tracing::{info, info_span};
/// # use uuid::Uuid;
/// # #[tokio::main]
/// # async fn main() {
/// # tracing_forest::new().on_registry().on(async {
/// let uuid = Uuid::new_v4();
/// 
/// info_span!("my_span", %uuid).in_scope(|| {
///     assert!(tracing_forest::id() == uuid);
/// })
/// # }).await;
/// # }
/// ```
///
/// # Panics
///
/// This function panics if there is no current subscriber, if the subscriber
/// isn't composed with a [`ForestLayer`], or if the subscriber isn't in a span.
///
/// [`ForestLayer`]: crate::layer::ForestLayer
#[must_use]
pub fn id() -> Uuid {
    tracing::dispatcher::get_default(|dispatch| {
        let subscriber = dispatch
            .downcast_ref::<Registry>()
            .unwrap_or_else(fail::subscriber_not_found::<Registry>);

        let current = subscriber.current_span();

        let id = current.id().unwrap_or_else(fail::no_current_span);

        subscriber
            .span(id)
            .unwrap_or_else(fail::span_not_in_ctx)
            .extensions()
            .get::<OpenedSpan>()
            .unwrap_or_else(fail::no_forest_layer)
            .uuid()
    })
}

// Credit: https://github.com/uuid-rs/uuid/blob/main/src/parser.rs

pub(crate) const fn try_parse(input: &[u8]) -> Result<Uuid, ()> {
    match (input.len(), input) {
        // Inputs of 32 bytes must be a non-hyphenated UUID
        (32, s) => parse_simple(s),
        // Hyphenated UUIDs may be wrapped in various ways:
        // - `{UUID}` for braced UUIDs
        // - `urn:uuid:UUID` for URNs
        // - `UUID` for a regular hyphenated UUID
        (36, s)
        | (38, [b'{', s @ .., b'}'])
        | (
            45,
            [b'u', b'r', b'n', b':', b'u', b'u', b'i', b'd', b':', s @ ..],
        ) => parse_hyphenated(s),
        // Any other shaped input is immediately invalid
        _ => Err(()),
    }
}

#[inline]
const fn parse_simple(s: &[u8]) -> Result<Uuid, ()> {
    // This length check here removes all other bounds
    // checks in this function
    if s.len() != 32 {
        return Err(());
    }

    let mut buf: [u8; 16] = [0; 16];
    let mut i = 0;

    while i < 16 {
        // Convert a two-char hex value (like `A8`)
        // into a byte (like `10101000`)
        let h1 = HEX_TABLE[s[i * 2] as usize];
        let h2 = HEX_TABLE[s[i * 2 + 1] as usize];

        // We use `0xff` as a sentinel value to indicate
        // an invalid hex character sequence (like the letter `G`)
        if h1 | h2 == 0xff {
            return Err(());
        }

        // The upper nibble needs to be shifted into position
        // to produce the final byte value
        buf[i] = SHL4_TABLE[h1 as usize] | h2;
        i += 1;
    }

    Ok(Uuid::from_bytes(buf))
}

#[inline]
const fn parse_hyphenated(s: &[u8]) -> Result<Uuid, ()> {
    // This length check here removes all other bounds
    // checks in this function
    if s.len() != 36 {
        return Err(());
    }

    // We look at two hex-encoded values (4 chars) at a time because
    // that's the size of the smallest group in a hyphenated UUID.
    // The indexes we're interested in are:
    //
    // uuid     : 936da01f-9abd-4d9d-80c7-02af85c822a8
    //            |   |   ||   ||   ||   ||   |   |
    // hyphens  : |   |   8|  13|  18|  23|   |   |
    // positions: 0   4    9   14   19   24  28  32

    // First, ensure the hyphens appear in the right places
    match [s[8], s[13], s[18], s[23]] {
        [b'-', b'-', b'-', b'-'] => {}
        _ => return Err(()),
    }

    let positions: [u8; 8] = [0, 4, 9, 14, 19, 24, 28, 32];
    let mut buf: [u8; 16] = [0; 16];
    let mut j = 0;

    while j < 8 {
        let i = positions[j];

        // The decoding here is the same as the simple case
        // We're just dealing with two values instead of one
        let h1 = HEX_TABLE[s[i as usize] as usize];
        let h2 = HEX_TABLE[s[(i + 1) as usize] as usize];
        let h3 = HEX_TABLE[s[(i + 2) as usize] as usize];
        let h4 = HEX_TABLE[s[(i + 3) as usize] as usize];

        if h1 | h2 | h3 | h4 == 0xff {
            return Err(());
        }

        buf[j * 2] = SHL4_TABLE[h1 as usize] | h2;
        buf[j * 2 + 1] = SHL4_TABLE[h3 as usize] | h4;
        j += 1;
    }

    Ok(Uuid::from_bytes(buf))
}

const HEX_TABLE: &[u8; 256] = &{
    let mut buf = [0; 256];
    let mut i: u8 = 0;

    loop {
        buf[i as usize] = match i {
            b'0'..=b'9' => i - b'0',
            b'a'..=b'f' => i - b'a' + 10,
            b'A'..=b'F' => i - b'A' + 10,
            _ => 0xff,
        };

        if i == 255 {
            break buf;
        }

        i += 1
    }
};

const SHL4_TABLE: &[u8; 256] = &{
    let mut buf = [0; 256];
    let mut i: u8 = 0;

    loop {
        buf[i as usize] = i.wrapping_shl(4);

        if i == 255 {
            break buf;
        }

        i += 1;
    }
};

