use syn::{Item, Type, parse_quote};

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
    let rrefed_types: Vec<Type> = rref_finder.find_rrefed(&ast).into_iter().collect();

    // Generate code
    let impls: Vec<Item> = rrefed_types.iter().enumerate().map(|(i, ty)| {
        let i = i as u64;
        Item::Impl(parse_quote!{
            impl TypeIdentifiable for #ty {
                fn type_id() -> u64 {
                    #i
                }
            }
        })
    }).collect();

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
            use crate::rref::traits::{CustomCleanup, TypeIdentifiable};

            /// Drops the pointer, assumes it is of type T
            fn drop_t<T: CustomCleanup + TypeIdentifiable>(ptr: *mut u8) {
                // println!("DROPPING {}", core::any::type_name::<T>());
                unsafe {
                    let ptr_t: *mut T = transmute(ptr);
                    // recursively invoke further shared heap deallocation in the tree of rrefs
                    (&mut *ptr_t).cleanup();
                }
            }

            struct DropMap(HashMap<u64, fn(*mut u8) -> ()>);

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

                pub fn get_drop(&self, type_id: u64) -> Option<&fn(*mut u8) -> ()> {
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