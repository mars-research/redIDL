#![feature(log_syntax, proc_macro_def_site)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, format_ident};
use syn::{ItemTrait, TraitItemMethod, Ident, FnArg, Token, TraitItem};
use syn::punctuated::Punctuated;

// #[proc_macro_attribute]
// pub fn interface(attr: TokenStream, item: TokenStream) -> TokenStream  {
//     generate_proxy(attr, item)
// }

/// Generate the proxy for an interface definition.
#[proc_macro_attribute]
pub fn generate_proxy(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input: ItemTrait = syn::parse(item).expect("interface definition must be a valid trait definition");
    let input_copy = input.clone();

    let trait_path = &input.ident;
    let beautified_trait_path = input.ident.to_string().replace("::", "_");
    let beautified_trait_path_lower_case = beautified_trait_path.to_lowercase();

    let proxy_ident = format_ident!("{}Proxy", trait_path);

    let proxy = quote! {
        pub struct #proxy_ident {
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

    
    // Remove non-method members
    let trait_methods: Vec<TraitItemMethod> = input.items
        .into_iter()
        .filter_map(|item| {
            match item {
                TraitItem::Method(method) => Some(method),
                _ => None,
            }
        })
        .collect();

    // Filter out `&self` and `&mut self`
    let cleaned_trait_methods = {
        let mut cleaned_trait_methods = trait_methods.clone();
        for method in &mut cleaned_trait_methods {
            let mut args = Punctuated::<FnArg, Token![,]>::new();
            for arg in &method.sig.inputs {
                match arg {
                    FnArg::Receiver(_) => {},
                    FnArg::Typed(typed) => args.push(FnArg::Typed(typed.clone())),
                }
            }
            method.sig.inputs = args;
        }
        cleaned_trait_methods
    };

    let proxy_impl = generate_proxy_impl(trait_path, &proxy_ident, &trait_methods[..], &cleaned_trait_methods[..]);
    let trampolines = generate_trampolines(trait_path, &beautified_trait_path_lower_case, &cleaned_trait_methods[..]);
    
    let output = quote! {
        // An extra copy of interface definition is copied over to the proxy crate so that 
        // we don't have to resolve the dependencies
        #input_copy

        #proxy

        #proxy_impl

        #trampolines
    };
    
    // Hand the output tokens back to the compiler
    TokenStream::from(output)
}

/// Generate trampolines for `methods`.
fn generate_trampolines(trait_path: &Ident, beautified_trait_path_lower_case: &str,  methods: &[TraitItemMethod]) -> proc_macro2::TokenStream {
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
fn generate_proxy_impl(trait_path: &syn::Ident, proxy_ident: &syn::Ident, methods: &[TraitItemMethod], cleaned_methods: &[TraitItemMethod]) -> proc_macro2::TokenStream {
    let proxy_impls = methods.iter().zip(cleaned_methods).map(|pair| generate_proxy_impl_one(pair.0, pair.1));

    quote! {
        impl #trait_path for #proxy_ident {
            #(#proxy_impls)*
        }
    }
}

/// Generate the proxy implementation for one single method
fn generate_proxy_impl_one(method: &TraitItemMethod, cleaned_method: &TraitItemMethod) -> proc_macro2::TokenStream {
    let sig = &method.sig;
    let ident = &sig.ident;
    let trampoline_ident = format_ident!("{}_tramp", ident);
    let args = &sig.inputs;
    let cleaned_args = &cleaned_method.sig.inputs;
    let return_ty = &sig.output;
    quote! {
        fn #ident(#args) #return_ty {
            // move thread to next domain
            let caller_domain = unsafe { sys_update_current_domain_id(self.domain_id) };
    
            #[cfg(not(feature = "trampoline"))]
            let r = self.domain.#ident(#cleaned_args);
            #[cfg(feature = "trampoline")]
            let r = unsafe { #trampoline_ident(&self.domain, #cleaned_args) };
    
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
