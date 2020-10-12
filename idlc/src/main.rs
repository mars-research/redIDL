use std::env::args;
use std::fs::{read_dir, remove_dir_all, remove_file};
use std::path::{Path, PathBuf};

mod ir;
mod mod_map;

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

/*
    Source map built into tree, rib hierarchy is implied, path resolution doable in single pass
    all Paths are represented as enums of either an unresolved path or of a typeid pointing at its definition
*/

struct ModVisit {
    level: isize,
}

impl<'ast> ModVisit {
    fn visit_module(&mut self, node: &'ast mod_map::Module) {
        for _ in 0..self.level {
            print!("\t");
        }

        println!(
            "Module \"{}\" had {} submodules",
            node.name,
            node.submodules.len()
        );

        self.level += 1;
        for submod in &node.submodules {
            self.visit_module(submod)
        }
        self.level -= 1;
    }
}

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
    
    // Plan is to have a fake root and sys module that have type resolution entries only, and no AST
    // As in the case of the sys module, we aren't generating anything, and for the root module of the crate
    // we're generating all of it
    
    // We can prototype type resolution on a by-domain basis, since all we need to do is merge them into a larger tree
    // to integrate everything else
    
    // This segment should probably live in mod_map
    let domain_crates: Vec<(String, PathBuf)> = args[2..]
        .iter()
        .map(|s| Path::new(s))
        .map(|p| (mod_map::get_file_stem(p), Path::new(p).join("idl")))
        .collect();
    for (name, path) in domain_crates {
        let mut dom_mod = mod_map::try_lower_dir_module(&path).expect("domain could not be lowered");
        dom_mod.name = name; // the idl module is "folded" into the domain
        let mut visit = ModVisit { level: 0 };
        visit.visit_module(&dom_mod)
    }

    clean_stale_glue_modules(glue_crate);
}
