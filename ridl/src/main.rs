extern crate syn;
extern crate quote;

use std::env;
use std::path;
use std::fs;

#[macro_use]
pub mod error;
mod verify;

use error::Result;

fn open_subdir(root: &path::Path, subdir: &str) -> Result<fs::ReadDir> {
    let mut subpath = path::PathBuf::new();
    subpath.push(root);
    subpath.push(subdir);
    Ok(try_with_msg!(
        fs::read_dir(subpath),
        "could not open directory \"{}\"",
        subdir)?)
}

fn create_subfile(root: &path::Path, subfile: &str) -> Result<fs::File> {
    let mut subpath = path::PathBuf::new();
    subpath.push(root);
    subpath.push(subfile);
    Ok(try_with_msg!(
        fs::File::create(&subpath),
        "could not create file \"{}\"",
        subpath.display())?)
}

fn walk_idl_files(idl_root: fs::ReadDir) -> Result<()> {
    for entry in idl_root {
        let entry = entry.expect("could not read item in IDL dir");
        let path = entry.path();
        let dpath = path.display();
        let meta = entry.metadata().expect("could not read entry metadata");
        if meta.is_dir() {
            walk_idl_files(fs::read_dir(entry.path())?)?;
            continue;
        }

        let src = try_with_msg!(
            fs::read_to_string(entry.path()),
            "could not read file \"{}\"",
            dpath)?;

        let ast = try_with_msg!(
            syn::parse_file(&src),
            "parsing failed for file \"{}\"",
            dpath)?;

        // Verify IDL contents
        try_with_msg!(
            verify::verify_file(&ast),
            "\"{}\" failed verification",
            dpath)?;
        // Collect information for generation
    }

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: ridl <redleaf-root>");
        return Ok(())
    }

    println!("Info: \"Cause: Compiler Error\" means a syntax problem");

    let root = path::Path::new(&args[1]);
    let idl_root = open_subdir(root, "sys/interfaces/usr/src/")?;
    let _create_root = open_subdir(root, "sys/interfaces/create/src/")?;
    let _proxy_gen = create_subfile(root, "usr/proxy/src/_gen.rs")?;
    let _create_gen = create_subfile(root, "src/_gen.rs")?;
    walk_idl_files(idl_root)?;

    Ok(())
}