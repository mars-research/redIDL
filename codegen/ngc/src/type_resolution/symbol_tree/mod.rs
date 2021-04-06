mod module;
mod symbol_tree;
mod symbol_tree_node;

pub use module::*;
pub use symbol_tree::*;
pub use symbol_tree_node::*;

use std::{
    borrow::Borrow,
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    hash::Hash,
    ops::{Deref, DerefMut},
    rc::Rc,
};
use std::collections::HashSet;

use super::utils::is_public;
use proc_macro2::Span;
use quote::format_ident;
use syn::{
    Ident, Item, ItemFn, ItemStruct, ItemTrait, Lit, LitInt, PathSegment, VisPublic, Visibility,
};

lazy_static::lazy_static!{
    pub static ref PATH_MODIFIERS: HashSet<String> = vec![
        (String::from("crate")),
        (String::from("super")),
        (String::from("self")),
    ].into_iter().collect();
}