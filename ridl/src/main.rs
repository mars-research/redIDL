extern crate syn;
extern crate quote;
extern crate fs_extra;

use std::env;
use std::path;
use std::fs;
use std::io::Write;

use fs_extra::dir;

#[macro_use]
pub mod error;
mod verify;

use error::Result;

fn open_subdir(root: &path::Path, subdir: &str) -> Result<fs::ReadDir> {
    let subpath = get_subpath(root, subdir);
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

fn get_subpath(root: &path::Path, subdir: &str) -> path::PathBuf {
    let mut subpath = path::PathBuf::new();
    subpath.push(root);
    subpath.push(subdir);
    subpath
}

fn walk_idl_files(idl_root: fs::ReadDir, gen_root: &path::Path) -> Result<()> {
    for entry in idl_root {
        let entry = entry.expect("could not read item in IDL dir");
        let path = entry.path();
        let dpath = path.display();
        let meta = entry.metadata().expect("could not read entry metadata");
        let mut subpath = path::PathBuf::new();
        subpath.push(gen_root);
        subpath.push(entry.file_name());
        if meta.is_dir() {
            walk_idl_files(fs::read_dir(entry.path())?, &subpath)?;
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
        let macros = try_with_msg!(
            verify::verify_file(&ast),
            "\"{}\" failed verification",
            dpath)?;

        println!("Info: attempting to create \"{}\"", subpath.display());
        let mut dst_file = fs::File::create(subpath)?;
        // TODO: add real RRef type here
        writeln!(dst_file, "{}\nextern crate red_idl;\
            red_idl::assert_type_eq_all!(RRef, rref::RRef);\n\
            red_idl::assert_type_eq_all!(OptRRef, red_idl::OptRRef);", src)?;

        macros.write(&mut dst_file)?;

        // Collect information for generation
    }

    Ok(())
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

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: ridl <redleaf-root>");
        return Ok(())
    }

    println!("Info: \"Cause: Compiler Error\" means a syntax problem");

    let root = path::Path::new(&args[1]);
    let usr_root = get_subpath(root, "sys/interfaces/usr/");
    let usr_gen_root = get_subpath(root, "sys/interfaces/_usr/");
    let usr_manifest = get_subpath(&usr_gen_root, "Cargo.toml");
    let idl_root = open_subdir(root, "sys/interfaces/usr/src/")?;
    let gen_idl_root = get_subpath(root, "sys/interfaces/_usr/src/");
    let _create_root = open_subdir(root, "sys/interfaces/create/src/")?;
    let _gen_create_root = get_subpath(root, "sys/interfaces/_create/src/");
    let _proxy_gen = create_subfile(root, "usr/proxy/src/_gen.rs")?;
    let _create_gen = create_subfile(root, "src/_gen.rs")?;

    try_with_msg!(
        fs::remove_dir_all(&usr_gen_root),
        "couldn't reset generated crate")?;

    let mut options = dir::CopyOptions::new();
    options.copy_inside = true;
    dir::copy(usr_root, usr_gen_root, &options)?;
    let mut cargo = fs::OpenOptions::new().write(true).append(true).open(usr_manifest)?;
    writeln!(cargo, "red_idl = {{ path = \"../../../ridl/red_idl\" }}")?;
    walk_idl_files(idl_root, &gen_idl_root)?;

    Ok(())
}