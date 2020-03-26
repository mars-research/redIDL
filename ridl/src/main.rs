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

fn get_ast(path: &Path) -> syn::File {
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
    
    return match syn::parse_file(&content) {
        Err(e) => {
            println!("[ERROR] Failed to parse because: {}", e);
            panic!()
        },
        Ok(v) => v
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        println!("[INFO] Usage: ridl <idl-root> <crate-root> <helper-root>");
        return ()
    }

    let idl_root = Path::new(&args[1]);
    let crate_root = Path::new(&args[2]);
    let mut src_root = PathBuf::new();
    src_root.push(crate_root);
    src_root.push("src");
    let red_idl_root = Path::new(&args[3]);

    // TODO: Until we have better delta handling
    let _ = fs::remove_dir_all(&crate_root);

    fs::create_dir_all(&src_root).expect("[ERROR] Could not create crate root");
    let idl_dir = fs::read_dir(idl_root).expect("[ERROR] Could not open the IDL root");

    let mut lpath = PathBuf::new();
    lpath.push(&src_root);
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
        let mut gen_path = PathBuf::new();
        gen_path.push(&src_root);
        gen_path.push(&name);
        gen_path.set_extension("rs");
        println!("{}", gen_path.to_str().expect("[ERROR] Could not convert path").to_string());
        let mut file = fs::File::create(&gen_path).expect("[ERROR] Could not open generated file");
        
        writeln!(lfile, "pub mod {};", name.to_string_lossy()).expect("[ERROR] Could not write re-export for module");
        writeln!(file, "use crate::*;").expect("[ERROR] could not write import fixup");

        // And this is the part where analysis/generation happens
        let ast = get_ast(&idl);
        let generated = quote::quote!{#ast}.to_string();
        writeln!(file, "{}", generated).expect("[ERROR] Could not write to generated file");
    }

    let cargo = format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2018\"\n\n[dependencies]\nred_idl = {{ path = \"red_idl\" }}\n",
        crate_root.file_name().expect("[ERROR] Crate has no root").to_string_lossy());
    let mut cpath = PathBuf::new();
    cpath.push(crate_root);
    cpath.push("Cargo.toml");
    let mut cfile = fs::File::create(&cpath).expect("[ERROR] Could not open Cargo.toml");
    write!(cfile, "{}", cargo).expect("[ERROR] Could not write Cargo.toml");

    let options = fs_extra::dir::CopyOptions {
        overwrite: false,
        skip_exist: true,
        buffer_size: 1024,
        copy_inside: false,
        depth: 0
    };
    fs_extra::dir::copy(red_idl_root, crate_root, &options).expect("[ERROR] Could not copy red_idl crate");
}