#![feature(log_syntax, proc_macro_def_site)]

use proc_macro::TokenStream;

mod helper;
mod proxy;

#[proc_macro_attribute]
pub fn redidl_resolve_module_and_generate_proxy(attr: TokenStream, item: TokenStream) -> TokenStream {
    crate::helper::redidl_resolve_module_and_generate_proxy_impl(attr, item)
}

#[proc_macro_attribute]
pub fn redidl_generate_import(attr: TokenStream, item: TokenStream) -> TokenStream {
    crate::helper::redidl_generate_import_impl(attr, item)
}


/// Generate the proxy for an interface definition.
#[proc_macro_attribute]
pub fn redidl_generate_proxy(attr: TokenStream, item: TokenStream) -> TokenStream {
    crate::proxy::redidl_generate_proxy_impl(attr, item)
}