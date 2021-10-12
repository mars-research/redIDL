use super::*;

use std::{
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    fmt::Debug,
    rc::Rc,
};
use std::{collections::HashSet, fmt};

use super::super::utils::is_public;
use log::trace;

use quote::format_ident;
use syn::{Ident, Visibility};

#[derive(Clone)]
pub struct ModuleInner {
    /// Absolute path to the module.
    /// This is the same as `self.node.path`.
    /// We are storing a seperate copy here to bypass the owernship check.
    pub path: Vec<Ident>,

    /// The node which this module lives.
    pub node: SymbolTreeNode,

    /// All items in this module, including symbols and modules.
    children: HashMap<Ident, SymbolTreeNode>,
}

impl fmt::Debug for ModuleInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Print the children's keys only.
        // There's should be a more effient way to do it than turning it into a set but I just
        // couldn't get the formatter to work.
        let children_keys: HashSet<_> = self.children.keys().collect();
        let result = f
            .debug_struct("Module")
            .field("path", &self.path)
            .field("children", &children_keys)
            .finish();
        result
    }
}

/// Generate insertions of default mappings for builtin types like u8.
macro_rules! insert_builtin_mapping {
    ($hashmap:path,$($arg:literal),*) => (
        {
            $(
                {
                    let ident = format_ident!($arg);
                    let node = SymbolTreeNode::new(ident.clone(), false, None, None, true, vec![format_ident!($arg)]);
                    node.borrow_mut().terminal = Some(Terminal::new(node.clone(), Definition::Builtin));
                    $hashmap.insert(ident, node);
                }
            )*
        }
    );
}

impl ModuleInner {
    fn new(_ident: &Ident, node: SymbolTreeNode) -> Self {
        let path = node.borrow().path.clone();

        let mut module = Self {
            node: node.clone(),
            path,
            children: HashMap::new(),
        };

        // Insert mappings for builtin types.
        insert_builtin_mapping!(
            module, "bool", "u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64", "usize", "Option"
        );

        // Insert mappings for relative path.
        module.insert(format_ident!("self"), node.clone());
        module.insert(
            format_ident!("super"),
            match node.borrow().parent.as_ref() {
                Some(parent) => parent.clone(),
                None => node.clone(),
            },
        );
        module.insert(format_ident!("crate"), node.root());

        module
    }

    // Insert a new node to this module.
    pub fn insert(&mut self, ident: Ident, node: SymbolTreeNode) -> Option<SymbolTreeNode> {
        trace!(
            "Adding {:?} to module {:?} with node {:?}",
            ident,
            self.path,
            node
        );
        self.children.insert(ident, node)
    }

    // Retrive a node from this module.
    pub fn get(&self, ident: &Ident) -> Option<&SymbolTreeNode> {
        let node = self.children.get(ident);
        trace!(
            "Getting {:?} from module {:?} with node {:?}",
            ident,
            self.path,
            node
        );
        node
    }

    // Return an iterator over its children.
    pub fn iter(&self) -> std::collections::hash_map::Iter<Ident, SymbolTreeNode> {
        self.children.iter()
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
        let new_node = SymbolTreeNode::new(
            ident.clone(),
            is_public(vis),
            Some(Self::borrow(self).node.clone()),
            None,
            true,
            path,
        );
        let new_module = Self::new(ident, new_node.clone());
        let definition = Definition::Module(new_module.clone());
        new_node.borrow_mut().terminal = Some(Terminal::new(new_node.clone(), definition));
        assert!(self
            .borrow_mut()
            .children
            .insert(ident.clone(), new_node)
            .is_none());
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
