mod blob_domain_create;
mod linked_domain_create;

use crate::{domain_entrypoint::DomainEntrypointFactory, has_attribute, remove_attribute};
use log::{debug, error, info, warn};
use quote::{format_ident, ToTokens};
use std::collections::HashMap;
use syn::{
    parse_quote, Expr, Ident, ImplItemMethod, Item, ItemFn, ItemTrait, Lit, Meta, NestedMeta, Path,
    TraitItem,
};

pub const LINKED_DOMAIN_CREATE_ATTR: &str = "domain_create";
pub const BLOB_DOMAIN_CREATE_ATTR: &str = "domain_create_blob";
pub const DOMAIN_CREATE_COMPONENTS_ATTR: &str = "domain_create_components";

#[derive(Debug, Clone, Copy)]
pub enum DomainCreateComponent {
    Domain,
    MMap,
    Heap,
}

impl DomainCreateComponent {
    fn creation_statement(&self) -> syn::Stmt {
        match self {
            &DomainCreateComponent::Domain => parse_quote! {
                let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)));
            },
            &DomainCreateComponent::MMap => parse_quote! {
                let pmmap_ = ::alloc::boxed::Box::new(crate::syscalls::Mmap::new());
            },
            &DomainCreateComponent::Heap => parse_quote! {
                let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
            },
        }
    }

    fn as_fn_argument(&self) -> syn::FnArg {
        match self {
            &DomainCreateComponent::Domain => parse_quote! {
                pdom_: ::alloc::boxed::Box<dyn syscalls::Syscall>
            },
            &DomainCreateComponent::MMap => parse_quote! {
                pmmap_: ::alloc::boxed::Box<dyn syscalls::Mmap>
            },
            &DomainCreateComponent::Heap => parse_quote! {
                pheap_: ::alloc::boxed::Box<dyn syscalls::Heap>
            },
        }
    }
}

/// Generation of domain create.
/// It also keep track of all the domain create it generates.
pub struct DomainCreateBuilder {
    /// The path to the root of the domains folder for RedLeaf, used for entrypoint generation
    domain_entrypoint_factory: Option<DomainEntrypointFactory>,
    domain_creates: Vec<(Path, ItemTrait)>,
}

impl DomainCreateBuilder {
    pub fn new() -> Self {
        Self {
            domain_entrypoint_factory: None,
            domain_creates: vec![],
        }
    }

    pub fn new_with_domains_folder(domains_folder: DomainEntrypointFactory) -> Self {
        Self {
            domain_entrypoint_factory: Some(domains_folder),
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

        // Default domain create components, can be overridden with #[domain_create_components(Domain, MMap, Heap)]
        let mut domain_components =
            vec![DomainCreateComponent::Domain, DomainCreateComponent::Heap];

        let metas = input.attrs.iter().map(|attr| attr.parse_meta().unwrap());
        for meta in metas {
            if meta.path().is_ident(DOMAIN_CREATE_COMPONENTS_ATTR) {
                let mut new_domain_components = vec![];
                // Override domain components
                match meta {
                    Meta::List(m) => {
                        for component in m.nested.iter() {
                            if let NestedMeta::Meta(comp) = component {
                                if let Some(ident) = comp.path().get_ident() {
                                    match ident.to_string().as_str() {
                                        "Domain" => new_domain_components
                                            .push(DomainCreateComponent::Domain),
                                        "MMap" => {
                                            new_domain_components.push(DomainCreateComponent::MMap)
                                        }
                                        "Heap" => {
                                            new_domain_components.push(DomainCreateComponent::Heap)
                                        }
                                        other => {
                                            panic!(
                                                "Unsupported domain_create_component '{:#?}'",
                                                &other
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }

                // info!("Domain Create Components: {:#?}", new_domain_components);
                domain_components = new_domain_components;
            }
        }

        remove_attribute!(input, DOMAIN_CREATE_COMPONENTS_ATTR);

        info!("Generating domain create for trait {:?}.", input.ident);

        // Compute the path to the trait.
        let trait_path: String = module_path
            .iter()
            .map(|ident| ident.to_string())
            .collect::<Vec<String>>()
            .join("::");
        let trait_path = format!("{}::{}", trait_path, input.ident);
        let trait_path: syn::Path = syn::parse_str(&trait_path).unwrap();

        // TODO: Remove
        // debug!("INPUT: {:}", &input.to_token_stream());
        // debug!("MODULE_PATH: {:#?}", &module_path);
        // debug!("TRAIT_PATH: {:#?}", trait_path);

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

        // Generate the entrypoint if we have a domain path and a relative path
        let domain_relative_path =
            self.get_relative_domain_path(&input, attrs.get("relative_path"));

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
                        // If we have a relative path then we'll generate an entrypoint
                        if self.domain_entrypoint_factory.is_some() {
                            if let Some(domain_relative_path) = domain_relative_path.as_ref() {
                                self.domain_entrypoint_factory
                                    .as_ref()
                                    .unwrap()
                                    .generate_domain_entrypoint_crates(
                                        domain_relative_path,
                                        &domain_components,
                                        method,
                                    );
                            }
                        }

                        self::linked_domain_create::generate_domain_create_for_trait_method(
                            &domain_path,
                            &domain_components,
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

    pub fn get_domain_paths(&self) -> Vec<&Path> {
        self.domain_creates.iter().map(|(path, _)| path).collect()
    }

    pub fn generate_create_init(&self) -> Item {
        let domain_create_paths = self.get_domain_paths();

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

    fn get_relative_domain_path(
        &self,
        input: &ItemTrait,
        relative_path: Option<&Option<Lit>>,
    ) -> Option<std::path::PathBuf> {
        if self.domain_entrypoint_factory.is_none() {
            return None;
        }

        if let Some(Some(path)) = relative_path {
            match path {
                Lit::Str(s) => {
                    // Check it's a valid path and crate
                    let domain_path = self
                        .domain_entrypoint_factory
                        .as_ref()
                        .unwrap()
                        .as_relative_to_domains_folder(&std::path::PathBuf::from(s.value()));
                    if Self::is_path_crate(&domain_path) {
                        return Some(domain_path);
                    } else {
                        return None;
                    }
                }
                _ => {
                    warn!("'relative_path' for trait {} is not a string!", input.ident);
                    return None;
                }
            }
        } else {
            warn!("The trait {} has no 'relative_path' specified, an entrypoint will not be generated", input.ident);
            return None;
        }
    }

    /// Checks that the path is a folder and checks that the folder contains Cargo.toml
    fn is_path_crate(path: &std::path::Path) -> bool {
        if !path.is_dir() {
            error!("Crate Path {:#?} is not a directory!", path.canonicalize());
            return false;
        }

        if !path.join("Cargo.toml").exists() {
            error!(
                "Crate Path {:#?} does not contain Cargo.toml!",
                path.canonicalize()
            );
            return false;
        }

        return true;
    }
}
