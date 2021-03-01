use std::iter::FromIterator;

use syn::{Ident, Path, PathArguments, PathSegment, Visibility, punctuated::Punctuated, token::{Colon2, Token}};

pub fn is_prviate(vis: &Visibility) -> bool {
    *vis == Visibility::Inherited
}

pub fn is_public(vis: &Visibility) -> bool {
    !is_prviate(vis)
}

pub fn idents_to_path(path_segments: Vec<Ident>) -> Path {
    let segments = Punctuated::from_iter(path_segments.iter().map(|ident| PathSegment{ident: ident.clone(), arguments: PathArguments::None}));
    Path {
        leading_colon: None,
        segments,
    }
}