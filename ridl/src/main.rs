extern crate syn;
extern crate quote;
extern crate fs_extra;

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

// Checks if type is a functional trait (i.e., contains member functions only)
fn is_functional(item: &syn::Item) -> bool {
    if let syn::Item::Trait(tr) = item {
        for item in &tr.items {
            if let syn::TraitItem::Method(_) = item {
            } else {
                return false
            }
        }

        true
    } else {
        false
    }
}

fn get_ident(item: &syn::Item) -> String {
    match item {
        syn::Item::Trait(tr) => tr.ident.to_string(),
        _ => panic!()
    }
}

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
    
    let mut lpath = PathBuf::new();
    lpath.push(&gen_src_dir);
    lpath.push("lib.rs");
    let mut lfile = fs::File::create(&lpath).expect("[ERROR] Could not open lib.rs");
    
    for item in idl_dir {
        let entry = item.expect("[ERROR] Could not inspect item");
        let metadata = entry.metadata().expect("[ERROR] Could not get item metadata");
        if !metadata.is_file() {
            println!("[WARN] Only IDL stored in the IDL root will be processed");
            continue
        }
        
        let idl = entry.path();
        let name = idl.file_stem().expect("[ERROR] Anonymous IDL files not allowed");
        let mut file = open_generated(&gen_src_dir, name);
        
        writeln!(lfile, "pub mod {};", name.to_string_lossy()).expect("[ERROR] Could not write re-export for module");
        writeln!(file, "use crate::*;").expect("[ERROR] could not write import fixup");
        
        // And this is the part where analysis/generation happens

        let (ast, content) = get_idl(&idl);

        let mut fn_traits: Vec<String> = Vec::new();

        for item in &ast.items {
            let fnc = is_functional(item);
            if fnc {
                fn_traits.push(get_ident(item));
            }
            
            println!("[DEBUG] Item was functional trait: {}", fnc);
        }

        writeln!(file, "{}", content).expect("[ERROR] Could not write to generated file");

        for fnt in fn_traits {
            writeln!(file, "red_idl::declare_functional!({});", fnt).expect("[ERROR] Could not write to generated file");
        }
    }
    
    write_manifest(gen_root);
    copy_helper_crate(&gen_root);
}