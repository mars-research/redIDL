#![feature(box_syntax, box_patterns)]

mod domain_create;
mod path_refactoring;
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
use domain_create::DomainCreateBuilder;
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
    remove_prelude_and_placeholder(&mut ast);

    // Generate code.
    let generated_domain_create = generate(&mut ast);

    // Write generated proxy.
    write_ast_to_file(&ast, output_path);

    // Write generated domain create.
    if let Some(domain_create_out) = args.value_of("domain_create_output") {
        let domain_create_ast: syn::File = parse_quote! {
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
    let mut domain_create_builder = DomainCreateBuilder::new();
    let mut generated_domain_create_items =
        generate_recurse(&mut ast.items, &mut domain_create_builder, &mut module_path);

    // Generate create_init and add it to generated domain creates.
    generated_domain_create_items.push(domain_create_builder.generate_create_init());

    // Finds the Generates the proxy struct inplace
    let proxy_mod = ast
        .items
        .iter_mut()
        .find_map(|item| match item {
            Item::Mod(md) => {
                if md.ident == "proxy" {
                    Some(md)
                } else {
                    None
                }
            }
            _ => None,
        })
        .unwrap();
    let (_, items) = proxy_mod.content.as_mut().unwrap();
    items.extend(proxy::generate_proxy(domain_create_builder.take()));

    // Return the generated domain creates.
    generated_domain_create_items
}

// Generate proxy and other stuff from `items` in place, recursively.
// Returns domain create generation.
fn generate_recurse(
    items: &mut Vec<syn::Item>,
    domain_create_builder: &mut DomainCreateBuilder,
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
                    generated_domain_create_items.extend(generate_recurse(
                        items,
                        domain_create_builder,
                        module_path,
                    ));
                    module_path.pop();
                }
            }
            Item::Trait(tr) => {
                // Attempt to generate proxy
                if let Some(generated) = crate::proxy::generate_interface_proxy(tr, module_path) {
                    generated_items.extend(generated);
                }

                // Attempt to generate domain creation
                if let Some(generated) =
                    domain_create_builder.generate_domain_create(tr, module_path)
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
fn remove_prelude_and_placeholder(ast: &mut syn::File) {
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
        // #[macro_use]
        // extern crate interface_attribute_placeholder;
        // ```
        if let Item::ExternCrate(item) = item {
            let ident = item.ident.to_string();
            if ident == "compiler_builtins"
                || ident == "core"
                || ident == "interface_attribute_placeholder"
            {
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
