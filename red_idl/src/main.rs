extern crate syn;

mod walk;

use std::fs;
use std::env;

fn main() {
    let args : Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage (unstable interface): red_idl <test-path>");
        return
    }

    let content = match fs::read_to_string(&args[1]) {
        Ok(text) => text,
        Err(error) => {
            println!("Couldn't open file: {}", error);
            return
        }
    };

    let ast = match syn::parse_file(&content) {
        Ok(ast) => ast,
        Err(error) => {
            println!("Couldn't parse file: {}", error);
            return
        }
    };

    let null_pass = walk::NullPass {};
    walk::walk_file(&ast, &null_pass);
}
