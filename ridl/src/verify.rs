extern crate syn;
extern crate quote;

#[macro_use]
use crate::error;
use error::Result;

fn is_primitive_type(path: &syn::TypePath) -> bool {
    if path.qself.is_some() {
        return false
    }

    if path.path.segments.len() != 1 {
        return false
    }

    const PRIMITIVE_TYPES: [&str; 12] = [
        "bool",
        "char",
        "u8",
        "u16",
        "u32",
        "u64",
        "usize",
        "i8",
        "i16",
        "i32",
        "i64",
        "isize"];

    PRIMITIVE_TYPES.contains(&path.path.segments[0].ident.to_string().as_str())
}

fn verify_field_type(ty: &syn::Type) -> Result<()> {
    match ty {
        syn::Type::Path(p) => {
            if is_primitive_type(p) {
                return Ok(())
            }

            fail_with_msg!("type \"{}\" is not a primitive type", quote::quote!(#ty))
        }
        _ => Ok(())
    }
}

fn verify_struct(st: &syn::ItemStruct) -> Result<()> {
    for field in &st.fields {
        try_with_msg!(
            verify_field_type(&field.ty),
            "invalid struct \"{}\"",
            st.ident)?;
    }
    Ok(())
}

fn verify_item(item: &syn::Item) -> Result<()> {
    match item {
        syn::Item::Fn(f) => fail_with_msg!("bare function \"{}\" not permitted", f.sig.ident),
        syn::Item::Struct(s) => verify_struct(s),
        _ => Ok(())
    }
}

pub fn verify_file(file: &syn::File) -> Result<()> {
    for item in &file.items {
        verify_item(item)?;
    }

    Ok(())
}