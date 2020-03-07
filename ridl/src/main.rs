extern crate syn;

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

// panic!() may not be the best of ideas

fn get_ast(path: &Path) -> syn::File {
    let mut file = match File::open(path) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            panic!()
        }
    };
    let mut content = String::new();
    file.read_to_string(&mut content).expect(&("[ERROR] Failed to open file: ".to_string() + path.to_str().expect("[ERROR] Failed to convert filename string")));
    return match syn::parse_file(&content) {
        Err(e) => {
            println!("[ERROR] Failed to parse because: {}", e);
            panic!()
        },
        Ok(v) => v
    }
}

fn analyze(path: &Path) {
    let ast = get_ast(path);
    let mut useds : Vec<syn::ItemUse> = Vec::new();
    for item in ast.items {
        match item {
            syn::Item::Use(used) => {
                println!("[INFO] Encountered use statement");
                useds.push(used)
            },
            syn::Item::Trait(tr) => (),
            syn::Item::Struct(st) => (),
            _ => {
                println!("[ERROR] IDL may only contain traits, structs, and use statements");
                panic!()
            }
        }
    }

    let mut imports : Vec<String> = Vec::new();
    for used in useds {
        match used.tree {
            syn::UseTree::Name(name) => {
                println!("[INFO] Used {}", name.ident);
                imports.push(name.ident.to_string())
            },
            _ => {
                println!("[ERROR] Only \"use <name>;\" is supported for use statements");
                panic!()
            }
        }
    }

    if imports.len() > 0 {
        println!("[INFO] Collecting ASTs for all imports of: {}", path.to_str().expect("[ERROR] Could not convert path to string"));
        for imp in imports {
            let mut buf = PathBuf::new();
            buf.push(path.parent().unwrap_or(Path::new("")));
            buf.push(imp + ".idl");
            analyze(buf.as_path())
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: <invocation> <filepath>");
        return ()
    }
    analyze(Path::new(&args[1]))
}