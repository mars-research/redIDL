use syn::{Item, parse_quote};

pub mod rrefed_finder;
pub mod type_resolver;
pub mod symbol_tree;
mod utils;

#[cfg(test)]
mod rrefed_finder_test;
#[cfg(test)]
mod e2e_test;

pub fn generate_typeid(ast: &mut syn::File) {
    // Resolve types
    let type_resolver = type_resolver::TypeResolver::new();
    let symbol_tree = type_resolver.resolve_types(&ast);

    // Find all `RRef`ed types
    let rref_finder = rrefed_finder::RRefedFinder::new(symbol_tree.clone());
    let rrefed_types = rref_finder.find_rrefed(&ast);

    // Generate code
    let impls = rrefed_types.iter().enumerate().map(|(i, ty)| {
        let i = i as u64;
        Item::Impl(parse_quote!{
            impl TypeIdentifiable for #ty {
                fn type_id() -> u64 {
                    #i
                }
            }
        })
    });

    let md = parse_quote! {
        pub mod typeid {
            pub trait TypeIdentifiable {
                fn type_id() -> u64;
            }

            #(#impls)*
        }
    };
    ast.items.push(Item::Mod(md));
}