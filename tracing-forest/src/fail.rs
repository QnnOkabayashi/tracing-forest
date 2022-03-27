use crate::{cfg_uuid, processor::ProcessReport};

#[cold]
#[inline(never)]
pub fn span_not_in_ctx<T>() -> T {
    panic!("Span not in context, this is a bug");
}

#[cold]
#[inline(never)]
pub fn opened_span_not_in_exts<T>() -> T {
    panic!("Span extension doesn't contain `OpenedSpan`, this is a bug");
}

cfg_uuid! {
    #[cold]
    #[inline(never)]
    pub fn subscriber_not_found<'a, S>() -> &'a S {
        panic!(
            "Subscriber could not be downcasted to `{}`",
            std::any::type_name::<S>()
        );
    }

    #[cold]
    #[inline(never)]
    pub fn no_current_span<T>() -> T {
        panic!("The subscriber isn't in any spans");
    }

    #[cold]
    #[inline(never)]
    pub fn no_forest_layer<T>() -> T {
        panic!("The span has no `Span` in extensions, perhaps you forgot to add a `ForestLayer` to your subscriber?");
    }
}

#[allow(clippy::needless_pass_by_value)]
#[cold]
#[inline(never)]
pub fn processing_error(report: ProcessReport) {
    panic!("Processing logs failed: {}", report);
}
