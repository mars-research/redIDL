use syn::*;
use visit::Visit;

/*
    Queries over type trees:
    - Is this an RRef-ed type? (implies RRefable is required)
    - Is this a reference to a dynamic trait? (requires Functional)
    - Is this a copy type?
    - Is this an OptRRef?
    - Is this type tree RRefable?
    
    I propose:
    - TypesCollectionPass (to construct an array of localized type trees)
    - FunctionalRequiredPass
    - RRefableRequiredPass
    // - CopyRequiredPass
    - PropagateRRefableRequiredPass
    - PropagateCopyRequiredPass
    - PropagateFunctionalRequiredPass (the equivalent for functional is a no-op)

    (Notice that since RRef isn't copy, RRef and OptRRef have identical type semantics but exist in mutually exclusive contexts)
    Also, RRefs can only refer to OptRRefs indirectly (i.e., you'll never see RRef<OptRRef<u32>>)
*/

// Need to tag the final type-trees with a location in the source code (by scope)
// i.e.: `trait Foo, method add_widget, parameter name`, or `struct Foo, field Bar`
// _Could_ fold all of this type-collection machinery into a single tree-walk state machine

#[derive(Clone, Copy)]
enum TypeAncestor {
    File,
    Module
}

pub struct TypeDefinitions<'ast> {
    pub traits: Vec<&'ast ItemTrait>,
    pub structs: Vec<&'ast ItemStruct>
}

pub struct TraitSignatures<'ast> {
    pub signatures: Vec<&'ast Signature>,
    pub ranges: Vec<std::ops::Range<usize>>
}

// Collects top-level structs and traits
pub struct TopLevelTypesPass<'ast, 'types> {
    ancestor: TypeAncestor,
    types: &'types mut TypeDefinitions<'ast>
}

// Intended to be run over trait subtrees
pub struct SignaturesCollectionPass<'ast, 'sigs> {
    signatures: &'sigs mut Vec<&'ast Signature>
}

struct _ArgumentTypePass;
struct _ReturnTypePass;
struct _FieldTypePass;

impl<'ast> TraitSignatures<'ast> {
    pub fn new() -> Self {
        Self {
            signatures: Vec::new(),
            ranges: Vec::new()
        }
    }
}

impl<'ast> TypeDefinitions<'ast> {
    pub fn new() -> Self {
        Self {
            traits: Vec::new(),
            structs: Vec::new()
        }
    }
}

impl<'ast, 'sigs> SignaturesCollectionPass<'ast, 'sigs> {
    pub fn new(sigs: &'sigs mut Vec<&'ast Signature>) -> Self {
        Self {
            signatures: sigs
        }
    }
}

impl<'ast, 'sigs> Visit<'ast> for SignaturesCollectionPass<'ast, 'sigs> {
    fn visit_signature(&mut self, node: &'ast Signature) {
        self.signatures.push(node);
        visit::visit_signature(self, node);
    }
}

// It's important to only collect type nodes that occur as children of specific nodes
// So the walk keeps a track of which possible ancestor is currently nearest
// Signature nodes can be children of TraitItemMethod, ImplItemMethod, or ItemFn
// We're only interested in signatures of TraitItemMethod for context information

// More interesting is the problem of only collecting top-level types
// I.e., we want to collect top-level struct and trait definitions

impl<'ast, 'types> TopLevelTypesPass<'ast, 'types> {
    pub fn new(types: &'types mut TypeDefinitions<'ast>) -> Self {
        Self {
            ancestor: TypeAncestor::File,
            types: types
        }
    }
}

impl<'ast, 'vecs> Visit<'ast> for TopLevelTypesPass<'ast, 'vecs> {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        let last_ancestor = self.ancestor;
        self.ancestor = TypeAncestor::Module;
        visit::visit_item_mod(self, node);
        self.ancestor = last_ancestor
    }

    fn visit_item_trait(&mut self, node: &'ast ItemTrait) {
        match self.ancestor {
            TypeAncestor::File => self.types.traits.push(node),
            TypeAncestor::Module => println!("IDL requires all types to be defined at global scope")
        }

        visit::visit_item_trait(self, node)
    }

    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        match self.ancestor {
            TypeAncestor::File => self.types.structs.push(node),
            TypeAncestor::Module => ()
        }

        visit::visit_item_struct(self, node)
    }

    fn visit_item_type(&mut self, node: &'ast ItemType) {
        println!("Typedefs are currently unsupported");
        visit::visit_item_type(self, node)
    }
}

pub fn collect_types(ast: &File) -> TypeDefinitions {
	let mut types = TypeDefinitions::new();
	let mut type_collector = TopLevelTypesPass::new(&mut types);
	type_collector.visit_file(&ast);

	for tr in &types.traits {
		println!("{}", quote! {#tr}.to_string())
	}

	for st in &types.structs {
		println!("{}", quote! {#st}.to_string())
	}

	types
}

pub fn collect_method_signatures<'ast>(traits: &[&'ast ItemTrait]) -> TraitSignatures<'ast> {
    let mut sigs = TraitSignatures::new();
	for tr in traits {
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
    
    sigs
}
