use std::{collections::HashMap, hash::Hash, ops::{Deref, DerefMut}, rc::Rc};

use proc_macro2::Span;
use quote::format_ident;
use syn::{Ident, PathSegment, Visibility};

/// A tree that contains all the symbols in the AST.
/// Each node is a module
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
    inner: Rc<ModuleNodeInner>,
}

impl ModuleNode {
    pub fn new(ident: &Ident, parent: Option<ModuleNode>) -> Self {
        Self {
            inner: Rc::new(ModuleNodeInner::new(ident, parent))
        }
    }

    /// Create a new child module and returns a reference to it. 
    pub fn push(&mut self, ident: &Ident) -> Self {
        // Attempt to insert a new node into children. Noop if there already exist one with the same
        // ident.
        let me = Some(self.clone());
        self.children.insert(ident.clone(), ModuleItem::Module(Self::new(&ident, me)));

        // We might have an existing node already so we need to do a lookup.
        match self.children.get(ident).unwrap() {
            ModuleItem::Module(md) => md.clone(),
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
        &self.inner
    }
}

impl DerefMut for ModuleNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Rc::get_mut(&mut self.inner).unwrap()
    }
}

#[derive(Debug)]
pub struct ModuleNodeInner {
    /// Caching the path to thid module so the user doesn't have to do the lookup manually.
    path: Vec<Ident>,
    /// The `super`, aka parent, module. 
    parent: Option<ModuleNode>,
    /// 
    children: HashMap<Ident, ModuleItem>
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

    pub fn add_symbol(&mut self, ident: &Ident, visibility: &Visibility) {
        // We assume that inherited mutability means private.
        if *visibility == Visibility::Inherited {
            self.children.insert(ident.clone(), ModuleItem::Type);
        } else {
            self.children.insert(ident.clone(), ModuleItem::PubType);
        }
    }
}

#[derive(Debug)]
pub struct ModuleTypeItem {
    // Fully qualified path
    path: Vec<Ident>,
    // Whether the type is public.
    public: bool,
}

#[derive(Debug)]
pub enum ModuleItem {
    /// Represent a non-module symbol in the module.
    /// The tuple is `(fully-qualified-path, is-private)
    Type(()),
    Module(ModuleNode),
}