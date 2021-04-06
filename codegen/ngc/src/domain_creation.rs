use crate::{has_attribute, remove_attribute};

use quote::format_ident;
use syn::{
    parse_quote, punctuated::Punctuated, FnArg, Ident, Item, ItemTrait, Token, TraitItem,
    TraitItemMethod,
};

const DOMAIN_CREATION_ATTR: &str = "domain_creation";

pub fn generate_domain_creation(
    input: &mut ItemTrait,
    _module_path: &Vec<Ident>,
) -> Option<Vec<Item>> {
    if !has_attribute!(input, DOMAIN_CREATION_ATTR) {
        return None;
    }
    let domain_path = input.attrs.iter().find_map(|attr| {
        if let syn::Meta::NameValue(kv) = attr.parse_meta().unwrap() {
            if kv.path.is_ident(DOMAIN_CREATION_ATTR) {
                return Some(kv.lit);
            }
        }
        None
    });

    let _domain_path = crate::expect!(
        domain_path,
        "Domain path not found for {} definition {}",
        DOMAIN_CREATION_ATTR,
        input.ident
    );

    // Remove the interface attribute and add a comment so we know it's an domain_creation
    remove_attribute!(input, DOMAIN_CREATION_ATTR);
    input.attrs.push(parse_quote!{#[doc = "redIDL Auto Generated: domain_creation trait. Generations are below"]});

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
    let _cleaned_trait_methods = {
        let mut cleaned_trait_methods = trait_methods;
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

    unimplemented!()
}

fn generate_domain_creation_one(
    domain_ident: &Ident,
    domain_path: &str,
    method: &TraitItemMethod,
    _cleaned_method: &TraitItemMethod,
) -> syn::Item {
    let domain_ident_str = domain_ident.to_string();
    let method_ident = &method.sig.ident;
    let generated_ident = format_ident!(
        "redidl_generated_domain_creation_{}_{}",
        domain_ident,
        method_ident
    );
    let canonicalized_domain_path = domain_path.replace("/", "_");
    let domain_start_ident = format_ident!("_binary_{}_start", canonicalized_domain_path);
    let domain_end_ident = format_ident!("_binary_{}_end", canonicalized_domain_path);

    let generated: syn::ItemFn = parse_quote! {
        fn #generated_ident -> {
            extern "C" {
                fn #domain_start_ident();
                fn #domain_end_ident();
            }

            let binary_range = (
                #domain_start_ident as *const u8,
                #domain_end_ident as *const u8,
            );

            type UserInit =
                fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>) -> Box<dyn interface::dom_c::DomC>;

            let (dom, entry) = unsafe { load_domain(name, binary_range) };

            let user_ep: UserInit = unsafe { ::core::mem::transmute::<*const (), UserInit>(entry) };

            let pdom = Box::new(PDomain::new(::alloc::syn::Arc::clone(&dom)));
            let pheap = Box::new(PHeap::new());

            // update current domain id
            let thread = thread::get_current_ref();
            let old_id = {
                let mut thread = thread.lock();
                let old_id = thread.current_domain_id;
                thread.current_domain_id = dom.lock().id;
                old_id
            };

            // Enable interrupts on exit to user so it can be preempted
            enable_irq();
            let domain = user_ep(pdom, pheap);
            disable_irq();

            // change domain id back
            {
                thread.lock().current_domain_id = old_id;
            }

            #[cfg(feature = "domain_creation_log")]
            println!("domain/{}: returned from entry point", #domain_ident_str);
            (::alloc::boxed::Box::new(PDomain::new(Arc::clone(&dom))), domain)
        }
    };

    syn::Item::Fn(generated)
}
