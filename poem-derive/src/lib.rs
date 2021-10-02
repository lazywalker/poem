//! Macros for poem

#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

mod utils;

use darling::FromMeta;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, AttributeArgs, FnArg, ItemFn, Member, Result};

/// Wrap an asynchronous function as an `Endpoint`.
///
/// # Example
///
/// ```ignore
/// #[handler]
/// async fn example() {
/// }
/// ```
#[proc_macro_attribute]
pub fn handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = match HandlerArgs::from_list(&parse_macro_input!(args as AttributeArgs)) {
        Ok(args) => args,
        Err(err) => return err.write_errors().into(),
    };

    match generate_handler(args, input) {
        Ok(stream) => stream,
        Err(err) => err.into_compile_error().into(),
    }
}

#[derive(FromMeta, Default)]
#[darling(default)]
struct HandlerArgs {
    internal: bool,
}

fn generate_handler(args: HandlerArgs, input: TokenStream) -> Result<TokenStream> {
    let crate_name = utils::get_crate_name(args.internal);
    let item_fn = syn::parse::<ItemFn>(input)?;
    let vis = &item_fn.vis;
    let docs = item_fn
        .attrs
        .iter()
        .filter(|attr| attr.path.is_ident("doc"))
        .cloned()
        .collect::<Vec<_>>();
    let ident = &item_fn.sig.ident;
    let call_await = if item_fn.sig.asyncness.is_some() {
        Some(quote::quote!(.await))
    } else {
        None
    };

    let mut extractors = Vec::new();
    let mut args = Vec::new();
    for (idx, input) in item_fn.sig.inputs.clone().into_iter().enumerate() {
        if let FnArg::Typed(pat) = input {
            let ty = &pat.ty;
            let id = quote::format_ident!("p{}", idx);
            args.push(id.clone());
            extractors.push(quote! {
                let #id = match <#ty as #crate_name::FromRequest>::from_request(&req, &mut body).await {
                    Ok(value) => value,
                    Err(err) => return ::std::convert::Into::<#crate_name::Error>::into(err).as_response(),
                };
            });
        }
    }

    let expanded = quote! {
        #(#docs)*
        #[allow(non_camel_case_types)]
        #vis struct #ident;

        #[#crate_name::async_trait]
        impl #crate_name::Endpoint for #ident {
            type Output = #crate_name::Response;

            #[allow(unused_mut)]
            async fn call(&self, mut req: #crate_name::Request) -> Self::Output {
                let (req, mut body) = req.split();
                #(#extractors)*
                #item_fn
                #crate_name::IntoResponse::into_response(#ident(#(#args),*)#call_await)
            }
        }
    };

    Ok(expanded.into())
}

#[doc(hidden)]
#[proc_macro]
pub fn generate_implement_middlewares(_: TokenStream) -> TokenStream {
    let mut impls = Vec::new();

    for i in 2..=16 {
        let idents = (0..i)
            .map(|i| format_ident!("T{}", i + 1))
            .collect::<Vec<_>>();
        let output_type = idents.last().unwrap();
        let first_ident = idents.first().unwrap();
        let mut where_clauses = vec![quote! { #first_ident: Middleware<E> }];
        let mut transforms = Vec::new();

        for k in 1..i {
            let prev_ident = &idents[k - 1];
            let current_ident = &idents[k];
            where_clauses.push(quote! { #current_ident: Middleware<#prev_ident::Output> });
        }

        for k in 0..i {
            let n = Member::from(k);
            transforms.push(quote! { let ep = self.#n.transform(ep); });
        }

        let expanded = quote! {
            impl<E, #(#idents),*> Middleware<E> for (#(#idents),*)
                where
                    E: Endpoint,
                    #(#where_clauses,)*
            {
                type Output = #output_type::Output;

                fn transform(&self, ep: E) -> Self::Output {
                    #(#transforms)*
                    ep
                }
            }
        };

        impls.push(expanded);
    }

    quote!(#(#impls)*).into()
}
