use syn::visit;
use visit::Visit;
use crate::utility::FlatMap;
use quote;

type TypeId = usize;

enum TypeCategory {
    Functional,
    RRefable,
    Copyable
}

#[derive(Clone, Copy)]
enum TypeState {
    None,
    Error,
    Reference,
    RRef,
    Copy
}

pub struct ProofGraphVisitor {
    state: TypeState,
    type_names: Vec<String>,
    graph: FlatMap<(TypeId, TypeId), TypeCategory>
}

impl ProofGraphVisitor {
    pub fn new() -> Self {
        Self {
            state: TypeState::None,
            type_names: Vec::new(),
            graph: FlatMap::new()
        }
    }
}

struct TypeHeap {
    names: Vec<String>
}

impl TypeHeap {
    fn insert(&mut self, name: String) -> TypeId {
        let mut iter = self.names.iter();
        match iter.position(|&v| v == name) {
            Some(id) => id,
            None => {
                self.names.push(name);
                self.names.len() - 1
            }
        }
    }

    pub fn dump(&self) {
        for name in self.names {
            println!("Type {}", name)
        }
    }

    fn new() -> Self {
        Self {
            names: Vec::new()
        }
    }
}

pub struct TypesCollectionPass<'ast> {
    pub type_heap: TypeHeap,
    types: FlatMap<TypeId, &'ast syn::Type>, // A string name? Really?
    locations: FlatMap<TypeId, String>,
    context: Vec<String>
}

impl<'ast> TypesCollectionPass<'ast> {
    pub fn new() -> Self {
        Self {
            type_heap: TypeHeap::new(),
            types: FlatMap::new(),
            locations: FlatMap::new(),
            context: Vec::new()
        }
    }
}

impl<'a: 'b, 'b> Visit<'a> for TypesCollectionPass<'b> {
    fn visit_item_struct(&mut self, node: &syn::ItemStruct) {
        self.context.push(quote::quote! {#node.ident}.to_string());
        visit::visit_item_struct(self, node);
        self.context.pop();
    }

    fn visit_field(&mut self, node: &syn::Field) {
        match node.ident {
            Some(id) => self.context.push(quote::quote! {id}.to_string()),
            None => self.context.push("<unnamed field>".to_string())
        }

        visit::visit_field(self, node);
        self.context.pop();
    }

    fn visit_type(&mut self, node: &'a syn::Type) {
        let id = self.type_heap.insert(quote::quote! {#node}.to_string());
        self.types.insert(id, &node);
    }
}

// A state machine
// Essentially visits top-level type and inserts constraints
// Another way to think of it is as a parser that operates on nodes,
// not tokens
// In which case the AST / Visitor is basically useless
// The fundamental issue here is that I have no good way of communicating
// result information

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

impl<'ast> visit::Visit<'ast> for ProofGraphVisitor {
    fn visit_path_segment(&mut self, node: &syn::PathSegment) {
        // Intentional cutoff here
        visit::visit_path_segment(self, node)
    }

    fn visit_angle_bracketed_generic_arguments(&mut self, node: &syn::AngleBracketedGenericArguments) {
        println!("Generic types are unsupported (can't track their dependencies yet) ({})", quote::quote! {#node})
    }

    fn visit_path(&mut self, node: &syn::Path) {
        if node.segments.len() > 1 {
            println!("All IDL types must be in global scope ({})", quote::quote! {#node});
            return
        }

        visit::visit_path(self, node)
    }
}
