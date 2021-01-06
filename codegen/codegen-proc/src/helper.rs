use proc_macro::TokenStream;
use syn::{parse_quote, Ident};
use quote::{quote, format_ident};

static USR_LIB_NAME: &'static str = "usr";

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

pub fn redidl_generate_import_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        panic!("Macro generate_proxy does not take any attribute. Attributes: {}", attr);
    }
    
    let input: syn::ItemStruct = syn::parse(item).expect("interface definition must be a valid trait definition");

    // Extract module path
    let module_path = input.attrs.iter().filter_map(
        |attr| {
            if let Ok(syn::Meta::NameValue(meta)) = attr.parse_meta(){
                if let Some(ident) = meta.path.get_ident() {
                    if ident.to_string() == "module_path" {
                        if let syn::Lit::Str(lit) = meta.lit {
                            return Some(lit);
                        } else {
                            panic!("module_path must be a string")
                        }
                    }
                }
            }
            None
        }
    ).next().expect("module_path not found").value();


    let import_path_segs = generate_import_path_segs(&module_path, &input.ident);

    TokenStream::from(quote!{
        use ::#(#import_path_segs)::*;
    })
}

pub(crate) fn generate_import_path_segs(module_path: &str, ident: &Ident) -> Vec<Ident> {
    let mut rtn: Vec<Ident> = vec![Ident::new(USR_LIB_NAME, ident.span())];
    for path_segment in module_path.split("::").skip(1) {
        rtn.push(Ident::new(path_segment, ident.span()));
    }
    rtn.push(format_ident!("{}", ident));
    rtn
}