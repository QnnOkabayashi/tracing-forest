#[doc(hidden)]
#[macro_export]
macro_rules! cfg_sync {
    ($($item:item)*) => {
        $( #[cfg(feature = "sync")] $item )*
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! cfg_json {
    ($($item:item)*) => {
        $( #[cfg(feature = "json")] $item )*
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! cfg_id {
    ($($item:item)*) => {
        $( #[cfg(feature = "id")] $item )*
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! cfg_derive {
    ($($item:item)*) => {
        $( #[cfg(feature = "tracing-forest-macros")] $item )*
    }
}
