use super::module::Module;
use super::symbol_tree_node::*;

use std::{
    borrow::Borrow,
};


use quote::format_ident;


/// A tree that contains all the symbols in the AST.
/// Each node is a module
#[derive(Debug, Clone)]
pub struct SymbolTree {
    pub root: SymbolTreeNode,
}

impl SymbolTree {
    pub fn new() -> Self {
        let root_ident = &format_ident!("crate");
        let root = SymbolTreeNode::new(
            root_ident.clone(),
            true,
            None,
            None,
            true,
            vec![format_ident!("crate")],
        );
        let definition = Definition::Module(Module::new(root_ident, root.clone()));
        root.borrow_mut().terminal = Some(Terminal::new(root.clone(), definition));
        Self { root }
    }

    /// Returns the root of the tree in as a `SymbolTreeNode`.
    pub fn root_module(&self) -> Module {
        match &self.root.borrow().terminal.as_ref().unwrap().definition {
            Definition::Module(md) => md.clone(),
            _ => panic!(),
        }
    }
}
