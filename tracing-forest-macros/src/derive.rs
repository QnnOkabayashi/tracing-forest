use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::parse::Parse;

pub fn tag(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let res = match &input.data {
        syn::Data::Struct(data) => impl_struct(data, &input),
        syn::Data::Enum(data) => impl_enum(data, &input),
        syn::Data::Union(_) => Err(syn::Error::new_spanned(
            input,
            "union tags are not supported",
        )),
    };

    res.unwrap_or_else(|err| err.to_compile_error()).into()
}

mod kw {
    syn::custom_keyword!(trace);
    syn::custom_keyword!(debug);
    syn::custom_keyword!(info);
    syn::custom_keyword!(warn);
    syn::custom_keyword!(error);
    syn::custom_keyword!(custom);
}

struct TagRepr {
    icon: Icon,
    _colon: syn::Token![:],
    message: syn::LitStr,
}

impl Parse for TagRepr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(TagRepr {
            icon: input.parse()?,
            _colon: input.parse()?,
            message: input.parse()?,
        })
    }
}

impl ToTokens for TagRepr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let message = &self.message;
        let icon = self.icon.value();
        (quote! { ::tracing_forest::private::TagData { message: #message, icon: #icon } })
            .to_tokens(tokens)
    }
}

enum Icon {
    Trace {
        _trace: kw::trace,
    },
    Debug {
        _debug: kw::debug,
    },
    Info {
        _info: kw::info,
    },
    Warn {
        _warn: kw::warn,
    },
    Error {
        _error: kw::error,
    },
    Custom {
        _custom: kw::custom,
        _paren: syn::token::Paren,
        icon: syn::LitChar,
    },
}

impl Icon {
    fn value(&self) -> TokenStream2 {
        match self {
            Icon::Trace { .. } => quote! { ::tracing_forest::private::TRACE_ICON },
            Icon::Debug { .. } => quote! { ::tracing_forest::private::DEBUG_ICON },
            Icon::Info { .. } => quote! { ::tracing_forest::private::INFO_ICON },
            Icon::Warn { .. } => quote! { ::tracing_forest::private::WARN_ICON },
            Icon::Error { .. } => quote! { ::tracing_forest::private::ERROR_ICON },
            Icon::Custom { icon, .. } => quote! { #icon },
        }
    }
}

impl Parse for Icon {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let icon = if input.peek(kw::trace) {
            Icon::Trace {
                _trace: input.parse()?,
            }
        } else if input.peek(kw::debug) {
            Icon::Debug {
                _debug: input.parse()?,
            }
        } else if input.peek(kw::info) {
            Icon::Info {
                _info: input.parse()?,
            }
        } else if input.peek(kw::warn) {
            Icon::Warn {
                _warn: input.parse()?,
            }
        } else if input.peek(kw::error) {
            Icon::Error {
                _error: input.parse()?,
            }
        } else if input.peek(kw::custom) {
            let content;
            Icon::Custom {
                _custom: input.parse::<kw::custom>()?,
                _paren: syn::parenthesized!(content in input),
                icon: content.parse::<syn::LitChar>()?,
            }
        } else {
            return Err(input
                .error("must begin with `trace`, `debug`, `info`, `warn`, `error`, or `custom`"));
        };

        Ok(icon)
    }
}

fn parse_tag_attr(
    span: &dyn ToTokens,
    fields: &syn::Fields,
    attrs: &[syn::Attribute],
) -> syn::Result<TagRepr> {
    if !matches!(fields, syn::Fields::Unit) {
        return Err(syn::Error::new_spanned(fields, "expected unit type"));
    }

    let mut tag = None;

    for attr in attrs.iter() {
        if !attr.path.is_ident("tag") {
            continue;
        }

        if tag.is_some() {
            return Err(syn::Error::new_spanned(
                attr,
                "cannot have multiple #[tag(...)] attributes on same item",
            ));
        }

        tag = Some(attr.parse_args()?);
    }

    tag.ok_or_else(|| syn::Error::new_spanned(span, "missing #[tag(...)] attribute"))
}

fn impl_struct(data: &syn::DataStruct, input: &syn::DeriveInput) -> syn::Result<TokenStream2> {
    let tag = parse_tag_attr(input, &data.fields, &input.attrs)?;

    let into_arms = quote! { _ => 0, };
    let from_arms = quote! { 0 => #tag, };

    Ok(impl_trait(&input.ident, into_arms, from_arms))
}

fn impl_enum(data: &syn::DataEnum, input: &syn::DeriveInput) -> syn::Result<TokenStream2> {
    let tags = data
        .variants
        .iter()
        .map(|variant| parse_tag_attr(variant, &variant.fields, &variant.attrs))
        .collect::<syn::Result<Vec<TagRepr>>>()?;

    let len = data.variants.len();
    let variant_names = data.variants.iter().map(|v| &v.ident);
    let ids = 0..len as u64;
    let into_arms = quote! { #( Self::#variant_names => #ids, )* };

    let ids = 0..len as u64;
    let from_arms = quote! { #( #ids => #tags, )* };

    Ok(impl_trait(&input.ident, into_arms, from_arms))
}

fn impl_trait(
    name: &proc_macro2::Ident,
    into_arms: TokenStream2,
    from_arms: TokenStream2,
) -> TokenStream2 {
    quote! {
        unsafe impl ::tracing_forest::Tag for #name {
            fn as_field(&self) -> u64 {
                match *self {
                    #into_arms
                }
            }

            fn from_field(value: u64) -> ::tracing_forest::private::TagData {
                match value {
                    #from_arms
                    _ => ::tracing_forest::private::unrecognized_tag_id(value),
                }
            }
        }
    }
}
