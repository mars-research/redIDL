use lazy_static::lazy_static;
use log::{debug, info, trace};

use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
};
use syn::{
    Expr, ExprLit, File, FnArg, GenericArgument, Ident, Item, ItemTrait, Lit, Path, PathArguments,
    PathSegment, ReturnType, TraitItem, TraitItemMethod, Type,
};

use super::symbol_tree::*;
use super::utils::*;

lazy_static! {
    static ref RREF_PATH: Vec<String> = vec![
        String::from("crate"),
        String::from("rref"),
        String::from("rref"),
        String::from("RRef"),
    ];
}

pub struct TypeResolver {
    /// The root module node, i.e. the `crate` node.
    symbol_tree: SymbolTree,
    /// The current module node that's used in recursive calls.
    current_module: Module,
}

impl TypeResolver {
    pub fn new(symbol_tree: SymbolTree) -> Self {
        let symbol_tree_node = symbol_tree.root_module();
        Self {
            symbol_tree,
            current_module: symbol_tree_node,
        }
    }

    /// Rewrite the ast such that types that we are interested in are in their fully-qualified paths.
    /// The types that we are interested in right now are the types in the trait methods.
    /// If the type-in-interest contains generic, the generic will be resolved.
    /// If the generic contains a constant, the constant will be resolved to a literal.
    pub fn resolve_types(mut self, ast: &mut File) {
        self.resolve_type_in_items(&mut ast.items);
    }

    fn resolve_type_in_items(&mut self, items: &mut [syn::Item]) {
        for item in items.iter_mut() {
            self.resolve_type_in_item(item)
        }
    }

