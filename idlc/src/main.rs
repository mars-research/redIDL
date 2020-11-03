mod ir;
mod sanity;

use std::env::args;
use std::fs::{read_dir, read_to_string, remove_dir_all, remove_file};
use std::path::Path;
use syn::parse_file;
use syn::File;

/*
    1. Parse all given IDL directories
    2. Lower AST
        - need to obtain type semantic info
            - i.e. how to access its fields
        - resolve RRef-like types and insert specialized nodes for them
    3. IR processing

    IR needs to deal with a few essential structures:
    - Exchangeable traits
        - RPC methods
            - Type kinds (TODO: ??, possibly only needed for advanced marshaling)
    - Verbatim AST subtrees, which we treat as opaque
    - Composites
    - Optional<RRef<>>-like members

    Need to know what's inside types (i.e. field trees) to answer questions like:
    - Is this type something we can safely copy?
        - primitives
        - special types (idk if we have any, but they'd be represented as AST nodes)
        - composites of (e.g.: arrays, tuples, structs, enums)
    - Is this type acceptable as an RRef<>?
        - anything that can be safely copied
        - structs containing only safely copyable things and Optionals of RRef-like types
    - In both cases we need a complete picture of what types are contained within other types

    We need to be able to resolve types within the IDL
    - From a more formal standpoint, every domain named creates its own module named with itself.
    E.g. in lib.rs of lib/glue:
        pub mod bdev;
        pub mod ixgbe;
        ...
    and then in lib/glue/bdev/mod.rs:
        pub mod foo;
        pub mod bar;

    - So from a type resolution standpoint, absolute paths for IDL types always take the form:
        crate::<domain>::<idl-mod>
    and absolute paths for system types (e.g. RRef, RRefArray, etc.) are crate::sys::<type>

    Types handed to RRef-like constructions must be acceptable for use as an RRef.
    During AST lowering we can find and label trait references, etc.
    Possibility of cycles

    1. Lower AST, speculatively identify argument kinds based off of syntax
        - lowering could also identify type kinds at definition sites? E.g.:
        "this is an RPC trait," "this is a safe-copy composite," or "this is an RRef-able composite"
            - these would have to be speculative, record which unresolved paths need to be what kind
                - can this always be done? Does the field type always imply what the requirements are (mostly yes)?
    2. Resolve argument types, query for appropriateness, cache results

    For resolution, compiler needs to track the implied IDL module structure (let's not support inlined modules
    just yet), and the special sys module.
*/

// IRREVERSIBLE!
// This will only leave the hand-written "sys" module alone
fn clean_stale_glue_modules(glue_root: &Path) {
    for item in read_dir(glue_root.join("src")).expect("Could not open glue sources") {
        let entry = item.expect("");
        let meta = entry.metadata().expect("");
        let path = entry.path();
        let filename = path.file_name().expect("");
        if filename == "sys" {
            continue;
        }

        if meta.file_type().is_dir() {
            remove_dir_all(path).expect("");
        } else {
            remove_file(path).expect("");
        }
    }
}

// TODO: more consistent error handling

fn get_dir_name(path: &Path) -> String {
    path.file_name()
        .expect("a directory name was expected but none was found")
        .to_str()
        .expect("directory name was not valid unicode")
        .to_string()
}

fn load_ast(path: &Path) -> std::result::Result<syn::File, ()> {
    let source = match read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            println!("\x1b[31merror:\x1b[0m {} ({:?})", err, path);
            return Err(());
        }
    };

    match parse_file(&source) {
        Ok(ast) => Ok(ast),
        Err(err) => {
            println!("\x1b[31merror:\x1b[0m {} ({:?})", err, path);
            return Err(());
        }
    }
}

fn load_idl_modules<'ast>(domains: &Vec<&Path>) -> std::result::Result<Vec<(String, File)>, ()> {
    let mut modules = Vec::<(String, File)>::new();
    for path in domains {
        let name = get_dir_name(path);
        let ast = load_ast(&path.join("idl.rs"))?;
        if !sanity::sanity_check_module(&ast) {
            println!("Module {} failed sanity check", name);
            return Err(());
        }

        modules.push((name, ast));
    }

    Ok(modules)
}

fn lower_idl_modules<'ir>(modules: &'ir Vec<(String, File)>) -> Vec<ir::Module<'ir>> {
    let mut ir_mods = Vec::new();
    ir_mods.reserve_exact(modules.len());
    for (name, ast) in modules {
        ir_mods.push(ir::produce_module(name, ast))
    }

    ir_mods
}

// Keep a table of "assigned type IDs"
// Every RRef, we compute the type ID (with path resolution accounted for) of its argument and insert into said table
// If the entry was newly inserted, we implement TypeIdentifiable for it using its type ID

/*
    NOTE: Deferring support for relative type paths, use statements, and generalized module collection
    The important things are getting absolute path type resolution working,
    constructing the type layout trees (all types, even anonymous ones, end up in a type table for memoization; this
    type table (tables?) is also used for TypeIdentifiable generation), then lowering those trees. Generation is
    *trivial* compared to this.
*/

fn main() {
    // Accepts a non-empty set of domains to process the IDL subdir from
    // Immediately preceded by the path to the glue crate, which will be automatically cleaned of everything
    // but the sys subdir, which implements the sys module

    let args: Vec<String> = args().collect();
    if args.len() < 3 {
        println!("Usage: idlc <glue-crate> <domain>+");
        return;
    }

    let glue_crate = Path::new(&args[1]);
    let domain_paths: Vec<&Path> = args[2..].iter().map(|s| Path::new(s)).collect();
    let idl_mods = match load_idl_modules(&domain_paths) {
        Ok(ret) => ret,
        Err(_) => return,
    };

    let mods = lower_idl_modules(&idl_mods);

    // The pseudo-identifier "crate" in every module is specially interpreted as referring to this root module
    // Similarly, references to the crate::sys crate (implemented via a sentinel module ID?) will not refer to
    // any actual module, but are specially handled, as these are types known to the compiler to be exchangeable in
    // certain contexts. For example, crate::sys::Syscalls, a trait that is always allowed to be passed, even though it
    // would not ordinarily pass type-checking

    // More accurately, we use "fake" types

    let _lib_mod = ir::Module {
        name: "lib".to_string(),
        submodules: mods,
        items: Vec::new(),
    };

    // Plan is to have a fake root and sys module that have type resolution entries only, and no AST
    // As in the case of the sys module, we aren't generating anything, and for the root module of the crate
    // we're generating all of it
    // We can prototype type resolution on a by-domain basis, since all we need to do is merge them into a larger tree
    // to integrate everything else
    // This segment should probably live in mod_map

    clean_stale_glue_modules(glue_crate)
}
