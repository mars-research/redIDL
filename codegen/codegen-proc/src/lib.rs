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

    let proxy_ident = quote::format_ident!("{}Proxy", trait_path);

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

    let trait_methods: Vec<&syn::TraitItemMethod> = input.items.iter()
        .filter(|item| {
            match item {
                syn::TraitItem::Method(_) => true,
                _ => false,
            }
        })
        .map(|item| {
            match item {
                syn::TraitItem::Method(method) => method,
                // Marked as `unimplemented` instead of `panic` because we might be able to allow other stuff here as well.
                _ => unreachable!(),
            }
        })
        .collect();

    let trampolines = generate_trampolines(trait_path, &beautified_trait_path_lower_case, &trait_methods[..]);

    let output = quote! {
        #proxy

        #trampolines
    };
    
    // Hand the output tokens back to the compiler
    TokenStream::from(output)
}

/// Generate trampolines for `methods`.
fn generate_trampolines(trait_path: &syn::Ident, beautified_trait_path_lower_case: &str,  methods: &[&syn::TraitItemMethod]) -> proc_macro2::TokenStream {
    let trampolines = methods.iter()
        .map(|method| {
            let sig = &method.sig;
            let ident = &sig.ident;
            let domain_ident = syn::Ident::new(&format!("generated_proxy_domain_{}", beautified_trait_path_lower_case), Span::call_site());
            let args = &sig.inputs;
            let return_ty = &sig.output;
            quote!(
                ::codegen_lib::generate_trampoline!(#domain_ident: &alloc::boxed::Box<dyn #trait_path>, #ident(#args) #return_ty);
            )
        });

    quote! { #(#trampolines)* }
}

/// Generate proxy implementation, e.g., `impl DomC for DomCProxy`.
fn generate_proxy_impl(trait_path: &syn::Ident, proxy_ident: &syn::Ident, methods: &[&syn::TraitItemMethod]) -> proc_macro2::TokenStream {


    quote! {
        impl #trait_path for #proxy_ident {
            fn no_arg(&self) -> RpcResult<()> {
                // move thread to next domain
                let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };
        
                #[cfg(not(feature = "tramp"))]
                let r = self.domain.no_arg();
                #[cfg(feature = "tramp")]
                let r = unsafe { no_arg_tramp(&self.domain) };
        
                // move thread back
                unsafe { sys_update_current_domain_id(caller_domain) };
        
                r
            }
        }
    }
}

/// Generate the proxy implementation for one signal method
fn generate_proxy_impl_one(method: &syn::TraitItemMethod) -> proc_macro2::TokenStream {
    quote! {
        fn no_arg(&self) -> RpcResult<()> {
            // move thread to next domain
            let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };
    
            #[cfg(not(feature = "tramp"))]
            let r = self.domain.no_arg();
            #[cfg(feature = "tramp")]
            let r = unsafe { no_arg_tramp(&self.domain) };
    
            // move thread back
            unsafe { sys_update_current_domain_id(caller_domain) };
    
            r
        }
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