    fn resolve_type_in_item(&mut self, item: &mut syn::Item) {
        match item {
            Item::Mod(md) => {
                info!("Finding `RRef`ed for module {:?}", md.ident);
                if let Some((_, items)) = &mut md.content {
                    // Push a frame
                    let current_node = self.current_module.borrow();
                    let next_frame = current_node.get(&md.ident);
                    let next_frame = crate::expect!(
                        next_frame,
                        "Module {:?} not found in {:#?}",
                        md.ident,
                        current_node
                    );
                    let next_frame = match &next_frame
                        .borrow()
                        .terminal
                        .as_ref()
                        .expect("Expecting a module; found unresolved.")
                        .definition
                    {
                        Definition::Module(md) => md.clone(),
                        _ => panic!("Expecting a module, not a symbol."),
                    };
                    drop(current_node);
                    self.current_module = next_frame;
                    // Recurse into the new frame.
                    self.resolve_type_in_items(items);
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
                self.resolve_type_in_trait(tr);
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

    fn resolve_type_in_trait(&mut self, tr: &mut ItemTrait) {
        for item in &mut tr.items {
            if let TraitItem::Method(method) = item {
                self.resolve_type_in_method(method);
            }
        }
    }

    fn resolve_type_in_method(&mut self, method: &mut TraitItemMethod) {
        for arg in &mut method.sig.inputs {
            self.resolve_type_in_fnarg(arg);
        }
        self.resolve_type_in_returntype(&mut method.sig.output);
    }

    fn resolve_type_in_fnarg(&mut self, arg: &mut FnArg) {
        if let FnArg::Typed(ty) = arg {
            self.resolve_type_in_type(&mut ty.ty, None);
        }
    }

    fn resolve_type_in_returntype(&mut self, rtn: &mut ReturnType) {
        if let ReturnType::Type(_, ty) = rtn {
            self.resolve_type_in_type(ty, None);
        }
    }

    /// Resolve type, put the type and the nested types, if there's any, into the typelist, and
    /// return the resolved type.
    fn resolve_type_in_type(
        &mut self,
        ty: &mut Type,
        generic_args: Option<&HashMap<Ident, GenericResult>>,
    ) -> GenericResult {
        match ty {
            Type::Array(arr) => {
                // Resolve the type.
                let mut resolved_type = arr.clone();
                resolved_type.elem = box self.resolve_type_in_type(&mut arr.elem, generic_args).ty();

                // Resolve the length to a literal
                resolved_type.len = match &mut arr.len {
                    Expr::Lit(lit) => Expr::Lit(lit.clone()),
                    Expr::Path(path) => {
                        let path = &mut path.path;
                        let mut rtn = None;

                        // If the path is a generic argument, return it's mapping
                        if let Some(ident) = path.get_ident() {
                            if let Some(generic_args) = generic_args {
                                if let Some(generic_param) = generic_args.get(ident) {
                                    match generic_param {
                                        GenericResult::Type(x) => panic!(
                                            "Expecting a generic literal, not a type. {:?}",
                                            x
                                        ),
                                        GenericResult::Literal(lit) => {
                                            rtn = Some(Expr::Lit(ExprLit {
                                                attrs: vec![],
                                                lit: lit.clone(),
                                            }))
                                        }
                                    }
                                }
                            }
                        }

                        // Otherwise, walk the path and resolve it to a literal.
                        if rtn.is_none() {
                            let (path, node) = self.resolve_path(path, generic_args);
                            let node = crate::expect!(node, "Array length experssion must not contains paths from external crates.\nType: {:?}\nForeign path: {:?}", ty, path);
                            let lit = match &node
                                .borrow()
                                .terminal
                                .as_ref()
                                .expect("expecting a literal; found unresolved")
                                .definition
                            {
                                Definition::Literal(lit) => Expr::Lit(ExprLit {
                                    attrs: vec![],
                                    lit: lit.clone(),
                                }),
                                _ => panic!(),
                            };
                            rtn = Some(lit);
                        }

                        rtn.unwrap()
                    }
                    _ => unimplemented!(),
                };

                // Rewrite the type to the resolved one.
                *arr = resolved_type.clone();
                let resolved_type = Type::Array(resolved_type);
                GenericResult::Type(resolved_type)
            }
            Type::Path(path) => {
                // Use the type parameter to resolve the path, if there's a matching one.
                if let Some(generic_args) = generic_args {
                    // The path is one single ident, which could be a generic argument.
                    if let Some(path) = path.path.get_ident() {
                        if let Some(resolved_type) = generic_args.get(path) {
                            return resolved_type.clone();
                        }
                    }
                }

                // Resolve the path and rewrite the type to the resolved one.
                let mut resolved_type = path.clone();
                resolved_type.path = self.resolve_path(&mut path.path, generic_args).0;
                *path = resolved_type.clone();
                let resolved_type = Type::Path(resolved_type);
                GenericResult::Type(resolved_type)
            }
            Type::Tuple(tu) => {
                let mut resolved_type = tu.clone();
                for elem in &mut resolved_type.elems {
                    *elem = self.resolve_type_in_type(elem, generic_args).ty();
                }
                *tu = resolved_type.clone();
                let resolved_type = Type::Tuple(resolved_type);
                GenericResult::Type(resolved_type)
            }
            Type::BareFn(x) => unimplemented!("{:#?}", x),
            Type::Group(x) => unimplemented!("{:#?}", x),
            Type::ImplTrait(x) => unimplemented!("{:#?}", x),
            Type::Infer(x) => unimplemented!("{:#?}", x),
            Type::Macro(_) => panic!("There's shouldn't be unexpended any macro at this point."),
            Type::Never(x) => unimplemented!("{:#?}", x),
            Type::Paren(x) => unimplemented!("{:#?}", x),
            Type::Ptr(ptr) => {
                GenericResult::Type(self.resolve_type_in_type(&mut ptr.elem, generic_args).ty())
            }
            Type::Reference(reference) => {
                GenericResult::Type(self.resolve_type_in_type(&mut reference.elem, generic_args).ty())
            }
            Type::Slice(slice) => {
                let mut resolved_type = slice.clone();
                *resolved_type.elem = self
                    .resolve_type_in_type(&mut resolved_type.elem, generic_args)
                    .ty();
                let resolved_type = Type::Slice(resolved_type);
                GenericResult::Type(resolved_type)
            }
            Type::TraitObject(tr) => {
                let mut resolved_type = tr.clone();
                for bound in resolved_type.bounds.iter_mut() {
                    match bound {
                        syn::TypeParamBound::Trait(tr) => {
                            tr.path = self.resolve_path(&mut tr.path, generic_args).0;
                        }
                        syn::TypeParamBound::Lifetime(_) => {}
                    }
                }
                *tr = resolved_type.clone();
                let resolved_type = Type::TraitObject(resolved_type);
                GenericResult::Type(resolved_type)
            }
            Type::Verbatim(x) => unimplemented!("{:#?}", x),
            Type::__Nonexhaustive => unimplemented!(),
        }
    }

    /// Resolve path in the current module and return the resolved path and its corresponding node,
    /// if it doesn't come from an external crate.
    /// The path itself must not be a generic argument, which means it should be resolved in other
    /// manner without calling this function.
    /// If the path is resolved into a struct, it will recursively goes in the struct to add any
    /// `RRef`ed member variables.
    fn resolve_path(
        &mut self,
        path: &mut Path,
        generic_args: Option<&HashMap<Ident, GenericResult>>,
    ) -> (Path, Option<SymbolTreeNode>) {
        trace!(
            "Resolving path {:?} with generic_args {:?}",
            path,
            generic_args
        );
        let mut current_node = self.current_module.clone();
        let mut path_segments: Vec<PathSegment> = path.segments.iter().cloned().collect();

        // If the path starts with `::` and doesn't come from `crate` or `super, or it comes from
        // some unknown module(external module), we know that it's already fully qualified.
        if path.leading_colon.is_some()
            && PATH_MODIFIERS.contains(&path.segments.first().unwrap().ident.to_string())
            || current_node.borrow().get(&path_segments[0].ident).is_none()
        {
            return (path.clone(), None);
        }

        // Walk the module tree and resolve the type.
        let mut final_segment = path_segments.remove(path_segments.len() - 1);
        for path_segment in path_segments {
            if path_segment.arguments != PathArguments::None {
                panic!("Path arguments(e.g. generics) is not supported in the inner path segments.\nViolating path: {:?}\nPath segment: {:?}", path, path_segment);
            }

            let current_node_ref = current_node.borrow();
            let next_node = current_node_ref.get(&path_segment.ident);
            let next_node = crate::expect!(
                next_node,
                "Unable to find {:?} in {:#?}",
                path_segment.ident,
                current_node
            )
            .clone();
            drop(current_node_ref);
            let next_node = next_node.borrow();
            current_node = match &next_node
                .terminal
                .as_ref()
                .expect("Expecting module, found unresolved")
                .definition
            {
                Definition::Module(md) => {
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
        let final_node = current_node_ref.get(&final_segment.ident);
        let final_node = crate::expect!(
            final_node,
            "Unable to find {:?} in {:#?}",
            final_segment.ident,
            current_node_ref
        )
        .clone();
        drop(current_node_ref);
        let final_node_ref = final_node.borrow();
        let mut resolved_path = match &final_node_ref.terminal.as_ref().unwrap().definition {
            Definition::Module(md) => panic!("Expecting a type, but found a module. {:?}", md),
            _ => idents_to_path(&final_node_ref.path),
        };
        drop(final_node_ref);

        // Resolve the generic arguments and rewrite the AST.
        self.resolve_path_arguments(&mut final_segment.arguments, generic_args);
        *resolved_path.segments.last_mut().unwrap() = final_segment;

        trace!("Path {:?} is resolved to {:?}.", path, resolved_path);
        *path = resolved_path.clone();
        (resolved_path, Some(final_node.clone()))
    }

    /// Resolve any types or constants in the path argument and returns the resolved path arguments back.
    fn resolve_path_arguments(
        &mut self,
        arguments: &mut PathArguments,
        generic_args: Option<&HashMap<Ident, GenericResult>>,
    ) {
        let mut resolved_arguments = arguments.clone();
        if let PathArguments::AngleBracketed(generic) = &mut resolved_arguments {
            for arg in generic.args.iter_mut() {
                trace!("Resolving generic argument {:?}", arg);
                let resolved_arg: Option<GenericArgument> = match arg {
                    syn::GenericArgument::Lifetime(_) => {
                        /* noop */
                        None
                    }
                    syn::GenericArgument::Type(ty) => {
                        // It is possible that `ty` is resolved into a constant literal.
                        Some(self.resolve_type_in_type(ty, generic_args).into())
                    }
                    syn::GenericArgument::Binding(x) => unimplemented!("{:#?}", x),
                    syn::GenericArgument::Constraint(x) => unimplemented!("{:#?}", x),
                    syn::GenericArgument::Const(expr) => {
                        let lit = match expr {
                            syn::Expr::Lit(lit) => lit.lit.clone(),
                            syn::Expr::Path(path) => {
                                let (path, node) = self.resolve_path(&mut path.path, generic_args);
                                let node = crate::expect!(node, "Generic experssion must not contains paths from external crates.\nPath arguments: {:?}\nForeign path: {:?}", arguments, path);
                                let lit = match &node.borrow().terminal.as_ref().unwrap().definition {
                                    Definition::Literal(lit) => lit.clone(),
                                    _ => panic!("All generic constants must be able to resolved to a compile time constant.\nPath: {:?}\nNode: {:?}", path, node.borrow()),
                                };
                                lit
                            }
                            _ => unimplemented!(),
                        };
                        Some(GenericArgument::Const(Expr::Lit(ExprLit {
                            attrs: vec![],
                            lit,
                        })))
                    }
                };

                if let Some(resolved_arg) = resolved_arg {
                    *arg = resolved_arg;
                }
            }
        }
        *arguments = resolved_arguments;
    }
}

/// An enum represents what a generic argument can resolved into.
/// In this compiler, a generic argument can be resolved into a type or a constant literal.
#[derive(Debug, Clone)]
enum GenericResult {
    Type(Type),
    Literal(Lit),
}

#[allow(dead_code)]
impl GenericResult {
    fn ty(self) -> Type {
        match self {
            GenericResult::Type(ty) => ty,
            GenericResult::Literal(lit) => panic!("Expecting a type, but found {:#?}", lit),
        }
    }

    fn lit(self) -> Lit {
        match self {
            GenericResult::Literal(lit) => lit,
            GenericResult::Type(ty) => panic!("Expecting a literal, but found {:#?}", ty),
        }
    }
}

impl Into<GenericArgument> for GenericResult {
    fn into(self) -> GenericArgument {
        match self {
            GenericResult::Type(ty) => GenericArgument::Type(ty),
            GenericResult::Literal(lit) => {
                GenericArgument::Const(Expr::Lit(ExprLit { attrs: vec![], lit }))
            }
        }
    }
}
