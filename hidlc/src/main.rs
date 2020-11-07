use std::env::args;
use std::fs::{read_to_string, File, OpenOptions};
use std::io::Write;
use std::path;
use std::path::PathBuf;
use std::process::Command;
use std::str::from_utf8;

use quote::quote;

use syn::visit::*;
use syn::visit_mut::*;
use syn::*;

/*
    Suppose we have:
    - a #[interface] attrib for traits (impl Proxy for them, RRefable for refs to them, generate their proxy)
    - a #[shared] attrib for structs (impl RRefable for them, create a dummy impl block to assert MemberRRefable)

    These attributes must not be expanded, as they use these traits legally.
    So we prune them
*/

struct AttribPruneVisitor;

// Note that we make the assumption that the aforementioned attributes are specified without paths
impl VisitMut for AttribPruneVisitor {
    fn visit_item_trait_mut(&mut self, node: &mut ItemTrait) {
        node.attrs.retain(|a| match a.path.get_ident() {
            Some(id) => id != "interface",
            None => true,
        });

        visit_item_trait_mut(self, node)
    }

    fn visit_item_struct_mut(&mut self, node: &mut ItemStruct) {
        node.attrs.retain(|a| match a.path.get_ident() {
            Some(id) => id != "shared",
            None => true,
        });

        visit_item_struct_mut(self, node)
    }
}

fn prune_all(ver_crate_path: &path::Path, idl_files: &IDLFiles) {
    let lib_path = ver_crate_path.join("lib.rs");
    let mut lib_file = File::create(lib_path).expect("Could not create lib.rs");
    lib_file
        .write_fmt(format_args!("mod sys;\n"))
        .expect("Could not prepare lib.rs");

    for (stem, ast) in idl_files {
        let mut pruner = AttribPruneVisitor {};
        let mut dup_file = ast.clone();
        pruner.visit_file_mut(&mut dup_file);

        lib_file
            .write_fmt(format_args!("mod {};\n", stem))
            .expect("Could not write IDL mod to lib.rs");

        // At this point dup_file is free of our special macros, so we may expand

        let dump_path = ver_crate_path.join(stem).with_extension("rs");
        File::create(dump_path)
            .expect("Could not create dump")
            .write_fmt(format_args!("{}", quote! {#dup_file}))
            .expect("Could not dump pruned IDL");
    }
}

const RES_WORDS: [&str; 4] = ["RRef", "RRefArray", "RRefDequeue", "Option"];

struct ResWordsPass;

// Strategy to prevent people from just doing impl RRefable is to have the verifier crate not import those

impl<'ast> Visit<'ast> for ResWordsPass {
    fn visit_item_foreign_mod(&mut self, _node: &ItemForeignMod) {
        panic!("Foreign modules are not permitted")
    }

    fn visit_trait_item_type(&mut self, _node: &TraitItemType) {
        panic!("Trait associated types are not permitted")
    }

    fn visit_impl_item_type(&mut self, _node: &ImplItemType) {
        panic!("Impl associated types are not permitted")
    }

    fn visit_item_type(&mut self, _node: &ItemType) {
        panic!("Type aliases are not supported")
    }

    fn visit_item_struct(&mut self, node: &ItemStruct) {
        if RES_WORDS.iter().any(|w| node.ident == w) {
            panic!("Struct declared with reserved name")
        }

        visit_item_struct(self, node)
    }

    fn visit_item_trait(&mut self, node: &ItemTrait) {
        if RES_WORDS.iter().any(|w| node.ident == w) {
            panic!("Trait declared with reserved name")
        }

        visit_item_trait(self, node)
    }

    fn visit_item_mod(&mut self, node: &ItemMod) {
        // sys is exempt, because it is trusted
        if node.ident != "sys" {
            visit_item_mod(self, node)
        }
    }
}

fn preverify(ver_crate_path: &path::Path, idl_files: &IDLFiles) {
    let ver_sources = ver_crate_path.join("src");
    prune_all(&ver_sources, idl_files);
    let expand_out = Command::new("cargo")
        .current_dir(&ver_crate_path)
        .arg("expand")
        .output()
        .expect("Expansion failed");
    let expanded =
        parse_file(from_utf8(&expand_out.stdout).expect("Could not decode expansion output"))
            .expect("Couldn't parse expansion output");

    let mut res_pass = ResWordsPass {};
    res_pass.visit_file(&expanded);

    let err_level = Command::new("cargo")
        .current_dir(&ver_crate_path)
        .arg("build")
        .status()
        .expect("Failed to start verifier crate build");
    if !err_level.success() {
        panic!("Failed to build verifier crate")
    }
}

fn parse_idl_files(idl_paths: Vec<PathBuf>) -> IDLFiles {
    let mut asts = IDLFiles::with_capacity(idl_paths.len());
    for path in idl_paths {
        let content = read_to_string(&path)
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
type IDLFiles = Vec<(String, syn::File)>;

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
fn generate_code(glue_crate_path: &path::Path, idl_files: &IDLFiles) {
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
        for tid in type_strings {
            dump.write_fmt(format_args!("crate::sys::hidl_macros::assign_id!({}, {});\n", count, tid))
                .expect("Could not write to dump");
            count += 1
        }
    }
}

// Sanity example

trait IsProxy {}
trait RRefable {}

fn need_copy<T: Copy>() {}
fn need_proxy<T: IsProxy + ?Sized>() {}

#[derive(Clone, Copy)]
struct Bar {}

impl RRefable for Bar {}

struct RRef<T: RRefable> {
    a: T,
}

trait FooBar {}

impl IsProxy for FooBar {}

trait Foo {
    fn do_foo(&self, a: RRef<Bar>, b: &dyn FooBar, c: Bar) -> ();
}

struct _AssertSaneFoo;

// generated by #[interface]
impl _AssertSaneFoo {
    fn _assert_sane() {
        // No need_rrefable: RRef<> already asserts this
        need_proxy::<dyn FooBar>();
        need_copy::<Bar>();
    }
}

impl IsProxy for &dyn Foo {}

// End

fn main() {
    let args: Vec<String> = args().collect();
    if args.len() < 4 {
        println!("Usage: hidlc <ver-crate> <glue-crate> <domain-idl-file>+");
    }

    let ver_crate_path = PathBuf::from(&args[1]);
    let glue_crate_path = PathBuf::from(&args[2]);
    let idl_paths: Vec<PathBuf> = args[3..].iter().map(|s| PathBuf::from(&s)).collect();
    let idl = parse_idl_files(idl_paths);

    preverify(&ver_crate_path, &idl);
    generate_code(&glue_crate_path, &idl);
}
