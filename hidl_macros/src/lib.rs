use proc_macro::TokenStream;
use quote::quote;
use syn::{
	parse::{Parse, ParseStream},
	*,
};

struct TypeId {
	id: LitInt,
	ast: Type,
}

impl Parse for TypeId {
	fn parse(stream: ParseStream) -> Result<Self> {
		let id = stream.parse::<LitInt>()?;
		stream.parse::<Token! [,]>()?;
		let ast = stream.parse::<Type>()?;
		Ok(Self {
			id: id,
			ast: ast,
		})
	}
}

#[proc_macro]
pub fn assign_id(toks: TokenStream) -> TokenStream {
	let ast = parse_macro_input!(toks as TypeId);
	let ty = ast.ast;
	let n = ast.id;
	let new_toks = quote! {
		impl crate::sys::TypeIdentifiable for #ty {
			fn get_id() -> u64 {
				#n
			}
		}
	};

	new_toks.into()
}
