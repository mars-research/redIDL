use proc_macro::TokenStream;
use syn::parse_quote;
use quote::quote;

pub fn redidl_resolve_module_and_generate_proxy_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        panic!("Macro generate_proxy does not take any attribute. Attributes: {}", attr);
    }
    
    let mut input: syn::ItemTrait = syn::parse(item).expect("interface definition must be a valid trait definition");

    // Add module_path and generate_prxy attributes and return
    input.attrs.push(
        parse_quote!(
            #[redidl_codegen_generate_proxy_placeholder_]
        )
    );
    input.attrs.push(
        parse_quote!(
            #[module_path = module_path!()]
        )
    );

    TokenStream::from(quote!(#input))
}