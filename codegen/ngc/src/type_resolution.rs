use mem::replace;
use syn::{File, FnArg, Item, ItemTrait, Path, PathSegment, ReturnType, TraitItem, TraitItemMethod, Type};
use std::collections::{HashMap, HashSet};
use std::mem;

pub type PathSegments = Vec<PathSegment>;

pub struct TypeSolver {
    /// All the fully qualified path of all `RRef`ed types.
    type_map:  HashSet<PathSegments>,
    /// A stack of maps a PathSegment to its fully qualified path
    scope_maps: Vec<HashMap<PathSegment, PathSegments>>,
}

impl TypeSolver {
    /// Takes a AST and returns a list of fully-qualified paths of all `RRef`ed types.
    pub fn resolve_types(&mut self, ast: &File) -> HashSet<PathSegments> {
        self.type_map.clear();
        self.scope_maps.clear();
        self.resolve_types_recursive(&ast.items);
        self.scope_maps.clear();
        std::mem::replace(&mut self.type_map, HashSet::new())
    }

    fn resolve_types_recursive(&mut self, items: &Vec<syn::Item>) {
        let mut generated_items = Vec::<syn::Item>::new();
        for item in items.iter() {
            match item {
                Item::Mod(md) => {
                    if let Some((_, items)) = &md.content {
                        self.scope_maps.push(HashMap::new());
                        self.resolve_types_recursive(items);
                        self.scope_maps.pop().unwrap();
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

    fn resolve_types_in_path(&mut self, path: &Path) {
        
    }
}
