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

    fs::create_dir_all(&src_root).expect("[ERROR] Could not create crate root");
    let idl_dir = fs::read_dir(idl_root).expect("[ERROR] Could not open the IDL root");
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
        gen_path.push(name);
        gen_path.set_extension("rs");
        println!("{}", gen_path.to_str().expect("[ERROR] Could not convert path").to_string());
        let mut file = fs::File::create(&gen_path).expect("[ERROR] Could not open generated file");
        
        // And this is the part where analysis/generation happens
        let ast = get_ast(&idl);
        let generated = quote::quote!{#ast}.to_string();
        write!(file, "{}", generated).expect("[ERROR] Could not write to generated file");
    }

    let options = fs_extra::dir::CopyOptions {
        overwrite: false,
        skip_exist: true,
        buffer_size: 1024,
        copy_inside: false,
        depth: 0
    };
    fs_extra::dir::copy(red_idl_root, crate_root, &options).expect("[ERROR] Could not copy red_idl crate");
}