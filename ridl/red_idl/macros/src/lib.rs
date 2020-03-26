extern crate proc_macro;
extern crate syn;
extern crate quote;

use proc_macro::TokenStream;

#[proc_macro]
pub fn is_copy(input: TokenStream) -> TokenStream {
    let parsed : syn::Type = syn::parse(input).expect("failed to parse");
    let out = quote::quote!{ red_idl::assert_impl_all!(#parsed: Copy); };
    out.into()
}

#[proc_macro]
pub fn is_functional(input: TokenStream) -> TokenStream {
    let parsed : syn::Type = syn::parse(input).expect("failed to parse");
    let out = quote::quote!{ red_idl::assert_impl_all!(#parsed: markers::Functional); };
    out.into()
}

#[proc_macro]
pub fn declare_functional(input: TokenStream) -> TokenStream {
    let parsed : syn::Type = syn::parse(input).expect("failed to parse");
    let out = quote::quote!{ impl red_idl::Functional for #parsed {} };
    out.into()
}

#[proc_macro]
pub fn is_rrefable(input: TokenStream) -> TokenStream {
    let parsed : syn::Type = syn::parse(input).expect("failed to parse");
    let out = quote::quote!{ red_idl::assert_impl_all!(#parsed: markers::RRefable); };
    out.into()
}

#[proc_macro]
pub fn declare_rrefable(input: TokenStream) -> TokenStream {
    let parsed : syn::Type = syn::parse(input).expect("failed to parse");
    let out = quote::quote!{ impl red_idl::RRefable for #parsed {} };
    out.into()
}
