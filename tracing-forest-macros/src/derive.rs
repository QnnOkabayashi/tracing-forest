use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};

pub fn tag(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    // Check that it's visible at the crate level.
    if let syn::Visibility::Restricted(vis) = &input.vis {
        if syn::parse_str::<syn::Path>("crate").expect("Parsing failure") == *vis.path {
            let res = match &input.data {
                syn::Data::Struct(data) => impl_struct(data, &input),
                syn::Data::Enum(data) => impl_enum(data, &input),
                syn::Data::Union(_) => Err(syn::Error::new_spanned(
                    input,
                    "union tags are not supported",
                )),
            };

            return res.unwrap_or_else(|err| err.to_compile_error()).into();
        }
    }
    syn::Error::new_spanned(input.vis, "must be visible in the crate, use `pub(crate)`")
        .to_compile_error()
        .into()
}

#[derive(Clone, Copy)]
enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Parse for Level {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<syn::Ident>()?;
        match ident.to_string().as_str() {
            "trace" => Ok(Level::Trace),
            "debug" => Ok(Level::Debug),
            "info" => Ok(Level::Info),
            "warn" => Ok(Level::Warn),
            "error" => Ok(Level::Error),
            value => {
                let message = format!("invalid level: {}", value);
                Err(syn::Error::new_spanned(ident, message))
            }
        }
    }
}

impl Level {
    fn quote(&self) -> TokenStream2 {
        match self {
            Level::Trace => quote! { trace },
            Level::Debug => quote! { debug },
            Level::Info => quote! { info },
            Level::Warn => quote! { warn },
            Level::Error => quote! { error },
        }
    }

    fn quote_icon(&self) -> TokenStream2 {
        let constant = match self {
            Level::Trace => quote! { TRACE_ICON },
            Level::Debug => quote! { DEBUG_ICON },
            Level::Info => quote! { INFO_ICON },
            Level::Warn => quote! { WARN_ICON },
            Level::Error => quote! { ERROR_ICON },
        };
        quote! { ::tracing_forest::private::#constant }
    }
}

struct TagRepr {
    level: Level,
    icon: Option<syn::LitChar>,
    message: syn::LitStr,
    tag_macro: Option<TagMacro>,
}

struct TagMacro {
    ident: syn::Ident,
    variant_path: TokenStream2,
}

impl ToTokens for TagRepr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let message = &self.message;
        let icon = self
            .icon
            .as_ref()
            .map(|icon| quote! { #icon })
            .unwrap_or_else(|| {
                let level = self.level.quote_icon();
                quote! { #level }
            });

        quote! { ::tracing_forest::tag::TagData { message: #message, icon: #icon } }
            .to_tokens(tokens)
    }
}

impl TagRepr {
    fn declare_macro(&self) -> Option<TokenStream2> {
        self.tag_macro.as_ref().map(|tag_macro| {
            let TagMacro {
                ident,
                variant_path,
            } = tag_macro;
            let level = self.level.quote();

            quote! {
                macro_rules! #ident {
                    ($tokens:tt) => {
                        ::tracing::#level!(
                            __event_tag = ::tracing_forest::Tag::as_field(
                                &$crate::tracing_forest_tag::#variant_path
                            ),
                            $tokens
                        )
                    };
                }
            }
        })
    }
}

fn parse_tag_attr(
    span: &dyn ToTokens,
    fields: &syn::Fields,
    attrs: &[syn::Attribute],
    path: TokenStream2,
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

        if let syn::Meta::List(list) = attr.parse_meta()? {
            let mut lvl = None;
            let mut icon = None;
            let mut msg = None;
            let mut tag_macro = None;
            for field in list.nested.iter() {
                match field {
                    syn::NestedMeta::Meta(syn::Meta::NameValue(namevalue)) => {
                        let ident = namevalue
                            .path
                            .get_ident()
                            .ok_or_else(|| {
                                syn::Error::new_spanned(&namevalue, "Must have a specified ident")
                            })?
                            .to_string()
                            .to_lowercase();
                        match ident.as_str() {
                            "icon" => {
                                if icon.is_some() {
                                    return Err(syn::Error::new_spanned(
                                        namevalue,
                                        "defined `icon` multiple times",
                                    ));
                                } else if let syn::Lit::Char(litchar) = &namevalue.lit {
                                    icon = Some(litchar.clone());
                                } else {
                                    return Err(syn::Error::new_spanned(
                                        namevalue.lit.clone(),
                                        "`icon` accepts a char argument",
                                    ));
                                }
                            }
                            "msg" => {
                                if msg.is_some() {
                                    return Err(syn::Error::new_spanned(
                                        namevalue,
                                        "defined `msg` multiple times",
                                    ));
                                } else if let syn::Lit::Str(litstr) = &namevalue.lit {
                                    msg = Some(litstr.clone());
                                } else {
                                    return Err(syn::Error::new_spanned(
                                        namevalue.lit.clone(),
                                        "`msg` accepts a string literal argument",
                                    ));
                                }
                            }
                            "lvl" => {
                                if lvl.is_some() {
                                    return Err(syn::Error::new_spanned(
                                        namevalue,
                                        "defined `lvl` multiple times",
                                    ));
                                } else if let syn::Lit::Str(litstr) = &namevalue.lit {
                                    match litstr.value().as_str() {
                                        "trace" => lvl = Some(Level::Trace),
                                        "debug" => lvl = Some(Level::Debug),
                                        "info" => lvl = Some(Level::Info),
                                        "warn" => lvl = Some(Level::Warn),
                                        "error" => lvl = Some(Level::Error),
                                        _ => {
                                            return Err(syn::Error::new_spanned(
                                                namevalue.lit.clone(),
                                                r#"`lvl` accepts either "trace", "debug", "info", "warn", or "error""#,
                                            ))
                                        }
                                    }
                                } else {
                                    return Err(syn::Error::new_spanned(
                                        namevalue.lit.clone(),
                                        "`lvl` accepts a string literal argument",
                                    ));
                                }
                            }
                            "macro" => {
                                if tag_macro.is_some() {
                                    return Err(syn::Error::new_spanned(
                                        namevalue,
                                        "defined `macro` multiple times",
                                    ));
                                } else if let syn::Lit::Str(litstr) = &namevalue.lit {
                                    match syn::parse_str::<syn::Ident>(&litstr.value()) {
                                        Ok(mut ident) => {
                                            ident.set_span(litstr.span());
                                            let path = path.clone();
                                            tag_macro = Some(TagMacro { ident, variant_path: path });
                                        }
                                        Err(_) => return Err(syn::Error::new_spanned(
                                            litstr,
                                            "`macro` requires a string literal of a valid ident, received an invalid ident",
                                        )),
                                    }
                                } else {
                                    return Err(syn::Error::new_spanned(
                                        namevalue.lit.clone(),
                                        "`macro` requires an ident as a string literal argument",
                                    ));
                                }
                            }
                            name => {
                                let message = format!(
                                    "Unknown argument `{}` is specified; expected one of: `lvl`, `msg`, `icon`, or `macro`",
                                    name,
                                );
                                return Err(syn::Error::new_spanned(namevalue, message));
                            }
                        }
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            r#"#[tag(..)] only accepts named arguments with literal values, try #[tag(level = "..", msg = "..")]"#,
                        ))
                    }
                }
            }

            if let (Some(level), Some(message)) = (lvl, msg) {
                tag = Some(TagRepr {
                    level,
                    icon,
                    message,
                    tag_macro,
                });
            } else {
                return Err(syn::Error::new_spanned(
                    span,
                    "`lvl` and `msg` are required fields",
                ));
            }
        } else {
            return Err(syn::Error::new_spanned(
                span,
                r#"#[tag(..)] attribute expects a list of arguments, try #[tag(icon = "..", msg = "..")]"#,
            ));
        }
    }

    tag.ok_or_else(|| syn::Error::new_spanned(span, "missing #[tag(..)] attribute"))
}

