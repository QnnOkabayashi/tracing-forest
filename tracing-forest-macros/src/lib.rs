//! Macros for use with `tracing-forest`.

#[cfg(any(feature = "attributes", feature = "derive"))]
use proc_macro::TokenStream;

#[cfg(feature = "attributes")]
mod attribute;
#[cfg(feature = "derive")]
mod derive;


/// Derive macro generating an implementation of the 
/// [`Tag`](../tracing-forest/tag/trait.Tag.html) trait.
///
/// See [`tag` module documentation](../tracing-forest/tag/index.html) 
/// for details on how to define and use tags.
#[cfg(feature = "derive")]
#[proc_macro_derive(Tag, attributes(tag))]
pub fn tag(input: TokenStream) -> TokenStream {
    derive::tag(input)
}

/// Marks test to run in the context of a 
/// [`TreeLayer`](../tracing_forest/layer/struct.TreeLayer.html)
/// subscriber, suitable to test environment.
///
/// For more configuration options, see the
/// [`builder` module documentation](../tracing_forest/builder/index.html)
///
/// # Examples
///
/// By default, logs are pretty-printed to stdout.
///
/// ```
/// #[tracing_forest::test]
/// fn test_subscriber() {
///     tracing::info!("Hello, world!");
/// }
/// ```
/// ```log
/// INFO     💬 [info]: Hello, world!
/// ```
/// Equivalent code not using `#[tracing_forest::test]`
/// ```
/// #[test]
/// fn test_subscriber() {
///     tracing_forest::builder()
///         .with_test_writer()
///         .blocking_layer()
///         .on_closure(|| {
///             tracing::info!("Hello, world!");
///         })
/// }
/// ```
///
/// ### Tags and formatting
///
/// Custom tags and formatting can be configured with the `tag` and `fmt`
/// arguments respectively. Currently, only `"pretty"` and `"json"` are
/// supported formats, and tag types must implement the 
/// [`Tag`](../tracing_forest/tag/trait.Tag.html) trait.
///
/// ```
/// tracing_forest::declare_tags! {
///     use tracing_forest::Tag;
///
///     #[derive(Tag)]
///     pub(crate) enum GreetingTag {
///         #[tag(lvl = "info", msg = "greeting", macro = "greeting")]
///         Greeting,
///     }
/// }
///
/// #[tracing_forest::test(tag = "GreetingTag", fmt = "json")]
/// fn test_tag_and_formatted() {
///     greeting!("Hello in JSON");
/// }
/// ```
/// ```json
/// {"level":"INFO","kind":{"Event":{"tag":,"message":"Hello in JSON","fields":{}}}}
/// ```
/// Equivalent code not using `#[tracing_forest::test]`
/// ```
/// tracing_forest::declare_tags! {
///     use tracing_forest::Tag;
///
///     #[derive(Tag)]
///     pub(crate) enum GreetingTag {
///         #[tag(lvl = "info", msg = "greeting", macro = "greeting")]
///         Greeting,
///     }
/// }
///
/// #[test]
/// fn test_tags_and_formatted() {
///     tracing_forest::builder()
///         .json()
///         .with_test_writer()
///         .with_tag::<crate::tracing_forest_tag::GreetingTag>()
///         .blocking_layer()
///         .on_closure(|| {
///             tracing::info!("Hello, world!");
///         })
/// }
/// ```
///
/// ### Using with Tokio runtime
///
/// When the `sync` feature is enabled, this attribute can also be proceeded by
/// the [`#[tokio::test]`] attribute to run the test in the context of an async 
/// runtime.
/// ```
/// #[tracing_forest::test]
/// #[tokio::test]
/// async fn test_tokio() {
///     tracing::info!("Hello from Tokio!");
/// }
/// ```
/// ```log
/// INFO     💬 [info]: Hello from Tokio!
/// ```
/// Equivalent code not using `#[tracing_forest::test]`
/// ```
/// #[tokio::test]
/// async fn test_tokio() {
///     tracing_forest::builder()
///         .with_test_writer()
///         .async_layer()
///         .on_future(async {
///             tracing::info!("Hello from Tokio!");
///         })
///         .await
/// }
/// ```
#[cfg(feature = "attributes")]
#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    attribute::test(args, item)
}

