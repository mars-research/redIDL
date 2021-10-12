use std::collections::HashMap;

use syn::{FnArg, NestedMeta};

#[macro_export]
macro_rules! has_attribute {
    ($item: ident, $attr: ident) => {{
        let mut ret = false;
        for attr in &$item.attrs {
            if let Ok(meta) = attr.parse_meta() {
                if meta.path().is_ident($attr) {
                    ret = true;
                    break;
                }
            }
        }
        ret
    }};
}

#[macro_export]
macro_rules! remove_attribute {
    ($item: ident, $attr: ident) => {
        $item.attrs.retain(|attr| {
            if let Ok(meta) = attr.parse_meta() {
                if meta.path().is_ident($attr) {
                    return false;
                }
            }
            true
        });
    };
}

#[macro_export]
macro_rules! add_attribute {
    ($item: ident, $attr: literal) => {
        $item.attrs.push({
            // let _att: syn::Attribute = parse_quote!{ asd };
            // let stream = proc_macro::TokenStream::from_str("asd");
            // let _att1: syn::Attribute = syn::parse_quote!{#$stream};
            // let _att1: syn::Attribute = syn::parse(TokenStream::from_str($attr)).unwrap();
            // syn::parse_str($attr).expect(concat!("Failed to parse attribute: {}", $attr))
            panic!("")
        });
    };
}

#[macro_export]
macro_rules! for_enums_add_attribute {
    ($item: ident, $attr: literal, $($variant: path)*) => {
        match $item {
            $($variant(x) => crate::add_attribute!(x, $attr),)*
            _ => {},
        }
    };
}

/// A faster alternative for `result.expect(fmt, args)`
/// `Result::expect` will run the formatter regardless of the result.
/// This macros allows us to not run the formatter when the result is a `Some`.
#[macro_export]
macro_rules! expect {
    ($result:expr, $fmt:expr, $($args:tt)*) => {
        match $result {
            Some(result) => result,
            None => panic!($fmt, $($args)*),
        }
    };
}

pub fn create_attribue_map(attrs: &Vec<syn::Attribute>) -> HashMap<String, Option<syn::Lit>> {
    let mut map = HashMap::new();
    for attr in attrs {
        map.extend(create_attribue_map_from_meta(&attr.parse_meta().unwrap()))
    }
    map
}

fn create_attribue_map_from_meta(meta: &syn::Meta) -> HashMap<String, Option<syn::Lit>> {
    let mut map = HashMap::new();
    match meta {
        syn::Meta::List(lst) => {
            for meta in &lst.nested {
                match meta {
                    NestedMeta::Meta(meta) => map.extend(create_attribue_map_from_meta(meta)),
                    NestedMeta::Lit(lit) => {
                        match lit {
                            syn::Lit::Str(str) => map.insert(str.value(), None),
                            _ => unimplemented!("{:#?}", lit),
                        };
                    }
                };
            }
        }
        syn::Meta::NameValue(kv) => {
            map.insert(
                kv.path.get_ident().unwrap().to_string(),
                Some(kv.lit.clone()),
            );
        }
        syn::Meta::Path(path) => {
            map.insert(path.get_ident().unwrap().to_string(), None);
        }
    }
    map
}

// Remove `self` from the argument list.
pub fn get_selfless_args<'a, T: Iterator<Item = &'a FnArg>>(args: T) -> Vec<&'a FnArg>{
    args
        .filter(|arg| match arg {
            FnArg::Receiver(_) => false,
            FnArg::Typed(_) => true,
        })
        .collect()
}

// Get `T` from `Boxed<T>`. Panic if it's not a box.  
pub fn get_type_inside_of_box(ty: &syn::Type) -> &syn::Type {
    match ty {
        syn::Type::Path(path) => {
            // TODO: check that this is actually a box.
            let last_segement = path.path.segments.iter().last().unwrap();
            match &last_segement.arguments {
                syn::PathArguments::AngleBracketed(args) => {
                    assert_eq!(args.args.len(), 1);
                    let arg = args.args.first().unwrap();
                    match arg {
                        syn::GenericArgument::Type(ty) => ty,
                        _ => panic!("Expecting a type in the box but get: {:#?}", arg),
                    }
                },
                _ => panic!("Expecting Boxed<T> but get: {:#?}", path),
            }
        }
        _ => panic!("Expecting a box but found: {:#?}", ty),
    }
}