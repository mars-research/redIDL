use crate::domain_create::DomainCreateComponent;
use std::{
    fs::{canonicalize, create_dir, remove_dir_all, File},
    path::{Path, PathBuf},
};

use log::{debug, error, info};
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

    pub fn generate_domain_entrypoint_crates(
        &self,
        domain_relative_path: &Path,
        domain_components: &Vec<DomainCreateComponent>,
        method: &TraitItemMethod,
    ) {
        debug!(
            "domain_relative_path: {:#?}, domain_components: {:#?}",
            domain_relative_path, domain_components,
        );

        debug!(
            "arguments: {:#?}",
            method
                .sig
                .inputs
                .iter()
                .filter_map(|arg| match arg {
                    syn::FnArg::Typed(a) => Some(a.to_token_stream().to_string()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        );
    }
}
