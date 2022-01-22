# tracing-forest
[![github-img]][github-url] [![crates-img]][crates-url] [![docs-img]][docs-url]

[github-url]: https://github.com/QnnOkabayashi/tracing-forest
[crates-url]: https://crates.io/crates/tracing-forest
[docs-url]: https://docs.rs/tracing-forest/latest/tracing_forest/
[github-img]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
[crates-img]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
[docs-img]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K

Preserve contextual coherence among trace data from concurrent tasks.

# Overview

[`tracing`] is a framework for instrumenting programs to collect structured
and async-aware diagnostics. The [`tracing-subscriber`] crate provides tools for
working with [`tracing`]. This crate extends [`tracing-subscriber`] by providing
types capable of preserving contextual coherence of trace data from concurrent 
tasks when logging.

This crate is intended for programs running many nontrivial and disjoint
tasks concurrently, like server backends. Unlike other `Subscriber`s which
simply keep track of the context of an event, `tracing-forest` preserves
the contextual coherence when writing logs, allowing readers to easily trace
a sequence of events from the same task.

`tracing-forest` provides many tools for authoring applications, but can
also be extended to author other libraries.

[`tracing`]: https://crates.io/crates/tracing
[`tracing-subscriber`]: https://crates.io/crates/tracing-subscriber

## Getting started

The easiest way to get started is to enable all features. Do this by
adding the following to your `Cargo.toml` file:
```toml
tracing-forest = { version = "1", features = ["full"] }
```
Then, add the `#[tracing_forest::main]` attribute to your main function:
```rust
#[tracing_forest::main]
fn main() {
    // do stuff here...
    tracing::trace!("Hello, world!");
}
```

## Contextual Coherence in action

This example contains two counters, one for evens and another for odds.
Running it will emit trace data at the root level, implying that all the
events are _independent_, meaning each trace will be processed and written
as it's collected. In this case, the logs will count up chronologically.
```rust
let evens = async {
    for i in 0..3 {
        tracing::info!("{}", i * 2);
        // pause for `odds`
        sleep(Duration::from_millis(100)).await;
    }
};

let odds = async {
    // pause for `evens`
    sleep(Duration::from_millis(50)).await;
    for i in 0..3 {
        tracing::info!("{}", i * 2 + 1);
        // pause for `evens`
        sleep(Duration::from_millis(100)).await;
    }
};

let _ = tokio::join!(evens, odds);
```
```log
INFO     💬 [info]: 0
INFO     💬 [info]: 1
INFO     💬 [info]: 2
INFO     💬 [info]: 3
INFO     💬 [info]: 4
INFO     💬 [info]: 5
```
Instrumenting the counters tells the `TreeLayer` in the current subscriber to 
preserve the contextual coherence of trace data from each task. Traces from the 
`even` counter will be grouped, and traces from the `odd` counter will be 
grouped.
```rust
let evens = async {
    // ...
}.instrument(tracing::trace_span!("counting_evens"));

let odds = async {
    // ...
}.instrument(tracing::trace_span!("counting_odds"));
    
let _ = tokio::join!(evens, odds);
```
```log
TRACE    counting_evens [ 409µs | 100.000% ]
INFO     ┕━ 💬 [info]: 0
INFO     ┕━ 💬 [info]: 2
INFO     ┕━ 💬 [info]: 4
TRACE    counting_odds [ 320µs | 100.000% ]
INFO     ┕━ 💬 [info]: 1
INFO     ┕━ 💬 [info]: 3
INFO     ┕━ 💬 [info]: 5
```
Although the numbers were logged chronologically, they appear grouped in the 
spans they originated from.

## License
`tracing-forest` is open-source software, distributed under the MIT license.