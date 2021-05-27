use std::collections::HashMap;

use crate::{has_attribute, remove_attribute};

use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::{Field, FnArg, Ident, ImplItem, ImplItemMethod, Item, ItemFn, ItemTrait, Path, Token, TraitItem, TraitItemMethod, parse_quote};

const INTERFACE_ATTR: &str = "interface";

/// Generate the proxy for a IPC interface trait.
pub fn generate_interface_proxy(input: &mut ItemTrait, _module_path: &[Ident]) -> Option<Vec<Item>> {
    // Noop if the input is not a proxy interface.
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

/// Generate the proxy itself and the impl block for it.
pub fn generate_proxy(domain_creates: Vec<(Path, ItemTrait)>) -> Vec<Item> {
    let mut generated_items = vec![];

    // Create a mapping between the names and the interfaces.
    let domain_creates: HashMap<Ident, (Path, ItemTrait)> = domain_creates.into_iter().map(|(mut path, definition)| {
        // Make sure the path starts with `crate` since proxy will be generated inside of interface.
        path.segments.first_mut().unwrap().ident = format_ident!("crate");

        // The first ident is skipped because it's redundant.
        let path_str = path.segments.iter().skip(1).map(|seg| {
            seg.ident.to_string()
        }).collect::<Vec<String>>().join("_");
        let name = format_ident!("{}", path_str);
        (name, (path, definition))
    }).collect();

    // Generate each struct field.
    let struct_fields: Vec<FnArg> = domain_creates.iter().map(
        |(name, (path, _))| {
            parse_quote! {
                #name: ::alloc::sync::Arc<dyn #path>
            }
        }
    ).collect();

    // Generate the struct.
    generated_items.push(Item::Struct(parse_quote! {
        #[derive(Clone)]
        pub struct Proxy {
                #(#struct_fields),*
        }
    }));

    // Generate unsafe impl for Send and Sync
    generated_items.push(Item::Impl(parse_quote! {
        unsafe impl Send for Proxy {}
    }));
    generated_items.push(Item::Impl(parse_quote! {
        unsafe impl Sync for Proxy {}
    }));

    // Generate the main impl block.
    let struct_fields_names_only: Vec<_> = domain_creates.iter().map(
        |(name, _)| {
            name
        }
    ).collect();
    generated_items.push(Item::Impl(parse_quote! {
        impl Proxy {
            pub fn new(#(#struct_fields),*) -> Self {
                Self {
                    #(#struct_fields_names_only),*
                }
            }
        }
    }));

    // Generate impl block for trait Proxy
    let as_fns: Vec<ImplItemMethod> = domain_creates.iter().map(|(name, (path, _))| {
        let ident = format_ident!("as_{}", name);
        parse_quote! {
            fn #ident(&self) -> Arc<dyn #path> {
                ::alloc::sync::Arc::new(self.clone())
            }
        }
    }).collect();
    generated_items.push(Item::Impl(parse_quote! {
        impl crate::proxy::Proxy for Proxy {
            // TODO: figure out how to do this without Arc::new every time
            #(#as_fns)*
        }            
    }));

    // Generate impls for domain create traits.
    generated_items.extend(domain_creates.iter().map(|(name, (path, tr))| {
        // Generate the fns inside of the impl block.
        let impl_fns: Vec<_> = tr.items.iter().filter_map(|item| {
            match item {
                TraitItem::Method(md) => {                    
                    let sig = &md.sig;
                    let ident = &sig.ident;

                    // Extract the return type of the usr_ep and generate the return statement for the proxy.
                    // We only support zero or one trait object currently. Nested and tuples are not supported.
                    let proxy_rtn_stmt: syn::Stmt = match &sig.output {
                        syn::ReturnType::Default => panic!("Invalid return type. {:?}", sig),
                        syn::ReturnType::Type(_, ty) => {
                            match &**ty {
                                syn::Type::Tuple(tuple) => {
                                    // Note that the domain create's return type follows the format
                                    // of "(Box<dyn Domain>, ()|Box<dyn SomeTraitObject>)"
                                    assert_eq!(tuple.elems.len(), 2);
                                    let usr_ep_rtn = &tuple.elems[1];
                                    match usr_ep_rtn {
                                        syn::Type::Path(_) => {
                                            let usr_ep_rtn_trait = crate::utils::get_type_inside_of_box(usr_ep_rtn);
                                            match usr_ep_rtn_trait {
                                                syn::Type::TraitObject(tr) => {
                                                    assert_eq!(tr.bounds.len(), 1);
                                                    let tr = &tr.bounds.iter().next().unwrap();
                                                    match tr {
                                                        syn::TypeParamBound::Trait(tr) => {
                                                            // The generated proxy is located in the same
                                                            // module as the trait.
                                                            // It's path should be "trait_module::TraitPath" + "Proxy".
                                                            let mut tr_proxy = tr.path.clone();
                                                            let tr_proxy_ident = tr_proxy.segments.last_mut().unwrap();
                                                            tr_proxy_ident.ident = format_ident!("{}Proxy", tr_proxy_ident.ident);
                                                            parse_quote! {
                                                                return (domain_, ::alloc::boxed::Box::new(#tr_proxy::new(domain_id_, rtn_)));
                                                            }
                                                        }
                                                        syn::TypeParamBound::Lifetime(_) => unimplemented!(),
                                                    }
                                                },
                                                _ => panic!("Expecting a boxed trait object but get {:#?}.", usr_ep_rtn),
                                            }
                                        },
                                        syn::Type::Tuple(tu) => {
                                            assert!(tu.elems.is_empty());
                                            parse_quote! {
                                                return (domain_, rtn_);
                                            }
                                        }
                                        _ => panic!("Invalid usr_ep return type: {:?}", usr_ep_rtn)
                                    }
                                }
                                _ => panic!("Invalid domain create return type: {:?}", ty),
                            }
                        }
                    };

                    let selfless_args = super::utils::get_selfless_args(sig.inputs.iter());                      
                    let rtn = Some(ImplItem::Method(parse_quote! {
                        #sig {
                            let (domain_, rtn_) = self.#name.#ident(#(#selfless_args),*);
                            let domain_id_ = domain_.get_domain_id();
                            #proxy_rtn_stmt
                        }
                    }));
                    drop(selfless_args);
                    rtn
                },
                _ => None,
            }
        }).collect();

        // Generate the impl block
        Item::Impl(parse_quote! {
            impl #path for Proxy {
                #(#impl_fns)*
            }          
        })
    }));

    // Return the generated items.
    generated_items
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
