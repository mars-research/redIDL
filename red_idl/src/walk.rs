use syn;

pub trait ASTPass {}
pub struct NullPass {}
impl ASTPass for NullPass {}

pub fn walk_item_struct<T: ASTPass>(_item_struct: &syn::ItemStruct, _pass: &T) {
    println!("Walked ItemStruct");
}

pub fn walk_item_trait<T: ASTPass>(_item_trait: &syn::ItemTrait, _pass: &T) {
    println!("Walked ItemTrait");
}

pub fn walk_item<T: ASTPass>(item: &syn::Item, pass: &T) {
    println!("Walked Item");
    match item {
        syn::Item::Trait(item_trait) => walk_item_trait(item_trait, pass),
        syn::Item::Struct(item_struct) => walk_item_struct(item_struct, pass),
        _ => ()
    }
}

pub fn walk_file<T: ASTPass>(file: &syn::File, pass: &T) {
    println!("Walked File");
    for item in &file.items {
        walk_item(&item, pass)
    }
}