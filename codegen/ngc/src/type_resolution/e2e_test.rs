use quote::quote;

fn expect_to_generate_typeid(input: &str, types: Vec<&str>) {
    // Get expected output
    let expected_output = format!("
        {}

        pub mod typeid {{
            pub trait TypeIdentifiable {{
                fn type_id() -> u64;
            }}

            {}
        }}
    ", input, types.iter().enumerate().map(|(i, ty)| {
        format!("
            impl TypeIdentifiable for {} {{
                fn type_id() -> u64 {{
                    {}u64
                }}
            }}
        ", ty, i)
    }).collect::<Vec<String>>().join(""));
    let expected_ast = syn::parse_file(&expected_output).unwrap();

    // Generate code.
    let mut ast = syn::parse_file(input).unwrap();
    super::generate_typeid(&mut ast);

    // Assert equality
    assert_eq!(quote!(#expected_ast).to_string(), quote!(#ast).to_string());
}

#[test]
fn simple_test() {
    let input = "
        #[interface]
        pub trait Foo {
            fn bar(&self, fd: usize) -> ();
        }
    ";

    expect_to_generate_typeid(input, vec!["usize", "()"]);
}

#[test]
fn test_generic() {
    let input = "
        use alloc::vec::Vec;
        use asd::X;

        #[interface]
        pub trait Foo {
            fn bar(&self, car: Vec<X>) -> ();
        }
    ";

    expect_to_generate_typeid(input, vec!["asd::X", "alloc::vec::Vec<asd::X>", "()"]);
}