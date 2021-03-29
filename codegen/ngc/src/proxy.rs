use crate::{has_attribute, remove_attribute};

use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::{parse_quote, FnArg, Ident, Item, ItemFn, ItemTrait, Token, TraitItem, TraitItemMethod};

const INTERFACE_ATTR: &'static str = "interface";

pub fn generate_proxy(input: &mut ItemTrait, _module_path: &Vec<Ident>) -> Option<Vec<Item>> {
    if !has_attribute!(input, INTERFACE_ATTR) {
        return None;
    }

    // Remove the interface attribute and add a comment so we know it's an interface
    remove_attribute!(input, INTERFACE_ATTR);
    input.attrs.push(
        parse_quote! {#[doc = "redIDL Auto Generated: interface trait. Generations are below"]},
    );

    let trait_ident = &input.ident;
    let proxy_ident = format_ident!("{}Proxy", trait_ident);

    let proxy = quote! {
        #[cfg(feature = "proxy")]
        pub struct #proxy_ident {
            domain: ::alloc::boxed::Box<dyn #trait_ident>,
            domain_id: u64,
        }

        #[cfg(feature = "proxy")]
        unsafe impl Sync for #proxy_ident {}
        #[cfg(feature = "proxy")]
        unsafe impl Send for #proxy_ident {}

        #[cfg(feature = "proxy")]
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
    let trait_methods: Vec<TraitItemMethod> = input
        .items
        .iter()
        .filter_map(|item| match item {
            TraitItem::Method(method) => Some(method.clone()),
            _ => None,
        })
        .collect();

    // Filter out `&self` and `&mut self`
    let cleaned_trait_methods = {
        let mut cleaned_trait_methods = trait_methods.clone();
        for method in &mut cleaned_trait_methods {
            let mut args = Punctuated::<FnArg, Token![,]>::new();
            for arg in &method.sig.inputs {
                match arg {
                    FnArg::Receiver(_) => {}
                    FnArg::Typed(typed) => args.push(FnArg::Typed(typed.clone())),
                }
            }
            method.sig.inputs = args;
        }
        cleaned_trait_methods
    };

    let proxy_impl = generate_proxy_impl(
        trait_ident,
        &proxy_ident,
        &trait_methods[..],
        &cleaned_trait_methods[..],
    );
    let trampolines = generate_trampolines(trait_ident, &cleaned_trait_methods[..]);

    let proxy_comment_begin_str = format!(
        "----------{} Proxy generation begins-------------",
        trait_ident
    );
    let tramp_comment_begin_str = format!(
        "----------{} Trampoline generation begins-------------",
        trait_ident
    );

    let output: syn::File = parse_quote! {
        #[doc = #proxy_comment_begin_str]
        #proxy

        #proxy_impl

        #[doc = #tramp_comment_begin_str]
        #trampolines
    };

    Some(output.items)
}

/// Generate trampolines for `methods`.
fn generate_trampolines(
    trait_ident: &Ident,
    methods: &[TraitItemMethod],
) -> proc_macro2::TokenStream {
    let trampolines = methods.iter()
        .map(|method| {
            let sig = &method.sig;
            let ident = &sig.ident;
            let args = &sig.inputs;
            let return_ty = &sig.output;

            let domain_variable_ident = format_ident!("redidl_generated_domain_{}", trait_ident.to_string().to_lowercase());
            let trampoline_ident = format_ident!("{}_{}", trait_ident, ident);
            let trampoline_err_ident = format_ident!("{}_{}_err", trait_ident, ident);
            let trampoline_addr_ident = format_ident!("{}_{}_addr", trait_ident, ident);
            let trampoline_tramp_ident = format_ident!("{}_{}_tramp", trait_ident, ident);

            quote! {
                // Wrapper of the original function.
                // The trampoline should call this after saving the continuation stack.
                #[cfg(feature = "trampoline")]
                #[cfg(feature = "proxy")]
                #[no_mangle]
                extern fn #trampoline_ident(#domain_variable_ident: &alloc::boxed::Box<dyn #trait_ident>, #args) #return_ty {
                    #domain_variable_ident.#ident(#args)
                }
    
                // When the call panics, the continuation stack will jump this function.
                // This function will return a `RpcError::panic` to the caller domain.
                #[cfg(feature = "trampoline")]
                #[cfg(feature = "proxy")]
                #[no_mangle]
                extern fn #trampoline_err_ident(#domain_variable_ident: &alloc::boxed::Box<dyn #trait_ident>, #args) #return_ty  {
                    #[cfg(feature = "proxy-log-error")]
                    ::console::println!("proxy: {} aborted", stringify!(#ident));
    
                    Err(unsafe{crate::rpc::RpcError::panic()})
                }
    
                // A workaround to get the address of the error function
                #[cfg(feature = "trampoline")]
                #[cfg(feature = "proxy")]
                #[no_mangle]
                extern "C" fn #trampoline_addr_ident() -> u64 {
                    #trampoline_err_ident as u64
                }
                
                // FFI to the trampoline.
                #[cfg(feature = "proxy")]
                #[cfg(feature = "trampoline")]

                extern {
                    fn #trampoline_tramp_ident(#domain_variable_ident: &alloc::boxed::Box<dyn #trait_ident>, #args) #return_ty;
                }
    
                #[cfg(feature = "proxy")]
                #[cfg(feature = "trampoline")]
                ::unwind::trampoline!(#trampoline_ident);
            }
        });

    quote! {
        #(#trampolines)*
    }
}

/// Generate proxy implementation, e.g., `impl DomC for DomCProxy`.
fn generate_proxy_impl(
    trait_ident: &Ident,
    proxy_ident: &Ident,
    methods: &[TraitItemMethod],
    cleaned_methods: &[TraitItemMethod],
) -> Item {
    let proxy_impls = methods
        .iter()
        .zip(cleaned_methods)
        .map(|pair| generate_proxy_impl_one(trait_ident, pair.0, pair.1));

    parse_quote! {
        #[cfg(feature = "proxy")]
        impl #trait_ident for #proxy_ident {
            #(#proxy_impls)*
        }
    }
}

/// Generate the proxy implementation for one single method
fn generate_proxy_impl_one(
    trait_ident: &Ident,
    method: &TraitItemMethod,
    cleaned_method: &TraitItemMethod,
) -> ItemFn {
    let sig = &method.sig;
    let ident = &sig.ident;
    let trampoline_ident = format_ident!("{}_{}_tramp", trait_ident, ident);
    let args = &sig.inputs;
    let cleaned_args = &cleaned_method.sig.inputs;
    let return_ty = &sig.output;
    parse_quote! {
        fn #ident(#args) #return_ty {
            // move thread to next domain
            let caller_domain = unsafe { ::libsyscalls::syscalls::sys_update_current_domain_id(self.domain_id) };

            #[cfg(not(feature = "trampoline"))]
            let r = self.domain.#ident(#cleaned_args);
            #[cfg(feature = "trampoline")]
            let r = unsafe { #trampoline_ident(&self.domain, #cleaned_args) };

            #[cfg(feature = "trampoline")]
            unsafe {
                ::libsyscalls::syscalls::sys_discard_cont();
            }

            // move thread back
            unsafe { ::libsyscalls::syscalls::sys_update_current_domain_id(caller_domain) };

            r
        }
    }
}
