use crate::{has_attribute, remove_attribute, get_proxy_mod};

use quote::{quote, format_ident};
use syn::{parse_quote, Item, ItemTrait, ItemFn, ItemMod, TraitItemMethod, Ident, FnArg, Token, TraitItem};
use syn::punctuated::Punctuated;


pub fn generate_proxy(input: &mut ItemTrait, _module_path: &Vec<Ident>) -> Option<ItemMod> {
    if !has_attribute!(input, "interface") {
        return None;
    }

    // Remove the interface attribute and add a comment so we know it's an interface
    remove_attribute!(input, "interface");
    input.attrs.push(parse_quote!{#[doc = "redIDL Auto Generated: interface trait. Generations are below"]});


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

    
    // Remove non-method members. We don't really care about them
    let trait_methods: Vec<TraitItemMethod> = input.items
        .iter()
        .filter_map(|item| {
            match item {
                TraitItem::Method(method) => Some(method.clone()),
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
    let proxy_mod = get_proxy_mod!();

    let output = parse_quote! {
        #[cfg(feature = "proxy")]
        pub mod #proxy_mod {
                    #proxy
            
                    #proxy_impl
            
                    #trampolines
        } 
    };
    
    Some(output)
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
fn generate_proxy_impl(trait_ident: &Ident, proxy_ident: &Ident, methods: &[TraitItemMethod], cleaned_methods: &[TraitItemMethod]) -> Item {
    let proxy_impls = methods.iter().zip(cleaned_methods).map(|pair| generate_proxy_impl_one(trait_ident, pair.0, pair.1));

    parse_quote! {
        impl #trait_ident for #proxy_ident {
            #(#proxy_impls)*
        }
    }
}

/// Generate the proxy implementation for one single method
fn generate_proxy_impl_one(trait_ident: &Ident, method: &TraitItemMethod, cleaned_method: &TraitItemMethod) -> ItemFn {
    let sig = &method.sig;
    let ident = &sig.ident;
    let trampoline_ident = format_ident!("{}_tramp", trampoline_ident(trait_ident, ident));
    let args = &sig.inputs;
    let cleaned_args = &cleaned_method.sig.inputs;
    let return_ty = &sig.output;
    parse_quote! {
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
