use mem::replace;
use syn::{File, FnArg, Ident, Item, ItemTrait, ItemUse, Path, PathSegment, ReturnType, TraitItem, TraitItemMethod, Type, UseTree, Visibility};
use std::collections::{HashMap, HashSet};
use std::mem;
use super::{module_tree::*, utils::is_public};

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
                    self.add_definition_symbol(&item.ident, &item.vis);
                }
                Item::Mod(item) => {
                    if let Some((_, items)) = &item.content {
                        self.module_node = self.module_node.push(&item.ident, &item.vis);
                        self.resolve_dependencies_recursive(items);
                        self.module_node = self.module_node.parent().unwrap();
                    }
                }
                Item::Struct(item) => {
                    self.add_definition_symbol(&item.ident, &item.vis);
                }
                Item::Type(item) => {
                    self.add_definition_symbol(&item.ident, &item.vis);
                }
                Item::Union(item) => {
                    self.add_definition_symbol(&item.ident, &item.vis);
                }
                Item::Use(item) => {
                    let path = vec!{};
                    self.resolve_dependencies_in_usetree_recursive(&item.tree, &item.vis, path);
                }
                _ => {},
            }
        }
    }

    fn resolve_dependencies_in_usetree_recursive(&mut self, tree: &UseTree, vis: &Visibility, mut path: Vec<Ident>) {
        match tree {
            // e.g. `a::b::{Baz}`. A Tree.
            syn::UseTree::Path(tree) => {
                path.push(tree.ident.clone());
                self.resolve_dependencies_in_usetree_recursive(&tree.tree, vis, path);
            }
            // e.g. `Baz`. Leaf node.
            syn::UseTree::Name(tree) => {
                path.push(tree.ident.clone());
                self.add_use_symbol(&tree.ident, vis, path);
            }
            // e.g. `Baz as Barz`. Leaf node but renamed.
            syn::UseTree::Rename(tree) => {
                path.push(tree.ident.clone());
                self.add_use_symbol(&tree.rename, vis, path);
            }
            // e.g. `a::*`. This is banned because pulling the depencies in and analyze them is not
            // within the scope of this project.
            syn::UseTree::Glob(tree) => {
                panic!("Use globe is disallowed in IDL. For example, you cannot do `use foo::*`. You did: {:#?}", tree)
            }
            // e.g. `{b::{Barc}, Car}`.
            syn::UseTree::Group(tree) => {
                for tree in &tree.items {
                    self.resolve_dependencies_in_usetree_recursive(&tree, vis, path.clone());
                }
            }
        }
    }

    /// Add a symbol from a `use` state,emt to the corrent scope.
    fn add_use_symbol(&mut self, ident: &Ident, vis: &Visibility, path: Vec<Ident>) {
        // If the path doesn't start with "crate" or "super", this means that it comes from
        // an external library, which means we should mark it as terminal.
        let first_segment = &path[0];
        let terminal = (first_segment != "crate") && (first_segment != "super");

        // Add the symbol to the module.
        self.add_symbol(ident, vis, path, terminal);
    }

    /// Add a symbol that's defined in the current scope. The symbol is terminal.
    fn add_definition_symbol(&mut self, ident: &Ident, vis: &Visibility) {
        // Construct fully qualified path.
        let mut path = self.module_node.path.clone();
        path.push(ident.clone());

        // Add symbol
        self.add_symbol(ident, vis, path, true);
    }
    
    /// Add a symbol to the corrent scope.
    fn add_symbol(&mut self, ident: &Ident, vis: &Visibility, path: Vec<Ident>, terminal: bool) {
        // Construct module item
        let module_item = ModuleItem {
            public: is_public(vis),
            terminal,
            item_type: ModuleItemType::Symbol(path)
        };

        self.module_node.insert(ident, module_item).expect("terminal node '{:?}' shouldn't apprear more than once");
    }
}
