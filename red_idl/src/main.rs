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

/*
	Thankfully, Dan seems to have gotten the auto trait stuff to work, so we just need to prune illegal types at the compiler-level,
	then do the old macro-style stuff, but with these auto traits, and constraint checking.

	Checking conditions for parameters is easy with a helper, (either checking a type explicitly or the parameter implied type)
	RRefables get checked automatically at site-of-use, Copy is trivial to check, and Functional does not need to be auto.

	A composite is exchangeable if all its members are exchangeable. References to proxies are exchangeable, and they are also copy. The restriction
	is that the trait be an Interface. OptRRefs are exchangeable, and they enforce their own rules via the RRefable auto trait. The Copy auto trait
	already implements the semantics we need. To support RRefable, we reject all syntax nodes for closure, function, or pointer types (blanket rejection of
	non-exchangeable). If it's a dyn reference to a trait object, the trait must be Interface. If it's an OptRRef, we can skip it
	(questions about getters and setters, tuples), since it'll enforce it itself (we could prune type trees containing OptRRefs, to avoid getter/setter nonsense).

	- Need to tag the "Exchangeable" expressions (dyn references to proxies, etc.) with RRefable directly
	- How to deal with generics? Functions are easy, since we place things in-scope
		- apparently have to place marker trait requirement in struct decl
		- i.e. decl re-writing (read: tree rewriting)
		- we need to deduce what the type must be
		- so marker traits for everything
		- or a feed-through "checker" type

	trait Marker {}

	fn has_marker<T: Marker>(_: T) {}

	struct Test<'a, T> {
		a: &'a T
	}

	impl<'a, T> Test<'a, T> {
		fn foo(&self) {
			has_marker(self.a);
		}
	}

	Let's not prioritize error messages at the moment, m'kay?
*/

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
