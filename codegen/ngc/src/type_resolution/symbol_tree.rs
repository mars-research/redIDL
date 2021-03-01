use std::{borrow::Borrow, cell::{Ref, RefCell, RefMut}, collections::HashMap, hash::Hash, ops::{Deref, DerefMut}, rc::Rc};

use proc_macro2::Span;
use quote::format_ident;
use syn::{Ident, PathSegment, VisPublic, Visibility};
use super::utils::is_public;

/// A tree that contains all the symbols in the AST.
/// Each node is a module
#[derive(Debug, Clone)]
pub struct SymbolTree {
    pub root: ModuleItem,
}

impl SymbolTree {
    pub fn new() -> Self {
        Self {
            root: ModuleItem::Module(ModuleNode::new(true, SymbolTreeNode::new(&format_ident!("crate"), None))),
        }
    }

    /// Returns the root of the tree in as a `SymbolTreeNode`.
    pub fn root_symbol_tree_node(&self) -> SymbolTreeNode { 
        match &self.root {
            ModuleItem::Module(md) => md.borrow().module.clone(),
            ModuleItem::Type(_) => unreachable!()
        }
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct SymbolTreeNodeInner {
    /// Caching the path to thid module so the user doesn't have to do the lookup manually.
    pub path: Vec<Ident>,
    /// The `super`, aka parent, module.
    #[derivative(Debug="ignore")] 
    pub parent: Option<SymbolTreeNode>,
    /// All items in this module, including symbols and modules.
    pub children: HashMap<Ident, ModuleItem>,
}

impl SymbolTreeNodeInner {
    fn new(ident: &Ident, parent: Option<SymbolTreeNode>) -> Self {
        let mut path = vec!{};
        if let Some(parent) = parent.as_ref() {
            path = parent.borrow().path.clone();
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
    pub inner: Rc<RefCell<SymbolTreeNodeInner>>,
}

impl SymbolTreeNode {
    pub fn new(ident: &Ident, parent: Option<SymbolTreeNode>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(SymbolTreeNodeInner::new(ident, parent)))
        }
    }

    /// Create a new child module and returns a reference to it. 
    pub fn add_module(&mut self, ident: &Ident, vis: &Visibility) -> SymbolTreeNode {
        // Attempt to insert a new node into children. Noop if there already exist one with the same
        // ident.
        let me = Some(self.clone());
        let new_module = Self::new(ident, me);
        let new_module = ModuleItem::Module(ModuleNode::new(is_public(vis), new_module));
        self.borrow_mut().children.insert(ident.clone(), new_module);

        // We might have an existing node already so we need to do a lookup.
        match &RefCell::borrow(&self.inner).children.get(ident).unwrap() {
            ModuleItem::Module(md) => md.borrow().module.clone(),
            _ => unreachable!("Should be a module."),
        }
    }

    pub fn parent(&self) -> Option<Self> {
        RefCell::borrow(&self.inner).parent.clone()
    }

    pub fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    pub fn borrow(&self) -> Ref<SymbolTreeNodeInner> {
        RefCell::borrow(&self.inner)
    }

    pub fn borrow_mut(&self) -> RefMut<SymbolTreeNodeInner> {
        RefCell::borrow_mut(&self.inner)
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
pub struct ModuleNode {
    inner: Rc<RefCell<ModuleNodeInner>>,
}

impl ModuleNode {
    pub fn new(public: bool, module: SymbolTreeNode) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ModuleNodeInner::new(public, module)))
        }
    }

    pub fn borrow(&self) -> Ref<ModuleNodeInner> {
        RefCell::borrow(&self.inner)
    }

    pub fn borrow_mut(&self) -> RefMut<ModuleNodeInner> {
        RefCell::borrow_mut(&self.inner)
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

    pub fn borrow(&self) -> Ref<TypeNodeInner> {
        RefCell::borrow(&self.inner)
    }

    pub fn borrow_mut(&self) -> RefMut<TypeNodeInner> {
        RefCell::borrow_mut(&self.inner)
    }
}