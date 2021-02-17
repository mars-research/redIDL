mod proxy;
mod utils;
mod domain_creation;
mod type_resolution;


use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::process::Command;

use clap::{App, Arg, ArgMatches};
use quote::quote;
use syn::{Item, Meta, NestedMeta};

fn main() {
    let matches = App::new("Proxy Generator")
        .version(env!("CARGO_PKG_VERSION"))
        .about("RedIDL New Generation Codegenerator.")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file.")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("Sets the output file.")
                .required(true)
                .index(2),
        )
        .get_matches();

    run(&matches).unwrap();
}

fn run(args: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let input_path = args.value_of("INPUT").unwrap();
    let output_path = args.value_of("OUTPUT").unwrap();
    let mut file = File::open(&input_path).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();

    let mut ast = syn::parse_file(&content).unwrap();

    // Clean the file
    remove_prelude(&mut ast);

    // Find all `RRef`ed types
    let mut resolver = type_resolution::type_resolver::TypeSolver::new();
    let types = resolver.resolve_types(&ast);
    panic!("{:#?}", types);
    
    // Generate code in place
    generate(&mut ast);

    // Write output
    let output = quote!(#ast).to_string();
    std::fs::write(&output_path, output).unwrap();

    // Format output file
    let _ = Command::new("bash")
        .arg("-c")
        .arg(format!("rustfmt {}", &output_path))
        .output();

    Ok(())
}

fn generate(ast: &mut syn::File) {
    let mut module_path = Vec::<syn::Ident>::new();
    generate_recurse(&mut ast.items, &mut module_path)
}

fn generate_recurse(items: &mut Vec<syn::Item>, module_path: &mut Vec<syn::Ident>) {
    let mut generated_items = Vec::<syn::Item>::new();
    for item in items.iter_mut() {
        match item {
            Item::Mod(md) => {
                if let Some((_, items)) = &mut md.content {
                    module_path.push(md.ident.clone());
                    generate_recurse(items, module_path);
                    module_path.pop();
                }
            }
            Item::Trait(tr) => {
                // Attempt to generate proxy
                if let Some(generated) = crate::proxy::generate_proxy(tr, module_path) {
                    generated_items.extend(generated);
                }

                // Attempt to generate domain creation
                if let Some(generated) = crate::domain_creation::generate_domain_creation(tr, module_path) {
                    generated_items.extend(generated);
                }
            }
            _ => {},
        }
    }

    items.extend(generated_items);
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
        const PRELUDE_IMPORT_ATTR: &'static str = "prelude_import";
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
