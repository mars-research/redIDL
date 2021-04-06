use super::{symbol_tree::*, utils::is_public};
use crate::expect;
use crate::type_resolution::symbol_tree::PATH_MODIFIERS;
use log::{debug, info, trace};



use std::{
    borrow::{Borrow, BorrowMut},
};
use syn::{
    spanned::Spanned, File, FnArg, Ident, Item, ItemTrait, ItemUse, Path, PathSegment, ReturnType,
    TraitItem, TraitItemMethod, Type, UseTree, Visibility,
};

const DEFINITION_POPULATION_TARGET: &str = "definition_population";
const RELATIVE_PATH_TARGET: &str = "relative_path_resolution";

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
    fn resolve_relative_paths_recursive_for_symbol_tree_node(&mut self, node: SymbolTreeNode) {
        let node_ref = node.borrow();
        trace!(
            target: RELATIVE_PATH_TARGET,
            "Resolving relative path for {:?} in {:?}",
            node_ref,
            self.current_module.borrow().path
        );

        // If the node is a non-module terminal node, nothing need to be done here.
        // If the node is a module, recursively go into the module and resolve relative paths there.
        if let Some(terminal) = &node_ref.terminal {
            match &terminal.definition {
                Definition::Module(item) => {
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
                    return;
                }
                Definition::Builtin
                | Definition::Type(_)
                | Definition::ForeignType(_)
                | Definition::Literal(_) => {
                    // noop. Non-module terminal node; no further resolution is needed.
                    return;
                }
            }
        }

        // Walk the relative path and try resolving the path.
        // Save the previous node because we there's no way to know the parent of a type currently.
        let mut current_node = match node_ref.leading_colon {
            true => self.symbol_tree.root.clone(),
            false => self.current_module.borrow().node.clone(),
        };

        for path_segment in &node_ref.path {
            trace!("Resolving path segment {:?}", path_segment);
            // Borrow it seperately so that we can assign to `current_node` later.
            let terminal = current_node.borrow().terminal.clone();

            // Resolve the path_segment into a node.
            match &expect!(
                terminal.as_ref(),
                "Expecting a module, found non-terminal: {:?}",
                current_node
            )
            .definition
            {
                Definition::Type(_) => panic!(
                    "Resolving {:#?} for {:#?}. Node {:#?} is a symbol and cannot have child.",
                    path_segment, node_ref.path, current_node
                ),
                Definition::Module(md) => {
                    let md = md.borrow();
                    let next_node = md.get(path_segment);
                    let next_node = crate::expect!(
                        next_node,
                        "When resolving {:?}, ident {:?} is not found in {:#?}",
                        node_ref.path,
                        path_segment,
                        md
                    );
                    assert!(next_node.borrow().public);
                    current_node = next_node.clone();
                }
                _ => panic!(),
            }
        }

        // Keep resolving recursively if the node that we get from resolving is not
        // terminal.
        // If the node is a module, we can treat it as terminal. It's up to the user to
        // resolve their types that in the module.
        if current_node.borrow().terminal.is_none() {
            self.resolve_relative_paths_recursive_for_symbol_tree_node(current_node.clone());
        }
        assert!(
            current_node.borrow().terminal.is_some(),
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
        drop(node_ref);

        // Update the node to a terminal node.
        let mut node = node.borrow_mut();
        let resolved_node = current_node.borrow();
        node.terminal = resolved_node.terminal.clone();
        node.path = resolved_node.path.clone();
    }

    /// Resolve all relative paths generated `resolve_types_recursive` by into terminal
    /// paths.
    fn resolve_relative_paths_recursive_for_module(&mut self, module: Module) {
        let module_path = &module.borrow().path;
        info!(
            target: RELATIVE_PATH_TARGET,
            "Resolving relative path for module {:?}", module_path
        );
        for (ident, child) in module.borrow().iter() {
            trace!(
                "Resolving relative path for symbol {:?} in module {:?}",
                ident,
                module_path
            );
            // If the module is a path modifier, noop.
            if PATH_MODIFIERS.get(&ident.to_string()).is_some() {
                trace!("Encountered path modifiler {:?}; noop", ident);
                continue;
            }
            self.resolve_relative_paths_recursive_for_symbol_tree_node(child.clone());
        }
    }

    /// Recursively add relative paths and terminal nodes into the module. The relative paths
    /// need to be resolved into terminal paths later.
    fn resolve_types_recursive(&mut self, items: &[syn::Item]) {
        for og_item in items.iter() {
            match og_item {
                Item::Const(item) => {
                    let mut path = self.current_module.borrow().node.borrow().path.clone();
                    path.push(item.ident.clone());
                    match &*item.expr {
                        syn::Expr::Lit(lit) => {
                            // Create a new node.
                            let node = SymbolTreeNode::new(
                                item.ident.clone(),
                                is_public(&item.vis),
                                Some(self.current_module.borrow().node.clone()),
                                None,
                                true,
                                path,
                            );
                            // Update its terminal
                            node.borrow_mut().terminal = Some(Terminal::new(
                                node.clone(),
                                Definition::Literal(lit.lit.clone()),
                            ));
                            // Insert the node into the current module.
                            self.current_module
                                .borrow_mut()
                                .insert(item.ident.clone(), node)
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
                panic!("Use globe is disallowed in IDL. For example, you cannot do `use foo::*`. Violating code: {:#?}, {:?}", tree, path)
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

    /// Add a symbol from a `use` statement to the corrent scope.
    fn add_use_symbol(
        &mut self,
        ident: &Ident,
        vis: &Visibility,
        path: Vec<Ident>,
        leading_colon: bool,
    ) {
        // Create a new node for the symbol.
        let node = SymbolTreeNode::new(
            ident.clone(),
            is_public(vis),
            Some(self.current_module.borrow().node.clone()),
            None,
            leading_colon,
            path.clone(),
        );

        // If the path doesn't start with "crate" or "super", this means that it comes from
        // an external library, which means we should mark it as terminal.
        let first_segment = &path[0];
        if PATH_MODIFIERS.get(&first_segment.to_string()).is_none() {
            node.borrow_mut().terminal =
                Some(Terminal::new(node.clone(), Definition::ForeignType(path)));
        };

        // Add the symbol to the module.
        self.current_module
            .borrow_mut()
            .insert(ident.clone(), node)
            .expect_none("type node shouldn't apprear more than once");
    }

    /// Add a symbol that's defined in the current scope. The symbol is terminal.
    fn add_definition_symbol(&mut self, ident: &Ident, vis: &Visibility, definition: &Item) {
        // Construct fully qualified path.
        let mut path = self.current_module.borrow().node.borrow().path.clone();
        path.push(ident.clone());

        // Add symbol.
        let node = SymbolTreeNode::new(
            ident.clone(),
            is_public(vis),
            Some(self.current_module.borrow().node.clone()),
            None,
            true,
            path,
        );
        node.borrow_mut().terminal = Some(Terminal::new(
            node.clone(),
            Definition::Type(definition.clone()),
        ));
        self.current_module.borrow_mut().insert(ident.clone(), node.clone()).expect_none(&format!("Trying to insert {:?} but already exist. Type node shouldn't apprear more than once", node));
    }
}
