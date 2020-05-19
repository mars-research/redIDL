use std::path;
use crate as ridl;
use std::io::Write;
use fs_extra::dir;
use std::fs;

use crate::error::Result;
use crate::verify;

pub fn gen_usr_crate(root: &path::Path) -> Result<()> {
    let usr_root = ridl::get_subpath(root, "sys/interfaces/usr/");
    let usr_gen_root = ridl::get_subpath(root, "sys/interfaces/_usr/");
    let usr_manifest = ridl::get_subpath(&usr_gen_root, "Cargo.toml");
    let idl_root = ridl::open_subdir(root, "sys/interfaces/usr/src/")?;
    let gen_idl_root = ridl::get_subpath(root, "sys/interfaces/_usr/src/");

    if usr_gen_root.exists() {
        try_with_msg!(
            fs::remove_dir_all(&usr_gen_root),
            "couldn't reset generated crate")?;
    }

    let mut options = dir::CopyOptions::new();
    options.copy_inside = true;
    dir::copy(usr_root, usr_gen_root, &options)?;
    let mut cargo = fs::OpenOptions::new().write(true).append(true).open(usr_manifest)?;
    writeln!(cargo, "red_idl = {{ path = \"../../../redIDL/red_idl\" }}")?;
    walk_idl_files(idl_root, &gen_idl_root)?;

    Ok(())
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

        let mut dst_file = fs::File::create(subpath)?;
        writeln!(dst_file, "{}\nextern crate red_idl;\n", src)?;

        macros.write(&mut dst_file)?;

        // Collect information for generation
    }

    Ok(())
}