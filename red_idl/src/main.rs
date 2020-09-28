extern crate syn;

#[macro_use]
extern crate quote;

use std::env;
use std::fs;

// mod utility;
mod types;

use syn::visit::Visit;
use types::{SignaturesCollectionPass, TraitSignatures, TypeDefinitions, TypesCollectionPass};

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
*/

fn main() {
	let args: Vec<String> = env::args().collect();

	if args.len() != 2 {
		println!("Usage (unstable interface): red_idl <test-path>");
		return;
	}

	let content = match fs::read_to_string(&args[1]) {
		Ok(text) => text,
		Err(error) => {
			println!("Couldn't open file: {}", error);
			return;
		}
	};

	let ast = match syn::parse_file(&content) {
		Ok(ast) => ast,
		Err(error) => {
			println!("Couldn't parse file: {}", error);
			return;
		}
	};

	let mut types = TypeDefinitions::new();
	let mut type_collector = TypesCollectionPass::new(&mut types);
	type_collector.visit_file(&ast);

	for tr in &types.traits {
		println!("{}", quote! {#tr}.to_string())
	}

	for st in types.structs {
		println!("{}", quote! {#st}.to_string())
	}

	let mut sigs = TraitSignatures::new();
	for tr in &types.traits {
		let start = sigs.signatures.len();
		let mut pass = SignaturesCollectionPass::new(&mut sigs.signatures);
		pass.visit_item_trait(tr);
		let end = sigs.signatures.len();

		if start == end {
			println!("No methods recorded")
		} else {
			println!("{} methods recorded", end - start);
			sigs.ranges.push(start..end);
		}
	}
}
