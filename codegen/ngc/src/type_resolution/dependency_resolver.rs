use mem::replace;
use syn::{File, FnArg, Ident, Item, ItemTrait, ItemUse, Path, PathSegment, ReturnType, TraitItem, TraitItemMethod, Type, UseTree, Visibility};
use std::{borrow::BorrowMut, collections::{HashMap, HashSet}, thread::current};
use std::mem;
use std::rc::Rc;
use super::{symbol_tree::*, utils::is_public};

pub struct DependencyResolver {
    /// A stack of maps a PathSegment to its fully qualified path.
    symbol_tree: SymbolTree,
    /// The current module(aka current frame).
    symbol_tree_node: SymbolTreeNode,
}

impl DependencyResolver {
    pub fn new() -> Self {
        let symbol_tree = SymbolTree::new();
        Self {
            symbol_tree,
            symbol_tree_node: symbol_tree.root_symbol_tree_node(),
        }
    }

    /// Takes a AST and returns a list of fully-qualified paths of all `RRef`ed types.
    pub fn resolve_dependencies(self, ast: &File) -> SymbolTree {
        self.resolve_dependencies_recursive(&ast.items);
        self.resolve_relative_paths_recursive_for_symbol_tree_node(self.symbol_tree_node);
        self.symbol_tree
    }

    /// Returns the resolved/terminal path of `module_item`.
    fn resolve_relative_paths_recursive_for_module_item(&self, module_item: ModuleItem) {
        match module_item {
            ModuleItem::Type(item) => {
                // No further resolution is required.
                if item.terminal {
                    return;
                }

                // Walk the relative path and try resolving the path.
                let mut current_node = match item.leading_colon {
                    true => self.symbol_tree.root.clone(),
                    false => module_item.clone()
                };
                for path_segment in &item.path {
                    match current_node {
                        ModuleItem::Type(_) => panic!("Resolving {:?} for {:?}. Node {:?} is a symbol and cannot have child.", path_segment, item.path, current_node),
                        ModuleItem::Module(md) => {
                            let next_node = md.module.children.get(path_segment);
                            let next_node = next_node.expect(&format!("ident {:?} not found in {:?} when resolving {:?}", path_segment, md.module.path, item.path));
                            match next_node {
                                ModuleItem::Type(ty) => assert!(ty.public, "Node is not public. {:?}", next_node),
                                ModuleItem::Module(md) => assert!(md.public, "Node is not public. {:?}", next_node),
                            }
                            current_node = next_node.clone();
                        }
                    }
                }

                // Keep resolving recursively if the node that we get from resolving is not
                // terminal.
                // If the node is a module, we can treat it as terminal. It's up to the user to
                // resolve their types that in the module. 
                match current_node {
                    ModuleItem::Module(_) => { /* noop */ }
                    ModuleItem::Type(ty) => {
                        if !ty.terminal {
                            self.resolve_relative_paths_recursive_for_module_item(current_node);
                        }
                        assert!(ty.terminal);
                    }
                }

                // Populate the absolute path to us.
                item.terminal = true;
                match current_node {
                    ModuleItem::Type(sym) => item.path.clone_from_slice(&sym.path[..]),
                    ModuleItem::Module(md) => item.path.clone_from_slice(&md.module.path[..]),
                }
               
            }
            ModuleItem::Module(item) => {
                assert!(item.public);
                // Go to the children frame and do recursive call.
                let old_frame = self.symbol_tree_node.clone();
                self.symbol_tree_node = item.module;
                self.resolve_relative_paths_recursive_for_symbol_tree_node(self.symbol_tree_node.clone());
                // Pop the frame and return back to the old frame.
                self.symbol_tree_node = self.symbol_tree_node.parent.unwrap().clone();
                // Sanity check that we are actually restoring to the correct frame.
                assert!(old_frame.same(&self.symbol_tree_node))
            }
        }
    }

    /// Resolve all relative paths generated `resolve_dependencies_recursive` by into terminal
    /// paths. 
    fn resolve_relative_paths_recursive_for_symbol_tree_node(&self, symbol_tree_node: SymbolTreeNode) {
        for (_, child) in symbol_tree_node.borrow_mut().children {
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
                        self.symbol_tree_node = self.symbol_tree_node.add_module(&item.ident, &item.vis);
                        self.resolve_dependencies_recursive(items);
                        self.symbol_tree_node = self.symbol_tree_node.parent().unwrap();
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
        let symbol = TypeNode::new(is_public(vis), terminal, leading_colon, path);
        self.symbol_tree_node.insert(ident, ModuleItem::Type(symbol)).expect("type node shouldn't apprear more than once");
    }

    /// Add a symbol that's defined in the current scope. The symbol is terminal.
    fn add_definition_symbol(&mut self, ident: &Ident, vis: &Visibility) {
        // Construct fully qualified path.
        let mut path = self.symbol_tree_node.path.clone();
        path.push(ident.clone());

        // Add symbol.
        let symbol = TypeNode::new(is_public(vis), true, true, path);
        self.symbol_tree_node.insert(ident, ModuleItem::Type(symbol)).expect("type node shouldn't apprear more than once");
    }
    
    // /// Add a symbol to the corrent scope.
    // fn add_symbol(&mut self, ident: &Ident, symbol: TypeNode) {
    //     // Construct module item
    //     let module_item = ModuleItem {
    //         public: is_public(vis),
    //         terminal,
    //         item_type: ModuleItem::Type(symbol)
    //     };

    //     self.symbol_tree_node.insert(ident, module_item).expect("type node shouldn't apprear more than once");
    // }
}
