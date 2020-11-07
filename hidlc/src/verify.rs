use crate::collect;

use std::fs;
use std::io::Write;
use std::path;
use std::process;
use std::str;

use syn::visit;
use syn::visit::Visit;
use syn::visit_mut;
use syn::visit_mut::VisitMut;

struct AttribPruneVisitor;

// Note that we make the assumption that the aforementioned attributes are specified without paths
impl visit_mut::VisitMut for AttribPruneVisitor {
    fn visit_item_trait_mut(&mut self, node: &mut syn::ItemTrait) {
        node.attrs.retain(|a| match a.path.get_ident() {
            Some(id) => id != "interface",
            None => true,
        });

        visit_mut::visit_item_trait_mut(self, node)
    }

    fn visit_item_struct_mut(&mut self, node: &mut syn::ItemStruct) {
        node.attrs.retain(|a| match a.path.get_ident() {
            Some(id) => id != "shared",
            None => true,
        });

        visit_mut::visit_item_struct_mut(self, node)
    }
}

fn prune_all(ver_crate_path: &path::Path, idl_files: &collect::IDLFiles) {
    let lib_path = ver_crate_path.join("lib.rs");
    let mut lib_file = fs::File::create(lib_path).expect("Could not create lib.rs");
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
        fs::File::create(dump_path)
            .expect("Could not create dump")
            .write_fmt(format_args!("{}", quote::quote! {#dup_file}))
            .expect("Could not dump pruned IDL");
    }
}

pub fn preverify(ver_crate_path: &path::Path, idl_files: &collect::IDLFiles) {
    let ver_sources = ver_crate_path.join("src");
    prune_all(&ver_sources, idl_files);
    let expand_out = process::Command::new("cargo")
        .current_dir(&ver_crate_path)
        .arg("expand")
        .output()
        .expect("Expansion failed");
    let expanded = syn::parse_file(
        str::from_utf8(&expand_out.stdout).expect("Could not decode expansion output"),
    )
    .expect("Couldn't parse expansion output");

    let mut res_pass = ResWordsPass {};
    res_pass.visit_file(&expanded);

    let err_level = process::Command::new("cargo")
        .current_dir(&ver_crate_path)
        .arg("build")
        .status()
        .expect("Failed to start verifier crate build");
    if !err_level.success() {
        panic!("Failed to build verifier crate")
    }
}

const RES_WORDS: [&str; 4] = ["RRef", "RRefArray", "RRefDequeue", "Option"];

struct ResWordsPass;

// Strategy to prevent people from just doing impl RRefable is to have the verifier crate not import those

impl<'ast> visit::Visit<'ast> for ResWordsPass {
    fn visit_item_foreign_mod(&mut self, _node: &syn::ItemForeignMod) {
        panic!("Foreign modules are not permitted")
    }

    fn visit_trait_item_type(&mut self, _node: &syn::TraitItemType) {
        panic!("Trait associated types are not permitted")
    }

    fn visit_impl_item_type(&mut self, _node: &syn::ImplItemType) {
        panic!("Impl associated types are not permitted")
    }

    fn visit_item_type(&mut self, _node: &syn::ItemType) {
        panic!("Type aliases are not supported")
    }

    fn visit_item_struct(&mut self, node: &syn::ItemStruct) {
        if RES_WORDS.iter().any(|w| node.ident == w) {
            panic!("Struct declared with reserved name")
        }

        visit::visit_item_struct(self, node)
    }

    fn visit_item_trait(&mut self, node: &syn::ItemTrait) {
        if RES_WORDS.iter().any(|w| node.ident == w) {
            panic!("Trait declared with reserved name")
        }

        visit::visit_item_trait(self, node)
    }

    fn visit_item_mod(&mut self, node: &syn::ItemMod) {
        // sys is exempt, because it is trusted
        if node.ident != "sys" {
            visit::visit_item_mod(self, node)
        }
    }
}
