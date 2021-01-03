use std::env::args;
use std::env::current_dir;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::process::Command;
use std::{path, path::PathBuf};
use std::str::from_utf8;

use quote::quote;

use syn::visit::*;
use syn::*;

mod collect;
mod verify;

/*
    Suppose we have:
    - a #[interface] attrib for traits (impl Proxy for them, RRefable for refs to them, generate their proxy)
    - a #[shared] attrib for structs (impl RRefable for them, create a dummy impl block to assert MemberRRefable)
    - a #[create] attrib that does much of the same type-checking, but is used to mark methods for collection into
    CreateProxy

    These attributes must not be expanded, as they use these traits legally.
    So we prune them
*/

const RREF_LIKE: [&str; 3] = ["RRef", "RRefArray", "RRefDequeue"];

struct RRefSearchPass<'ret> {
    tid_list: &'ret mut Vec<String>,
}

impl<'ast, 'ret> Visit<'ast> for RRefSearchPass<'ret> {
    fn visit_type_path(&mut self, node: &TypePath) {
        let path = &node.path;
        let last = path.segments.len() - 1;
        let id = &path.segments[last];
        if RREF_LIKE.iter().any(|s| id.ident == s) {
            let ty = match &id.arguments {
                PathArguments::AngleBracketed(args) => match &args.args[0] {
                    GenericArgument::Type(ty) => ty,
                    _ => panic!("Unexpected RRef or RRef-like argument"),
                },
                _ => panic!("Unexpected RRef or RRef-like syntax"),
            };

            self.tid_list.push(quote! {#ty}.to_string());
        }

        visit_type_path(self, node)
    }
}

// I don't think it's very efficient to quote! an unmodified AST in here
fn generate_code(glue_crate_path: &path::Path, idl_files: &collect::IDLFiles) {
    let glue_src_path = glue_crate_path.join("src");
    let mut lib_file = File::create(glue_src_path.join("lib.rs")).expect("Could not create lib.rs");
    lib_file
        .write_fmt(format_args!("mod sys;\n"))
        .expect("Could not prepare lib.rs");

    // A crate-global type ID counter
    let mut count = 0;

    for (stem, ast) in idl_files {
        //println!("{:#?}", ast);
        lib_file
            .write_fmt(format_args!("mod {};\n", stem))
            .expect("Could not write IDL mod to lib.rs");

        let dump_path = glue_src_path.join(stem).with_extension("rs");
        {
            let mut dump = File::create(&dump_path).expect("Could not create dump");
            dump.write_fmt(format_args!("{}", quote! {#ast}))
                .expect("Could not dump IDL");
        }

        let mut type_strings = Vec::<String>::new();
        let mut rref_pass = RRefSearchPass {
            tid_list: &mut type_strings,
        };
        rref_pass.visit_file(&ast);

        Command::new("rustfmt")
            .arg(
                dump_path
                    .to_str()
                    .expect("Could not pass dump file path to rustfmt"),
            )
            .status()
            .expect("Could not invoke rustfmt on dump");

        // Here we can append to <dump> all the type-id assignments we pessimistically generated for <ast>,
        // and we know exactly at what line they begin after, and that there is one assignment per line
        // Calling rustc at this point will likely generate error messages with <file>:<line>:<column> location
        // tags, if the error occurs within the range we produced, assuming our macros are correctly written,
        // the error must be a duplicate impl error, and we know exactly which line's type id assignment must be erased
        // If the error occurs before our type id section of the file, we know it is an "actual" compiler error, and
        // we abort

        let mut dump = OpenOptions::new()
            .write(true)
            .append(true)
            .open(&dump_path)
            .expect("Couldn't reopen dump");

        dump.write_fmt(format_args!("\n")).expect("Could not write to dump");
        for tid in type_strings {
            dump.write_fmt(format_args!(
                "crate::sys::hidl_macros::assign_id!({}, {});\n",
                count, tid
            ))
            .expect("Could not write to dump");
            count += 1
        }
    }

    println!(
        "CWD is {:?}",
        current_dir().expect("could not obtain current dir")
    );
    
    let out = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(glue_crate_path.join("Cargo.toml"))
        .output()
        .expect("could not observe speculative build output");
    
    // TODO: parse error info, vacate duplicate type IDs
    println!("{}", from_utf8(&out.stdout).expect("could not UTF-8 encode build output"));
    println!("{}", from_utf8(&out.stderr).expect("could not UTF-8 encode build output"));
}

fn main() {
    let args: Vec<String> = args().collect();
    if args.len() < 4 {
        println!("Usage: hidlc <ver-crate> <glue-crate> <domain-idl-file>+");
        return;
    }

    let ver_crate_path = PathBuf::from(&args[1]);
    let glue_crate_path = PathBuf::from(&args[2]);
    let idl_paths: Vec<PathBuf> = args[3..].iter().map(|s| PathBuf::from(&s)).collect();
    let idl = collect::parse_idl_files(idl_paths);

    verify::preverify(&ver_crate_path, &idl);
    generate_code(&glue_crate_path, &idl)
}
