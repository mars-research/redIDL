use std::{collections::HashMap, hash::Hash, ops::{Deref, DerefMut}, rc::Rc};

use syn::{Ident, PathSegment, Visibility};

pub struct ModuleTree {
    pub root: ModuleNode,
}

impl ModuleTree {
    pub fn new() -> Self {
        Self {
            root: ModuleNode::new(None),
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
    pub fn new(parent: Option<ModuleNode>) -> Self {
        Self {
            inner: Rc::new(ModuleNodeInner::new(parent))
        }
    }

    /// Create a new child module and returns a reference to it. 
    pub fn push(&mut self, ident: &Ident) -> Self {
        // Attempt to insert a new node into children. Noop if there already exist one with the same
        // ident.
        let me = Some(self.clone());
        self.children.insert(ident.clone(), ModuleItem::Module(Self::new(me)));

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
    parent: Option<ModuleNode>,
    children: HashMap<Ident, ModuleItem>
}

impl ModuleNodeInner {
    fn new(parent: Option<ModuleNode>) -> Self {
        Self {
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
pub enum ModuleItem {
    Type,
    PubType,
    Module(ModuleNode),
}