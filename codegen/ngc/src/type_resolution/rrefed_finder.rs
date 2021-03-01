use mem::replace;
use syn::{File, FnArg, Item, ItemTrait, Path, PathSegment, ReturnType, TraitItem, TraitItemMethod, Type};
use std::collections::{HashMap, HashSet};
use std::mem;

use super::symbol_tree::*;
use super::utils::*;

pub type PathSegments = Vec<PathSegment>;


pub struct RRefedFinder {
    /// All the fully qualified path of all `RRef`ed types.
    type_list:  HashSet<Type>,
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
    pub fn find_rrefed(mut self, ast: &File) -> HashSet<Type> {
        self.find_rrefed_recursive(&ast.items);
        self.type_list
    }

    fn find_rrefed_recursive(&mut self, items: &Vec<syn::Item>) {
        let mut generated_items = Vec::<syn::Item>::new();
        for item in items.iter() {
            match item {
                Item::Mod(md) => {
                    println!("Finding RRefed for module {:?}", md.ident);
                    if let Some((_, items)) = &md.content {
                        let current_node = self.symbol_tree_node.borrow();
                        let next_frame = current_node.children.get(&md.ident);
                        let next_frame = next_frame.expect(&format!("Module {:?} not found in {:#?}", md.ident, current_node));
                        let next_frame = match next_frame {
                            ModuleItem::Module(md) => md.borrow().module.clone(),
                            ModuleItem::Type(_) => unreachable!("Expecting a module, not a symbol.")
                        };
                        drop(current_node);
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

    /// Resolve type, put the type and the nested types, if there's any, into the typelist, and
    /// return the resolved type.
    fn find_rrefed_in_type(&mut self, ty: &Type) -> Type {
        match ty {
            Type::Array(ty) => {
                let mut resolved_type = ty.clone();
                resolved_type.elem = box self.find_rrefed_in_type(&ty.elem);
                let resolved_type = Type::Array(resolved_type);
                self.type_list.insert(resolved_type.clone());
                resolved_type
            },
            Type::Path(ty) => {
                let mut resolved_type = ty.clone();
                resolved_type.path = self.resolve_path(&ty.path);
                let resolved_type = Type::Path(resolved_type);
                self.type_list.insert(resolved_type.clone());
                resolved_type
            },
            Type::Tuple(ty) => {
                let mut resolved_type = ty.clone();
                for elem in &mut resolved_type.elems {
                    *elem = self.find_rrefed_in_type(&elem);
                }
                let resolved_type = Type::Tuple(resolved_type);
                self.type_list.insert(resolved_type.clone());
                resolved_type
            },
            Type::BareFn(x) => unimplemented!("{:#?}", x),
            Type::Group(x) => unimplemented!("{:#?}", x),
            Type::ImplTrait(x) => unimplemented!("{:#?}", x),
            Type::Infer(x) => unimplemented!("{:#?}", x),
            Type::Macro(_) => panic!("There's shouldn't be unexpended any macro at this point."),
            Type::Never(x) => unimplemented!("{:#?}", x),
            Type::Paren(x) => unimplemented!("{:#?}", x),
            Type::Ptr(x) => unimplemented!("{:#?}", x),
            Type::Reference(reference) => self.find_rrefed_in_type(&reference.elem),
            Type::Slice(slice) => {
                let mut resolved_type = slice.clone();
                *resolved_type.elem = self.find_rrefed_in_type(&resolved_type.elem);
                let resolved_trait = Type::Slice(resolved_type);
                resolved_trait
            }
            Type::TraitObject(tr) => {
                let mut resolved_type = tr.clone();
                for bound in resolved_type.bounds.iter_mut() {
                    match bound {
                        syn::TypeParamBound::Trait(tr) => {
                            tr.path = self.resolve_path(&tr.path);
                        }
                        syn::TypeParamBound::Lifetime(_) => {}
                    }
                }
                let resolved_trait = Type::TraitObject(resolved_type);
                resolved_trait
            }
            Type::Verbatim(x) => unimplemented!("{:#?}", x),
            Type::__Nonexhaustive => unimplemented!(),
        }
    }

    /// Resolve path in the current module and return the resolved path.
    fn resolve_path(&mut self, path: &Path) -> Path {
        let mut current_node = self.symbol_tree_node.clone();
        let mut path_segments: Vec<PathSegment> = path.segments.iter().map(|x| x.clone()).collect();
        let crate_or_super = {
            if path_segments[0].ident == "crate" {
                current_node = self.symbol_tree.root_symbol_tree_node();
                path_segments.remove(0);
                true
            } else if path_segments[0].ident == "super" {
                current_node = current_node.parent().unwrap();
                path_segments.remove(0);
                true
            } else {
                false
            }
        };

        // If the path starts with `::` and doesn't come from `crate` or `super, or it comes from
        // some unknown module(external module), we know that it's already fully qualified.
        if path.leading_colon.is_some() && !crate_or_super || current_node.borrow().children.get(&path_segments[0].ident).is_none() {
            return path.clone();
        }

        // Walk the module tree and resolve the type.
        let final_segment = path_segments.remove(path_segments.len() - 1);
        for path_segment in path_segments {
            let current_node_ref = current_node.borrow();
            let next_node = current_node_ref.children.get(&final_segment.ident);
            let next_node = next_node.expect(&format!("Unable to find {:?} in {:#?}", final_segment.ident, current_node)).clone();
            drop(current_node_ref);
            current_node = match next_node {
                ModuleItem::Type(_) => panic!("Resolving {:#?} for {:#?}. Node {:#?} is a symbol and cannot have child.", path_segment, current_node.borrow().path, next_node),
                ModuleItem::Module(md) => {
                    let md = md.borrow();
                    assert!(md.public);
                    md.module.clone()
                }
            };
        }

        let current_node = current_node.borrow();
        let final_node = current_node.children.get(&final_segment.ident);
        let final_node = final_node.expect(&format!("Unable to find {:?} in {:#?}", final_segment.ident, current_node));
        match final_node {
            ModuleItem::Module(md) => panic!("Expecting a type, but found a module. {:?}", md),
            ModuleItem::Type(ty) => {
                let ty = ty.borrow();
                // assert!(ty.public, "Expecting {:?} to be public when resolving {:?}", ty, path);
                idents_to_path(ty.path.clone())
            }
        }

        // TODO: fix 
    }
}
