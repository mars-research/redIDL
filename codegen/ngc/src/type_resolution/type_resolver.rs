use mem::replace;
use syn::{File, FnArg, Item, ItemTrait, Path, PathSegment, ReturnType, TraitItem, TraitItemMethod, Type};
use std::collections::{HashMap, HashSet};
use std::mem;

use super::module_tree::*;

pub type PathSegments = Vec<PathSegment>;


pub struct TypeSolver {
    /// All the fully qualified path of all `RRef`ed types.
    type_list:  HashSet<PathSegments>,
    /// The root module node, i.e. the `crate` node.
    root_module_node: ModuleNode,
    /// The current module node that's used in recursive calls.
    module_node: ModuleNode,
}

impl TypeSolver {
    pub fn new(module_tree: ModuleTree) -> Self {
        let root_node = module_tree.root.clone();
        Self {
            type_list: HashSet::new(),
            root_module_node: root_node,
            module_node: root_node.clone(),
        }
    }

    /// Takes a AST and returns a list of fully-qualified paths of all `RRef`ed types.
    pub fn resolve_types(&mut self, ast: &File) -> HashSet<PathSegments> {
        self.type_list.clear();
        self.module_node.clear();
        self.resolve_types_recursive(&ast.items);
        self.module_node.clear();
        std::mem::replace(&mut self.type_list, HashSet::new())
    }

    fn resolve_types_recursive(&mut self, items: &Vec<syn::Item>) {
        let mut generated_items = Vec::<syn::Item>::new();
        for item in items.iter() {
            match item {
                Item::Mod(md) => {
                    if let Some((_, items)) = &md.content {
                        self.module_node = self.module_node.push(&md.ident);
                        self.resolve_types_recursive(items);
                        self.module_node = self.module_node.parent().unwrap();
                    }
                }
                Item::Trait(tr) => {
                    self.resolve_types_in_trait(tr);
                }
                _ => {},
            }
        }
    }

    fn resolve_types_in_trait(&mut self, tr: &ItemTrait) {
        for item in &tr.items {
            if let TraitItem::Method(method) = item {
                self.resolve_types_in_method(&method);
            }
        }
    }

    fn resolve_types_in_method(&mut self, method: &TraitItemMethod) {
        for arg in &method.sig.inputs {
            self.resolve_types_in_fnarg(&arg);
        }
    }

    fn resolve_types_in_fnarg(&mut self, arg: &FnArg) {
        if let FnArg::Typed(ty) = arg {
            self.resolve_types_in_type(&ty.ty);
        } 
    }

    fn resolve_types_in_returntype(&mut self, rtn: &ReturnType) {
        if let ReturnType::Type(_, ty) = rtn {
            self.resolve_types_in_type(ty);
        }
    }

    fn resolve_types_in_type(&mut self, ty: &Type) {
        match ty {
            Type::Array(ty) => {
                self.resolve_types_in_type(&ty.elem);
            },
            Type::Path(ty) => {
                self.resolve_types_in_path(&ty.path);
            },
            Type::Tuple(ty) => {
                for elem in &ty.elems {
                    self.resolve_types_in_type(&elem);
                }
            },
            _ => unimplemented!()
        }
    }

    // Potential problem: B does `pub use A::Bar as Car`, Foo does `use A::Bar; use B::Car;`. These two
    // are the same type and will result a compilation error in typeid?
    fn resolve_types_in_path(&mut self, path: &Path) {
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
