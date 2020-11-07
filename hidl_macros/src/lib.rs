use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};

struct TypeId {
    id: syn::LitInt,
    ast: syn::Type,
}

impl Parse for TypeId {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        let id = stream.parse::<syn::LitInt>()?;
        stream.parse::<syn::Token! [,]>()?;
        let ast = stream.parse::<syn::Type>()?;
        Ok(Self { id: id, ast: ast })
    }
}

#[proc_macro]
pub fn assign_id(toks: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(toks as TypeId);
    let ty = ast.ast;
    let n = ast.id;
    let new_toks = quote! {
        impl crate::sys::TypeIdentifiable for #ty {
            fn get_id() -> u64 {
                #n
            }
        }
    };

    new_toks.into()
}
