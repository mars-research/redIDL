use std::fs::{read_dir, read_to_string};
use std::path::{Path, PathBuf};
use syn::{parse_file, File};

/*
	The general strategy here is to produce a tree of modules, mirroring the on-disk format
	This is the skeletal backbone of our analysis tree, and after inline modules are added into it,
	it constitutes a complete map of all modules in the IDL files. It is this information that is vital
	for processing the semantics of use statements, and the eventual resolution of paths to type definitions.

	It is *impossible* to resolve our speculative type checks before paths are fully resolved.
*/

pub struct Module {
	pub name: String, // TODO: does Ident to string heap optimization?
	raw_ast: File,
	pub submodules: Vec<Module>, // Will be extended as ModuleDef nodes are processed
}

struct DirChildren {
	rs_files: Vec<PathBuf>,
	dirs: Vec<PathBuf>,
}

fn get_filename(path: &Path) -> String {
	let fname = path.file_name().expect("path did not have a filename");
	let fname_str = fname.to_str().expect("filename was not valid unicode");
	fname_str.to_string()
}

pub fn get_file_stem(path: &Path) -> String {
	let fname = path.file_stem().expect("path did not have a filename");
	let fname_str = fname.to_str().expect("file stem was not valid unicode");
	fname_str.to_string()
}

fn enumerate_children(path: &Path) -> DirChildren {
	let mut files = Vec::<PathBuf>::new();
	let mut dirs = Vec::<PathBuf>::new();

	for item in read_dir(path).expect("unable to read directory") {
		let entry = item.expect("unable to read item");
		let meta = entry
			.metadata()
			.expect("unable to read item entry metadata");
		let path = entry.path();
		if meta.is_dir() {
			dirs.push(path);
			continue;
		}

		if meta.is_file() {
			if let Some(ext) = path.extension() {
				if ext == "rs" {
					files.push(path);
					continue;
				}
			}
		}
	}

	DirChildren {
		rs_files: files,
		dirs: dirs,
	}
}

fn read_ast(path: &Path) -> File {
	let contents = read_to_string(path).expect(&format!("couldn't read {:?}", path));
	// TODO: proper error-reporting (issue is isolated to this module, thankfully)
	parse_file(&contents).expect(&format!("couldn't parse {:?}", path))
}

fn lower_file_module(path: &Path) -> Module {
	let ast = read_ast(path);
	let name = get_file_stem(path);
	Module {
		name: name,
		raw_ast: ast,
		submodules: Vec::new(),
	}
}

pub fn try_lower_dir_module(path: &Path) -> Option<Module> {
	let DirChildren {
		rs_files: mut files,
		dirs,
	} = enumerate_children(path);

	let mod_def = files.iter().position(|p| get_filename(p) == "mod.rs");
	let mod_file = match mod_def {
		None => {
			println!("{:?} did not have a mod.rs and was not processed", path);
			return None;
		}
		Some(i) => files.remove(i),
	};

	let mut submodules = Vec::<Module>::new();
	for file in &files {
		submodules.push(lower_file_module(file));
	}

	for dir in &dirs {
		match try_lower_dir_module(dir) {
			Some(m) => submodules.push(m),
			_ => (),
		}
	}

	Some(Module {
		name: get_filename(path),
		raw_ast: read_ast(&mod_file),
		submodules: submodules,
	})
}
