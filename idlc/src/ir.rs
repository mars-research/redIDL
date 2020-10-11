use syn::*;

pub struct SpecRpcTraitRef<'ast> {
    path: &'ast Path,
}

pub struct SpecRRefLikeOrBitwise<'ast> {
    path: &'ast Path,
}

pub struct SpecRRefLikeImmutRef<'ast> {
    path: &'ast Path,
    verbatim_args: &'ast AngleBracketedGenericArguments,
}

pub struct SpecBitwise<'ast> {
    verbatim: &'ast Type,
}

pub enum SpecExchangeableType<'ast> {
    RpcTraitRef(SpecRpcTraitRef<'ast>),
    SpecRRefLikeImmutRef(SpecRRefLikeImmutRef<'ast>),
    SpecRRefLikeOrBitwise(SpecRRefLikeOrBitwise<'ast>), // Should be eliminated by the time paths are resolved
    SpecBitwise(SpecBitwise<'ast>),
}

pub struct _RpcMethod<'ast> {
    name: &'ast Ident,                          // TODO: is Ident cheap to copy / clone?
    arguments: Vec<SpecExchangeableType<'ast>>, // TODO: won't be using spec nodes at the end of the day
    is_static: bool,
}

pub struct _RpcTraitDef<'ast> {
    name: &'ast Ident,
    methods: Vec<_RpcMethod<'ast>>,
}

pub struct _StructDef<'ast> {
    name: &'ast Ident,
    field_names: Vec<&'ast Ident>,
    field_types: Vec<&'ast Type>,
}

pub enum _IdlDef<'ast> {
    RpcTraitDef(_RpcTraitDef<'ast>),
    StructDef(_StructDef<'ast>),
}

pub struct _Module<'ast> {
    verbatim: Vec<&'ast Item>,
    use_statements: Vec<&'ast ItemUse>,
    idl_defs: Vec<&'ast _IdlDef<'ast>>,
}

fn try_lower_rpc_trait_ref<'ast>(root: &'ast syn::TypeReference) -> Option<SpecRpcTraitRef<'ast>> {
    let elem = &*root.elem;
    let obj = match elem {
        Type::TraitObject(obj) => obj,
        _ => return None,
    };

    if obj.bounds.len() != 1 {
        return None; // Since we don't allow these sorts of shenanigans
    }

    let bound = &obj.bounds[0];
    let tr_bound = match bound {
        TypeParamBound::Trait(tr_bound) => tr_bound,
        _ => return None,
    };

    if let Some(_) = &tr_bound.paren_token {
        println!("We don't know how to deal with parenthesized traits yet");
        return None;
    }

    if let Some(_) = &tr_bound.lifetimes {
        println!("We don't know how to deal \"for<'a>\" statements yet");
        return None;
    }

    Some(SpecRpcTraitRef {
        path: &tr_bound.path,
    })
}

// This is really speculative until paths are resolved
fn try_lower_spec_rref_like_immut_ref<'ast>(
    root: &'ast syn::TypeReference,
) -> Option<SpecRRefLikeImmutRef<'ast>> {
    if let Some(_) = root.mutability {
        return None;
    }

    let elem = &*root.elem;
    let path = match elem {
        Type::Path(path) => &path.path,
        _ => return None,
    };

    let segments = &path.segments;
    let len = segments.len();
    let end = &segments[len - 1];
    let args = match &end.arguments {
        PathArguments::AngleBracketed(args) => args,
        _ => return None,
    };

    Some(SpecRRefLikeImmutRef {
        path: &path,
        verbatim_args: &args,
    })
}

// We can't actually distinguish between Bitwise paths and RRefLike ones *just* yet, in this context
fn try_lower_spec_rref_like_or_bitwise<'ast>(
    root: &'ast syn::TypePath,
) -> Option<SpecRRefLikeOrBitwise<'ast>> {
    match &root.qself {
        Some(_) => {
            println!("[debug] We don't know how to deal with self types yet");
            return None;
        }
        _ => (),
    }

    Some(SpecRRefLikeOrBitwise { path: &root.path })
}

pub fn try_lower_spec_exchangeable_type<'ast>(root: &'ast Type) -> Option<SpecExchangeableType> {
    match root {
        Type::Reference(type_ref) => {
            if let Some(node) = try_lower_rpc_trait_ref(type_ref) {
                Some(SpecExchangeableType::RpcTraitRef(node))
            } else if let Some(node) = try_lower_spec_rref_like_immut_ref(type_ref) {
                Some(SpecExchangeableType::SpecRRefLikeImmutRef(node))
            } else {
                None
            }
        }
        Type::Path(path) => {
            if let Some(node) = try_lower_spec_rref_like_or_bitwise(path) {
                Some(SpecExchangeableType::SpecRRefLikeOrBitwise(node))
            } else {
                None
            }
        }
        Type::Ptr(_) => None,
        Type::BareFn(_) => None,
        _ => Some(SpecExchangeableType::SpecBitwise(SpecBitwise {
            verbatim: root,
        })),
    }
}
