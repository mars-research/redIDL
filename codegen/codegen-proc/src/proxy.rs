use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{ItemTrait, TraitItemMethod, Ident, FnArg, Token, TraitItem};
use syn::punctuated::Punctuated;

pub fn redidl_generate_proxy_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input: ItemTrait = syn::parse(item).expect("interface definition must be a valid trait definition");

    // Extract module path
    let module_path = input.attrs.iter().filter_map(
        |attr| {
            if let Ok(syn::Meta::NameValue(meta)) = attr.parse_meta(){
                if let Some(ident) = meta.path.get_ident() {
                    if ident.to_string() == "module_path" {
                        if let syn::Lit::Str(lit) = meta.lit {
                            return Some(lit);
                        } else {
                            panic!("module_path must be a string")
                        }
                    }
                }
            }
            None
        }
    ).next().expect("module_path not found").value();

    let trait_ident = &input.ident;

    let proxy_ident = format_ident!("{}Proxy", trait_ident);

    let proxy = quote! {
        pub struct #proxy_ident {
            domain: ::alloc::boxed::Box<dyn #trait_ident>,
            domain_id: u64,
        }
        
        unsafe impl Sync for #proxy_ident {}
        unsafe impl Send for #proxy_ident {}
        
        impl #proxy_ident {
            pub fn new(domain_id: u64, domain: ::alloc::boxed::Box<dyn #trait_ident>) -> Self {
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

    let proxy_impl = generate_proxy_impl(trait_ident, &proxy_ident, &trait_methods[..], &cleaned_trait_methods[..]);
    let trampolines = generate_trampolines(trait_ident, &cleaned_trait_methods[..]);

    let import_path_segs = crate::helper::generate_import_path_segs(&module_path, trait_ident);
    
    let output = quote! {
        // An extra copy of interface definition is copied over to the proxy crate so that 
        // we don't have to resolve the dependencies
        use ::#(#import_path_segs)::*;

        #proxy

        #proxy_impl

        #trampolines
    };
    
    // Hand the output tokens back to the compiler
    TokenStream::from(output)
}



/// Generate trampolines for `methods`.
fn generate_trampolines(trait_ident: &Ident, methods: &[TraitItemMethod]) -> proc_macro2::TokenStream {
    let trampolines = methods.iter()
        .map(|method| {
            let sig = &method.sig;
            let ident = &sig.ident;
            let domain_ident = format_ident!("generated_proxy_domain_{}", trait_ident);
            let args = &sig.inputs;
            let return_ty = &sig.output;
            quote!(
                ::codegen_lib::generate_trampoline!(#domain_ident: &alloc::boxed::Box<dyn #trait_ident>, #ident(#args) #return_ty);
            )
        });

    quote! { #(#trampolines)* }
}

/// Generate proxy implementation, e.g., `impl DomC for DomCProxy`.
fn generate_proxy_impl(trait_ident: &syn::Ident, proxy_ident: &syn::Ident, methods: &[TraitItemMethod], cleaned_methods: &[TraitItemMethod]) -> proc_macro2::TokenStream {
    let proxy_impls = methods.iter().zip(cleaned_methods).map(|pair| generate_proxy_impl_one(trait_ident, pair.0, pair.1));

    quote! {
        impl #trait_ident for #proxy_ident {
            #(#proxy_impls)*
        }
    }
}

/// Generate the proxy implementation for one single method
fn generate_proxy_impl_one(trait_ident: &syn::Ident, method: &TraitItemMethod, cleaned_method: &TraitItemMethod) -> proc_macro2::TokenStream {
    let sig = &method.sig;
    let ident = &sig.ident;
    let trampoline_ident = format_ident!("{}_tramp", trampoline_ident(trait_ident, ident));
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

/// Convert the method name to "{trait_name}_{method_name}"
/// This step is necessary because there could be mutiple interface
/// traits with the same method names.
fn trampoline_ident(trait_ident: &Ident, method: &Ident) -> Ident {
    format_ident!("generated_proxy_domain_{}_{}", trait_ident, method)
}
