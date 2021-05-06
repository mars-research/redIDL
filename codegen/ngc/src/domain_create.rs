use std::collections::HashMap;

use crate::{has_attribute, remove_attribute};

use log::info;
use quote::format_ident;
use syn::{
    parse_quote, FnArg, Ident, Item, ItemTrait, Lit, Token, TraitItem,
    TraitItemMethod,
};

const DOMAIN_CREATE_ATTR: &str = "domain_create";

pub fn generate_domain_create(input: &mut ItemTrait, module_path: &[Ident]) -> Option<Item> {
    // Only traits with `DOMAIN_CREATE_ATTR` will be processed.
    if !has_attribute!(input, DOMAIN_CREATE_ATTR) {
        return None;
    }

    info!("Generating domain create for trait {:?}.", input.ident);

    // Create an attribute map
    let attrs: HashMap<String, Option<Lit>> = crate::utils::create_attribue_map(&input.attrs);

    // Extract the domain path.
    let domain_path = crate::expect!(
        attrs.get("path"),
        "Domain path not found for trait {}",
        input.ident
    );
    let domain_path = domain_path.as_ref().expect("Domain path is empty");
    let domain_path = match domain_path {
        Lit::Str(domain_path) => domain_path.value(),
        _ => panic!("Expecting a string."),
    };

    // Remove the interface attribute and add a comment so we know it's an domain_create
    remove_attribute!(input, DOMAIN_CREATE_ATTR);
    input.attrs.push(
        parse_quote! {#[doc = "redIDL Auto Generated: domain_create trait. Generations are below"]},
    );

    // Generate code. Proxy is generated inplace and domain create is returned.
    let generated_impl_items: Vec<syn::ImplItemMethod> = input
        .items
        .iter()
        .map(|item| match item {
            TraitItem::Method(method) => {
                generate_domain_create_for_trait_method(&domain_path, method)
            }
            _ => unimplemented!("Non-method member found in trait {:#?}", input),
        })
        .collect();

    // Compute the path to the trait.
    let mut trait_path: String = module_path.iter().map(|ident| {
        ident.to_string()
    }).collect::<Vec<String>>().join("::");
    let trait_path = format!("{}::{}", trait_path, input.ident);
    let trait_path: syn::Path = syn::parse_str(&trait_path).unwrap();

    // Generate the impl block.
    let generated: syn::ItemImpl = parse_quote! {
        impl #trait_path for crate::syscalls::PDomain {
            #(#generated_impl_items)*
        }
    };

    Some(Item::Impl(generated))
}

fn generate_domain_create_for_trait_method(
    domain_path: &str,
    method: &TraitItemMethod,
) -> syn::ImplItemMethod {
    // Remove `self` from the argument list
    let selfless_args: Vec<_> = method
        .sig
        .inputs
        .iter()
        .filter(|arg| match arg {
            FnArg::Receiver(_) => false,
            FnArg::Typed(_) => true,
        })
        .collect();

    // Extract essential variables for generation.
    let method_ident = &method.sig.ident;
    let method_args = &method.sig.inputs;
    let method_sig = &method.sig;
    let canonicalized_domain_path = domain_path.replace("/", "_");
    let domain_start_ident = format_ident!("_binary_domains_build_{}_start", canonicalized_domain_path);
    let domain_end_ident = format_ident!("_binary_domains_build_{}_end", canonicalized_domain_path);
    let ep_rtn = match &method.sig.output {
        syn::ReturnType::Type(_, ty) => match ty {
            box syn::Type::Tuple(tuple) => {
                assert_eq!(tuple.elems.iter().count(), 2, "Expecting a tuple of two in the return type of method {:?} of domain {:?}", method_ident, domain_path);
                tuple.elems.iter().skip(1).next().unwrap()
            },
            _ => panic!("Expecting a tuple of two in the return type of method {:?} of domain {:?}", method_ident, domain_path),
        },
        syn::ReturnType::Default => panic!("Method {:?} of domain {:?} does not have a return type. Expecting a tuple of two.", method_ident, domain_path),
    };

    parse_quote! {
        #method_sig {
            // Entering kernel, disable irq
            crate::interrupt::disable_irq();

            extern "C" {
                fn #domain_start_ident();
                fn #domain_end_ident();
            }

            let binary_range_ = (
                #domain_start_ident as *const u8,
                #domain_end_ident as *const u8,
            );

            type UserInit_ =
                fn(Box<dyn ::syscalls::Syscall>, Box<dyn ::syscalls::Heap>, #(#selfless_args),*) -> #ep_rtn;

            let (dom_, entry_) = unsafe { crate::domain::load_domain(#domain_path, binary_range_) };

            // Type cast the pointer to entry point to the correct type.
            let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };

            let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)));
            let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());

            // update current domain id.
            let thread_ = thread::get_current_ref();
            let old_id_ = {
                let mut thread = thread_.lock();
                let old_id = thread.current_domain_id;
                thread.current_domain_id = dom_.lock().id;
                old_id
            };

            // Enable interrupts on exit to user so it can be preempted.
            crate::interrupt::enable_irq();
            // Jumps to the domain entry point.
            let ep_rtn_ = user_ep_(pdom_, pheap_, #(#selfless_args),*);
            // Disable interrupts as we are back to the kernel.
            crate::interrupt::disable_irq();

            // change domain id back
            {
                thread_.lock().current_domain_id = old_id_;
            }

            #[cfg(feature = "domain_create_log")]
            println!("domain/{}: returned from entry point", #domain_path);

            // Setup the return object.
            let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)));
            let rtn_ = (dom_, ep_rtn_);

            // Leaving kernel, reable irq
            crate::interrupt::enable_irq();

            // Returns the domain to caller.
            rtn_
        }
    }
}
