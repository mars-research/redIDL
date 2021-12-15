use crate::domain_create::DomainCreateComponent;
use std::{
    fs::{canonicalize, create_dir, remove_dir_all, File},
    iter::Map,
    path::{Path, PathBuf},
};

use log::{debug, error, info, warn};
use quote::{format_ident, ToTokens};
use syn::{parse_quote, TraitItemMethod};

pub struct DomainEntrypointFactory {
    output_folder_created: bool,
    output_folder_path: PathBuf,

    domains_folder: PathBuf,
}

impl DomainEntrypointFactory {
    pub fn new(domains_folder: PathBuf) -> Self {
        let output_folder_path = &domains_folder.join("generated");

        Self::setup_output_folder(output_folder_path);

        DomainEntrypointFactory {
            output_folder_created: true,
            output_folder_path,
            domains_folder,
        }
    }

    fn setup_output_folder(output_path: &Path) {
        // Check if folder already exists and if there're files inside
        let output_path = Path::new(output_path);

        // Check we're not overriding something important!
        if output_path.exists()
            && !output_path
                .join("ngc_generated_domain_entrypoints")
                .exists()
        {
            panic!("NGC will not override directories that it did not generate. {:#?} does not appear to be generated!", output_path)
        }

        // If we made it here, we're okay!

        if output_path.exists() {
            // Delete the folder and replace it with a new one
            info!("Going to delete {:#?}", canonicalize(output_path).unwrap());
            let res = remove_dir_all(output_path);
            if res.is_err() {
                panic!("Failed to remove generated directory {:#?}", output_path);
            }
        }

        // At this point we need to create the directory and the `ngc_generated_domain_entrypoints` file
        if let Err(e) = create_dir(output_path) {
            panic!(
                "Failed to create directory at {:#?}, ERROR: {:#?}",
                output_path, e
            );
        }
        File::create(output_path.join("ngc_generated_domain_entrypoints"))
            .expect("Failed to create indicator file for entrypoint crates");
    }

    pub fn as_relative_to_domains_folder(&self, path: &Path) -> PathBuf {
        self.domains_folder.join(path)
    }

    pub fn generate_entrypoint_cargo(
        &self,
        domain_relative_path: &Path,
        domain_name: &str,
    ) -> String {
        format!(
            r#"
                [package]
                name = "{:}"
                version = "0.1.0"
                authors = ["Redleaf team <aburtsev@uci.edu>"]
                edition = "2018"
    
                # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html
    
                [[bin]]
                name = "{:}"
                path = "src/main.rs"
    
                [dependencies]
                interface = {{ path = "../../../interface/generated" }}
                libsyscalls = {{ path = "../../../lib/core/libsyscalls" }}
                syscalls = {{ path = "../../../lib/core/interfaces/syscalls" }}
                console = {{ path = "../../../lib/core/console" }}
    
                {:} = {{ path = "../../{:}" }}       
                "#,
            domain_name.to_owned() + "_entry_point",
            domain_name,
            domain_name,
            domain_relative_path
                .strip_prefix("../../../../domains/")
                .unwrap()
                .display()
        )
    }

    pub fn generate_entrypoint_main_rs(
        &self,
        domain_name: &str,
        domain_components: &Vec<DomainCreateComponent>,
        method: &TraitItemMethod,
    ) -> String {
        let domain_components_args = domain_components
            .iter()
            .map(|comp| match comp {
                &DomainCreateComponent::Domain => {
                    parse_quote! {s: Box<dyn syscalls::Syscall + Send + Sync>}
                }
                &DomainCreateComponent::Heap => {
                    parse_quote! {heap: Box<dyn syscalls::Heap + Send + Sync>}
                }
                &DomainCreateComponent::MMap => {
                    parse_quote! {mmap: Box<dyn syscalls::Mmap + Send + Sync}
                }
            })
            .collect::<Vec<syn::FnArg>>();

        let domain_components_init_statements = domain_components.iter().map(|comp| match comp {
            &DomainCreateComponent::Domain => {
                parse_quote! {  libsyscalls::syscalls::init(s); }
            }
            &DomainCreateComponent::Heap => {
                parse_quote! {  interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id()); }
            }
            &DomainCreateComponent::MMap => {
                parse_quote! {  libsyscalls::syscalls::init_mmap(m);}
            }
        }).collect::<Vec<syn::Stmt>>();

        let other_domain_args = method
            .sig
            .inputs
            .iter()
            .filter(|arg| match arg {
                syn::FnArg::Typed(pat) => true,
                syn::FnArg::Receiver(_) => false,
            })
            .collect::<Vec<_>>();

        let domain_args = domain_components_args
            .iter()
            .chain(other_domain_args.clone())
            .collect::<Vec<_>>();

        let domain_args_idents = other_domain_args.iter().filter_map(|a| match a {
            syn::FnArg::Typed(t) => match t.pat.as_ref() {
                syn::Pat::Ident(id) => Some(&id.ident),
                _ => {
                    warn!("Unsupported argument identity");
                    None
                }
            },
            _ => None,
        });

        let domain_ident = format_ident!("{}", domain_name);

        let domain_return_type = match &method.sig.output {
            syn::ReturnType::Type(arrow, return_type) => match return_type.as_ref() {
                syn::Type::Tuple(tuple_type) => {
                    let iter = tuple_type.elems.iter();
                    if iter.len() <= 1 {
                        panic!("Return Type doesn't have enough elements, minimum of two required");
                    }

                    let returned_types = iter.skip(1);
                    let return_type: syn::Type = parse_quote! {
                        Box<( #(#returned_types),* )>
                    };

                    return_type
                }
                _ => panic!("Domain must return a tuple!"),
            },
            syn::ReturnType::Default => panic!("Domain must return a tuple!"),
        };

        let main_rs: syn::File = parse_quote!(
            #![no_std]
            #![no_main]

            extern crate alloc;

            use alloc::boxed::Box;
            use console::println;

            use #domain_ident;

            #[no_mangle]
            pub fn trusted_entry(
                #(#domain_args),*
            ) -> #domain_return_type {
                #(#domain_components_init_statements)*

                #domain_ident::main(#(#domain_args_idents),*)
            }
        );

        return main_rs.to_token_stream().to_string();
    }

    pub fn write_entrypoint_crate(&self, crate_name: &str, cargo_toml: String, main_rs: String) {
        let output_path = self.output_folder_path.join(crate_name + "_entry_point");

        std::fs::create_dir(output_path);
        std::fs::write(output_path.join("Cargo.toml"), cargo_toml);

        std::fs::create_dir(output_path.join("src"));
        std::fs::write(output_path.join("src/main.rs"), main_rs);
    }

    pub fn generate_domain_entrypoint_crates(
        &self,
        domain_relative_path: &Path,
        domain_components: &Vec<DomainCreateComponent>,
        method: &TraitItemMethod,
    ) {
        let method_name = method.sig.ident.to_string();
        if !method_name.starts_with("create_domain_") {
            warn!(
                "Method name {:} does not start with 'create_domain_', skipping entrypoint generation",
                method_name
            );
            return;
        }

        let domain_name = method_name.trim_start_matches("create_domain_");

        let cargo_toml = self.generate_entrypoint_cargo(domain_relative_path, domain_name);
        debug!("CARGO TOML: {:}", cargo_toml);

        let main_rs = self.generate_entrypoint_main_rs(domain_name, domain_components, method);
        debug!("main.rs: {:}", main_rs);

        self.write_entrypoint_crate(domain_name, cargo_toml, main_rs);
        panic!("END");
    }
}
