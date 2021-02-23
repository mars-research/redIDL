use mem::replace;
use syn::{File, FnArg, Ident, Item, ItemTrait, ItemUse, Path, PathSegment, ReturnType, TraitItem, TraitItemMethod, Type, UseTree, Visibility};
use std::{borrow::BorrowMut, collections::{HashMap, HashSet}, thread::current};
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
        self.resolve_relative_paths_recursive_for_module_node(self.module_tree.root.clone());
        let old_tree = std::mem::replace(&mut self.module_tree, ModuleTree::new());
        self.module_node = self.module_tree.root.clone();
        old_tree
    }

    /// Returns the resolved/terminal path of `module_item`.
    fn resolve_relative_paths_recursive_for_module_item(&self, module_item: &mut ModuleItem) {
        match module_item.item_type {
            ModuleItemType::Symbol(item) => {
                // No further resolution is required.
                if module_item.terminal {
                    return;
                }

                // Walk the relative path and try resolving the path.
                let mut current_node = module_item.clone();
                if item.leading_colon {
                    current_node = ModuleItem {
                        public: true,
                        terminal: false,
                        item_type: ModuleItemType::Module(self.module_tree.root.clone()),
                    };
                }
                for path_segment in &item.path {
                    match current_node.item_type {
                        ModuleItemType::Symbol(_) => panic!("Resolving {:?} for {:?}. Node {:?} is a symbol and cannot have child.", path_segment, item.path, current_node),
                        ModuleItemType::Module(md) => {
                            let next_node = md.children.get(path_segment);
                            let next_node = next_node.expect(&format!("ident {:?} not found in {:?} when resolving {:?}", path_segment, md.path, item.path));
                            assert!(next_node.public, "Node is not public. {:?}", next_node);
                            current_node = next_node.clone();
                        }
                    }
                }

                // Keep resolving recursively if the node that we get from resolving is not
                // terminal.
                if !current_node.terminal {
                    self.resolve_relative_paths_recursive_for_module_item(current_node);
                }
                assert!(current_node.terminal);

                module_item.borrow_mut().terminal = true;
                match current_node.item_type {
                    ModuleItemType::Symbol(sym) => item.path.clone_from_slice(&sym.path[..]),
                    ModuleItemType::Module(md) => item.path.clone_from_slice(&md.path[..]),
                }
               
            }
            ModuleItemType::Module(item) => {
                assert!(module_item.terminal);
                self.resolve_relative_paths_recursive_for_module_node(item.clone());
            }
        }
    }

    /// Resolve all relative paths generated `resolve_dependencies_recursive` by into terminal
    /// paths. 
    fn resolve_relative_paths_recursive_for_module_node(&self, module_node: ModuleNode) {
        for (_, child) in module_node.borrow_mut().children {
            self.resolve_relative_paths_recursive_for_module_item(child);
        }
    }

    /// Recursively add relative paths and terminal nodes into the module. The relative paths
    /// need to be resolved into terminal paths later.
    fn resolve_dependencies_recursive(&mut self, items: &Vec<syn::Item>) {
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
                    self.resolve_dependencies_in_usetree_recursive(&item.tree, &item.vis, path, item.leading_colon.is_some());
                }
                _ => {},
            }
        }
    }

    fn resolve_dependencies_in_usetree_recursive(&mut self, tree: &UseTree, vis: &Visibility, mut path: Vec<Ident>, leading_colon: bool) {
        match tree {
            // e.g. `a::b::{Baz}`. A Tree.
            syn::UseTree::Path(tree) => {
                path.push(tree.ident.clone());
                self.resolve_dependencies_in_usetree_recursive(&tree.tree, vis, path, leading_colon);
            }
            // e.g. `Baz`. Leaf node.
            syn::UseTree::Name(tree) => {
                path.push(tree.ident.clone());
                self.add_use_symbol(&tree.ident, vis, path, leading_colon);
            }
            // e.g. `Baz as Barz`. Leaf node but renamed.
            syn::UseTree::Rename(tree) => {
                path.push(tree.ident.clone());
                self.add_use_symbol(&tree.rename, vis, path, leading_colon);
            }
            // e.g. `a::*`. This is banned because pulling the depencies in and analyze them is not
            // within the scope of this project.
            syn::UseTree::Glob(tree) => {
                panic!("Use globe is disallowed in IDL. For example, you cannot do `use foo::*`. You did: {:#?}", tree)
            }
            // e.g. `{b::{Barc}, Car}`.
            syn::UseTree::Group(tree) => {
                for tree in &tree.items {
                    self.resolve_dependencies_in_usetree_recursive(&tree, vis, path.clone(), leading_colon);
                }
            }
        }
    }

    /// Add a symbol from a `use` state,emt to the corrent scope.
    fn add_use_symbol(&mut self, ident: &Ident, vis: &Visibility, path: Vec<Ident>, leading_colon: bool) {
        // If the path doesn't start with "crate" or "super", this means that it comes from
        // an external library, which means we should mark it as terminal.
        let first_segment = &path[0];
        let terminal = (first_segment != "crate") && (first_segment != "super");

        // Add the symbol to the module.
        let symbol = SymbolNode::new(leading_colon, path);
        self.add_symbol(ident, vis, terminal, symbol);
    }

    /// Add a symbol that's defined in the current scope. The symbol is terminal.
    fn add_definition_symbol(&mut self, ident: &Ident, vis: &Visibility) {
        // Construct fully qualified path.
        let mut path = self.module_node.path.clone();
        path.push(ident.clone());

        // Add symbol
        let symbol = SymbolNode::new(true, path);
        self.add_symbol(ident, vis, true, symbol);
    }
    
    /// Add a symbol to the corrent scope.
    fn add_symbol(&mut self, ident: &Ident, vis: &Visibility, terminal: bool, symbol: SymbolNode) {
        // Construct module item
        let module_item = ModuleItem {
            public: is_public(vis),
            terminal,
            item_type: ModuleItemType::Symbol(symbol)
        };

        self.module_node.insert(ident, module_item).expect("terminal node '{:?}' shouldn't apprear more than once");
    }
}
