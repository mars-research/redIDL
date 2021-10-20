use quote::format_ident;
use quote::quote;
use syn::parse_quote;
use super::*;


#[test]
fn test_refactor_path_in_path() {
    let src = format_ident!("crate");
    let dest = format_ident!("interface");
    let mut ast: Path = parse_quote! {
        crate::foo
    };
    refactor_path_in_path(&src, &dest, &mut ast);
    assert_eq!(ast, parse_quote!{
        interface::foo
    }, "\nPrettified token stream: {}", quote!(#ast).to_string());
}

#[test]
fn test_refactor_path_in_path_with_generic() {
    let src = format_ident!("crate");
    let dest = format_ident!("interface");
    let mut ast: Path = parse_quote! {
        crate::foo<crate::bar>
    };
    refactor_path_in_path(&src, &dest, &mut ast);
    assert_eq!(ast, parse_quote!{
        interface::foo<interface::bar>
    }, "\nPrettified token stream: {}", quote!(#ast).to_string());
}

#[test]
fn test_refactor_path_in_trait_item_method() {
    let src = format_ident!("crate");
    let dest = format_ident!("interface");
    let mut ast: TraitItemMethod = parse_quote! {
        fn create_domain_pci(&self) -> (Box<dyn syscalls::Domain>, Box<dyn crate::pci::PCI>);
    };
    refactor_path_in_trait_item_method(&src, &dest, &mut ast);
    assert_eq!(ast, parse_quote!{
        fn create_domain_pci(&self) -> (Box<dyn syscalls::Domain>, Box<dyn interface::pci::PCI>);
    }, "\nPrettified token stream: {}", quote!(#ast).to_string());
}

#[test]
fn test_refactor_path_in_tuple() {
    let src = format_ident!("crate");
    let dest = format_ident!("interface");
    let mut ast: syn::Type = parse_quote! {
        (Box<dyn syscalls::Domain>, Box<dyn crate::pci::PCI>)
    };
    refactor_path_in_type(&src, &dest, &mut ast);
    assert_eq!(ast, parse_quote!{
        (Box<dyn syscalls::Domain>, Box<dyn interface::pci::PCI>)
    }, "\nPrettified token stream: {}", quote!(#ast).to_string());
}

#[test]
fn test_refactor_path_in_trait_object() {
    let src = format_ident!("crate");
    let dest = format_ident!("interface");
    let mut ast: syn::Type = parse_quote! {
        Box<dyn crate::pci::PCI>
    };
    refactor_path_in_type(&src, &dest, &mut ast);
    assert_eq!(ast, parse_quote!{
        Box<dyn interface::pci::PCI>
    }, "\nPrettified token stream: {}", quote!(#ast).to_string());
}