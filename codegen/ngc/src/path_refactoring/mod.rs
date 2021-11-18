///! Utility functions that refactor the paths in the AST nodes.

#[cfg(test)]
mod tests;

use syn::{
    FnArg, GenericArgument, Ident, Item, ItemTrait, Path, PathArguments, ReturnType, TraitItem,
    TraitItemMethod, Type,
};

pub fn refactor_path_in_ast(src: &Ident, dest: &Ident, ast: &mut syn::File) {
    refactor_path_in_items(src, dest, &mut ast.items)
}

pub fn refactor_path_in_items(src: &Ident, dest: &Ident, items: &mut [Item]) {
    for item in items {
        refactor_path_in_item(src, dest, item)
    }
}

pub fn refactor_path_in_item(src: &Ident, dest: &Ident, item: &mut Item) {
    match item {
        Item::Const(_) => {}
        Item::Enum(_) => {}
        Item::ExternCrate(_) => {}
        Item::Fn(_) => {}
        Item::ForeignMod(_) => {}
        Item::Impl(_) => {}
        Item::Macro(_) => {}
        Item::Macro2(_) => {}
        Item::Mod(_) => {}
        Item::Static(_) => {}
        Item::Struct(_) => {}
        Item::Trait(tr) => refactor_path_in_trait(src, dest, tr),
        Item::TraitAlias(_) => {}
        Item::Type(_) => {}
        Item::Union(_) => {}
        Item::Use(_) => {}
        _ => {}
    }
}

pub fn refactor_path_in_trait(src: &Ident, dest: &Ident, tr: &mut ItemTrait) {
    refactor_path_in_trait_items(src, dest, &mut tr.items)
}

pub fn refactor_path_in_trait_items(src: &Ident, dest: &Ident, items: &mut [TraitItem]) {
    for item in items {
        refactor_path_in_trait_item(src, dest, item)
    }
}

pub fn refactor_path_in_trait_item(src: &Ident, dest: &Ident, item: &mut TraitItem) {
    match item {
        TraitItem::Const(_) => {}
        TraitItem::Method(method) => refactor_path_in_trait_item_method(src, dest, method),
        TraitItem::Type(_) => {}
        TraitItem::Macro(_) => {}
        _ => {}
    }
}

pub fn refactor_path_in_trait_item_method(src: &Ident, dest: &Ident, method: &mut TraitItemMethod) {
    let sig = &mut method.sig;
    refactor_path_in_return_type(src, dest, &mut sig.output);
    for arg in &mut sig.inputs {
        refactor_path_in_fn_arg(src, dest, arg)
    }
}

pub fn refactor_path_in_return_type(src: &Ident, dest: &Ident, rtn: &mut ReturnType) {
    match rtn {
        ReturnType::Default => {}
        ReturnType::Type(_, ty) => refactor_path_in_type(src, dest, ty),
    }
}

pub fn refactor_path_in_fn_arg(src: &Ident, dest: &Ident, arg: &mut FnArg) {
    match arg {
        FnArg::Receiver(_) => {}
        FnArg::Typed(ty) => refactor_path_in_type(src, dest, &mut ty.ty),
    }
}
pub fn refactor_path_in_type(src: &Ident, dest: &Ident, ty: &mut Type) {
    match ty {
        Type::Array(arr) => refactor_path_in_type(src, dest, &mut arr.elem),
        Type::BareFn(_) => {}
        Type::Group(_) => {}
        Type::ImplTrait(_) => {}
        Type::Infer(_) => {}
        Type::Macro(_) => {}
        Type::Never(_) => {}
        Type::Paren(_) => {}
        Type::Path(path) => refactor_path_in_path(src, dest, &mut path.path),
        Type::Ptr(_) => {}
        Type::Reference(_) => {}
        Type::Slice(_) => {}
        Type::TraitObject(tr) => {
            for bound in &mut tr.bounds {
                match bound {
                    syn::TypeParamBound::Trait(tr) => {
                        refactor_path_in_path(src, dest, &mut tr.path)
                    }
                    syn::TypeParamBound::Lifetime(_) => {}
                }
            }
        }
        Type::Tuple(tuple) => {
            for elem in &mut tuple.elems {
                refactor_path_in_type(src, dest, elem)
            }
        }
        _ => {}
    }
}

pub fn refactor_path_in_path(src: &Ident, dest: &Ident, path: &mut Path) {
    // Refactor the first segement if match.
    let first_segment = path.segments.first_mut().unwrap();
    if &first_segment.ident == src {
        first_segment.ident = dest.clone();
    }

    // Refactor the generic arguments, if there's any.
    for segement in &mut path.segments {
        refactor_path_in_path_arguments(src, dest, &mut segement.arguments);
    }
}

pub fn refactor_path_in_path_arguments(src: &Ident, dest: &Ident, args: &mut PathArguments) {
    match args {
        PathArguments::None => {}
        PathArguments::AngleBracketed(args) => {
            for arg in &mut args.args {
                refactor_path_in_generic_argument(src, dest, arg)
            }
        }
        PathArguments::Parenthesized(_) => {}
    }
}

pub fn refactor_path_in_generic_argument(src: &Ident, dest: &Ident, arg: &mut GenericArgument) {
    match arg {
        GenericArgument::Lifetime(_) => {}
        GenericArgument::Type(ty) => refactor_path_in_type(src, dest, ty),
        GenericArgument::Binding(_) => {}
        GenericArgument::Constraint(_) => {}
        GenericArgument::Const(_) => {}
    }
}
