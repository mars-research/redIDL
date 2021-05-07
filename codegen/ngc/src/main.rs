#![feature(option_expect_none, box_syntax, box_patterns, option_unwrap_none)]

mod domain_create;
mod proxy;
mod type_resolution;
#[macro_use]
mod utils;

#[macro_use]
extern crate derivative;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::process::Command;

use clap::{App, Arg, ArgMatches};
use log::{info, warn};
use quote::{format_ident, quote};
use syn::{parse_quote, Item, ItemMod, Meta, NestedMeta, Type};

fn main() {
    // Initialze logging
    env_logger::init();

    // Parse arguments and run the compiler
    let matches = App::new("Proxy Generator")
        .version(env!("CARGO_PKG_VERSION"))
        .about("RedIDL New Generation Compiler(NGC).")
        .arg(
            Arg::with_name("INPUT")
                .help("Path to the interface file.")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("Path to the output file.")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::with_name("domain_create_output")
                .value_name("domain_create_output")
                .long("domain_create_output")
                .help("Path to the domain create generation output.")
                .takes_value(true),
        )
        .get_matches();

    run(&matches).unwrap();
}

fn run(args: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let input_path = args.value_of("INPUT").unwrap();
    let output_path = args.value_of("OUTPUT").unwrap();
    info!("Running redIDL on {}", input_path);
    let mut file = File::open(&input_path).expect("Failed to open input file");
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();

    let mut ast = syn::parse_file(&content).unwrap();

    // Clean the file
    remove_prelude(&mut ast);

    // Generate code.
    let generated_domain_create = generate(&mut ast);

    // Write generated proxy.
    write_ast_to_file(&ast, output_path);

    // Write generated domain create.
    if let Some(domain_create_out) = args.value_of("domain_create_output") {
        let domain_create_ast: syn::File = parse_quote! {
            use interface::domain_create;
            use interface::proxy;
            use syscalls;
            use interface;

            use alloc::boxed::Box;
            use alloc::sync::Arc;

            use crate::domain::load_domain;
            use crate::heap::PHeap;
            use crate::interrupt::{disable_irq, enable_irq};
            use crate::thread;
            use syscalls::{Heap, Domain, Interrupt};
            use interface::{bdev::{BDev, NvmeBDev}, vfs::VFS, usrnet::UsrNet, rv6::Rv6, dom_a::DomA, dom_c::DomC, net::Net, pci::{PCI, PciBar, PciResource}};
            use interface::error::Result;
            use interface::tpm::UsrTpm;
            use interface::domain_create::*;

            #(#generated_domain_create)*
        };

        info!("Writting interface output to {}", domain_create_out);
        write_ast_to_file(&domain_create_ast, domain_create_out)
    }

    info!("Writting interface output to {}", output_path);
    let output = quote!(#ast).to_string();
    std::fs::write(&output_path, output).unwrap();

    // Format output file
    let _ = Command::new("bash")
        .arg("-c")
        .arg(format!("rustfmt {}", &output_path))
        .output();

    Ok(())
}

// Generate proxy and other stuff from `items` in place.
// Save Returns domain create generation.
fn generate(ast: &mut syn::File) -> Vec<syn::Item> {
    // Generate type id
    crate::type_resolution::generate_typeid(ast);

    // Generate proxy and domain creations.
    let mut module_path = vec![format_ident!("interface")];
    generate_recurse(&mut ast.items, &mut module_path)
}

// Generate proxy and other stuff from `items` in place, recursively.
// Returns domain create generation.
fn generate_recurse(
    items: &mut Vec<syn::Item>,
    module_path: &mut Vec<syn::Ident>,
) -> Vec<syn::Item> {
    let mut generated_items = Vec::<syn::Item>::new();
    let mut generated_domain_create_items = Vec::<syn::Item>::new();
    for item in items.iter_mut() {
        match item {
            Item::Mod(md) => {
                if let Some((_, items)) = &mut md.content {
                    // Recursive into the submodule.
                    module_path.push(md.ident.clone());
                    generated_domain_create_items.extend(generate_recurse(items, module_path));
                    module_path.pop();
                }
            }
            Item::Trait(tr) => {
                // Attempt to generate proxy
                if let Some(generated) = crate::proxy::generate_proxy(tr, module_path) {
                    generated_items.extend(generated);
                }

                // Attempt to generate domain creation
                if let Some(generated) =
                    crate::domain_create::generate_domain_create(tr, module_path)
                {
                    generated_domain_create_items.extend(generated);
                }
            }
            _ => {}
        }
    }

    // Insert the generated proxy inplace.
    items.extend(generated_items);

    // Return the generated domain create.
    generated_domain_create_items
}

/// Remove unwanted stuff generated by cargo-expand
fn remove_prelude(ast: &mut syn::File) {
    // Remove `#![feature(prelude_import)]`
    ast.attrs.retain(|attr| {
        if let Ok(Meta::List(meta)) = attr.parse_meta() {
            if !meta.path.is_ident("feature") {
                return true;
            }

            for meta in meta.nested {
                if let NestedMeta::Meta(meta) = meta {
                    if meta.path().is_ident("prelude_import") {
                        return false;
                    }
                }
            }
        }

        true
    });

    ast.items.retain(|item| {
        // Remove ```
        // #[prelude_import]
        // use core::prelude::v1::*;
        // ```
        const PRELUDE_IMPORT_ATTR: &str = "prelude_import";
        if let Item::Use(item) = item {
            if has_attribute!(item, PRELUDE_IMPORT_ATTR) {
                return false;
            }
        }

        // Remove
        // ```
        // #[macro_use]
        // extern crate compiler_builtins;
        // #[macro_use]
        // extern crate core;
        // ```
        if let Item::ExternCrate(item) = item {
            let ident = item.ident.to_string();
            if ident == "compiler_builtins" || ident == "core" {
                return false;
            }
        }

        true
    });
}

// Write ast to file and run formatter.
fn write_ast_to_file(ast: &syn::File, output_path: &str) {
    // Write output
    let output = quote!(#ast).to_string();
    std::fs::write(&output_path, output).expect("Failed to write output file");

    // Format output file
    if let Err(err) = Command::new("bash")
        .arg("-c")
        .arg(format!("rustfmt {}", &output_path))
        .output()
    {
        warn!(
            "Failed to run formatter on output file {}. Formatting is skipped. Error {}",
            output_path, err
        )
    }
}
