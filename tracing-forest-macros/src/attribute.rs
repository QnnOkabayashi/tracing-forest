use crate::AttributeArgs;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::Parser;

fn token_stream_to_compile_err(mut tokens: TokenStream, err: syn::Error) -> TokenStream {
    tokens.extend(TokenStream::from(err.into_compile_error()));
    tokens
}

pub fn main(args: TokenStream, item: TokenStream) -> TokenStream {
    let input: syn::ItemFn = match syn::parse(item.clone()) {
        Ok(input) => input,
        Err(e) => return token_stream_to_compile_err(item, e),
    };

    impl_attribute(input, args, false).unwrap_or_else(|e| token_stream_to_compile_err(item, e))
}

pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    let input: syn::ItemFn = match syn::parse(item.clone()) {
        Ok(input) => input,
        Err(e) => return token_stream_to_compile_err(item, e),
    };

    if let Some(attr) = input.attrs.iter().find(|attr| attr.path.is_ident("test")) {
        let msg = "Second #[test] attribute is supplied";
        return token_stream_to_compile_err(item, syn::Error::new_spanned(&attr, msg));
    }

    impl_attribute(input, args, true).unwrap_or_else(|e| token_stream_to_compile_err(item, e))
}

#[cfg(feature = "sync")]
fn ident(string: &str) -> proc_macro2::Ident {
    proc_macro2::Ident::new(string, proc_macro2::Span::call_site())
}

#[cfg(feature = "sync")]
fn tokio_attribute_path(is_test: bool) -> syn::Path {
    let mut segments = syn::punctuated::Punctuated::new();
    segments.push(ident("tokio").into());
    segments.push(ident(if is_test { "test" } else { "main" }).into());

    syn::Path {
        leading_colon: None,
        segments,
    }
}

fn impl_attribute(
    input: syn::ItemFn,
    args: TokenStream,
    is_test: bool,
) -> syn::Result<TokenStream> {
    if !input.sig.inputs.is_empty() {
        let msg = "Cannot accept arguments";
        return Err(syn::Error::new_spanned(&input.sig.ident, msg));
    }

    let args = AttributeArgs::parse_terminated.parse(args)?;

    if let Some(_async) = input.sig.asyncness {
        #[cfg(not(feature = "sync"))]
        return Err(syn::Error::new_spanned(
            _async,
            "feature `sync` required for async functions",
        ));

        #[cfg(feature = "sync")]
        {
            let path = tokio_attribute_path(is_test);

            if !input.attrs.iter().any(|attr| attr.path == path) {
                let msg = if is_test {
                    "Attribute must be succeeded by #[tokio::test] for async tests"
                } else {
                    "Attribute must be succeeded by #[tokio::main] for async functions"
                };
                return Err(syn::Error::new_spanned(args, msg));
            }

            impl_async(Config::parse(args, is_test)?, input)
        }
    } else {
        impl_sync(Config::parse(args, is_test)?, input)
    }
}

#[cfg(feature = "sync")]
fn impl_async(config: Config, mut input: syn::ItemFn) -> syn::Result<TokenStream> {
    let builder = config.builder();

    let brace_token = input.block.brace_token;
    let block = input.block;
    input.block = syn::parse2(quote! {
        {
            #builder.build_async().in_future(async #block).await
        }
    })
    .expect("Parsing failure");
    input.block.brace_token = brace_token;

    Ok(quote! { #input }.into())
}

fn impl_sync(config: Config, mut input: syn::ItemFn) -> syn::Result<TokenStream> {
    let header = if config.is_test {
        quote! { #[::core::prelude::v1::test] }
    } else {
        quote! {}
    };

    let builder = config.builder();

    let brace_token = input.block.brace_token;
    let block = input.block;
    input.block = syn::parse2(quote! {
        {
            #builder.build_blocking().in_closure(|| #block)
        }
    })
    .expect("Parsing failure");
    input.block.brace_token = brace_token;

    Ok(quote! {
        #header
        #input
    }
    .into())
}

enum Formatter {
    Json,
    Pretty,
}

impl ToTokens for Formatter {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(match self {
            Formatter::Json => quote! { .json() },
            Formatter::Pretty => quote! { .pretty() },
        })
    }
}

struct Config {
    formatter: Option<Formatter>,
    tag: Option<proc_macro2::Ident>,
    is_test: bool,
}

impl Config {
    fn new(is_test: bool) -> Self {
        Config {
            formatter: None,
            tag: None,
            is_test,
        }
    }

    fn parse(args: AttributeArgs, is_test: bool) -> syn::Result<Self> {
        let mut config = Config::new(is_test);

        for arg in args {
            match arg {
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
                        "tag" => config.set_tag(&namevalue)?,
                        "fmt" => config.set_formatter(&namevalue)?,
                        name => {
                            let message = format!(
                                "Unknown argument `{}` is specified; expected one of: `tag`, `fmt`",
                                name,
                            );
                            return Err(syn::Error::new_spanned(namevalue, message));
                        }
                    }
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "Unknown argument inside the macro",
                    ));
                }
            }
        }

        Ok(config)
    }

    fn set_formatter(&mut self, namevalue: &syn::MetaNameValue) -> syn::Result<()> {
        if self.formatter.is_some() {
            Err(syn::Error::new_spanned(
                namevalue,
                "Argument `fmt` is defined multiple times",
            ))
        } else if let syn::Lit::Str(ref s) = namevalue.lit {
            match s.value().as_str() {
                "json" => self.formatter = Some(Formatter::Json),
                "pretty" => self.formatter = Some(Formatter::Pretty),
                value => {
                    let msg = format!(
                        "Argument `fmt` expects either `pretty` or `json`, but found: `{}`",
                        value
                    );
                    return Err(syn::Error::new_spanned(&namevalue.lit, msg));
                }
            }
            Ok(())
        } else {
            Err(syn::Error::new_spanned(
                &namevalue.lit,
                "Argument `fmt` expects a string literal value",
            ))
        }
    }

    fn set_tag(&mut self, namevalue: &syn::MetaNameValue) -> syn::Result<()> {
        if self.tag.is_some() {
            Err(syn::Error::new_spanned(
                namevalue,
                "Argument `tag` is defined multiple times",
            ))
        } else if let syn::Lit::Str(s) = &namevalue.lit {
            let ident = proc_macro2::Ident::new(s.value().as_str(), s.span());
            self.tag = Some(ident);
            Ok(())
        } else {
            Err(syn::Error::new_spanned(
                namevalue,
                "Argument `tag` expects a string literal value",
            ))
        }
    }

    fn builder(self) -> proc_macro2::TokenStream {
        let mut builder = quote! { ::tracing_forest::builder() };

        if let Some(formatter) = self.formatter {
            builder = quote! { #builder #formatter }
        }

        if self.is_test {
            builder = quote! { #builder.with_test_writer() };
        }

        if let Some(tag) = self.tag {
            builder = quote! { #builder.with_tag::<#tag>() };
        }

        builder
    }
}
