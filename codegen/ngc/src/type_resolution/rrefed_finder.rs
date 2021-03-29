use log::info;
use mem::replace;
use std::collections::{HashMap, HashSet};
use std::mem;
use syn::{
    Expr, ExprLit, File, FnArg, Item, ItemTrait, Path, PathArguments, PathSegment, ReturnType,
    TraitItem, TraitItemMethod, Type,
};

use super::symbol_tree::*;
use super::utils::*;

pub struct RRefedFinder {
    /// All the fully qualified path of all `RRef`ed types.
    type_list: HashSet<Type>,
    /// The root module node, i.e. the `crate` node.
    symbol_tree: SymbolTree,
    /// The current module node that's used in recursive calls.
    current_module: Module,
}

impl RRefedFinder {
    pub fn new(symbol_tree: SymbolTree) -> Self {
        let symbol_tree_node = symbol_tree.root_module();
        Self {
            type_list: HashSet::new(),
            symbol_tree: symbol_tree,
            current_module: symbol_tree_node,
        }
    }

    /// Takes a AST and returns a list of fully-qualified paths of all `RRef`ed types.
    pub fn find_rrefed(mut self, ast: &File) -> HashSet<Type> {
        self.find_rrefed_recursive(&ast.items);
        self.type_list
    }

    fn find_rrefed_recursive(&mut self, items: &Vec<syn::Item>) {
        for item in items.iter() {
            match item {
                Item::Mod(md) => {
                    info!("Finding RRefed for module {:?}", md.ident);
                    if let Some((_, items)) = &md.content {
                        // Push a frame
                        let current_node = self.current_module.borrow();
                        let next_frame = current_node.children.get(&md.ident);
                        let next_frame = next_frame.expect(&format!(
                            "Module {:?} not found in {:#?}",
                            md.ident, current_node
                        ));
                        let next_frame = match &next_frame.borrow().terminal {
                            Terminal::Module(md) => md.clone(),
                            _ => panic!("Expecting a module, not a symbol."),
                        };
                        drop(current_node);
                        self.current_module = next_frame;
                        // Recurse into the new frame.
                        self.find_rrefed_recursive(items);
                        // Pop a frame
                        let parent_module = self
                            .current_module
                            .borrow()
                            .node
                            .borrow()
                            .get_parent_module();
                        self.current_module = parent_module;
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
        self.find_rrefed_in_returntype(&method.sig.output);
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
                // Resolve the type.
                let mut resolved_type = ty.clone();
                resolved_type.elem = box self.find_rrefed_in_type(&ty.elem);

                // Resolve the length to a literal
                resolved_type.len = match &ty.len {
                    Expr::Lit(lit) => Expr::Lit(lit.clone()),
                    Expr::Path(path) => {
                        let (path, node) = self.resolve_path(&path.path);
                        let node = node.expect(&format!("Array length experssion must not contains paths from external crates.\nType: {:?}\nForeign path: {:?}", ty, path));
                        let lit = match &node.borrow().terminal {
                            Terminal::Literal(lit) => Expr::Lit(ExprLit {
                                attrs: vec![],
                                lit: lit.clone(),
                            }),
                            _ => panic!(),
                        };
                        lit
                    }
                    _ => unimplemented!(),
                };

                // Put the resolved type into the type list.
                let resolved_type = Type::Array(resolved_type);
                self.type_list.insert(resolved_type.clone());
                resolved_type
            }
            Type::Path(ty) => {
                let mut resolved_type = ty.clone();
                resolved_type.path = self.resolve_path(&ty.path).0;
                let resolved_type = Type::Path(resolved_type);
                self.type_list.insert(resolved_type.clone());
                resolved_type
            }
            Type::Tuple(ty) => {
                let mut resolved_type = ty.clone();
                for elem in &mut resolved_type.elems {
                    *elem = self.find_rrefed_in_type(&elem);
                }
                let resolved_type = Type::Tuple(resolved_type);
                self.type_list.insert(resolved_type.clone());
                resolved_type
            }
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
                            tr.path = self.resolve_path(&tr.path).0;
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

    /// Resolve path in the current module and return the resolved path and its corresponding node,
    /// if it doesn't come from an external crate.
    fn resolve_path(&mut self, path: &Path) -> (Path, Option<SymbolTreeNode>) {
        let mut current_node = self.current_module.clone();
        let mut path_segments: Vec<PathSegment> = path.segments.iter().map(|x| x.clone()).collect();
        let crate_or_super = {
            if path_segments[0].ident == "crate" {
                current_node = self.symbol_tree.root_module();
                path_segments.remove(0);
                true
            } else if path_segments[0].ident == "super" {
                let parent_module = current_node.borrow().node.borrow().get_parent_module();
                current_node = parent_module;
                path_segments.remove(0);
                true
            } else if path_segments[0].ident == "self" {
                path_segments.remove(0);
                true
            } else {
                false
            }
        };

        // If the path starts with `::` and doesn't come from `crate` or `super, or it comes from
        // some unknown module(external module), we know that it's already fully qualified.
        if path.leading_colon.is_some() && !crate_or_super
            || current_node
                .borrow()
                .children
                .get(&path_segments[0].ident)
                .is_none()
        {
            return (path.clone(), None);
        }

        // Walk the module tree and resolve the type.
        let final_segment = path_segments.remove(path_segments.len() - 1);
        for path_segment in path_segments {
            if path_segment.arguments != PathArguments::None {
                panic!("Path arguments(e.g. generics) is not supported in the inner path segments.\nPath: {:?}\nPath segment: {:?}", path, path_segment);
            }

            let current_node_ref = current_node.borrow();
            let next_node = current_node_ref.children.get(&path_segment.ident);
            let next_node = next_node
                .expect(&format!(
                    "Unable to find {:?} in {:#?}",
                    path_segment.ident, current_node
                ))
                .clone();
            drop(current_node_ref);
            let next_node = next_node.borrow();
            current_node = match &next_node.terminal {
                Terminal::Module(md) => {
                    assert!(next_node.public);
                    md.clone()
                }
                _ => panic!(
                    "Resolving {:#?} for {:#?}. Node {:#?} is a symbol and cannot have child.",
                    path_segment,
                    current_node.borrow().node.borrow().path,
                    next_node
                ),
            };
        }

        let current_node_ref = current_node.borrow();
        let final_node = current_node_ref.children.get(&final_segment.ident);
        let final_node = final_node
            .expect(&format!(
                "Unable to find {:?} in {:#?}",
                final_segment.ident, current_node_ref
            ))
            .clone();
        drop(current_node_ref);
        let final_node_ref = final_node.borrow();
        let mut resolved_path = match &final_node_ref.terminal {
            Terminal::Module(md) => panic!("Expecting a type, but found a module. {:?}", md),
            Terminal::None => panic!("Path not resolved: {:?}", final_node_ref.path),
            _ => idents_to_path(&final_node_ref.path),
        };
        drop(final_node_ref);

        // Resolve the generics.
        resolved_path.segments.last_mut().unwrap().arguments =
            self.resolve_path_arguments(&final_segment.arguments);
        (resolved_path, Some(final_node.clone()))
    }

    /// Resolve any types or constants in the path argument and returns the resolved path arguments back.
    fn resolve_path_arguments(&mut self, arguments: &PathArguments) -> PathArguments {
        let mut resolved_arguments = arguments.clone();
        if let PathArguments::AngleBracketed(generic) = &mut resolved_arguments {
            for arg in generic.args.iter_mut() {
                match arg {
                    syn::GenericArgument::Lifetime(_) => { /* noop */ }
                    syn::GenericArgument::Type(ty) => *ty = self.find_rrefed_in_type(ty),
                    syn::GenericArgument::Binding(x) => unimplemented!("{:#?}", x),
                    syn::GenericArgument::Constraint(x) => unimplemented!("{:#?}", x),
                    syn::GenericArgument::Const(expr) => {
                        let lit = match expr {
                            syn::Expr::Lit(lit) => lit.lit.clone(),
                            syn::Expr::Path(path) => {
                                let (path, node) = self.resolve_path(&path.path);
                                let node = node.expect(&format!("Generic experssion must not contains paths from external crates.\nPath arguments: {:?}\nForeign path: {:?}", arguments, path));
                                let lit = match &node.borrow().terminal {
                                    Terminal::Literal(lit) => lit.clone(),
                                    _ => panic!("All generic constants must be able to resolved to a compile time constant.\nPath: {:?}\nNode: {:?}", path, node.borrow()),
                                };
                                lit
                            }
                            _ => unimplemented!(),
                        };
                        *expr = Expr::Lit(ExprLit { attrs: vec![], lit });
                    }
                }
            }
        }
        resolved_arguments
    }
}
