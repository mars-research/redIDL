extern crate syn;
extern crate quote;

use std::fs;
use std::io::Write;

use crate::error;
use error::Result;

pub struct DeferredChecks {
    _safe_copy_types: Vec<String>,
    _rrefable_types: Vec<String>,
    _functional_types: Vec<String>,
    safe_copy_needed: Vec<String>,
    rrefable_needed: Vec<String>,
    _functional_needed: Vec<String>
}

impl DeferredChecks {
    pub fn new() -> DeferredChecks {
        DeferredChecks {
            _safe_copy_types: Vec::new(),
            _rrefable_types: Vec::new(),
            _functional_types: Vec::new(),
            safe_copy_needed: Vec::new(),
            rrefable_needed: Vec::new(),
            _functional_needed: Vec::new()
        }
    }

    pub fn write(&self, file: &mut fs::File) -> Result<()> {
        for entry in &self.safe_copy_needed {
            writeln!(file, "red_idl::require_copy!({0});\nred_idl::require_safe_copy!({0});", entry)?;
        }

        Ok(())
    }
}

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

fn is_opt_rref_type(path: &syn::TypePath) -> bool {
    if path.qself.is_some() {
        return false
    }

    // TODO: is this necessary?
    if path.path.segments.len() != 1 {
        return false
    }

    path.path.segments[0].ident.to_string() == "OptRRef"
}

// Need for a source -> dest transformation from unverified -> verified IDL
// For now, just use the _* convention for testing dirs

fn verify_rref_type(ty: &syn::Type, macros: &mut DeferredChecks) -> Result<()> {
    match ty {
        syn::Type::Path(p) => {
            macros.rrefable_needed.push(quote::quote!(#p).to_string());
            Ok(())
        }
        _ => Ok(())
    }
}

fn verify_field_type(ty: &syn::Type, macros: &mut DeferredChecks) -> Result<()> {
    match ty {
        syn::Type::Path(p) => {
            if is_primitive_type(p) {
                return Ok(())
            }

            if is_opt_rref_type(p) {
                try_with_msg!(
                    verify_rref_type(ty, macros),
                    "type not rref-able")?;
            }

            // At this point, we should make deferred check for SafeCopy-ness
            macros.safe_copy_needed.push(quote::quote!(#p).to_string());

            Ok(())
        }
        _ => Ok(())
    }
}

fn verify_struct(st: &syn::ItemStruct, macros: &mut DeferredChecks) -> Result<()> {
    for field in &st.fields {
        try_with_msg!(
            verify_field_type(&field.ty, macros),
            "invalid struct \"{}\"",
            st.ident)?;
    }
    Ok(())
}

fn verify_item(item: &syn::Item, macros: &mut DeferredChecks) -> Result<()> {
    match item {
        syn::Item::Fn(f) => fail_with_msg!("bare function \"{}\" not permitted", f.sig.ident),
        syn::Item::Struct(s) => verify_struct(s, macros),
        _ => Ok(())
    }
}

pub fn verify_file(file: &syn::File) -> Result<DeferredChecks> {
    let mut macros = DeferredChecks::new();

    for item in &file.items {
        verify_item(item, &mut macros)?;
    }

    Ok(macros)
}