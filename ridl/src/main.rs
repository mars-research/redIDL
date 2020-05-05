extern crate syn;
extern crate quote;

use std::env;
use std::path;
use std::fs;

fn open_subdir(root: &path::Path, subdir: &str) -> Option<fs::ReadDir> {
    let mut subpath = path::PathBuf::new();
    subpath.push(root);
    subpath.push(subdir);
    if let Result::Ok(dir) = fs::read_dir(&subpath) {
        Some(dir)
    }
    else {
        println!("Error: couldn't open {}", subpath.display());
        None
    }
}

fn create_subfile(root: &path::Path, subfile: &str) -> Option<fs::File> {
    let mut subpath = path::PathBuf::new();
    subpath.push(root);
    subpath.push(subfile);
    if let Result::Ok(dir) = fs::File::create(&subpath) {
        Some(dir)
    }
    else {
        println!("Error: couldn't open {}", subpath.display());
        None
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: ridl <redleaf-root>");
        return
    }

    let root = path::Path::new(&args[1]);
    let usr_idl = open_subdir(root, "sys/interfaces/usr/src/");
    let create_idl = open_subdir(root, "sys/interfaces/create/src/");
    let proxy_gen = create_subfile(root, "usr/proxy/src/_gen.rs");
    let create_gen = create_subfile(root, "src/_gen.rs");
    if usr_idl.is_none()
        || create_idl.is_none()
        || proxy_gen.is_none()
        || create_gen.is_none()
    {
        return
    }
}