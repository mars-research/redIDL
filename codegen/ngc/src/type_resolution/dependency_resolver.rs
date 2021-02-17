use mem::replace;
use syn::{File, FnArg, Item, ItemTrait, ItemUse, Path, PathSegment, ReturnType, TraitItem, TraitItemMethod, Type, UseTree, Visibility};
use std::collections::{HashMap, HashSet};
use std::mem;
use super::module_tree::*;

pub struct DependencyResolver {
    /// A stack of maps a PathSegment to its fully qualified path
    module_tree: ModuleTree,
    /// The current module
    module_node: ModuleNode,
}

impl DependencyResolver {
    pub fn new() -> Self {
        let module_tree = ModuleTree::new();
        let root = module_tree.root.clone();
        Self {
            module_tree,
            module_node: root,
        }
    }

    /// Takes a AST and returns a list of fully-qualified paths of all `RRef`ed types.
    pub fn resolve_dependencies(&mut self, ast: &File) -> ModuleTree {
        self.module_tree.clear();
        self.module_node = self.module_tree.root.clone();
        self.resolve_dependencies_recursive(&ast.items);
        let old_tree = std::mem::replace(&mut self.module_tree, ModuleTree::new());
        self.module_node = self.module_tree.root.clone();
        old_tree
    }

    fn resolve_dependencies_recursive(&mut self, items: &Vec<syn::Item>) {
        let mut generated_items = Vec::<syn::Item>::new();
        for item in items.iter() {
            match item {
                Item::Enum(item) => {
                    self.module_node.add_symbol(&item.ident, &item.vis);
                }
                Item::Mod(item) => {
                    if let Some((_, items)) = &item.content {
                        self.module_node = self.module_node.push(&item.ident);
                        self.resolve_dependencies_recursive(items);
                        self.module_node = self.module_node.parent().unwrap();
                    }
                }
                Item::Struct(item) => {
                    self.module_node.add_symbol(&item.ident, &item.vis);
                }
                Item::Type(item) => {
                    self.module_node.add_symbol(&item.ident, &item.vis);
                }
                Item::Union(item) => {
                    self.module_node.add_symbol(&item.ident, &item.vis);
                }
                Item::Use(item) => {
                    self.resolve_dependencies_in_usetree(&item.tree, &item.vis);
                }
                _ => {},
            }
        }
    }

    fn resolve_dependencies_in_usetree(&mut self, tree: &UseTree, visibility: &Visibility) {
        match tree {
            syn::UseTree::Path(tree) => {
                self.resolve_dependencies_in_usetree(&tree.tree, visibility);
            }
            syn::UseTree::Name(tree) => {
                self.module_node.add_symbol(&tree.ident, visibility);
            }
            syn::UseTree::Rename(tree) => {
                self.module_node.add_symbol(&tree.rename, visibility);
            }
            syn::UseTree::Glob(tree) => {
                panic!("Use globe is disallowed in IDL. For example, you cannot do `use foo::*`. You did: {:#?}", tree)
            }
            syn::UseTree::Group(tree) => {
                for tree in &tree.items {
                    self.resolve_dependencies_in_usetree(&tree, visibility);
                }
            }
        }
    }
}
