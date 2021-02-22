use syn::Visibility;

pub fn is_prviate(vis: &Visibility) -> bool {
    *vis == Visibility::Inherited
}

pub fn is_public(vis: &Visibility) -> bool {
    !is_prviate(vis)
}