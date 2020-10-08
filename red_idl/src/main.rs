extern crate syn;
extern crate quote;
extern crate idl_macros;

use std::env;
use std::fs::read_to_string;

// mod utility;
mod reject;
mod types;

use syn::{parse_file, File};
use syn::visit::Visit;
use types::{collect_method_signatures, collect_types};
use idl_macros::*;

#[proxy]
trait Foo {
}

fn test_proxy<T: idl_types::Proxy + ?Sized>() {}

fn load_ast(path: &str) -> Result<File, ()> {
	let content = match read_to_string(path) {
		Ok(text) => Ok(text),
		Err(error) => {
			println!("Couldn't open file: {}", error);
			Err(())
		}
	}?;

	let ast = match parse_file(&content) {
		Ok(ast) => Ok(ast),
		Err(error) => {
			println!("Couldn't parse file: {}", error);
			Err(())
		}
	}?;

	Ok(ast)
}

fn main() -> Result<(), ()> {
	// Need to block trait impl blocks that use the idl_types directly
	test_proxy::<dyn Foo>();

	let args: Vec<String> = env::args().collect();

	if args.len() != 2 {
		println!("Usage (unstable interface): red_idl <test-path>");
		return Ok(());
	}

	let ast = load_ast(&args[1])?;
	let mut rejector = reject::RejectPass { is_legal: true };
	rejector.visit_file(&ast);
	let types = collect_types(&ast)?;
	let _sigs = collect_method_signatures(&types.traits);

	Ok(())
}
