use log::info;
use syn::{parse_quote, Item, Type};

pub mod rrefed_finder;
pub mod symbol_tree;
pub mod type_resolver;
mod utils;

#[cfg(test)]
mod e2e_test;
#[cfg(test)]
mod rrefed_finder_test;

pub fn generate_typeid(ast: &mut syn::File) {
    // Resolve types
    info!("Resolving types");
    let type_resolver = type_resolver::TypeResolver::new();
    let symbol_tree = type_resolver.resolve_types(&ast);

    // Find all `RRef`ed types
    info!("Finding `RRef`ed types");
    let rref_finder = rrefed_finder::RRefedFinder::new(symbol_tree);
    let rrefed_types: Vec<Type> = rref_finder.find_rrefed(&ast).into_iter().collect();

    // Generate code
    info!("Generating `TypeIdentifiable`");
    let impls: Vec<Item> = rrefed_types
        .iter()
        .enumerate()
        .map(|(i, ty)| {
            let i = i as u64;
            Item::Impl(parse_quote! {
                impl TypeIdentifiable for #ty {
                    fn type_id() -> u64 {
                        #i
                    }
                }
            })
        })
        .collect();

    // Remove the existing typeid module
    ast.items.retain(|item| {
        if let Item::Mod(item) = item {
            return item.ident != "typeid";
        }
        true
    });

    // Inject the new typeid module
    let md = parse_quote! {
        pub mod typeid {
            /// BEGIN Generated TypeIdentifiable
            pub trait TypeIdentifiable {
                fn type_id() -> u64;
            }
            #(#impls)*
            /// END Generated TypeIdentifiable

            // BEGIN Generated DropMap
            use hashbrown::HashMap;
            use crate::rref::traits::{CustomCleanup};

            /// Drops the pointer, assumes it is of type T
            unsafe fn drop_t<T: CustomCleanup + TypeIdentifiable>(ptr: *mut u8) {
                // Cast the pointer to a pointer of type T
                let ptr_t: *mut T = core::mem::transmute(ptr);
                // Drop the object pointed by the buffer. This should recursively drop all the
                // nested fields of type T, if there's any.
                core::mem::drop(ptr_t);
            }

            pub struct DropMap(HashMap<u64, unsafe fn(*mut u8) -> ()>);

            impl DropMap {
                pub fn new() -> Self {
                    let mut drop_map = Self(HashMap::new());
                    drop_map.populate_drop_map();
                    drop_map
                }

                fn add_type<T: 'static + CustomCleanup + TypeIdentifiable>(&mut self) {
                    let type_id = T::type_id();
                    let type_erased_drop = drop_t::<T>;
                    self.0.insert(type_id, type_erased_drop);
                }

                pub fn get_drop(&self, type_id: u64) -> Option<&unsafe fn(*mut u8) -> ()> {
                    self.0.get(&type_id)
                }

                fn populate_drop_map(&mut self) {
                    #(self.add_type::<#rrefed_types>();)*
                }
            }
            // END Generated DropMap
        }
    };
    ast.items.push(Item::Mod(md));
}
