use std::{cell::RefCell, collections::HashMap, hash::Hash, ops::{Deref, DerefMut}, rc::Rc};

use proc_macro2::Span;
use quote::format_ident;
use syn::{Ident, PathSegment, Visibility};
use super::utils::is_public;

/// A tree that contains all the symbols in the AST.
/// Each node is a module
#[derive(Debug)]
pub struct ModuleTree {
    pub root: ModuleNode,
}

impl ModuleTree {
    pub fn new() -> Self {
        Self {
            root: ModuleNode::new(&format_ident!("create"), None),
        }
    }

    pub fn clear(&mut self) {
        self.root.clear();
    }
}

#[derive(Debug, Clone)]
pub struct ModuleNode {
    inner: Rc<RefCell<ModuleNodeInner>>,
}

impl ModuleNode {
    pub fn new(ident: &Ident, parent: Option<ModuleNode>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ModuleNodeInner::new(ident, parent)))
        }
    }

    /// Create a new child module and returns a reference to it. 
    pub fn push(&mut self, ident: &Ident, vis: &Visibility) -> Self {
        // Attempt to insert a new node into children. Noop if there already exist one with the same
        // ident.
        let me = Some(self.clone());
        let module_item = ModuleItem {
            public: is_public(vis),
            terminal: true,
            item_type: ModuleItemType::Module(Self::new(&ident, me))
        };
        self.children.insert(ident.clone(), module_item);

        // We might have an existing node already so we need to do a lookup.
        match &self.children.get(ident).unwrap().item_type {
            ModuleItemType::Module(md) => md.clone(),
            _ => unreachable!("Should be a module."),
        }
    }

    pub fn parent(&self) -> Option<Self> {
        self.parent.clone()
    }
}

impl Deref for ModuleNode {
    type Target = ModuleNodeInner;

    fn deref(&self) -> &Self::Target {
        &self.inner.borrow()
    }
}

impl DerefMut for ModuleNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.get_mut()
    }
}

#[derive(Debug)]
pub struct ModuleNodeInner {
    /// Caching the path to thid module so the user doesn't have to do the lookup manually.
    pub path: Vec<Ident>,
    /// The `super`, aka parent, module. 
    pub parent: Option<ModuleNode>,
    /// All items in this module, including symbols and modules.
    pub children: HashMap<Ident, ModuleItem>
}

impl ModuleNodeInner {
    fn new(ident: &Ident, parent: Option<ModuleNode>) -> Self {
        let mut path = vec!{};
        if let Some(parent) = parent.as_ref() {
            path = parent.path.clone();
        }
        path.push(ident.clone());
        Self {
            path,
            parent,
            children: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.parent.take();
        self.children.clear();
    }

    pub fn insert(&mut self, ident: &Ident, module_item: ModuleItem) -> Option<ModuleItem> {
        self.children.insert(ident.clone(), module_item)
    }
}

// TBH, I think `public` and `terminal` should be members of `item_type`.
/// Represent an item in the module tree. It could be a module or a symbol.
#[derive(Debug, Clone)]
pub struct ModuleItem {
    /// Whether the type is public.
    pub public: bool,
    /// If true, this node is mapped to its definition and no further resolution is needed.
    pub terminal: bool,
    /// Fully qualified path
    pub item_type: ModuleItemType,
}

#[derive(Debug, Clone)]
pub enum ModuleItemType {
    /// Represent a non-module symbol in the module. Contains the most-qualified path that we know
    /// so far.
    Symbol(SymbolNode),
    /// A child module.
    Module(ModuleNode),
}

#[derive(Debug)]
pub struct SymbolNodeInner {
    pub leading_colon: bool,
    pub path: Vec<Ident>,
}

#[derive(Debug, Clone)]
pub struct SymbolNode {
    inner: Rc<RefCell<SymbolNodeInner>>,
}

impl SymbolNode {
    pub fn new(leading_colon: bool, path: Vec<Ident>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(SymbolNodeInner {
                leading_colon,
                path,
            }))
        }
    }
}

impl Deref for SymbolNode {
    type Target = SymbolNodeInner;

    fn deref(&self) -> &Self::Target {
        &self.inner.borrow()
    }
}

impl DerefMut for SymbolNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.get_mut()
    }
}