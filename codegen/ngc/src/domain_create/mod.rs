mod blob_domain_create;
mod linked_domain_create;

use crate::{has_attribute, remove_attribute};
use log::info;
use quote::format_ident;
use std::collections::HashMap;
use syn::{
    parse_quote, Expr, FnArg, Ident, ImplItemMethod, Item, ItemFn, ItemTrait, Lit, Path, TraitItem,
    TraitItemMethod,
};

pub const LINKED_DOMAIN_CREATE_ATTR: &str = "domain_create";
pub const BLOB_DOMAIN_CREATE_ATTR: &str = "domain_create_blob";

/// Generation of domain create.
/// It also keep track of all the domain create it generates.
pub struct DomainCreateBuilder {
    domain_creates: Vec<(Path, ItemTrait)>,
}

impl DomainCreateBuilder {
    pub fn new() -> Self {
        Self {
            domain_creates: vec![],
        }
    }

    /// Generates the domain create for `input` if it has the `DOMAIN_CREATE_ATTR` attribute.
    pub fn generate_domain_create(
        &mut self,
        input: &mut ItemTrait,
        module_path: &[Ident],
    ) -> Option<Vec<Item>> {
        // Create an attribute map.
        let attrs: HashMap<String, Option<Lit>> = crate::utils::create_attribue_map(&input.attrs);

        // Filter out non-domain_create traits and remove domain_create attributes.
        let is_blob_domain_create;
        if has_attribute!(input, LINKED_DOMAIN_CREATE_ATTR) {
            is_blob_domain_create = false;
            remove_attribute!(input, LINKED_DOMAIN_CREATE_ATTR);
        } else if has_attribute!(input, BLOB_DOMAIN_CREATE_ATTR) {
            is_blob_domain_create = true;
            remove_attribute!(input, BLOB_DOMAIN_CREATE_ATTR);
        } else {
            return None;
        }

        info!("Generating domain create for trait {:?}.", input.ident);

        // Compute the path to the trait.
        let trait_path: String = module_path
            .iter()
            .map(|ident| ident.to_string())
            .collect::<Vec<String>>()
            .join("::");
        let trait_path = format!("{}::{}", trait_path, input.ident);
        let trait_path: syn::Path = syn::parse_str(&trait_path).unwrap();

        // Put the trait path into the list of domain creates.
        self.domain_creates
            .push((trait_path.clone(), input.clone()));

        // Create a copy of the input, refactor the path from `crate` to `interface, and we will be
        // working with the refactored one from now on.
        // The reason is that domain create will be generated into the kernel, which has a different
        // dependency path to the interface
        let mut input_copy = input.clone();
        crate::path_refactoring::refactor_path_in_trait(
            &format_ident!("crate"),
            &format_ident!("interface"),
            &mut input_copy,
        );

        // Add a comment so we know it's generated.
        input.attrs.push(
            parse_quote! {#[doc = "redIDL Auto Generated: domain_create trait. Generations are below"]},
        );
        let input = input_copy;

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
                    if is_blob_domain_create {
                        self::blob_domain_create::generate_domain_create_for_trait_method(
                            &domain_path,
                            method,
                        )
                    } else {
                        self::linked_domain_create::generate_domain_create_for_trait_method(
                            &domain_path,
                            method,
                        )
                    }
                }
                _ => unimplemented!("Non-method member found in trait {:#?}", input),
            })
            .unzip();

        // Generate the impl block.
        let mut generated: Vec<Item> = Vec::new();
        generated.push(Item::Impl(parse_quote! {
            impl #trait_path for crate::syscalls::PDomain {
                #(#generated_impl_items)*
            }
        }));

        // Append the fn blocks
        generated.extend(generated_fns.into_iter().map(Item::Fn));

        // Return the generated code.
        Some(generated)
    }

    pub fn generate_create_init(&self) -> Item {
        let domain_create_paths: Vec<_> =
            self.domain_creates.iter().map(|(path, _)| path).collect();

        let arcs: Vec<Expr> = self.domain_creates.iter().map(|_| {
            parse_quote! {
                ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom)))
            }
        }).collect();

        parse_quote! {

            pub fn create_domain_init() -> ::alloc::boxed::Box<dyn ::syscalls::Domain> {
                let name = "init";

                extern "C" {
                    fn _binary_domains_build_redleaf_init_start();
                    fn _binary_domains_build_redleaf_init_end();
                }

                let binary_range = (
                    _binary_domains_build_redleaf_init_start as *const u8,
                    _binary_domains_build_redleaf_init_end as *const u8,
                );

                type UserInit = fn(
                    ::alloc::boxed::Box<dyn ::syscalls::Syscall + Send + Sync>,
                    ::alloc::boxed::Box<dyn ::syscalls::Heap + Send + Sync>,
                    ::alloc::boxed::Box<dyn ::syscalls::Interrupt>,

                    #(::alloc::sync::Arc<dyn #domain_create_paths>,)*
                );

                let (dom, entry) = unsafe { crate::domain::load_domain(name, binary_range) };

                let user_ep: UserInit = unsafe { ::core::mem::transmute::<*const (), UserInit>(entry) };

                // update current domain id
                let thread = crate::thread::get_current_ref();
                let old_id = {
                    let mut thread = thread.lock();
                    let old_id = thread.current_domain_id;
                    thread.current_domain_id = dom.lock().id;
                    old_id
                };

                // Enable interrupts on exit to user so it can be preempted
                crate::interrupt::enable_irq();
                user_ep(
                    ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom))),
                    ::alloc::boxed::Box::new(crate::heap::PHeap::new()),
                    ::alloc::boxed::Box::new(crate::syscalls::Interrupt::new()),
                    #(#arcs),*
                );
                crate::interrupt::disable_irq();

                // change domain id back
                {
                    thread.lock().current_domain_id = old_id;
                }

                #[cfg(feature = "domain_create_log")]
                println!("domain/{}: returned from entry point", name);
                ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom)))
            }
        }
    }

    pub fn take(self) -> Vec<(Path, ItemTrait)> {
        self.domain_creates
    }
}
