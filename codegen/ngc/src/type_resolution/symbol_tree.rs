use std::{cell::RefCell, collections::HashMap, hash::Hash, ops::{Deref, DerefMut}, rc::Rc};

use proc_macro2::Span;
use quote::format_ident;
use syn::{Ident, PathSegment, VisPublic, Visibility};
use super::utils::is_public;

/// A tree that contains all the symbols in the AST.
/// Each node is a module
#[derive(Debug)]
pub struct SymbolTree {
    pub root: SymbolTreeNode,
}

impl SymbolTree {
    pub fn new() -> Self {
        Self {
            root: SymbolTreeNode::new(&format_ident!("crate"), None),
        }
    }

    pub fn clear(&mut self) {
        self.root.clear();
    }
}

#[derive(Debug, Clone)]
pub struct SymbolTreeNodeInner {
    /// Caching the path to thid module so the user doesn't have to do the lookup manually.
    pub path: Vec<Ident>,
    /// The `super`, aka parent, module. 
    pub parent: Option<SymbolTreeNode>,
    /// All items in this module, including symbols and modules.
    pub children: HashMap<Ident, ModuleItem>,
}

impl SymbolTreeNodeInner {
    fn new(ident: &Ident, parent: Option<SymbolTreeNode>) -> Self {
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

#[derive(Debug, Clone)]
pub struct SymbolTreeNode {
    inner: Rc<RefCell<SymbolTreeNodeInner>>,
}

impl Deref for SymbolTreeNode {
    type Target = SymbolTreeNodeInner;

    fn deref(&self) -> &Self::Target {
        &self.inner.borrow()
    }
}

impl DerefMut for SymbolTreeNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.get_mut()
    }
}

impl SymbolTreeNode {
    pub fn new(ident: &Ident, parent: Option<SymbolTreeNode>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(SymbolTreeNodeInner::new(ident, parent)))
        }
    }

    /// Create a new child module and returns a reference to it. 
    pub fn add_module(&mut self, ident: &Ident, vis: &Visibility) -> ModuleNode {
        // Attempt to insert a new node into children. Noop if there already exist one with the same
        // ident.
        let me = Some(self.clone());
        let new_module = Self::new(ident, me);
        let new_module = ModuleItem::Module(ModuleNode::new(is_public(vis), new_module));
        self.children.insert(ident.clone(), new_module);

        // We might have an existing node already so we need to do a lookup.
        match &self.children.get(ident).unwrap() {
            ModuleItem::Module(md) => md.clone(),
            _ => unreachable!("Should be a module."),
        }
    }

    pub fn parent(&self) -> Option<Self> {
        self.parent.clone()
    }
}


#[derive(Debug, Clone)]
pub struct ModuleNode {
    inner: Rc<RefCell<ModuleNodeInner>>,
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

impl ModuleNode {
    fn new(public: bool, module: SymbolTreeNode) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ModuleNodeInner::new(public, module)))
        }
    }
}


#[derive(Debug)]
pub struct ModuleNodeInner {
    /// Whether the module is public.
    pub public: bool,
    /// The module itself.
    pub module: SymbolTreeNode,
}

impl ModuleNodeInner {
    fn new(public: bool, module: SymbolTreeNode) -> Self {
        Self {
            public,
            module,
        }
    }   
}

#[derive(Debug, Clone)]
pub enum ModuleItem {
    /// Represent a non-module symbol in the module. Contains the most-qualified path that we know
    /// so far.
    Type(TypeNode),
    /// A child module.
    Module(ModuleNode),
}

#[derive(Debug)]
pub struct TypeNodeInner {
    /// Whether the type is public.
    pub public: bool,
    /// If true, this node is mapped to its definition and no further resolution is needed.
    pub terminal: bool,
    /// Whether the path has a leading colon. Usually means that it's absolute.
    pub leading_colon: bool,
    /// Current best known absolute path of the symbol.
    pub path: Vec<Ident>,
}

#[derive(Debug, Clone)]
pub struct TypeNode {
    inner: Rc<RefCell<TypeNodeInner>>,
}

impl TypeNode {
    pub fn new(public: bool, terminal: bool, leading_colon: bool, path: Vec<Ident>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(TypeNodeInner {
                public,
                terminal,
                leading_colon,
                path,
            }))
        }
    }
}

impl Deref for TypeNode {
    type Target = TypeNodeInner;

    fn deref(&self) -> &Self::Target {
        &self.inner.borrow()
    }
}

impl DerefMut for TypeNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.get_mut()
    }
}