use std::collections::HashMap;

use crate::{has_attribute, remove_attribute};

use log::info;
use quote::format_ident;

use syn::{
    parse_quote, Expr, FnArg, Ident, ImplItemMethod, Item, ItemFn, ItemTrait, Lit, Path, TraitItem,
    TraitItemMethod,
};

#[derive(Debug, Clone, Copy)]
pub enum DomainCreateComponent {
    Domain,
    MMap,
    Heap,
}

/// This generates a public fn and a impl method.
/// This public fn is exposed to the kernel while the impl method is exposed to the users.
pub fn generate_domain_create_for_trait_method(
    domain_path: &str,
    domain_components: &Vec<DomainCreateComponent>,
    method: &TraitItemMethod,
) -> (syn::ImplItemMethod, syn::ItemFn) {
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
    let _method_args = method.sig.inputs.iter().collect::<Vec<_>>();
    let method_sig = &method.sig;
    let canonicalized_domain_path = domain_path.replace("/", "_");
    let generated_fn_ident = format_ident!("{}_{}", canonicalized_domain_path, method_ident);
    let domain_start_ident =
        format_ident!("_binary_domains_build_{}_start", canonicalized_domain_path);
    let domain_end_ident = format_ident!("_binary_domains_build_{}_end", canonicalized_domain_path);
    let rtn = &method.sig.output;
    let ep_rtn = match &method.sig.output {
        syn::ReturnType::Type(_, ty) => match ty {
            box syn::Type::Tuple(tuple) => {
                assert_eq!(
                    tuple.elems.iter().count(),
                    2,
                    "Expecting a tuple of two in the return type of method {:?} of domain {:?}",
                    method_ident,
                    domain_path
                );
                tuple.elems.iter().nth(1).unwrap()
            }
            _ => panic!(
                "Expecting a tuple of two in the return type of method {:?} of domain {:?}",
                method_ident, domain_path
            ),
        },
        syn::ReturnType::Default => panic!(
            "Method {:?} of domain {:?} does not have a return type. Expecting a tuple of two.",
            method_ident, domain_path
        ),
    };

    info!("SELFLESS: {:#?}", selfless_args);

    // Statements to initialize the components needed by the domain
    let domain_component_creation = domain_components.iter().map(|component| {
        match component {
            &DomainCreateComponent::Domain => parse_quote! {
                let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)));
            },
            &DomainCreateComponent::MMap => parse_quote! {
                let pmmap_ = ::alloc::boxed::Box::new(crate::syscalls::Mmap::new());
            },
            &DomainCreateComponent::Heap => parse_quote! {
                let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
            }
        }
    }).collect::<Vec<syn::Stmt>>();

    info!("MADE IT!");

    let domain_entrypoint_components: Vec<FnArg> = domain_components
        .iter()
        .map(|component| match component {
            &DomainCreateComponent::Domain => parse_quote! {
                pdom_: ::alloc::boxed::Box<dyn syscalls::Syscall>
            },
            &DomainCreateComponent::MMap => parse_quote! {
                pmmap_: ::alloc::boxed::Box<dyn syscalls::Mmap>
            },
            &DomainCreateComponent::Heap => parse_quote! {
                pheap_: ::alloc::boxed::Box<dyn syscalls::Heap>
            },
        })
        .collect();

    info!("MADE IT 2! {:#?}", domain_entrypoint_components);

    let entry_point_args: Vec<&FnArg> = domain_entrypoint_components
        .iter()
        .chain(selfless_args.clone())
        .collect();

    // Generate impl method.
    let generated_impl = parse_quote! {
        #method_sig {
            // Entering kernel, disable irq
            crate::interrupt::disable_irq();

            let rtn_ = #generated_fn_ident(#(#selfless_args),*);

            // Leaving kernel, reable irq
            crate::interrupt::enable_irq();

            // Returns the domain to caller.
            rtn_
        }
    };

    // Generated fn.
    let mut fn_sig = method_sig.clone();
    fn_sig.ident = generated_fn_ident.clone();

    // The different domain_create_components required for starting the domain
    // By default this is only ::syscalls:Syscall / PDomain and syscalls::Heap / PHeap

    let generated_fn = parse_quote! {
        pub(crate) fn #generated_fn_ident(#(#selfless_args),*) #rtn {
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
                fn(#(#entry_point_args),*) -> #ep_rtn;

            let (dom_, entry_) = unsafe { crate::domain::load_domain(#domain_path, binary_range_) };

            // Type cast the pointer to entry point to the correct type.
            let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };

            // Creation of entry point arguments
            #(#domain_component_creation)*

            // update current domain id.
            let thread_ = crate::thread::get_current_ref();
            let old_id_ = {
                let mut thread = thread_.lock();
                let old_id = thread.current_domain_id;
                thread.current_domain_id = dom_.lock().id;
                old_id
            };

            // Enable interrupts on exit to user so it can be preempted.
            crate::interrupt::enable_irq();
            // Jumps to the domain entry point.
            let ep_rtn_ = user_ep_(#(#entry_point_args),*);
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
    };

    (generated_impl, generated_fn)
}
