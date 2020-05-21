extern crate syn;
extern crate quote;
extern crate fs_extra;

use std::env;
use std::path;
use std::fs;

#[macro_use]
pub mod error;
mod verify;
mod usr_gen;
mod proxies;
mod creation_proxies;

use error::Result;

pub fn open_subdir(root: &path::Path, subdir: &str) -> Result<fs::ReadDir> {
    let subpath = get_subpath(root, subdir);
    Ok(try_with_msg!(
        fs::read_dir(subpath),
        "could not open directory \"{}\"",
        subdir)?)
}

pub fn create_subfile(root: &path::Path, subfile: &str) -> Result<fs::File> {
    let mut subpath = path::PathBuf::new();
    subpath.push(root);
    subpath.push(subfile);
    Ok(try_with_msg!(
        fs::File::create(&subpath),
        "could not create file \"{}\"",
        subpath.display())?)
}

pub fn get_subpath(root: &path::Path, subdir: &str) -> path::PathBuf {
    let mut subpath = path::PathBuf::new();
    subpath.push(root);
    subpath.push(subdir);
    subpath
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: ridl <redleaf-root>");
        return Ok(())
    }

    println!("Warning: All trait and signature checks are disabled");
    println!("Info: \"Cause: Compiler Error\" means a syntax problem");

    let root = path::Path::new(&args[1]);
    usr_gen::gen_usr_crate(root)?;
    creation_proxies::generate(root)?;
    proxies::generate(root)?;

    Ok(())
}