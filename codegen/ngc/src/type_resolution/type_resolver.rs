use super::{symbol_tree::*, utils::is_public};
use log::{debug, info};
use mem::replace;
use std::mem;
use std::rc::Rc;
use std::{
    borrow::{Borrow, BorrowMut},
    collections::{HashMap, HashSet},
    thread::current,
};
use syn::{
    spanned::Spanned, File, FnArg, Ident, Item, ItemTrait, ItemUse, Path, PathSegment, ReturnType,
    TraitItem, TraitItemMethod, Type, UseTree, Visibility,
};

const DEFINITION_POPULATION_TARGET: &'static str = "definition_population";
const RELATIVE_PATH_TARGET: &'static str = "relative_path_resolution";


pub struct TypeResolver {
    /// A stack of maps a PathSegment to its fully qualified path.
    symbol_tree: SymbolTree,
    /// The current module(aka current frame).
    current_module: Module,
}

impl TypeResolver {
    pub fn new() -> Self {
        let symbol_tree = SymbolTree::new();
        let root = symbol_tree.root_module();
        Self {
            symbol_tree,
            current_module: root,
        }
    }

    /// Takes a AST and returns a list of fully-qualified paths of all `RRef`ed types.
    pub fn resolve_types(mut self, ast: &File) -> SymbolTree {
        self.resolve_types_recursive(&ast.items);
        self.resolve_relative_paths_recursive_for_module(self.current_module.clone());
        self.symbol_tree
    }

    /// Returns the resolved/terminal path of `module_item`.
    fn resolve_relative_paths_recursive_for_module_item(&mut self, node: SymbolTreeNode) {
        let mut node_ref = node.borrow();
        debug!(
            target: RELATIVE_PATH_TARGET,
            "Resolving relative path for {:?} in {:?}",
            node_ref.path,
            self.current_module.borrow().path
        );
        let resolved_node = match &node_ref.terminal {
            Terminal::Module(item) => {
                // Go to the children frame and do recursive call.
                let old_frame = self.current_module.clone();
                self.current_module = item.clone();
                self.resolve_relative_paths_recursive_for_module(self.current_module.clone());
                // Pop the frame and return back to the old frame.
                let parent_frame = self
                    .current_module
                    .borrow()
                    .node
                    .borrow()
                    .get_parent_module();

                self.current_module = parent_frame;
                // Sanity check that we are actually restoring to the correct frame.
                assert!(old_frame.same(&self.current_module));

                // We don't need to update the path
                None
            }
            Terminal::None => {
                // Walk the relative path and try resolving the path.
                // Save the previous node because we there's no way to know the parent of a type currently.
                let mut previous_node = None;
                let mut current_node = match node_ref.leading_colon {
                    true => self.symbol_tree.root.clone(),
                    false => node.clone(),
                };
                let path = {
                    let mut path = node_ref.path.clone();
                    if path[0].to_string() == "crate" {
                        current_node = self.symbol_tree.root.clone();
                        path.remove(0);
                    } else if path[0].to_string() == "super" {
                        current_node = node.borrow().parent.as_ref().unwrap().clone();
                        path.remove(0);
                    } else if path[0].to_string() == "self" {
                        path.remove(0);
                    }
                    path
                };
                for path_segment in &path {
                    // Borrow it seperately so that we can assign to `current_node` later.
                    let terminal = current_node.borrow().terminal.clone();

                    match terminal {
                        Terminal::Type(_) => panic!("Resolving {:#?} for {:#?}. Node {:#?} is a symbol and cannot have child.", path_segment, node_ref.path, current_node),
                        Terminal::Module(md) => {
                            let md = md.borrow();
                            let next_node = md.children.get(path_segment);
                            let next_node = next_node.expect(&format!("When resolving {:?}, ident {:?} is not found in {:#?}", node_ref.path, path_segment, md));
                            assert!(next_node.borrow().public);
                            previous_node = Some(current_node.clone());
                            current_node = next_node.clone();
                        }
                        _ => panic!()
                    }
                }

                // Keep resolving recursively if the node that we get from resolving is not
                // terminal.
                // If the node is a module, we can treat it as terminal. It's up to the user to
                // resolve their types that in the module.
                if !current_node.borrow().terminal.is_terminal() {
                    let parent = match &previous_node.unwrap().borrow().terminal {
                        Terminal::Module(md) => md.clone(),
                        _ => panic!(),
                    };
                    self.resolve_relative_paths_recursive_for_module_item(current_node.clone());
                }
                assert!(
                    current_node.borrow().terminal.is_terminal(),
                    "Node is not terminal: {:#?}",
                    current_node
                );
                // Copy the terminal node and absolute path to us.
                debug!(
                    target: RELATIVE_PATH_TARGET,
                    "{:?} is resolved to {:?}",
                    node_ref.path,
                    current_node.borrow().path
                );
                Some(current_node)
            }
            Terminal::Builtin
            | Terminal::Type(_)
            | Terminal::ForeignType
            | Terminal::Literal(_) => {
                // noop. Terminal node; no further resolution is needed.
                None
            }
        };
        drop(node_ref);

        if let Some(resolved_node) = resolved_node {
            let mut node = node.borrow_mut();
            let resolved_node = resolved_node.borrow();
            node.terminal = resolved_node.terminal.clone();
            node.path = resolved_node.path.clone();
        }
    }

