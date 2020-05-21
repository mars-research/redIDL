use crate::error::Result;
use std::path::Path;
use std::fs::File;
use std::io::{Read/*, Write*/};

// Long story short, each method of the Create* proxies must generate the whole irq/free fn call stuff
// plus a whole load of other crap

fn process_creator(m: &syn::TraitItemMethod) -> Result<()> {
    let _rt = quote::quote!(m.sig.output).to_string();

    Ok(())
}

fn process_tr(tr: &syn::ItemTrait) -> Result<()> {
    for item in &tr.items {
        match item {
            syn::TraitItem::Method(m) => process_creator(m)?,
            _ => fail_with_msg!("creation traits may only have methods")
        }
    }

    Ok(())
}

pub fn generate(root: &Path) -> Result<()> {
    let mut i_defs_file = File::open(crate::get_subpath(root, "sys/interfaces/create/src/lib.rs"))?;
    let mut i_defs = String::new();
    try_with_msg!(
        i_defs_file.read_to_string(&mut i_defs),
        "couldn't read domain creation IDL")?;
    
    let ast: syn::File = try_with_msg!(
        syn::parse_str(&i_defs),
        "couldn't parse domain creation IDL")?;

    for item in &ast.items {
        match item {
            syn::Item::Trait(tr) => try_with_msg!(
                process_tr(tr),
                "illegal trait {}",
                tr.ident.to_string())?,
            _ => ()
        }
    }

    Ok(())
}