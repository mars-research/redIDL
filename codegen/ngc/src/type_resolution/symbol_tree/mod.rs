mod module;
mod symbol_tree;
mod symbol_tree_node;

pub use module::*;
pub use symbol_tree::*;
pub use symbol_tree_node::*;

use std::collections::HashSet;

lazy_static::lazy_static! {
    pub static ref PATH_MODIFIERS: HashSet<String> = vec![
        (String::from("crate")),
        (String::from("super")),
        (String::from("self")),
    ].into_iter().collect();
}
