use std::fs;
use std::path;

pub fn parse_idl_files(idl_paths: Vec<path::PathBuf>) -> IDLFiles {
    let mut asts = IDLFiles::with_capacity(idl_paths.len());
    for path in idl_paths {
        let content = fs::read_to_string(&path)
            .expect(&format!("Could not open IDL file \"{}\"", path.display()));
        let ast = syn::parse_file(&content).expect("Couldn't parse IDL file");
        asts.push((get_stem(&path), ast));
    }

    asts
}

fn get_stem(path: &path::Path) -> String {
    path.file_stem()
        .expect("IDL file had no stem")
        .to_str()
        .expect("IDL file stem un-translatable")
        .to_string()
}

// TODO: actually manage string references correctly
pub type IDLFiles = Vec<(String, syn::File)>;