/// Marks function to run in the context of a 
/// [`TreeLayer`](../tracing_forest/layer/struct.TreeLayer.html) subscriber.
///
/// For more configuration options, see the
/// [`builder` module documentation](../tracing_forest/builder/index.html)
///
/// # Examples
///
/// By default, logs are pretty-printed to stdout.
///
/// ```
/// # #[allow(clippy::needless_doctest_main)]
/// #[tracing_forest::main]
/// fn main() {
///     tracing::info!("Hello, world!");
/// }
/// ```
/// ```log
/// INFO     💬 [info]: Hello, world!
/// ```
/// Equivalent code not using `#[tracing_forest::main]`
/// ```
/// # #[allow(clippy::needless_doctest_main)]
/// fn main() {
///     tracing_forest::builder()
///         .blocking_layer()
///         .on_closure(|| {
///             tracing::info!("Hello, world!");
///         })
/// }
/// ```
///
/// ### Tags and formatting
///
/// Custom tags and formatting can be configured with the `tag` and `fmt`
/// arguments respectively. Currently, only `"pretty"` and `"json"` are
/// supported formats, and tag types must implement the 
/// [`Tag`](../tracing_forest/tag/trait.Tag.html) trait.
///
/// ```
/// tracing_forest::declare_tags! {
///     use tracing_forest::Tag;
///
///     #[derive(Tag)]
///     pub(crate) enum GreetingTag {
///         #[tag(lvl = "info", msg = "greeting", macro = "greeting")]
///         Greeting,
///     }
/// }
///
/// #[tracing_forest::main(tag = "GreetingTag", fmt = "json")]
/// fn main() {
///     greeting!("Hello in JSON");
/// }
/// ```
/// ```json
/// {"level":"INFO","kind":{"Event":{"tag":"greeting","message":"Hello in JSON","fields":{}}}}
/// ```
/// Equivalent code not using `#[tracing_forest::main]`
/// ```
/// tracing_forest::declare_tags! {
///     use tracing_forest::Tag;
///
///     #[derive(Tag)]
///     pub(crate) enum GreetingTag {
///         #[tag(lvl = "info", msg = "greeting", macro = "greeting")]
///         Greeting,
///     }
/// }
///
/// # #[allow(clippy::needless_doctest_main)]
/// fn main() {
///     tracing_forest::builder()
///         .json()
///         .with_tag::<crate::tracing_forest_tag::GreetingTag>()
///         .blocking_layer()
///         .on_closure(|| {
///             greeting!("Hello in JSON");
///         })
/// }
/// ```
///
/// ### Using with Tokio runtime
///
/// When the `sync` feature is enabled, this attribute can also be proceeded by
/// the [`#[tokio::main]`] attribute to run the function in the
/// context of an async runtime.
/// ```
/// #[tracing_forest::main]
/// #[tokio::main(flavor = "current_thread")]
/// async fn main() {
///     tracing::info!("Hello from Tokio!");
/// }
/// ```
/// ```log
/// INFO     💬 [info]: Hello from Tokio!
/// ```
/// Equivalent code not using `#[tracing_forest::main]`
/// ```
/// # #[allow(clippy::needless_doctest_main)]
/// #[tokio::main(flavor = "current_thread")]
/// async fn main() {
///     tracing_forest::builder()
///         .async_layer()
///         .on_future(async {
///             tracing::info!("Hello from Tokio!");
///         })
///         .await
/// }
/// ```
#[cfg(feature = "attributes")]
#[proc_macro_attribute]
pub fn main(args: TokenStream, item: TokenStream) -> TokenStream {
    attribute::main(args, item)
}
