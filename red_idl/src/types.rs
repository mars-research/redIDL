use syn::visit;
use visit::Visit;

#[derive(Clone, Copy)]
enum TypeAncestor {
    File,
    Module
}

pub struct TypeDefinitions<'ast> {
    pub traits: Vec<&'ast syn::ItemTrait>,
    pub structs: Vec<&'ast syn::ItemStruct>
}

impl<'ast> TypeDefinitions<'ast> {
    pub fn new() -> Self {
        Self {
            traits: Vec::new(),
            structs: Vec::new()
        }
    }
}

// Collects top-level structs and traits
pub struct TypesCollectionPass<'ast, 'types> {
    ancestor: TypeAncestor,
    types: &'types mut TypeDefinitions<'ast>
}

// It's important to only collect type nodes that occur as children of specific nodes
// So the walk keeps a track of which possible ancestor is currently nearest
// Signature nodes can be children of TraitItemMethod, ImplItemMethod, or ItemFn
// We're only interested in signatures of TraitItemMethod for context information

// More interesting is the problem of only collecting top-level types
// I.e., we want to collect top-level struct and trait definitions

impl<'ast, 'types> TypesCollectionPass<'ast, 'types> {
    pub fn new(types: &'types mut TypeDefinitions<'ast>) -> Self {
        Self {
            ancestor: TypeAncestor::File,
            types: types
        }
    }
}

impl<'ast, 'vecs> Visit<'ast> for TypesCollectionPass<'ast, 'vecs> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        let last_ancestor = self.ancestor;
        self.ancestor = TypeAncestor::Module;
        visit::visit_item_mod(self, node);
        self.ancestor = last_ancestor
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        match self.ancestor {
            TypeAncestor::File => self.types.traits.push(node),
            TypeAncestor::Module => println!("IDL requires all types to be defined at global scope")
        }

        visit::visit_item_trait(self, node)
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        match self.ancestor {
            TypeAncestor::File => self.types.structs.push(node),
            TypeAncestor::Module => ()
        }

        visit::visit_item_struct(self, node)
    }

    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        println!("Typedefs are currently unsupported");
        visit::visit_item_type(self, node)
    }
}

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
