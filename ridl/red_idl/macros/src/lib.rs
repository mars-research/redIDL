extern crate proc_macro;
extern crate syn;
extern crate quote;

use proc_macro::TokenStream;

#[proc_macro]
pub fn require_safe_copy(input: TokenStream) -> TokenStream {
    let parsed : syn::Type = syn::parse(input).expect("failed to parse");
    let out = quote::quote!{ red_idl::assert_impl_all!(#parsed: red_idl::SafeCopy); };
    out.into()
}

#[proc_macro]
pub fn require_copy(input: TokenStream) -> TokenStream {
    let parsed : syn::Type = syn::parse(input).expect("failed to parse");
    let out = quote::quote!{ red_idl::assert_impl_all!(#parsed: Copy); };
    out.into()
}

#[proc_macro]
pub fn declare_safe_copy(input: TokenStream) -> TokenStream {
    let parsed : syn::Type = syn::parse(input).expect("failed to parse");
    let out = quote::quote!{ impl red_idl::SafeCopy for #parsed {} };
    out.into()
}

#[proc_macro]
pub fn require_functional(input: TokenStream) -> TokenStream {
    let parsed : syn::Type = syn::parse(input).expect("failed to parse");
    let out = quote::quote!{ red_idl::assert_impl_all!(#parsed: red_idl::Functional); };
    out.into()
}

#[proc_macro]
pub fn declare_functional(input: TokenStream) -> TokenStream {
    let parsed : syn::Type = syn::parse(input).expect("failed to parse");
    let out = quote::quote!{ impl red_idl::Functional for dyn #parsed {} };
    out.into()
}

#[proc_macro]
pub fn require_rrefable(input: TokenStream) -> TokenStream {
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
