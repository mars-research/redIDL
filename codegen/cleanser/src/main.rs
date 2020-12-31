use std::env;
use std::error::Error;
use std::env::VarError;
use std::fs::File;
use std::io::Read;

use clap::{Arg, App, ArgMatches};
use syn::{Item, Stmt, Meta, NestedMeta, parse_quote};
use quote::quote;

fn main() {
    let matches = App::new("Proxy Generator")
                            .version(env!("CARGO_PKG_VERSION"))
                            .about("Generate proxy")
                            .arg(Arg::with_name("INPUT")
                                .help("Sets the input file to use.")
                                .required(true)
                                .index(1))
                            .arg(Arg::with_name("OUTPUT")
                                .help("Sets the output file to use.")
                                .required(true)
                                .index(2))
                            .arg(Arg::with_name("remove-prelude")
                                .long("remove-prelude")
                                .help("Remove prelude if set."))
                            .arg(Arg::with_name("inject-dependency")
                                .long("inject-dependency")
                                .help("Inject dependency if set."))
                            .get_matches();


    run(matches).unwrap();
}



fn run(args: ArgMatches) -> Result<(), Box<dyn Error>> {
    let mut file = File::open(args.value_of("INPUT").ok_or(VarError::NotPresent)?)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let mut ast = syn::parse_file(&content)?;

    // Remove prelude stuff
    if args.is_present("remove-prelude") {
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
        // Remove ```
        // #[prelude_import]
        // use core::prelude::v1::*;
        // ```
        ast.items.retain(|item| {
            if let Item::Use(item) = item {
                for attr in &item.attrs {
                    if let Ok(meta) = attr.parse_meta() {
                        if meta.path().is_ident("prelude_import") {
                            return false;
                        }
                    } 
                }
            }
    
            true
        });
    }

    // Inject dependencies
    if args.is_present("inject-dependency") {
        // Inject required features
        ast.attrs.push(
            parse_quote!(#![feature(global_asm, type_ascription, core_intrinsics, fmt_internals, derive_clone_copy, structural_match, rustc_private, derive_eq, extended_key_value_attributes)])
        );
    
    
        // Recursively inject import statements in each module
        for item in ast.items.iter_mut() {
            inject_import_recursive(item)?
        }
    }


    // Write output
    let output = quote!(#ast).to_string();
    std::fs::write(&args.value_of("OUTPUT").ok_or(VarError::NotPresent)?, output)?;

    Ok(())
}

// Recursively inject import statements in each module
fn inject_import_recursive(item: &mut Item) -> Result<(), Box<dyn Error>> {
    match item {
        Item::Mod(md) => {
            if let Some((_, content)) = &mut md.content {
                for item in content.iter_mut() {
                    inject_import_recursive(item)?;
                }

                let injected_import_statements: Vec<Stmt> =  parse_quote! {
                    use codegen_proc::redidl_resolve_module_and_generate_proxy as interface;
                    use codegen_proc::redidl_generate_proxy;
                    use unwind::trampoline;
                    use libsyscalls::syscalls::{sys_get_current_domain_id, sys_update_current_domain_id, sys_discard_cont};
                };
    
                let mut injected_import_statements: Vec<Item> = injected_import_statements
                                                    .into_iter()
                                                    .map(|stmt| {
                                                        match stmt {
                                                            Stmt::Item(item) => item,
                                                            _ => unreachable!(),
                                                        }
                                                    })
                                                    .collect();
                
                // Prepend the injected statments to the current statements
                injected_import_statements.append(content);
                *content = injected_import_statements;
            }
        },
        _ => {},
    };

    Ok(())
}