fn impl_struct(data: &syn::DataStruct, input: &syn::DeriveInput) -> syn::Result<TokenStream2> {
    let ident = &input.ident;
    let tag = parse_tag_attr(input, &data.fields, &input.attrs, quote! { #ident })?;

    let into_arms = quote! { _ => 0, };
    let from_arms = quote! { 0 => #tag, };

    let impl_trait = impl_trait(&input.ident, into_arms, from_arms);
    let declare_macro = tag.declare_macro().unwrap_or_else(|| quote! {});
    Ok(quote! {
        #impl_trait
        #declare_macro
    })
}

fn impl_enum(data: &syn::DataEnum, input: &syn::DeriveInput) -> syn::Result<TokenStream2> {
    let ident = &input.ident;
    let tags = data
        .variants
        .iter()
        .map(|variant| {
            let var_ident = &variant.ident;
            parse_tag_attr(
                variant,
                &variant.fields,
                &variant.attrs,
                quote! { #ident::#var_ident },
            )
        })
        .collect::<syn::Result<Vec<TagRepr>>>()?;

    let len = data.variants.len();
    let variant_names = data.variants.iter().map(|v| &v.ident);
    let ids = 0..len as u64;
    let into_arms = quote! { #( Self::#variant_names => #ids, )* };

    let ids = 0..len as u64;
    let from_arms = quote! { #( #ids => #tags, )* };

    let impl_trait = impl_trait(&input.ident, into_arms, from_arms);
    let declare_macros = tags.iter().filter_map(TagRepr::declare_macro);

    Ok(quote! {
        #impl_trait
        #( #declare_macros )*
    })
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

            fn from_field(value: u64) -> ::tracing_forest::tag::TagData {
                match value {
                    #from_arms
                    _ => panic!("A tag type was set, but an unrecognized tag was sent: {}. Make sure you're using the same tag type, and that you're not using `__event_tag` as a field name for anything except tags.", value),
                }
            }
        }
    }
}
