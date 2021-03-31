use super::*;

use std::{
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    hash::Hash,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use super::super::utils::is_public;
use log::{debug, trace};
use proc_macro2::Span;
use quote::format_ident;
use syn::{
    Ident, Item, ItemFn, ItemStruct, ItemTrait, Lit, LitInt, PathSegment, VisPublic, Visibility,
};

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct ModuleInner {
    /// Absolute path to the module.
    /// This is the same as `self.node.path`.
    /// We are storing a seperate copy here to bypass the owernship check.
    pub path: Vec<Ident>,

    /// The node which this module lives.
    #[derivative(Debug = "ignore")]
    pub node: SymbolTreeNode,

    /// All items in this module, including symbols and modules.
    pub children: HashMap<Ident, SymbolTreeNode>,
}

/// Generate default mappings for builtin types like u8.
macro_rules! generate_builtin_mapping {
    ($($arg:literal),*) => (
        {
            let mut hashmap = HashMap::new();

            $( 
                {
                    let mut node = SymbolTreeNode::new(false, None, None, true, vec![format_ident!($arg)]);
                    node.borrow_mut().terminal = Some(Terminal::new(node.clone(), Definition::Builtin));
                    hashmap.insert(format_ident!($arg), node);
                }
            )*


            hashmap
        }
    );
}

impl ModuleInner {
    fn new(ident: &Ident, node: SymbolTreeNode) -> Self {
        let path = node.borrow().path.clone();
        Self {
            node,
            path,
            children: generate_builtin_mapping!(
                "bool", "u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64", "usize", "Option"
            ),
        }
    }

    pub fn clear(&mut self) {
        self.children.clear();
    }

    pub fn insert(&mut self, ident: &Ident, node: SymbolTreeNode) -> Option<SymbolTreeNode> {
        trace!(
            "Adding {:?} to module {:?} with node {:?}",
            ident,
            self.path,
            node
        );
        self.children.insert(ident.clone(), node)
    }
}

#[derive(Debug, Clone)]
pub struct Module {
    pub inner: Rc<RefCell<ModuleInner>>,
}

impl Module {
    pub fn new(ident: &Ident, node: SymbolTreeNode) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ModuleInner::new(ident, node))),
        }
    }

    /// Create a new child module and returns a reference to it.
    pub fn create_module(&mut self, ident: &Ident, vis: &Visibility) -> Module {
        let mut path = Self::borrow(self).node.borrow().path.clone();
        path.push(ident.clone());
        let mut new_node = SymbolTreeNode::new(
            is_public(vis),
            Some((Self::borrow(self).node.clone())),
            None,
            true,
            path,
        );
        let new_module = Self::new(ident, new_node.clone());
        let definition = Definition::Module(new_module.clone());
        new_node.borrow_mut().terminal = Some(Terminal::new(new_node.clone(), definition));
        self.borrow_mut()
            .children
            .insert(ident.clone(), new_node)
            .unwrap_none();
        new_module
    }

    pub fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    pub fn borrow(&self) -> Ref<ModuleInner> {
        RefCell::borrow(&self.inner)
    }

    pub fn borrow_mut(&self) -> RefMut<ModuleInner> {
        RefCell::borrow_mut(&self.inner)
    }
}
