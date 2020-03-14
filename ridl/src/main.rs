extern crate syn;

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

// panic!() may not be the best of ideas

// NOTE: Rust uses the leading "::" for global qualification

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

struct IDLFile {
    ast: syn::File,
    deps: Vec<usize>
}

fn find_deps(path: &Path, idl_store: &mut Vec<IDLFile>, seen: &mut Vec<PathBuf>) -> usize {
    match seen.iter().position(|x| x == path) {
        Some(id) => {
            println!("\tAlready saw IDL file at {:?}", seen[id]);
            return id
        },
        None => ()
    };

    let mut p = PathBuf::new();
    p.push(path);
    idl_store.push(IDLFile {ast: get_ast(path), deps: Vec::new()});
    seen.push(p);
    let id = idl_store.len() - 1;
    
    let mut imports : Vec<String> = Vec::new();
    {
        let idl = &idl_store[id];
        let ast = &idl.ast;
        let mut useds : Vec<syn::ItemUse> = Vec::new();
        for item in &ast.items {
            match item {
                syn::Item::Use(used) => {
                    println!("[INFO] Encountered use statement");
                    useds.push(used.clone())
                },
                syn::Item::Trait(_) => (),
                syn::Item::Struct(_) => (),
                _ => {
                    println!("[ERROR] IDL may only contain traits, structs, and use statements");
                    panic!()
                }
            }
        }

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
    }    

    let mut deps : Vec<usize> = Vec::new();
    if imports.len() > 0 {
        println!(
            "[INFO] Collecting ASTs for all imports of: {}",
            path.to_str().expect("[ERROR] Could not convert path to string"));
        
        for imp in imports {
            let mut buf = PathBuf::new();
            buf.push(path.parent().unwrap_or(Path::new("")));
            buf.push(imp + ".idl");
            println!("\t{}", buf.to_str().expect("Could not convert from path to str"));
            deps.push(find_deps(buf.as_path(), idl_store, seen))
        }
    }

    idl_store[id].deps = deps;

    return idl_store.len() - 1;
}

fn build_sym_table(tree: usize, idls_store: &Vec<IDLFile>, table: &mut Vec<String>) {
    let idl = &idls_store[tree];
    for dep in &idl.deps {
        build_sym_table(*dep, idls_store, table);
    }
    for item in &idl.ast.items {
        match item {
            syn::Item::Trait(tr) => table.push(tr.ident.to_string()),
            syn::Item::Struct(st) => table.push(st.ident.to_string()),
            syn::Item::Use(_) => (),
            _ => panic!()
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: <invocation> <filepath>");
        return ()
    }
    let mut table : Vec<String> = Vec::new();
    let mut idl_store : Vec<IDLFile> = Vec::new();
    let mut seen : Vec<PathBuf> = Vec::new();
    let tree = find_deps(Path::new(&args[1]), &mut idl_store, &mut seen);
    // TODO: figure out how type checking is going to work in a dependency graph that itself has cycles
    build_sym_table(tree, &idl_store, &mut table);
    println!("Types found in tree: {:?}", table)
}