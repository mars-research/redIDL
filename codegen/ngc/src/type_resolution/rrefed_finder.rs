use log::{debug, info, trace};
use mem::replace;
use std::mem;
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
                    info!("Finding `RRef`ed for module {:?}", md.ident);
                    if let Some((_, items)) = &md.content {
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
            self.find_rrefed_in_type(&ty.ty, None);
        }
    }

    fn find_rrefed_in_returntype(&mut self, rtn: &ReturnType) {
        if let ReturnType::Type(_, ty) = rtn {
            self.find_rrefed_in_type(ty, None);
        }
    }

    /// Resolve type, put the type and the nested types, if there's any, into the typelist, and
    /// return the resolved type.
    fn find_rrefed_in_type(
        &mut self,
        ty: &Type,
        generic_args: Option<&HashMap<Ident, GenericResult>>,
    ) -> GenericResult {
        match ty {
            Type::Array(ty) => {
                // Resolve the type.
                let mut resolved_type = ty.clone();
                resolved_type.elem = box self.find_rrefed_in_type(&ty.elem, generic_args).ty();

                // Resolve the length to a literal
                resolved_type.len = match &ty.len {
                    Expr::Lit(lit) => Expr::Lit(lit.clone()),
                    Expr::Path(path) => {
                        let path = &path.path;
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
                            let (path, node) = self.resolve_path(&path, generic_args);
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

                // Put the resolved type into the type list.
                let resolved_type = Type::Array(resolved_type);
                self.type_list.insert(resolved_type.clone());
                GenericResult::Type(resolved_type)
            }
            Type::Path(ty) => {
                // Use the type parameter to resolve the path, if there's a matching one.
                if let Some(generic_args) = generic_args {
                    // The path is one single ident, which could be a generic argument.
                    if let Some(path) = ty.path.get_ident() {
                        if let Some(resolved_type) = generic_args.get(path) {
                            return resolved_type.clone();
                        }
                    }
                }

                // Resolve the path and insert resolved type into the type_list.
                let mut resolved_type = ty.clone();
                // TODO(tianjiao): resolve generic here
                resolved_type.path = self.resolve_path(&ty.path, generic_args).0;
                let resolved_type = Type::Path(resolved_type);
                self.type_list.insert(resolved_type.clone());
                GenericResult::Type(resolved_type)
            }
            Type::Tuple(ty) => {
                let mut resolved_type = ty.clone();
                for elem in &mut resolved_type.elems {
                    *elem = self.find_rrefed_in_type(&elem, generic_args).ty();
                }
                let resolved_type = Type::Tuple(resolved_type);
                self.type_list.insert(resolved_type.clone());
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
                GenericResult::Type(self.find_rrefed_in_type(&ptr.elem, generic_args).ty())
            }
            Type::Reference(reference) => {
                GenericResult::Type(self.find_rrefed_in_type(&reference.elem, generic_args).ty())
            }
            Type::Slice(slice) => {
                let mut resolved_type = slice.clone();
                *resolved_type.elem = self
                    .find_rrefed_in_type(&resolved_type.elem, generic_args)
                    .ty();
                let resolved_type = Type::Slice(resolved_type);
                GenericResult::Type(resolved_type)
            }
            Type::TraitObject(tr) => {
                let mut resolved_type = tr.clone();
                for bound in resolved_type.bounds.iter_mut() {
                    match bound {
                        syn::TypeParamBound::Trait(tr) => {
                            tr.path = self.resolve_path(&tr.path, generic_args).0;
                        }
                        syn::TypeParamBound::Lifetime(_) => {}
                    }
                }
                let resolved_type = Type::TraitObject(resolved_type);
                GenericResult::Type(resolved_type)
            }
            Type::Verbatim(x) => unimplemented!("{:#?}", x),
            Type::__Nonexhaustive => unimplemented!(),
        }
    }

    fn find_rrefed_in_struct(&mut self, node: &SymbolTreeNode, args: &PathArguments) {
        // Noop if the node is not a terminal struct node.
        let node_ref = node.borrow();
        let (terminal, st) = if let Some(terminal) = &node_ref.terminal {
            let st = if let Definition::Type(Item::Struct(st)) = &terminal.definition {
                st
            } else {
                return;
            };
            (terminal, st)
        } else {
            return;
        };

        debug!("Finding nested `RRef`ed in struct {:?}", st.ident);

        // Sanity checks.
        let args = match &args {
            PathArguments::None => return,
            PathArguments::Parenthesized(x) => unimplemented!("{:#?}", x),
            PathArguments::AngleBracketed(args) => args,
        };
        let args = &args.args;
        assert_eq!(st.generics.params.len(), args.len());

        // Change the scope to where the struct is defined
        let original_scope = self.current_module.clone();
        // TODO: walk the path and find struct module;
        let struct_scope = terminal.node.borrow().get_parent_module();
        self.current_module = struct_scope;
        trace!(
            "To find `RRef`ed in struct {:?}, the scope is changed to {:?}",
            st.ident,
            self.current_module.borrow().path
        );

        // Create a mapping between the generic params and their corresponding arguments.
        // For example, for definition `RRef<T>` and usage `RRef<u8>`, we map `T` to `u8`.
        let generic_map: HashMap<Ident, GenericResult> = st
            .generics
            .params
            .iter()
            .zip(args)
            .map(|(param, arg)| {
                let param = match param {
                    syn::GenericParam::Lifetime(x) => unimplemented!("{:#?}", x),
                    syn::GenericParam::Const(c) => c.ident.clone(),
                    syn::GenericParam::Type(param) => param.ident.clone(),
                };

                let arg = match arg {
                    syn::GenericArgument::Lifetime(x) => unimplemented!("{:#?}", x),
                    syn::GenericArgument::Binding(x) => unimplemented!("{:#?}", x),
                    syn::GenericArgument::Constraint(x) => unimplemented!("{:#?}", x),
                    syn::GenericArgument::Const(expr) => match expr {
                        Expr::Array(x) => unimplemented!("{:#?}", x),
                        Expr::Assign(x) => unimplemented!("{:#?}", x),
                        Expr::AssignOp(x) => unimplemented!("{:#?}", x),
                        Expr::Async(x) => unimplemented!("{:#?}", x),
                        Expr::Await(x) => unimplemented!("{:#?}", x),
                        Expr::Binary(x) => unimplemented!("{:#?}", x),
                        Expr::Block(x) => unimplemented!("{:#?}", x),
                        Expr::Box(x) => unimplemented!("{:#?}", x),
                        Expr::Break(x) => unimplemented!("{:#?}", x),
                        Expr::Call(x) => unimplemented!("{:#?}", x),
                        Expr::Cast(x) => unimplemented!("{:#?}", x),
                        Expr::Closure(x) => unimplemented!("{:#?}", x),
                        Expr::Continue(x) => unimplemented!("{:#?}", x),
                        Expr::Field(x) => unimplemented!("{:#?}", x),
                        Expr::ForLoop(x) => unimplemented!("{:#?}", x),
                        Expr::Group(x) => unimplemented!("{:#?}", x),
                        Expr::If(x) => unimplemented!("{:#?}", x),
                        Expr::Index(x) => unimplemented!("{:#?}", x),
                        Expr::Let(x) => unimplemented!("{:#?}", x),
                        Expr::Lit(lit) => GenericResult::Literal(lit.lit.clone()),
                        Expr::Loop(x) => unimplemented!("{:#?}", x),
                        Expr::Macro(x) => unimplemented!("{:#?}", x),
                        Expr::Match(x) => unimplemented!("{:#?}", x),
                        Expr::MethodCall(x) => unimplemented!("{:#?}", x),
                        Expr::Paren(x) => unimplemented!("{:#?}", x),
                        Expr::Path(x) => unimplemented!("{:#?}", x),
                        Expr::Range(x) => unimplemented!("{:#?}", x),
                        Expr::Reference(x) => unimplemented!("{:#?}", x),
                        Expr::Repeat(x) => unimplemented!("{:#?}", x),
                        Expr::Return(x) => unimplemented!("{:#?}", x),
                        Expr::Struct(x) => unimplemented!("{:#?}", x),
                        Expr::Try(x) => unimplemented!("{:#?}", x),
                        Expr::TryBlock(x) => unimplemented!("{:#?}", x),
                        Expr::Tuple(x) => unimplemented!("{:#?}", x),
                        Expr::Type(x) => unimplemented!("{:#?}", x),
                        Expr::Unary(x) => unimplemented!("{:#?}", x),
                        Expr::Unsafe(x) => unimplemented!("{:#?}", x),
                        Expr::Verbatim(x) => unimplemented!("{:#?}", x),
                        Expr::While(x) => unimplemented!("{:#?}", x),
                        Expr::Yield(x) => unimplemented!("{:#?}", x),
                        Expr::__Nonexhaustive => unimplemented!(),
                    },
                    syn::GenericArgument::Type(arg) => GenericResult::Type(arg.clone()),
                };

                (param, arg)
            })
            .collect();
        for field in &st.fields {
            // Resolve field type
            let resolved_type = self.find_rrefed_in_type(&field.ty, Some(&generic_map)).ty();
            debug!(
                "Field {:?} of struct {:?} is resolved to {:?}",
                field.ident, st.ident, resolved_type
            );
            self.type_list.insert(resolved_type);
        }

        // Restore back to the current scope.
        self.current_module = original_scope;
    }

    /// Resolve path in the current module and return the resolved path and its corresponding node,
    /// if it doesn't come from an external crate.
    /// The path itself must not be a generic argument, which means it should be resolved in other
    /// manner without calling this function.
    /// If the path is resolved into a struct, it will recursively goes in the struct to add any
    /// `RRef`ed member variables.
    fn resolve_path(
        &mut self,
        path: &Path,
        generic_args: Option<&HashMap<Ident, GenericResult>>,
    ) -> (Path, Option<SymbolTreeNode>) {
        trace!(
            "Resolving path {:?} with generic_args {:?}",
            path,
            generic_args
        );
        let mut current_node = self.current_module.clone();
        let mut path_segments: Vec<PathSegment> = path.segments.iter().map(|x| x.clone()).collect();

        // Walk the module tree and resolve the type.
        let final_segment = path_segments.remove(path_segments.len() - 1);
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

        // Resolve the generic arguments.
        resolved_path.segments.last_mut().unwrap().arguments =
            self.resolve_path_arguments(&final_segment.arguments, generic_args);

        // Find nested `RRef`ed types
        self.find_rrefed_in_struct(
            &final_node,
            &resolved_path.segments.last().unwrap().arguments,
        );

        trace!("Path {:?} is resolved to {:?}.", path, resolved_path);
        (resolved_path, Some(final_node.clone()))
    }

    /// Resolve any types or constants in the path argument and returns the resolved path arguments back.
    fn resolve_path_arguments(
        &mut self,
        arguments: &PathArguments,
        generic_args: Option<&HashMap<Ident, GenericResult>>,
    ) -> PathArguments {
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
                        Some(self.find_rrefed_in_type(ty, generic_args).into())
                    }
                    syn::GenericArgument::Binding(x) => unimplemented!("{:#?}", x),
                    syn::GenericArgument::Constraint(x) => unimplemented!("{:#?}", x),
                    syn::GenericArgument::Const(expr) => {
                        let lit = match expr {
                            syn::Expr::Lit(lit) => lit.lit.clone(),
                            syn::Expr::Path(path) => {
                                let (path, node) = self.resolve_path(&path.path, generic_args);
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
        resolved_arguments
    }
}

/// An enum represents what a generic argument can resolved into.
/// In this compiler, a generic argument can be resolved into a type or a constant literal.
#[derive(Debug, Clone)]
enum GenericResult {
    Type(Type),
    Literal(Lit),
}

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
