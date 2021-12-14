use crate::domain_create::DomainCreateComponent;
use std::{
    fs::{canonicalize, create_dir, remove_dir_all, File},
    path::{Path, PathBuf},
};

use log::{debug, error, info, warn};
use quote::ToTokens;
use syn::TraitItemMethod;

pub struct DomainEntrypointFactory {
    output_folder_created: bool,

    domains_folder: PathBuf,
}

impl DomainEntrypointFactory {
    pub fn new(domains_folder: PathBuf) -> Self {
        Self::setup_output_folder(&domains_folder.join("generated"));

        DomainEntrypointFactory {
            output_folder_created: true,
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

    pub fn generate_domain_entrypoint_crates(
        &self,
        domain_relative_path: &Path,
        domain_components: &Vec<DomainCreateComponent>,
        method: &TraitItemMethod,
    ) {
        // debug!(
        //     "domain_relative_path: {:#?}, domain_components: {:#?}",
        //     domain_relative_path, domain_components,
        // );

        // debug!("method name: {:#?}", method.sig.ident.to_string());

        // debug!(
        //     "arguments: {:#?}",
        //     method
        //         .sig
        //         .inputs
        //         .iter()
        //         .filter_map(|arg| match arg {
        //             syn::FnArg::Typed(a) => Some(a.to_token_stream().to_string()),
        //             _ => None,
        //         })
        //         .collect::<Vec<_>>()
        // );

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
    }
}
