#![feature(log_syntax)]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Error, parse_macro_input, DeriveInput, Data};
use syn::spanned::Spanned;

#[proc_macro_derive(MyMacro)]
pub fn my_macro(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);


    let recurse = match &input.data {
        Data::Struct(st) => {
            st.fields.iter().map(|field| -> Option<syn::Type> {
                match &field.ty {
                    syn::Type::BareFn(f) => {
                        match &f.output {
                            syn::ReturnType::Default => None,
                            syn::ReturnType::Type(_, ty) => {
                                Some((**ty).clone())
                            },
                        }
                    },
                    _ => panic!("only function is supported"),
                }
            })
            .filter(|t| t.is_some()) 
            .enumerate()
            .map(|(i, ty)| {
                let copy_ident = syn::Ident::new(&format!("_AssertCopy_{}", i), proc_macro2::Span::call_site());
                let sync_ident = syn::Ident::new(&format!("_AssertSync_{}", i), proc_macro2::Span::call_site());
                let ty = ty.unwrap();
                quote_spanned! {ty.span()=>
                    struct #copy_ident where #ty: std::marker::Copy;
                    struct #sync_ident where #ty: std::marker::Sync;
                }
            })
        }
        _ => panic!("only struct is supported"),
    };

    let expanded = quote! {
        #(#recurse)*
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn generate_trampoline(_attr: TokenStream, item: TokenStream) -> TokenStream {
    unimplemented!()
}



#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
