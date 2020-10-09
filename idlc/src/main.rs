use std::env::args;
use std::fs::{read_dir, remove_file, remove_dir_all};
use std::path;
use syn::{Ident, Path, Type};

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

struct _RpcTraitRef<'ast> {
    path: &'ast Path,
}

struct _RRefLike<'ast> {
    kind: &'ast Ident,
    verbatim_type: &'ast Type,
}

struct _RRefLikeImmutRef<'ast> {
    kind: &'ast Ident,
    verbatim_type: &'ast Type,
}

struct _Bitwise<'ast> {
    verbatim_type: &'ast Type,
}

enum _ExchangeableItem<'ast> {
    RpcTraitRef(_RpcTraitRef<'ast>),
    RRefLike(_RRefLike<'ast>),
    RRefLikeImmutRef(_RRefLikeImmutRef<'ast>),
    Bitwise(_Bitwise<'ast>),
}

struct _RpcMethod<'ast> {
    name: &'ast Ident, // TODO: is Ident cheap to copy / clone?
    arguments: Vec<_ExchangeableItem<'ast>>,
    is_static: bool,
}

fn clean_stale_glue_modules(glue_root: &path::Path) {
    for item in read_dir(glue_root.join("src")).expect("Could not open glue sources") {
        let entry = item.expect("");
        let meta = entry.metadata().expect("");
        let path = entry.path();
        let filename = path.file_name().expect("");
        if filename == "sys" || filename == "lib.rs" {
            continue;
        }

        if meta.file_type().is_dir() {
            remove_dir_all(path).expect("");
        } else {
            remove_file(path).expect("");
        }
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

    let glue_crate = path::Path::new(&args[0]);
    let domain_crates: Vec<&path::Path> = args[1..].iter().map(|s| path::Path::new(s)).collect();
    clean_stale_glue_modules(glue_crate);
}
