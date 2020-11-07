#![feature(log_syntax, proc_macro_def_site)]

use proc_macro::{TokenStream};
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::{Error, parse_macro_input};
use syn::spanned::Spanned;

#[proc_macro_attribute]
pub fn generate_proxy(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input: syn::ItemTrait = syn::parse(item).expect("interface definition must be a valid trait definition");

    let trait_path = &input.ident;
    let beautified_trait_path = input.ident.to_string().replace("::", "_");
    let beautified_trait_path_lower_case = beautified_trait_path.to_lowercase();

    let proxy_ident = syn::Ident::new(&format!("{}Proxy", trait_path), Span::call_site());

    let proxy = quote! {
        struct #proxy_ident {
            domain: ::alloc::boxed::Box<dyn #trait_path>,
            domain_id: u64,
        }
        
        unsafe impl Sync for #proxy_ident {}
        unsafe impl Send for #proxy_ident {}
        
        impl #proxy_ident {
            fn new(domain_id: u64, domain: ::alloc::boxed::Box<dyn #trait_path>) -> Self {
                Self {
                    domain,
                    domain_id,
                }
            }
        }
    };

    let trampolines = input.items.iter().map(
        |item| {
            match item {
                syn::TraitItem::Method(method) => {
                    
                    let domain_ident = syn::Ident::new(&format!("generated_proxy_domain_{}", beautified_trait_path_lower_case), Span::call_site());
                    quote!(
                        ::codegen_lib::generate_trampoline!(#domain_ident: &alloc::boxed::Box<dyn #trait_path>, no_arg() -> RpcResult<()>);
                    )
                },
                // Marked as `unimplemented` instead of `panic` because we might be able to allow other stuff here as well.
                _ => unimplemented!("Only methods are allowed in an interface trait definition."),
            }
        }
    );

    let output = quote! {
        #proxy

        #(#trampolines)*
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(output)
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
