use syn::visit;
use visit::Visit;
use crate::utility::FlatMap;

type TypeId = usize;

struct TypeHeap {
    names: Vec<String>
}

impl TypeHeap {
    fn insert(&mut self, name: String) -> TypeId {
        let mut iter = self.names.iter();
        match iter.position(|v| v == &name) {
            Some(id) => id,
            None => {
                self.names.push(name);
                self.names.len() - 1
            }
        }
    }

    fn new() -> Self {
        Self {
            names: Vec::new()
        }
    }
}

pub struct TypesCollectionPass<'ast> {
    should_index: bool,
    type_heap: TypeHeap,
    types: FlatMap<TypeId, &'ast syn::Type>,
    locations: FlatMap<TypeId, Vec<String>>,
    context: Vec<String>
}

impl<'ast> TypesCollectionPass<'ast> {
    pub fn new() -> Self {
        Self {
            should_index: false,
            type_heap: TypeHeap::new(),
            types: FlatMap::new(),
            locations: FlatMap::new(),
            context: Vec::new()
        }
    }

    pub fn dump(&self) {
        for id in 0..self.type_heap.names.len() {
            println!("Type \"{}\"", self.type_heap.names[id]);
            for loc in self.locations.get(id).expect("location table did not exist") {
                println!("\tAt {}", loc)
            }
        }
    }
}

impl<'ast> Visit<'ast> for TypesCollectionPass<'ast> {
    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        let id = &node.ident;
        self.context.push(quote! {#id}.to_string());
        visit::visit_item_struct(self, node);
        self.context.pop();
    }

    fn visit_field(&mut self, node: &'ast syn::Field) {
        self.should_index = true;        
        match &node.ident {
            Some(id) => self.context.push(quote! {#id}.to_string()),
            None => self.context.push("<unnamed field>".to_string())
        }

        visit::visit_field(self, node);
        self.context.pop();
        self.should_index = false;
    }

    fn visit_type(&mut self, node: &'ast syn::Type) {
        if !self.should_index {
            return
        }

        let id = self.type_heap.insert(quote! {#node}.to_string());
        let mut loc = String::new();
        for scope in &self.context {
            loc = format!("{}::{}", loc, scope);
        }

        if self.types.insert(id, node) {
            self.locations.insert(id, vec![loc]);
        }
        else {
            self.locations.get_mut(id).expect("type use table did not exist").push(loc);
        }
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
