extern crate syn;
extern crate quote;

use std::fs;
use std::io::Write;

use crate::error;
use error::Result;

pub struct DeferredChecks {
    safe_copy_types: Vec<String>,
    rrefable_types: Vec<String>,
    functional_types: Vec<String>,
    safe_copy_needed: Vec<String>,
    rrefable_needed: Vec<String>,
    _functional_needed: Vec<String>
}

/*
    Another type system revision!
    Note that no IDL type may exist outside of this
    Introducing SafeCopy -
        - Is Copy (so we can bitwise copy)
        - Does not have references or pointers of any kind (so we know that we can copy it out of a domain,
            and it won't reference anything in that domain)
        //- Is a struct (for now)

    Enum is blanket allow (RRefable)
    
    Introducing the *new* RRefable -
        - Extends SafeCopy, allowing OptRRef<> members
    
    Functional remains the same

    make sure the macros get the full "generic typename", have them extract the required info
    but what about cases where the full name is implied?
*/

impl DeferredChecks {
    pub fn new() -> DeferredChecks {
        DeferredChecks {
            safe_copy_types: Vec::new(),
            rrefable_types: Vec::new(),
            functional_types: Vec::new(),
            safe_copy_needed: Vec::new(),
            rrefable_needed: Vec::new(),
            _functional_needed: Vec::new()
        }
    }

    pub fn write(&self, file: &mut fs::File) -> Result<()> {
        for entry in &self.safe_copy_types {
            writeln!(file, "red_idl::declare_safe_copy!({0});", entry)?;
        }

        for entry in &self.rrefable_types {
            writeln!(file, "red_idl::declare_rrefable!({});", entry)?;
        }

        for entry in &self.functional_types {
            writeln!(file, "red_idl::declare_functional!({});", entry)?;
        }

        for entry in &self.safe_copy_needed {
            writeln!(file, "red_idl::require_copy!({0});\nred_idl::require_safe_copy!({});", entry)?;
        }

        for entry in &self.rrefable_needed {
            writeln!(file, "red_idl::require_rrefable!({});", entry)?;
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

fn is_opt_rref_path(path: &syn::TypePath) -> bool {
    if path.qself.is_some() {
        return false
    }

    // TODO: is this necessary?
    if path.path.segments.len() != 1 { // Just OptRRef<>
        return false
    }

    path.path.segments[0].ident.to_string() == "OptRRef" // 
}

// impl red_idl::Functional for Foo {} (Disallow)
/* Allow
impl Foo {

}
*/

fn verify_ident(id: &syn::Ident) -> Result<()> {
    if id == "OptRRef" || id == "RRef" {
        fail_with_msg!("illegal use of reserved name \"{}\"", id.to_string())
    }

    Ok(())
}

// Need for a source -> dest transformation from unverified -> verified IDL
// For now, just use the _* convention for testing dirs

// TODO: type nesting handling

fn verify_rref_type(ty: &syn::Type, macros: &mut DeferredChecks) -> Result<()> {
    match ty {
        syn::Type::Path(p) => { // Example "Path"s: "DeferredChecks", "syn::Type", "usize"
            macros.rrefable_needed.push(quote::quote!(#p).to_string());
            Ok(())
        }
        _ => Ok(()) // OptRRef<&FooTrait>, OptRRef<usize*> (Revise)
    }
}

enum FieldType {
    Normal,
    OptRRef
}

fn verify_field_type(ty: &syn::Type, macros: &mut DeferredChecks) -> Result<FieldType> {
    match ty {
        syn::Type::Path(p) => {
            if is_primitive_type(p) {
                return Ok(FieldType::Normal)
            }

            if is_opt_rref_path(p) {
                // TODO: broken, doesn't check the type parameter
                try_with_msg!(
                    verify_rref_type(ty, macros),
                    "type not rref-able")?;

                // At this point, we should make deferred check for SafeCopy-ness
                macros.safe_copy_needed.push(quote::quote!(#p).to_string());

                return Ok(FieldType::OptRRef)
            }

            // At this point, we should make deferred check for SafeCopy-ness
            macros.safe_copy_needed.push(quote::quote!(#p).to_string());

            Ok(FieldType::Normal)
        },
        syn::Type::BareFn(_) => fail_with_msg!("fields cannot be function pointers"), // 
        syn::Type::Ptr(_) => fail_with_msg!("fields cannot be pointers"), // 
        _ => Ok(FieldType::Normal) // (Revise)
    }
}

fn verify_struct(st: &syn::ItemStruct, macros: &mut DeferredChecks) -> Result<()> {
    verify_ident(&st.ident)?;

    let mut is_safe_copy = true;
    for field in &st.fields {
        let ftype = try_with_msg!(
            verify_field_type(&field.ty, macros),
            "invalid struct \"{}\"",
            st.ident)?;

        if let FieldType::OptRRef = ftype {
            is_safe_copy = false;
        }
    }

    let ident = &st.ident;
    let generics = &st.generics;
    if is_safe_copy {
        macros.safe_copy_types.push(quote::quote!(#ident#generics).to_string())
    }
    else {
        macros.rrefable_types.push(quote::quote!(#ident#generics).to_string())
    }

    Ok(())
}

// Currently assume that all enums are SafeCopy (this automatically requires that it be Copy)
fn verify_enum(e: &syn::ItemEnum, macros: &mut DeferredChecks) -> Result<()> {
    macros.safe_copy_types.push(e.ident.to_string()); // TODO: this needs to take into account contained types
    Ok(())
}

// TODO: get method signature checking working (after deadline)
fn verify_trait(tr: &syn::ItemTrait, macros: &mut DeferredChecks) -> Result<()> {
    for item in &tr.items {
        match item {
            syn::TraitItem::Method(_) => (), // this is where signature checks *would* go, if they were enabled
            _ => fail_with_msg!("traits must only contain methods")
        }
    }

    macros.functional_types.push(tr.ident.to_string());

    Ok(())
}

// Incomplete
fn verify_item(item: &syn::Item, macros: &mut DeferredChecks) -> Result<()> {
    match item {
        syn::Item::Fn(f) => fail_with_msg!("bare function \"{}\" not permitted", f.sig.ident), // 
        syn::Item::Struct(s) => verify_struct(s, macros), //
        syn::Item::Enum(e) => verify_enum(e, macros),
        syn::Item::Trait(tr) => verify_trait(tr, macros),
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