use std::collections::HashMap;

use crate::{has_attribute, remove_attribute};

use log::info;
use quote::format_ident;
use syn::{FnArg, Ident, ImplItemMethod, Item, ItemFn, ItemTrait, Lit, Token, TraitItem, TraitItemMethod, parse_quote};

const DOMAIN_CREATE_ATTR: &str = "domain_create";

pub fn generate_domain_create(input: &mut ItemTrait, module_path: &[Ident]) -> Option<Vec<Item>> {
    // Only traits with `DOMAIN_CREATE_ATTR` will be processed.
    if !has_attribute!(input, DOMAIN_CREATE_ATTR) {
        return None;
    }

    info!("Generating domain create for trait {:?}.", input.ident);

    // Create a copy of the input, refactor the path from `crate` to `interface, and we will be
    // working with the refactored one from now on.
    // The reason is that domain create will be generated into the kernel, which has a different
    // dependency path to the interface
    let mut input_copy = input.clone();
    crate::path_refactoring::refactor_path_in_trait(&format_ident!("crate"), &format_ident!("interface"), &mut input_copy);

    // Remove the interface attribute and add a comment so we know it's an domain_create
    remove_attribute!(input, DOMAIN_CREATE_ATTR);
    input.attrs.push(
        parse_quote! {#[doc = "redIDL Auto Generated: domain_create trait. Generations are below"]},
    );
    let input = input_copy;

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

    // Generate code. Proxy is generated inplace and domain create is returned.
    let (generated_impl_items, generated_fns): (Vec<ImplItemMethod>, Vec<ItemFn>) = input
        .items
        .iter()
        .map(|item| match item {
            TraitItem::Method(method) => {
                generate_domain_create_for_trait_method(&domain_path, method)
            }
            _ => unimplemented!("Non-method member found in trait {:#?}", input),
        })
        .unzip();

    // Compute the path to the trait.
    let mut trait_path: String = module_path.iter().map(|ident| {
        ident.to_string()
    }).collect::<Vec<String>>().join("::");
    let trait_path = format!("{}::{}", trait_path, input.ident);
    let trait_path: syn::Path = syn::parse_str(&trait_path).unwrap();

    // Generate the impl block.
    let mut generated: Vec<Item> = Vec::new();
    generated.push(Item::Impl(parse_quote! {
        impl #trait_path for crate::syscalls::PDomain {
            #(#generated_impl_items)*
        }
    }));

    generated.extend(generated_fns.into_iter().map(|f| Item::Fn(f)));

    Some(generated)
}

/// This generates a public fn and a impl method.
/// This public fn is exposed to the kernel while the impl method is exposed to the users.
fn generate_domain_create_for_trait_method(
    domain_path: &str,
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
    let method_args = method.sig.inputs.iter().collect::<Vec<_>>();
    let method_sig = &method.sig;
    let canonicalized_domain_path = domain_path.replace("/", "_");
    let generated_fn_ident = format_ident!("{}_{}", canonicalized_domain_path, method_ident);
    let domain_start_ident = format_ident!("_binary_domains_build_{}_start", canonicalized_domain_path);
    let domain_end_ident = format_ident!("_binary_domains_build_{}_end", canonicalized_domain_path);
    let rtn = &method.sig.output;
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
                fn(::alloc::boxed::Box<dyn ::syscalls::Syscall>, ::alloc::boxed::Box<dyn ::syscalls::Heap>, #(#selfless_args),*) -> #ep_rtn;

            let (dom_, entry_) = unsafe { crate::domain::load_domain(#domain_path, binary_range_) };

            // Type cast the pointer to entry point to the correct type.
            let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };

            let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)));
            let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());

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
    };

    (generated_impl, generated_fn)
}
