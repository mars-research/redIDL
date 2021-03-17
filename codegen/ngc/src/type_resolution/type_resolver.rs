use mem::replace;
use syn::{File, FnArg, Ident, Item, ItemTrait, ItemUse, Path, PathSegment, ReturnType, TraitItem, TraitItemMethod, Type, UseTree, Visibility, spanned::Spanned};
use std::{borrow::BorrowMut, collections::{HashMap, HashSet}, thread::current};
use std::mem;
use std::rc::Rc;
use super::{symbol_tree::*, utils::is_public};

pub struct TypeResolver {
    /// A stack of maps a PathSegment to its fully qualified path.
    symbol_tree: SymbolTree,
    /// The current module(aka current frame).
    symbol_tree_node: SymbolTreeNode,
}

impl TypeResolver {
    pub fn new() -> Self {
        let symbol_tree = SymbolTree::new();
        let root = symbol_tree.root_symbol_tree_node();
        Self {
            symbol_tree,
            symbol_tree_node: root,
        }
    }

    /// Takes a AST and returns a list of fully-qualified paths of all `RRef`ed types.
    pub fn resolve_types(mut self, ast: &File) -> SymbolTree {
        self.resolve_types_recursive(&ast.items);
        self.resolve_relative_paths_recursive_for_symbol_tree_node(self.symbol_tree_node.clone());
        self.symbol_tree
    }

    /// Returns the resolved/terminal path of `module_item`.
    fn resolve_relative_paths_recursive_for_module_item(&mut self, module_item: ModuleItem, module: SymbolTreeNode) {
        match module_item.clone() {
            ModuleItem::Type(item) => {
                // No further resolution is required.
                if item.borrow().terminal.is_terminal() {
                    return;
                }

                // Walk the relative path and try resolving the path.
                // Save the previous node because we there's no way to know the parent of a type currently.
                let mut previous_node = None;
                let mut current_node = match item.borrow().leading_colon {
                    true => self.symbol_tree.root.clone(),
                    false => module_item.clone()
                };
                {
                    let path = &mut item.borrow_mut().path;
                    if path[0].to_string() == "crate" {
                        current_node = self.symbol_tree.root.clone();
                        path.remove(0);
                    } else if path[0].to_string() == "super" {
                        current_node = ModuleItem::Module(ModuleNode::new(true, module.parent().unwrap()));
                        path.remove(0);
                    } else if path[0].to_string() == "self" {
                        path.remove(0);
                    }
                }
                for path_segment in &item.borrow().path {
                    match current_node.clone() {
                        ModuleItem::Type(_) => panic!("Resolving {:#?} for {:#?}. Node {:#?} is a symbol and cannot have child.", path_segment, item.borrow().path, current_node),
                        ModuleItem::Module(md) => {
                            let md_ref = md.borrow();
                            let md_ref_ref = md_ref.module.borrow();
                            let next_node = md_ref_ref.children.get(path_segment);
                            let next_node = next_node.expect(&format!("ident {:#?} not found in {:#?} when resolving {:#?}", path_segment, md_ref_ref, item.borrow().path));
                            match next_node {
                                ModuleItem::Type(ty) => assert!(ty.borrow().public, "Node is not public. {:?}", next_node),
                                ModuleItem::Module(md) => assert!(md.borrow().public, "Node is not public. {:?}", next_node),
                            }
                            previous_node = Some(current_node.clone());
                            current_node = next_node.clone();
                        }
                    }
                }

                // Keep resolving recursively if the node that we get from resolving is not
                // terminal.
                // If the node is a module, we can treat it as terminal. It's up to the user to
                // resolve their types that in the module. 
                item.borrow_mut().terminal = match current_node.clone() {
                    ModuleItem::Module(_) => { Terminal::Module }
                    ModuleItem::Type(ty) => {
                        if !ty.borrow().terminal.is_terminal() {
                            let parent = match previous_node.unwrap() {
                                ModuleItem::Module(md) => md.borrow().module.clone(),
                                ModuleItem::Type(_) => unreachable!()
                            };
                            self.resolve_relative_paths_recursive_for_module_item(current_node.clone(), parent);
                        }
                        assert!(ty.borrow().terminal.is_terminal());
                        ty.borrow().terminal.clone()
                    }
                };
                assert!(item.borrow().terminal.is_terminal());

                // Populate the absolute path to us.
                match current_node.clone() {
                    ModuleItem::Type(sym) => item.borrow_mut().path = sym.borrow().path.clone(),
                    ModuleItem::Module(md) => item.borrow_mut().path = md.borrow().module.borrow().path.clone(),
                }
               
            }
            ModuleItem::Module(item) => {
                // Go to the children frame and do recursive call.
                let old_frame = self.symbol_tree_node.clone();
                self.symbol_tree_node = item.borrow().module.clone();
                self.resolve_relative_paths_recursive_for_symbol_tree_node(self.symbol_tree_node.clone());
                // Pop the frame and return back to the old frame.
                let parent_frame = self.symbol_tree_node.borrow().parent.clone().unwrap();
                self.symbol_tree_node = parent_frame;
                // Sanity check that we are actually restoring to the correct frame.
                assert!(old_frame.same(&self.symbol_tree_node))
            }
        }
    }

