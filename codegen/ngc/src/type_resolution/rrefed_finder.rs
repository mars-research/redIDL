use mem::replace;
use syn::{File, FnArg, Item, ItemTrait, Path, PathSegment, ReturnType, TraitItem, TraitItemMethod, Type};
use std::collections::{HashMap, HashSet};
use std::mem;

use super::symbol_tree::*;

pub type PathSegments = Vec<PathSegment>;


pub struct RRefedFinder {
    /// All the fully qualified path of all `RRef`ed types.
    type_list:  HashSet<PathSegments>,
    /// The root module node, i.e. the `crate` node.
    symbol_tree: SymbolTree,
    /// The current module node that's used in recursive calls.
    symbol_tree_node: SymbolTreeNode,
}

impl RRefedFinder {
    pub fn new(symbol_tree: SymbolTree) -> Self {
        let symbol_tree_node = symbol_tree.root_symbol_tree_node();
        Self {
            type_list: HashSet::new(),
            symbol_tree: symbol_tree,
            symbol_tree_node,
        }
    }

    /// Takes a AST and returns a list of fully-qualified paths of all `RRef`ed types.
    pub fn find_rrefed(&mut self, ast: &File) -> HashSet<PathSegments> {
        self.type_list.clear();
        self.symbol_tree_node.borrow_mut().clear();
        self.find_rrefed_recursive(&ast.items);
        self.symbol_tree_node.borrow_mut().clear();
        std::mem::replace(&mut self.type_list, HashSet::new())
    }

    fn find_rrefed_recursive(&mut self, items: &Vec<syn::Item>) {
        let mut generated_items = Vec::<syn::Item>::new();
        for item in items.iter() {
            match item {
                Item::Mod(md) => {
                    if let Some((_, items)) = &md.content {
                        let next_frame = match &self.symbol_tree_node.borrow().children[&md.ident] {
                            ModuleItem::Module(md) => md.borrow().module.clone(),
                            ModuleItem::Type(_) => unreachable!("Expecting a module, not a symbol.")
                        };
                        self.symbol_tree_node = next_frame;
                        self.find_rrefed_recursive(items);
                        self.symbol_tree_node = self.symbol_tree_node.parent().unwrap();
                    }
                }
                Item::Trait(tr) => {
                    self.find_rrefed_in_trait(tr);
                }
                
                Item::Const(_) => {}
                Item::Enum(_) => {}
                Item::ExternCrate(_) => {}
                Item::Fn(_) => {}
                Item::ForeignMod(_) => {}
                Item::Impl(_) => {}
                Item::Macro(_) => {}
                Item::Macro2(_) => {}
                Item::Static(_) => {}
                Item::Struct(_) => {}
                Item::TraitAlias(_) => {}
                Item::Type(_) => {}
                Item::Union(_) => {}
                Item::Use(_) => {}
                Item::Verbatim(_) => {}
                Item::__Nonexhaustive => {}
            }
        }
    }

    fn find_rrefed_in_trait(&mut self, tr: &ItemTrait) {
        for item in &tr.items {
            if let TraitItem::Method(method) = item {
                self.find_rrefed_in_method(&method);
            }
        }
    }

    fn find_rrefed_in_method(&mut self, method: &TraitItemMethod) {
        for arg in &method.sig.inputs {
            self.find_rrefed_in_fnarg(&arg);
        }
    }

    fn find_rrefed_in_fnarg(&mut self, arg: &FnArg) {
        if let FnArg::Typed(ty) = arg {
            self.find_rrefed_in_type(&ty.ty);
        } 
    }

    fn find_rrefed_in_returntype(&mut self, rtn: &ReturnType) {
        if let ReturnType::Type(_, ty) = rtn {
            self.find_rrefed_in_type(ty);
        }
    }

    fn find_rrefed_in_type(&mut self, ty: &Type) {
        match ty {
            Type::Array(ty) => {
                self.find_rrefed_in_type(&ty.elem);
            },
            Type::Path(ty) => {
                self.find_rrefed_in_path(&ty.path);
            },
            Type::Tuple(ty) => {
                for elem in &ty.elems {
                    self.find_rrefed_in_type(&elem);
                }
            },
            _ => unimplemented!()
        }
    }

    // Potential problem: B does `pub use A::Bar as Car`, Foo does `use A::Bar; use B::Car;`. These two
    // are the same type and will result a compilation error in typeid?
    fn find_rrefed_in_path(&mut self, path: &Path) {
        // If the path starts with `::` or `crate`, we know that it's already fully qualified.
        // No further is required. We can just put it into the list.
        let mut path_is_fully_qualified = path.leading_colon.is_some();
        if let Some(seg) = path.segments.iter().next() {
            path_is_fully_qualified |= seg.ident == "crate";
        }
        if path_is_fully_qualified {
            self.type_list.insert(path.segments.iter().map(|e| e.clone()).collect());
            return;
        }

        // Walk the module tree and resolve the type.
        // TODO
    }
}
