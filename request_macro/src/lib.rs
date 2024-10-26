use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Data, DeriveInput, Expr, Fields, Ident, LitStr, Token, TypePath,
};

struct Args {
    method: Method,
    path: UrlPath,
    response: TypePath,
}

struct UrlPath {
    raw: String,
    parameters: HashSet<Ident>,
}

enum Method {
    Get,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vars = Punctuated::<Expr, Token![,]>::parse_terminated(input)?;

        let span = input.span();
        let err = |s: &str| syn::Error::new(span, s);

        let vars = vars
            .into_iter()
            .map(|expr| expr.to_token_stream())
            .collect::<Vec<_>>();
        let [method, path, response] = &vars[..] else {
            return Err(err(&format!(
                "Expected exactly 3 arguments, got {}",
                vars.len()
            )));
        };

        let method: Ident = syn::parse2(method.clone())?;
        let method: String = method.to_string();
        let method: Method = match method.as_str() {
            "GET" => Method::Get,
            _ => return Err(err(&format!("Invalid HTTP method: {method:?}"))),
        };

        let path: UrlPath = syn::parse2(path.clone())?;

        let response: TypePath = syn::parse2(response.clone())?;

        Ok(Args {
            method,
            path,
            response,
        })
    }
}

impl Parse for UrlPath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let raw: LitStr = Parse::parse(input)?;
        let raw = raw.value();

        let mut parameters = HashSet::new();

        let mut parse_str = &raw[..];

        while !parse_str.is_empty() {
            let Some((_, after_opening_bracket)) = parse_str.split_once('{') else {
                break;
            };

            let Some((within_brackets, after_closing_bracket)) =
                after_opening_bracket.split_once('}')
            else {
                return Err(syn::Error::new(input.span(), "Missing '}'"));
            };

            parse_str = after_closing_bracket;

            if !parameters.insert(Ident::new(within_brackets, input.span())) {
                return Err(syn::Error::new(
                    input.span(),
                    "Duplicate path parameter name",
                ));
            }
        }

        Ok(UrlPath { raw, parameters })
    }
}

impl Method {
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
        }
    }
}

#[proc_macro_attribute]
pub fn request(
    attrs: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let attrs = parse_macro_input!(attrs as Args);

    inner(attrs, input)
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}

fn inner(args: Args, input: DeriveInput) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;
    let responder_name = format_ident!("{struct_name}Responder");

    let Data::Struct(data_struct) = &input.data else {
        return Err(syn::Error::new_spanned(&input, "Only works for structs."));
    };

    let Fields::Named(fields) = &data_struct.fields else {
        return Err(syn::Error::new_spanned(
            &input,
            "Only works for structs with named fields.",
        ));
    };
    let path_str = &args.path.raw;
    let method = &args.method.as_str();

    // All fields of the struct that should be parsed from the url path
    // TODO: consider fields that should be parsed from query params, headers, and the body.
    let struct_path_fields: HashSet<Ident> = fields
        .named
        .iter()
        .map(|field| field.ident.clone().unwrap())
        .collect();

    let mut path_error = None;
    for ident in args
        .path
        .parameters
        .symmetric_difference(&struct_path_fields)
    {
        let error = syn::Error::new(
            ident.span(),
            "{ident} must appear both as a struct field, and in the URL path.",
        );

        match &mut path_error {
            None => path_error = Some(error),
            Some(existing_error) => existing_error.combine(error),
        }
    }

    if let Some(error) = path_error {
        return Err(error);
    }

    let mut path_arg_types = quote! {};
    // TODO: only path args
    for field in &fields.named {
        let ty = &field.ty;
        path_arg_types.append_all(quote! { #ty });
    }
    path_arg_types = quote! { (#path_arg_types) };

    let handler_args = quote! {
        ::actix_web::web::Path<#path_arg_types>
    };

    //panic!("{}", handler_args);

    Ok(quote! {
        #input

        impl #struct_name{
            #[::actix_web::route(#path_str, method = #method)]
            pub fn route_handler(#handler_args) -> impl actix_web::Responder {
                todo!()
            }
        }

    })
}
