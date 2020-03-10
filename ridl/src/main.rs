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
    deps: Vec<IDLFile>
}

fn find_deps(path: &Path) -> IDLFile {
    let ast = get_ast(path);
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

    let mut deps : Vec<IDLFile> = Vec::new();
    if imports.len() > 0 {
        println!("[INFO] Collecting ASTs for all imports of: {}", path.to_str().expect("[ERROR] Could not convert path to string"));
        for imp in imports {
            let mut buf = PathBuf::new();
            buf.push(path.parent().unwrap_or(Path::new("")));
            buf.push(imp + ".idl");
            println!("\t{}", buf.to_str().expect("Could not convert from path to str"));
            deps.push(find_deps(buf.as_path()))
        }
    }

    // Once all imports have been "analyzed" (this could also involve code generation), analyze this file
    // This is to allow for the whole type-checking thing
    // Or we could split the functionality and have a recursive function build a tree of IDL ASTs
    // Which we then traverse for analysis
    // And again for generation (generation may need to have identifier table information for fully qualified names)

    return IDLFile {ast, deps};
}

fn build_sym_table(tree: &IDLFile, table: &mut Vec<String>) {
    for dep in &tree.deps {
        build_sym_table(&dep, table);
    }
    for item in &tree.ast.items {
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
    let tree = find_deps(Path::new(&args[1]));
    build_sym_table(&tree, &mut table);
    println!("Types found in tree: {:?}", table)
}