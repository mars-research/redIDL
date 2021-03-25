use std::iter::FromIterator;

use syn::{Ident, Path, PathArguments, PathSegment, Type, Visibility, punctuated::Punctuated, token::{Colon2, Token}};

/// Return truf if the visibility is private.
pub fn is_prviate(vis: &Visibility) -> bool {
    *vis == Visibility::Inherited
}

/// Return truf if the visibility is public.
pub fn is_public(vis: &Visibility) -> bool {
    !is_prviate(vis)
}

/// Construct a path from a list of identifiers.
pub fn idents_to_path(path_segments: &Vec<Ident>) -> Path {
    let segments = Punctuated::from_iter(path_segments.iter().map(|ident| PathSegment{ident: ident.clone(), arguments: PathArguments::None}));
    Path {
        leading_colon: None,
        segments,
    }
}
