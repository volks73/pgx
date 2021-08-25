use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Token,
};

#[derive(Debug, Clone)]
pub struct PgxAttributes {
    pub attrs: Punctuated<Attribute, Token![,]>,
}

impl Parse for PgxAttributes {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        Ok(Self {
            attrs: input.parse_terminated(Attribute::parse)?,
        })
    }
}

impl ToTokens for PgxAttributes {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let attrs = &self.attrs;
        let quoted = quote! {
            vec![#attrs]
        };
        tokens.append_all(quoted);
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Attribute {
    Immutable,
    Strict,
    Stable,
    Volatile,
    Raw,
    NoGuard,
    ParallelSafe,
    ParallelUnsafe,
    ParallelRestricted,
    Error(syn::LitStr),
    Schema(syn::LitStr),
    Name(syn::LitStr),
    SkipInventory,
}

impl ToTokens for Attribute {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let quoted = match self {
            Attribute::Immutable => quote! { pgx::inventory::ExternArgs::Immutable },
            Attribute::Strict => quote! { pgx::inventory::ExternArgs::Strict },
            Attribute::Stable => quote! { pgx::inventory::ExternArgs::Stable },
            Attribute::Volatile => quote! { pgx::inventory::ExternArgs::Volatile },
            Attribute::Raw => quote! { pgx::inventory::ExternArgs::Raw },
            Attribute::NoGuard => quote! { pgx::inventory::ExternArgs::NoGuard },
            Attribute::ParallelSafe => quote! { pgx::inventory::ExternArgs::ParallelSafe },
            Attribute::ParallelUnsafe => quote! { pgx::inventory::ExternArgs::ParallelUnsafe },
            Attribute::ParallelRestricted => quote! { pgx::inventory::ExternArgs::ParallelRestricted },
            Attribute::Error(s) => quote! { pgx::inventory::ExternArgs::Error(String::from(#s)) },
            Attribute::Schema(s) => quote! { pgx::inventory::ExternArgs::Schema(String::from(#s)) },
            Attribute::Name(s) => quote! { pgx::inventory::ExternArgs::Name(String::from(#s)) },
            Attribute::SkipInventory => quote! { pgx::inventory::ExternArgs::SkipInventory },
        };
        tokens.append_all(quoted);
    }
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let ident: syn::Ident = input.parse()?;
        let found = match ident.to_string().as_str() {
            "immutable" => Attribute::Immutable,
            "strict" => Attribute::Strict,
            "stable" => Attribute::Stable,
            "volatile" => Attribute::Volatile,
            "raw" => Attribute::Raw,
            "no_guard" => Attribute::NoGuard,
            "parallel_safe" => Attribute::ParallelSafe,
            "parallel_unsafe" => Attribute::ParallelUnsafe,
            "parallel_restricted" => Attribute::ParallelRestricted,
            "error" => {
                let _eq: Token![=] = input.parse()?;
                let literal: syn::LitStr = input.parse()?;
                Attribute::Error(literal)
            }
            "schema" => {
                let _eq: Token![=] = input.parse()?;
                let literal: syn::LitStr = input.parse()?;
                Attribute::Schema(literal)
            }
            "name" => {
                let _eq: Token![=] = input.parse()?;
                let literal: syn::LitStr = input.parse()?;
                Attribute::Name(literal)
            }
            "skip_inventory" => Attribute::SkipInventory,
            _ => return Err(syn::Error::new(Span::call_site(), "Invalid option")),
        };
        Ok(found)
    }
}