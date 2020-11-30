use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;

use syn::{Item, Stmt, parse_quote};
use quote::{quote, format_ident};

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
    // let mut output: Vec<Item> = vec![];

    for item in ast.items.iter_mut() {
        run_recursive(item)?
    }


    Ok(ast)
}

fn run_recursive(item: &mut Item) -> Result<(), Box<dyn Error>> {
    match item {
        Item::Mod(md) => {
            if let Some((_, content)) = &mut md.content {
                for item in content.iter_mut() {
                    run_recursive(item)?;
                }

                let inject_import_statements: Vec<Stmt> =  parse_quote! {
                    use codegen_proc::generate_proxy as interface;
                    use unwind::trampoline;
                    use libsyscalls::syscalls::{sys_get_current_domain_id, sys_update_current_domain_id, sys_discard_cont};
                };
    
                let mut inject_import_statements: Vec<Item> = inject_import_statements
                                                    .into_iter()
                                                    .map(|stmt| {
                                                        match stmt {
                                                            Stmt::Item(item) => item,
                                                            _ => unreachable!(),
                                                        }
                                                    })
                                                    .collect();
                
                // Prepend the injected statments to the current statements
                inject_import_statements.append(content);
                *content = inject_import_statements;
            }

            Ok(())
        },
        _ => Ok(()),
    }

}

