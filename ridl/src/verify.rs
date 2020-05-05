extern crate syn;
extern crate quote;

#[macro_use]
use crate::error;
use error::Result;

fn walk_item(item: &syn::Item) -> Result<()> {
    match item {
        syn::Item::Fn(f) => fail_with_msg!("bare function \"{}\" not permitted", f.sig.ident),
        _ => Ok(())
    }
}

pub fn walk_file(file: &syn::File) -> Result<()> {
    for item in &file.items {
        walk_item(item)?;
    }

    Ok(())
}