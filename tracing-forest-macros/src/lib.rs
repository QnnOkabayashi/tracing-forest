//! `tracing-forest` macros.

use proc_macro::TokenStream;
pub(crate) type AttributeArgs = syn::punctuated::Punctuated<syn::NestedMeta, syn::Token![,]>;

#[cfg(feature = "attributes")]
mod attribute;
#[cfg(feature = "derive")]
mod derive;

#[cfg(feature = "derive")]
#[proc_macro_derive(Tag, attributes(tag))]
pub fn tag(input: TokenStream) -> TokenStream {
    // quote::quote!{}.into()
    derive::tag(input)
}

#[cfg(feature = "attributes")]
#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    attribute::test(args, item)
}

#[cfg(feature = "attributes")]
#[proc_macro_attribute]
pub fn main(args: TokenStream, item: TokenStream) -> TokenStream {
    attribute::main(args, item)
}
