use super::module::Module;
use super::symbol_tree_node::*;

use std::{
    borrow::Borrow,
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    hash::Hash,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use proc_macro2::Span;
use quote::format_ident;
use syn::{
    Ident, Item, ItemFn, ItemStruct, ItemTrait, Lit, LitInt, PathSegment, VisPublic, Visibility,
};

/// A tree that contains all the symbols in the AST.
/// Each node is a module
#[derive(Debug, Clone)]
pub struct SymbolTree {
    pub root: SymbolTreeNode,
}

impl SymbolTree {
    pub fn new() -> Self {
        let mut root = SymbolTreeNode::new(
            true,
            None,
            None,
            true,
            vec![format_ident!("crate")],
        );
        let definition = Definition::Module(Module::new(&format_ident!("crate"), root.clone()));
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
