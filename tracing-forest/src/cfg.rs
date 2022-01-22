#[doc(hidden)]
#[macro_export]
macro_rules! cfg_sync {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "sync")]
            #[cfg_attr(docsrs, doc(cfg(feature = "sync")))]
            $item
        )*
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! cfg_json {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "json")]
            #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
            $item
        )*
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! cfg_uuid {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "uuid")]
            #[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
            $item
        )*
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! cfg_chrono {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "chrono")]
            #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
            $item
        )*
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! cfg_derive {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "derive")]
            #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
            $item
        )*
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! cfg_attributes {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "attributes")]
            #[cfg_attr(docsrs, doc(cfg(feature = "attributes")))]
            $item
        )*
    }
}
