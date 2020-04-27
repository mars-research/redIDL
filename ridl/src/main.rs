extern crate syn;
extern crate quote;
extern crate fs_extra;

mod types;

use std::env;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

// panic!() may not be the best of ideas

// NOTE: Rust uses the leading "::" for global qualification

fn get_idl(path: &Path) -> (syn::File, String) {
    let mut file = match fs::File::open(path) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            panic!()
        }
    };

    let mut content = String::new();
    file.read_to_string(&mut content)
        .expect(&(
            "[ERROR] Failed to open file: ".to_string()
            + path.to_str().expect("[ERROR] Failed to convert filename string")));
    
    let parsed = match syn::parse_file(&content) {
        Err(e) => {
            println!("[ERROR] Failed to parse because: {}", e);
            panic!()
        },
        Ok(v) => v
    };

    (parsed, content)
}

fn copy_helper_crate(gen_root: &Path) {
    let options = fs_extra::dir::CopyOptions {
        overwrite: true,
        skip_exist: false,
        buffer_size: 1024,
        copy_inside: false,
        depth: 0
    };

    println!("[INFO] Copying helper crate");
    fs_extra::dir::copy(Path::new("red_idl"), gen_root, &options)
        .expect("[ERROR] Could not copy helper crate");
}

fn write_manifest(gen_root: &Path) {
    let cargo = format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2018\"\n\n\
            [dependencies]\nred_idl = {{ path = \"red_idl\" }}\n",
        gen_root.file_name().expect("[ERROR] Crate has no root").to_string_lossy());

    let mut cpath = PathBuf::new();
    cpath.push(gen_root);
    cpath.push("Cargo.toml");

    let mut cfile = fs::File::create(&cpath).expect("[ERROR] Could not open Cargo.toml");
    write!(cfile, "{}", cargo).expect("[ERROR] Could not write Cargo.toml");
}

fn open_generated(gen_src_dir: &Path, name: &std::ffi::OsStr) -> fs::File {
    let mut gen_path = PathBuf::new();
    gen_path.push(&gen_src_dir);
    gen_path.push(&name);
    gen_path.set_extension("rs");
    println!(
        "[INFO] Opening {}",
        gen_path
            .to_str()
            .expect("[ERROR] Could not convert path to readable string")
            .to_string());
    
    fs::File::create(&gen_path).expect("[ERROR] Could not open generated file")
}

// Reject things like enums, bare functions, constants, etc. Anything that isn't a
// using, a struct, or a trait. These are the "unused" parts of Rust in our IDL subset
fn reject_unsupported(item: &syn::Item) -> bool {
    match item {
        syn::Item::Struct(_) => false,
        syn::Item::Trait(_) => false,
        syn::Item::Use(_) => false,
        _ => true
    }
}

// Rejects any module-level identifier of RRef<> or OptRRef<> (TODO: is this enough?)
fn reject_reserved(item: &syn::Item) -> bool {
    match item {
        syn::Item::Struct(st) => {
            let name = st.ident.to_string();
            name == "RRef" || name == "OptRRef"
        },
        syn::Item::Trait(tr) => {
            let name = tr.ident.to_string();
            name == "RRef" || name == "OptRRef"
        },
        _ => false
    }
}

/*
    Another type system revision!
    Note that no IDL type may exist outside of this
    Introducing SafeCopy -
        - Is Copy (so we can bitwise copy)
        - Does not have references or pointers of any kind (so we know that we can copy it out of a domain,
            and it won't reference anything in that domain)
        - Is a struct (for now)
    
    Introducing the *new* RRefable -
        - Extends SafeCopy, allowing OptRRef<> members
    
    Functional remains the same
*/

// TODO: Don't just panic!() on bad IDL
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("[INFO] Usage: ridl <idl-root> <crate-root>");
        return ()
    }

    let idl_root = Path::new(&args[1]);
    let gen_root = Path::new(&args[2]);
    let mut gen_src_dir = PathBuf::new();
    gen_src_dir.push(gen_root);
    gen_src_dir.push("src");
    
    // TODO: Until we have better delta handling
    let _ = fs::remove_dir_all(&gen_root);
    
    fs::create_dir_all(&gen_src_dir).expect("[ERROR] Could not create crate root");
    let idl_dir = fs::read_dir(idl_root).expect("[ERROR] Could not open the IDL root");
    
    let mut lib_path = PathBuf::new();
    lib_path.push(&gen_src_dir);
    lib_path.push("lib.rs");
    let mut lib_file = fs::File::create(&lib_path).expect("[ERROR] Could not open lib.rs");
    
    for item in idl_dir {
        let entry = item.expect("[ERROR] Could not inspect item");
        let metadata = entry.metadata().expect("[ERROR] Could not get item metadata");
        if !metadata.is_file() {
            println!("[WARN] Only IDL stored in the IDL root will be processed");
            continue
        }
        
        let idl_path = entry.path();
        let idl_name = idl_path.file_stem().expect("[ERROR] Anonymous IDL files not allowed");
        let mut gen_file = open_generated(&gen_src_dir, idl_name);
        
        // Standard preamble

        writeln!(lib_file, "pub mod {};", idl_name.to_string_lossy()).expect("[ERROR] Could not write re-export for module");
        writeln!(gen_file, "use crate::*;").expect("[ERROR] could not write import fixup");
        writeln!(gen_file, "use red_idl::*;").expect("[ERROR] could not write helper fixup");
        
        // And this is the part where analysis/generation happens

        let (ast, content) = get_idl(&idl_path);
        let mut type_decls = types::TypeSystemDecls::new();

        for item in &ast.items {
            if reject_unsupported(item) {
                println!("[ERROR] Not a recognized IDL syntax");
                return
            }

            if reject_reserved(item) {
                println!("[ERROR] RRef and OptRRef are reserved for IDL use");
                return
            }

            if !type_decls.classify(item) {
                println!("[ERROR] This is an invalid type");
                return
            }
        }

        writeln!(gen_file, "{}\n", content).expect("[ERROR] Could not copy required definitions to generated file");
        type_decls.write_decls(&mut gen_file);
    }
    
    write_manifest(gen_root);
    copy_helper_crate(&gen_root);
}