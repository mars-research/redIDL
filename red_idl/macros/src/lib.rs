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
    let parsed : syn::Path = syn::parse(input).expect("failed to parse");
    if let syn::PathArguments::AngleBracketed(args) = &parsed.segments[0].arguments {
        let out = quote::quote!{
            impl#args red_idl::SafeCopy for #parsed {}
            impl#args red_idl::RRefable for #parsed {}
        };
    
        out.into()
    }
    else {
        let out = quote::quote!{
            impl red_idl::SafeCopy for #parsed {}
            impl red_idl::RRefable for #parsed {}
        };

        out.into()
    }
}

#[proc_macro]
pub fn require_functional(input: TokenStream) -> TokenStream {
    let parsed : syn::Path = syn::parse(input).expect("failed to parse");
    let out = quote::quote!{ red_idl::assert_impl_all!(dyn #parsed: red_idl::Functional); };
    out.into()
}

#[proc_macro]
pub fn declare_functional(input: TokenStream) -> TokenStream {
    let parsed : syn::Path = syn::parse(input).expect("failed to parse");
    if let syn::PathArguments::AngleBracketed(args) = &parsed.segments[0].arguments {
        let out = quote::quote!{ impl#args red_idl::Functional for #parsed {} };
        out.into()
    }
    else {
        let out = quote::quote!{ impl red_idl::Functional for #parsed {} };
        out.into()
    }
}

// foo.idl
/*
    #[derive(SafeCopy)] // unfortunately, would parse, isn't rejected
    struct Foo {
        a: u8,
        b: Foo
    }

    declare_safe_copy!(Foo); --> impl red_idl::SafeCopy for Foo
    require_safe_copy!(Foo);

    bar.idl

    struct Bar {
        a: Foo // What type does "Foo" refer to (Is foo defined here? or somewhere else? is it defined at all?)
    }

    require_safe_copy!(Foo);

    idl crate -> generated crate -> compiled
*/

#[proc_macro]
pub fn require_rrefable(input: TokenStream) -> TokenStream {
    let parsed : syn::Path = syn::parse(input).expect("failed to parse");
    let out = quote::quote!{ red_idl::assert_impl_all!(#parsed: red_idl::RRefable); };
    out.into()
}

#[proc_macro]
pub fn declare_rrefable(input: TokenStream) -> TokenStream {
    let parsed : syn::Path = syn::parse(input).expect("failed to parse");
    if let syn::PathArguments::AngleBracketed(args) = &parsed.segments[0].arguments {
        let out = quote::quote!{ impl#args red_idl::RRefable for #parsed {} };
        out.into()
    }
    else {
        let out = quote::quote!{ impl red_idl::RRefable for #parsed {} };
        out.into()
    }
}
