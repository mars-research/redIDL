extern crate syn;

use std::fs;
use std::env;

mod prune;

use syn::visit::Visit;

/*
    For nicer error contexts, we need to compute an attribute for every node that contains
    the full chain of scopes to it. This can be computed in a pass, but this means that we need to build a second tree,
    and consider the AST to be a more of a raw parse tree.

    This second tree, in turn, requires its own visitor system. And its own types.
    
    We construct a graph of types, and an index of types that must conform to certain specifications:
    - dyn references must refer to an exchangeable trait
    - RRefs must refer to RRef-able types
    - others must be Copy

    The graph expresses dependency relationships between types, so we traverse the graph in the process of "proving"
    This happens every time a new type definition is encountered; we walk this graph looking for type category information.
    We will inevitably encounter type dependencies in this "proof", these are used to assemble a proof dependency graph.
    We discover "roots" of this graph, resolve their proofs, then attempt to prove their immediate neighbors, etc.
    If a proof cycle is detected, the strategy is to pick an end to the cycle, assume that the proof exists, and eventually determine if
    a proof existed or not. Since there are many type categories, possible types, etc., and such cycle-breaking strategies might result in proof branching,
    we only construct proofs in cases where they are needed. This is aided by the fact that the context in which a type is used unambiguously
    determines what category it must be. To be more precise, this work is done only for trait methods.

    Since the usage of a type unambiguously identifies what category it *must* be in order for the use to be valid, it's possible to, based simply off the assumption
    that a type definition is legal, determine exactly what category it must be, and exactly what category dependent types must be in for said type to be valid.

    So a proof graph would label types with their category and their assumed category, and if a mismatch is detected, somehow determine which node assigned it
    the wrong category. There are several possible error cases, such as the type being used in mutually-exclusive contexts, or a type being used as one category but being another.
    If the type is used consistently in several places, but is a different category than needed, then the error is likely that the type was intended to be the former.
    In order to determine the nature of the error, it would be useful to collect information on all known usages of the type, how it was used, and what category it was.
    And of course, there are cases where the very structure of the type disqualifies it. I.e., using a function type.

    For a proof graph, the nodes would have labeled arrows to the other nodes. The representation must be fast for editing.
    Every arrow must be labelled not just by the expected category, but some representation of the context that created the dependency.

    Besides the proof graph, it's necessary to collect information on the location of RRefs (to insert getters and setters, and generate these),
    and traits' method signatures (for proxy generation). RRefs can be represented as DAGs, since the order in which we walk them from the parent type
    is the same as during ownership transfer, and due to the nature of OptRRefs, we will never walk a cycle. As for proxies, it's mostly a template, since
    proxies can be passed around freely, 
*/

enum TypeCategory {
    Functional,
    RRefable,
    Copyable
}

fn main() {
    let args: Vec<String> = env::args().collect();

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

    let mut rejector = prune::PruningVisitor::new();
    rejector.visit_file(&ast);
}
