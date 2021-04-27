use std::collections::HashMap;

use crate::{has_attribute, remove_attribute};

use log::info;
use quote::format_ident;
use syn::{FnArg, Ident, Item, ItemTrait, Lit, Token, TraitItem, TraitItemMethod, parse_quote, punctuated::Punctuated};

const DOMAIN_CREATE_ATTR: &str = "domain_create";

pub fn generate_domain_create(
    input: &mut ItemTrait,
    _module_path: &[Ident],
) -> Option<Item> {
    // Only traits with `DOMAIN_CREATE_ATTR` will be processed.
    if !has_attribute!(input, DOMAIN_CREATE_ATTR) {
        return None;
    }

    info!("Generating domain create for trait {:?}.", input.ident);

    // Create an attribute map
    let attrs: HashMap<String, Option<Lit>> = input.attrs.iter().map(|attr| {
        match attr.parse_meta().unwrap() {
            syn::Meta::List(x) => unimplemented!("{:?}", x),
            syn::Meta::NameValue(kv) => {
                (kv.path.get_ident().unwrap().to_string(), Some(kv.lit))
            },
            syn::Meta::Path(path) => {
                (path.get_ident().unwrap().to_string(), None)
            }
        }
    }).collect();

    // Extract the domain path.
    let domain_path = crate::expect!(
        attrs.get("path"),
        "Domain path not found for trait {}",
        input.ident
    );
    let domain_path = domain_path.as_ref().expect("Domain path is empty");
    let domain_path = match domain_path {
        Lit::Str(domain_path) => {
            domain_path.value()
        }
        _ => panic!("Expecting a string."),
    };

    // Extract the domain name.
    let domain_name = crate::expect!(
        attrs.get("name"),
        "Domain name not found for trait {}",
        input.ident
    );
    let domain_name = domain_name.as_ref().expect("Domain name is empty");
    let domain_name = match domain_name {
        Lit::Str(domain_name) => {
            domain_name.value()
        }
        _ => panic!("Expecting a string."),
    };


    // Remove the interface attribute and add a comment so we know it's an domain_create
    remove_attribute!(input, DOMAIN_CREATE_ATTR);
    input.attrs.push(parse_quote!{#[doc = "redIDL Auto Generated: domain_create trait. Generations are below"]});

    // Generate code. Proxy is generated inplace and domain create is returned.
    let generated_impl_items: Vec<syn::ImplItemMethod> = input.items.iter().map(|item| {
        match item {
            TraitItem::Method(method) => {
                generate_domain_create_for_trait_method(&domain_name, &domain_path, method)
            },
            _ => unimplemented!("Non-method member found in trait {:#?}", input),
        }
    }).collect();


    let generated: syn::ItemImpl = parse_quote!(
        impl #domain_name for ::crate::syscalls::PDomain -> {
            #(#generated_impl_items)*
        }
    );

    Some(Item::Impl(generated))
}

fn generate_domain_create_for_trait_method(
    domain_name: &str,
    domain_path: &str,
    method: &TraitItemMethod,
) -> syn::ImplItemMethod {
    // Remove `self` from the argument list
    let selfless_args: Vec<_> = method.sig.inputs.iter().filter(|arg| {
        match arg {
            FnArg::Receiver(_) => false,
            FnArg::Typed(typed) => true,
        }
    }).collect();

    let method_ident = &method.sig.ident;
    let method_args = &method.sig.inputs;
    let method_sig = &method.sig;
    let generated_ident = format_ident!(
        "redidl_generated_domain_create_{}_{}",
        domain_name,
        method_ident
    );
    let canonicalized_domain_path = domain_path.replace("/", "_");
    let domain_start_ident = format_ident!("_binary_{}_start", canonicalized_domain_path);
    let domain_end_ident = format_ident!("_binary_{}_end", canonicalized_domain_path);
    let rtn = &method.sig.output;

    parse_quote! {   
        #method_sig {
            // Entering kernel, disable irq
            ::crate::interrupt::disable_irq();

            extern "C" {
                fn #domain_start_ident();
                fn #domain_end_ident();
            }

            let binary_range = (
                #domain_start_ident as *const u8,
                #domain_end_ident as *const u8,
            );

            type UserInit =
                fn(Box<dyn ::syscalls::Syscall>, Box<dyn ::syscalls::Heap>, #(#selfless_args),*) -> (alloc::boxed::Box<dyn ::syscalls::Domain>, #rtn);

            let (dom, entry) = unsafe { ::crate::domain::load_domain(name, binary_range) };

            // Type cast the pointer to entry point to the correct type.
            let user_ep: UserInit = unsafe { ::core::mem::transmute::<*const (), UserInit>(entry) };

            let pdom = ::alloc::Box::new(PDomain::new(::alloc::syn::Arc::clone(&dom)));
            let pheap = ::alloc::Box::new(PHeap::new());

            // update current domain id.
            let thread = thread::get_current_ref();
            let old_id = {
                let mut thread = thread.lock();
                let old_id = thread.current_domain_id;
                thread.current_domain_id = dom.lock().id;
                old_id
            };

            // Enable interrupts on exit to user so it can be preempted.
            enable_irq();
            // Jumps to the domain entry point.
            let domain = user_ep(pdom, pheap);
            // Disable interrupts as we are back to the kernel.
            disable_irq();

            // change domain id back
            {
                thread.lock().current_domain_id = old_id;
            }

            #[cfg(feature = "domain_create_log")]
            println!("domain/{}: returned from entry point", #domain_name);
            (::alloc::boxed::Box::new(PDomain::new(Arc::clone(&dom))), domain)

            // Leaving kernel, reable irq
            ::crate::interrupt::enable_irq();
        }
    }
}