    /// Resolve all relative paths generated `resolve_types_recursive` by into terminal
    /// paths.
    fn resolve_relative_paths_recursive_for_module(&mut self, module: Module) {
        info!(
            target: RELATIVE_PATH_TARGET,
            "Resolving relative path for module {:?}",
            module.borrow().path[module.borrow().path.len() - 1]
        );
        for (_, child) in &module.borrow().children {
            self.resolve_relative_paths_recursive_for_module_item(child.clone());
        }
    }

    /// Recursively add relative paths and terminal nodes into the module. The relative paths
    /// need to be resolved into terminal paths later.
    fn resolve_types_recursive(&mut self, items: &Vec<syn::Item>) {
        for og_item in items.iter() {
            match og_item {
                Item::Const(item) => {
                    let mut path = self.current_module.borrow().node.borrow().path.clone();
                    path.push(item.ident.clone());
                    match &*item.expr {
                        syn::Expr::Lit(lit) => {
                            let node = SymbolTreeNode::new(
                                is_public(&item.vis),
                                Some(self.current_module.borrow().node.clone()),
                                Terminal::Literal(lit.lit.clone()),
                                true,
                                path,
                            );
                            self.current_module
                                .borrow_mut()
                                .insert(&item.ident, node)
                                .expect_none("type node shouldn't apprear more than once");
                        }
                        _ => self.add_definition_symbol(&item.ident, &item.vis, &og_item),
                    }
                }
                Item::Enum(item) => {
                    self.add_definition_symbol(&item.ident, &item.vis, &og_item);
                }
                Item::Mod(item) => {
                    if let Some((_, items)) = &item.content {
                        // Push a frame.
                        self.current_module =
                            self.current_module.create_module(&item.ident, &item.vis);
                        // Recurse into the new frame.
                        self.resolve_types_recursive(items);
                        // Pop a frame.
                        let old_frame = self
                            .current_module
                            .borrow()
                            .node
                            .borrow()
                            .get_parent_module();
                        self.current_module = old_frame;
                    }
                }
                Item::Struct(item) => {
                    self.add_definition_symbol(&item.ident, &item.vis, &og_item);
                }
                Item::Trait(item) => {
                    self.add_definition_symbol(&item.ident, &item.vis, &og_item);
                }
                Item::Type(item) => {
                    self.add_definition_symbol(&item.ident, &item.vis, &og_item);
                }
                Item::Union(item) => {
                    self.add_definition_symbol(&item.ident, &item.vis, &og_item);
                }
                Item::Fn(item) => {
                    self.add_definition_symbol(&item.sig.ident, &item.vis, &og_item);
                }
                Item::Use(item) => {
                    let path = vec![];
                    self.resolve_types_in_usetree_recursive(
                        &item.tree,
                        &item.vis,
                        path,
                        item.leading_colon.is_some(),
                    );
                }
                _ => {}
            }
        }
    }

    fn resolve_types_in_usetree_recursive(
        &mut self,
        tree: &UseTree,
        vis: &Visibility,
        mut path: Vec<Ident>,
        leading_colon: bool,
    ) {
        match tree {
            // e.g. `a::b::{Baz}`. A Tree.
            syn::UseTree::Path(tree) => {
                if tree.ident == "self" {
                    path = self.current_module.borrow().node.borrow().path.clone();
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
                    self.resolve_types_in_usetree_recursive(
                        &tree,
                        vis,
                        path.clone(),
                        leading_colon,
                    );
                }
            }
        }
    }

    /// Add a symbol from a `use` state,emt to the corrent scope.
    fn add_use_symbol(
        &mut self,
        ident: &Ident,
        vis: &Visibility,
        path: Vec<Ident>,
        leading_colon: bool,
    ) {
        // If the path doesn't start with "crate" or "super", this means that it comes from
        // an external library, which means we should mark it as terminal.
        let first_segment = &path[0];
        let terminal = match (first_segment != "crate") && (first_segment != "super") {
            true => Terminal::ForeignType,
            false => Terminal::None,
        };

        // Add the symbol to the module.
        let node = SymbolTreeNode::new(
            is_public(vis),
            Some(self.current_module.borrow().node.clone()),
            terminal,
            leading_colon,
            path,
        );
        self.current_module
            .borrow_mut()
            .insert(ident, node)
            .expect_none("type node shouldn't apprear more than once");
    }

    /// Add a symbol that's defined in the current scope. The symbol is terminal.
    fn add_definition_symbol(&mut self, ident: &Ident, vis: &Visibility, definition: &Item) {
        // Construct fully qualified path.
        let mut path = self.current_module.borrow().node.borrow().path.clone();
        path.push(ident.clone());

        // Add symbol.
        let node = SymbolTreeNode::new(
            is_public(vis),
            Some(self.current_module.borrow().node.clone()),
            Terminal::Type(definition.clone()),
            true,
            path,
        );
        self.current_module.borrow_mut().insert(ident, node.clone()).expect_none(&format!("Trying to insert {:?} but already exist. Type node shouldn't apprear more than once", node));
    }
}
