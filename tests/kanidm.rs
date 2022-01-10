use tracing_forest::Tag;

#[derive(Tag)]
pub enum KanidmTag {
    #[tag(info: "admin.info")]
    AdminInfo,
    #[tag(error: "request.error")]
    RequestError,
    #[tag(custom('🔐'): "security.critical")]
    SecurityCritical,
}

#[allow(unused_macros)]
macro_rules! admin_info {
    ($tokens:tt) => {
        ::tracing::info!(
            __event_tag = ::tracing_forest::Tag::as_field(&$crate::KanidmTag::AdminInfo),
            $tokens
        )
    };
}

#[allow(unused_macros)]
macro_rules! request_error {
    ($tokens:tt) => {
        ::tracing::error!(
            __event_tag = ::tracing_forest::Tag::as_field(&$crate::KanidmTag::RequestError),
            $tokens
        )
    };
}

#[allow(unused_macros)]
macro_rules! security_critical {
    ($tokens:tt) => {
        ::tracing::error!(
            __event_tag = ::tracing_forest::Tag::as_field(&$crate::KanidmTag::SecurityCritical),
            $tokens
        )
    };
}
