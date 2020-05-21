use crate::error::Result;
use std::path::Path;
use std::fs::File;
use std::io::{Read, Write};

// Long story short, each method of the Create* proxies must generate the whole irq/free fn call stuff
// plus a whole load of other crap

fn process_create_signature(m: &syn::TraitItemMethod, file: &mut File) -> Result<()> {
    let sig = &m.sig;
    let sig_str = quote::quote!(#sig).to_string();
    let mut call_str = m.sig.ident.to_string() + "(";
    let mut first_arg = true;
    for item in &sig.inputs {
        match item {
            syn::FnArg::Typed(pat) => {
                let name = &pat.pat;
                if !first_arg {
                    call_str += ", ";
                }

                call_str += &quote::quote!(#name).to_string();                
                first_arg = false
            },
            _ => ()
        }
    }

    call_str += ")";
    writeln!(file, "\t{} {{", sig_str)?;
    writeln!(file, "\t\tdisable_irq();\n\t\tlet r = {};\n\t\tenable_irq();\n\t\tr\n\t}}", call_str)?;
    Ok(())
}

fn process_trait(tr: &syn::ItemTrait, file: &mut File) -> Result<()> {
    let name = tr.ident.to_string();
    writeln!(file, "impl create::{} for PDomain {{", name)?;

    for item in &tr.items {
        match item {
            syn::TraitItem::Method(m) => process_create_signature(m, file)?,
            _ => fail_with_msg!("creation traits may only have methods")
        }
    }

    writeln!(file, "}}\n")?;

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

    let mut file = try_with_msg!(
        crate::create_subfile(root, "src/_gen.rs"),
        "couldn't open src/_gen.rs")?;

    for item in &ast.items {
        match item {
            syn::Item::Trait(tr) => try_with_msg!(
                process_trait(tr, &mut file),
                "illegal trait {}",
                tr.ident.to_string())?,
            _ => ()
        }
    }

    Ok(())
}