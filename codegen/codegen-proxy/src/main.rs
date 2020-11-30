use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;

use syn::{Item, Stmt, Meta, NestedMeta, parse_quote};
use quote::quote;

fn main() {
    let filename: String = env::args().skip(1).next().unwrap();
    println!("{:?}", filename);
    let file = run(&filename).unwrap();
    let output = quote!(#file).to_string();
    std::fs::write("out.rs", output).unwrap();
}



fn run(filename: &str) -> Result<syn::File, Box<dyn Error>> {
    let mut file = File::open(filename)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let mut ast = syn::parse_file(&content)?;

    // Remove prelude stuff
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


    // Recursively inject import statements in each 
    for item in ast.items.iter_mut() {
        inject_import_recursive(item)?
    }


    Ok(ast)
}

fn inject_import_recursive(item: &mut Item) -> Result<(), Box<dyn Error>> {
    match item {
        Item::Mod(md) => {
            if let Some((_, content)) = &mut md.content {
                for item in content.iter_mut() {
                    inject_import_recursive(item)?;
                }

                let injected_import_statements: Vec<Stmt> =  parse_quote! {
                    use codegen_proc::generate_proxy as interface;
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