    /// Resolve all relative paths generated `resolve_types_recursive` by into terminal
    /// paths. 
    fn resolve_relative_paths_recursive_for_symbol_tree_node(&mut self, symbol_tree_node: SymbolTreeNode) {
        println!("Resolving relative path for module {:?}", symbol_tree_node.borrow().path[symbol_tree_node.borrow().path.len() - 1]);
        for (_, child) in &symbol_tree_node.borrow().children {
            self.resolve_relative_paths_recursive_for_module_item(child.clone(), symbol_tree_node.clone());
        }
    }

    /// Recursively add relative paths and terminal nodes into the module. The relative paths
    /// need to be resolved into terminal paths later.
    fn resolve_types_recursive(&mut self, items: &Vec<syn::Item>) {
        for item in items.iter() {
            match item {
                Item::Const(item) => {
                    let mut path = self.symbol_tree_node.borrow().path.clone();
                    path.push(item.ident.clone());
                    match &*item.expr {
                        syn::Expr::Lit(lit) => {
                            let symbol = TypeNode::new(is_public(&item.vis), Terminal::Literal(lit.lit.clone()), true, path);
                            self.symbol_tree_node.borrow_mut().insert(&item.ident, ModuleItem::Type(symbol)).expect_none("type node shouldn't apprear more than once");
                        }
                        _ => self.add_definition_symbol(&item.ident, &item.vis),
                    }
                }
                Item::Enum(item) => {
                    self.add_definition_symbol(&item.ident, &item.vis);
                }
                Item::Mod(item) => {
                    if let Some((_, items)) = &item.content {
                        self.symbol_tree_node = self.symbol_tree_node.add_module(&item.ident, &item.vis);
                        self.resolve_types_recursive(items);
                        self.symbol_tree_node = self.symbol_tree_node.parent().unwrap();
                    }
                }
                Item::Struct(item) => {
                    self.add_definition_symbol(&item.ident, &item.vis);
                }
                Item::Trait(item) => {
                    self.add_definition_symbol(&item.ident, &item.vis);
                }
                Item::Type(item) => {
                    self.add_definition_symbol(&item.ident, &item.vis);
                }
                Item::Union(item) => {
                    self.add_definition_symbol(&item.ident, &item.vis);
                }
                Item::Fn(item) => {
                    self.add_definition_symbol(&item.sig.ident, &item.vis);
                }
                Item::Use(item) => {
                    let path = vec!{};
                    self.resolve_types_in_usetree_recursive(&item.tree, &item.vis, path, item.leading_colon.is_some());
                }
                _ => {},
            }
        }
    }

    fn resolve_types_in_usetree_recursive(&mut self, tree: &UseTree, vis: &Visibility, mut path: Vec<Ident>, leading_colon: bool) {
        match tree {
            // e.g. `a::b::{Baz}`. A Tree.
            syn::UseTree::Path(tree) => {
                if tree.ident == "self" {
                    path = self.symbol_tree_node.borrow().path.clone();
                } else {
                    path.push(tree.ident.clone());
                }
                self.resolve_types_in_usetree_recursive(&tree.tree, vis, path, leading_colon);
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
                panic!("Use globe is disallowed in IDL. For example, you cannot do `use foo::*`. You did: {:#?}, {:?}", tree, path)
            }
            // e.g. `{b::{Barc}, Car}`.
            syn::UseTree::Group(tree) => {
                for tree in &tree.items {
                    self.resolve_types_in_usetree_recursive(&tree, vis, path.clone(), leading_colon);
                }
            }
        }
    }

    /// Add a symbol from a `use` state,emt to the corrent scope.
    fn add_use_symbol(&mut self, ident: &Ident, vis: &Visibility, path: Vec<Ident>, leading_colon: bool) {
        // If the path doesn't start with "crate" or "super", this means that it comes from
        // an external library, which means we should mark it as terminal.
        let first_segment = &path[0];
        let terminal = match (first_segment != "crate") && (first_segment != "super") {
            true => Terminal::ForeignType,
            false => Terminal::None,
        };

        // Add the symbol to the module.
        let symbol = TypeNode::new(is_public(vis), terminal, leading_colon, path);
        self.symbol_tree_node.borrow_mut().insert(ident, ModuleItem::Type(symbol)).expect_none("type node shouldn't apprear more than once");
    }

    /// Add a symbol that's defined in the current scope. The symbol is terminal.
    fn add_definition_symbol(&mut self, ident: &Ident, vis: &Visibility) {
        // Construct fully qualified path.
        let mut path = self.symbol_tree_node.borrow().path.clone();
        path.push(ident.clone());

        // Add symbol.
        let symbol = TypeNode::new(is_public(vis), Terminal::Type, true, path);
        self.symbol_tree_node.borrow_mut().insert(ident, ModuleItem::Type(symbol.clone())).expect_none(&format!("Trying to insert {:?} but already exist. Type node shouldn't apprear more than once", symbol));
    }
}
