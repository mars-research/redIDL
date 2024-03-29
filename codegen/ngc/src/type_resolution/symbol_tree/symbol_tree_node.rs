use super::module::Module;

use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use syn::{Ident, Item, Lit};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct SymbolTreeNodeInner {
    /// Identifier.
    pub ident: Ident,
    /// Whether the type is public.
    pub public: bool,
    /// If `self` is a module, `self.parent` is the `super` module, aka parent module.
    /// Otherwise, it is the module it belongs to.
    #[derivative(Debug = "ignore")]
    pub parent: Option<SymbolTreeNode>,
    /// If true, this node is mapped to its definition and no further resolution is needed.
    pub terminal: Option<Terminal>,
    /// Whether the path has a leading colon. If it has one, it usually means that it's absolute.
    pub leading_colon: bool,
    /// Current best known absolute path of the symbol.
    pub path: Vec<Ident>,
}

impl SymbolTreeNodeInner {
    /// Get the parent module from this node.
    pub fn get_parent_module(&self) -> Module {
        match &self
            .parent
            .as_ref()
            .unwrap()
            .borrow()
            .terminal
            .as_ref()
            .unwrap()
            .definition
        {
            Definition::Module(md) => md.clone(),
            _ => panic!("Expecting the parent of a module to be a module"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SymbolTreeNode {
    inner: Rc<RefCell<SymbolTreeNodeInner>>,
}

impl SymbolTreeNode {
    pub fn new(
        ident: Ident,
        public: bool,
        parent: Option<SymbolTreeNode>,
        terminal: Option<Terminal>,
        leading_colon: bool,
        path: Vec<Ident>,
    ) -> Self {
        Self {
            inner: Rc::new(RefCell::new(SymbolTreeNodeInner {
                ident,
                public,
                parent,
                terminal,
                leading_colon,
                path,
            })),
        }
    }

    pub fn borrow(&self) -> Ref<SymbolTreeNodeInner> {
        RefCell::borrow(&self.inner)
    }

    pub fn borrow_mut(&self) -> RefMut<SymbolTreeNodeInner> {
        RefCell::borrow_mut(&self.inner)
    }

    /// Returns the root node of the tree.
    pub fn root(&self) -> Self {
        match self.borrow().parent.as_ref() {
            Some(parent) => parent.root(),
            None => self.clone(),
        }
    }
}

/// A terminal node.
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct Terminal {
    /// The node where the terminal node is defined.
    #[derivative(Debug = "ignore")]
    pub node: SymbolTreeNode,
    /// The definition of the terminal node.
    pub definition: Definition,
}

impl Terminal {
    pub fn new(node: SymbolTreeNode, definition: Definition) -> Self {
        Self { node, definition }
    }
}

/// The definition of a terminal node.
#[derive(Debug, Clone)]
pub enum Definition {
    /// A builtin type like primitive types and Option
    Builtin,
    /// A type. It could be a struct, a trait, or a function.
    Type(Item),
    /// An imported type that comes from a `use` statment on an external library.
    /// It contains its import path.
    ForeignType(Vec<Ident>),
    /// A module.
    Module(Module),
    /// A literal.
    Literal(Lit),
}
