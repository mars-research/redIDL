#![feature(proc_macro_diagnostic)]

extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::{Diagnostic, Level, TokenStream};
use quote::quote;

use export::*;
use punctuated::Punctuated;
use spanned::Spanned;
use syn::*;
use token::Brace;
use visit::Visit;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

#[proc_macro_derive(Exchangeable)]
pub fn derive_exchangeable(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let _struct_tree = match &ast.data {
        Data::Struct(_) => println!("ok"),
        _ => panic!("only understand structs right now"),
    };

    TokenStream::from(quote! {})
}

#[proc_macro_derive(RRefable)]
pub fn derive_rrefable(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let _struct_tree = match &ast.data {
        Data::Struct(_) => println!("ok"),
        _ => panic!("only understand structs right now"),
    };

    TokenStream::from(quote! {})
}

fn error_on_spanned<T: Spanned>(node: &T, msg: &str) {
    let span = node.span().unwrap();
    let dg = Diagnostic::spanned(vec![span], Level::Error, msg);
    dg.emit();
}

struct EnsureProxyVisitor;

impl<'ast> Visit<'ast> for EnsureProxyVisitor {
    fn visit_trait_item_type(&mut self, node: &'ast TraitItemType) {
        error_on_spanned(node, "traits marked #[proxy] may only contain methods");
        visit::visit_trait_item_type(self, node);
    }

    fn visit_trait_item_const(&mut self, node: &'ast TraitItemConst) {
        error_on_spanned(node, "traits marked #[proxy] may only contain methods");
        visit::visit_trait_item_const(self, node);
    }

    fn visit_trait_item_macro(&mut self, node: &'ast TraitItemMacro) {
        error_on_spanned(node, "traits marked #[proxy] may only contain methods");
        visit::visit_trait_item_macro(self, node);
    }
}

#[proc_macro_attribute]
pub fn proxy(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as ItemTrait);
    let mut path = Punctuated::new();
    // FIXME: future bug with generic proxies
    path.push(PathSegment {
        ident: ast.ident.clone(),
        arguments: PathArguments::None,
    });

    let mut marker = Punctuated::new();
    marker.push(PathSegment {
        ident: Ident::new("idl_types", Span::call_site()),
        arguments: PathArguments::None,
    });
    marker.push(PathSegment {
        ident: Ident::new("Proxy", Span::call_site()),
        arguments: PathArguments::None,
    });

    let mut bounds = Punctuated::new();
    bounds.push(TypeParamBound::Trait(TraitBound {
        paren_token: None,
        modifier: TraitBoundModifier::None,
        lifetimes: None,
        path: Path {
            leading_colon: None,
            segments: path,
        },
    }));

    let impl_block = ItemImpl {
        attrs: Vec::new(),
        defaultness: None,
        unsafety: None,
        impl_token: Token![impl](Span::call_site()),
        generics: Generics {
            lt_token: None,
            params: Punctuated::new(),
            gt_token: None,
            where_clause: None,
        },
        trait_: Some((
            None,
            Path {
                leading_colon: None,
                segments: marker,
            },
            Token![for](Span::call_site()),
        )),
        self_ty: Box::new(Type::TraitObject(TypeTraitObject {
            dyn_token: Some(Token![dyn](Span::call_site())),
            bounds: bounds,
        })),
        brace_token: Brace {
            span: Span::call_site(),
        },
        items: Vec::new(),
    };

    let mut visitor = EnsureProxyVisitor {};
    visitor.visit_item_trait(&ast);

    let result = quote! {
        #ast
        #impl_block
    };

    println!("{}", result);

    TokenStream::from(result)
}
