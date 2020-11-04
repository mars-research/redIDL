use syn::{*, visit::*};

// NOTE: A lot of information is simply left in the original AST
// NOTE:

pub struct Module<'ast> {
    pub name: String,                  // TODO: does Ident to string heap optimization?
    pub submodules: Vec<Module<'ast>>, // Will be extended as ModuleDef nodes are processed
    pub items: Vec<ModItem<'ast>>,
}

// NOTE: We take control of access specifiers, explicit ones are not permitted

pub struct DomainTrait<'ast> {
    pub name: String, // TODO: Replace with interned strings
    pub raw: &'ast ItemTrait,
    pub methods: Vec<DomainRpc<'ast>>,
}

pub struct DomainRpc<'ast> {
    pub raw: &'ast TraitItemMethod,
    pub raw_types: Vec<&'ast Type>, // replace with IDL defs
    // NOTE: Tian, this'll probably move around a bit, but it's always a child node of DomainRpc
    pub lowered_types: Vec<LoweredType<'ast>>,
}

pub struct StructDef<'ast> {
    pub name: String,
    pub raw: &'ast ItemStruct,
    pub raw_types: Vec<&'ast Type>, // replace with IDL reps
    // Used to match uses of generic idents "within", paths get generic args, resolved later
    pub generic_names: Vec<String>,
}

pub enum ModItem<'ast> {
    DomainTrait(Box<DomainTrait<'ast>>),
    StructDef(Box<StructDef<'ast>>),
}

trait IRVisit<'ir, 'ast> {
    fn visit_module(&mut self, node: &'ir Module<'ast>);
    fn visit_mod_item(&mut self, node: &'ir ModItem<'ast>);
    fn visit_domain_trait(&mut self, node: &'ir DomainTrait<'ast>);
    fn visit_domain_rpc(&mut self, node: &'ir DomainRpc<'ast>);
    fn visit_struct_def(&mut self, node: &'ir StructDef<'ast>);
}

// Type structures

enum TypeStructure<'ast, 'ir> {
    Tuple(Box<Tuple<'ast, 'ir>>),
    Array(Box<Array<'ast, 'ir>>),
    NamedType(NamedType<'ast, 'ir>),
}

struct Tuple<'ast, 'ir> {
    pub raw: &'ast TypeTuple,
    pub contents: Vec<TypeStructure<'ast, 'ir>>,
}

struct Array<'ast, 'ir> {
    pub raw: &'ast TypeArray,
    pub contents: Vec<TypeStructure<'ast, 'ir>>,
}

// TODO: resolving these is the fun part
enum NamedType<'ast, 'ir> {
    Raw(&'ast syn::Path),
    Def(&'ir ModItem<'ast>),
    Prim(()), // something goes here, probably a type ID
}

// NOTE: Tian: you can always quote!{} these in generation

pub enum LoweredType<'ast> {
    RRefLike(Box<RRefLike<'ast>>),
    RefImmutRRefLike(Box<RefImmutRRefLike<'ast>>),
    Bitwise(Box<Bitwise<'ast>>),
    DomainTraitRef(Box<DomainTraitRef<'ast>>),
}

pub struct RRefLike<'ast> {
    pub raw: &'ast Type,
}

pub struct RefImmutRRefLike<'ast> {
    pub raw: &'ast Type,
}

pub struct Bitwise<'ast> {
    pub raw: &'ast Type,
}

pub struct DomainTraitRef<'ast> {
    pub raw: &'ast Type,
}

// So how is the IR AST actually built?
// We run into borrow-checking issues
// Could probably just box this stuff
// But a vector of boxes is just nasty
// We need absolute references
// or a reference that is known to live long enough
// Let's box it by default

struct DomainRpcTypeCollection<'ast> {
    types: Vec<&'ast syn::Type>,
}

struct DomainRpcCollection<'ast> {
    rpcs: Vec<DomainRpc<'ast>>,
}

struct ModItemCollection<'ast> {
    items: Vec<ModItem<'ast>>,
}

struct StructDefCollection<'ast> {
    generics: Vec<String>,
    types: Vec<&'ast Type>,
}

// We have no need to iterate deeper in any of these

impl<'ast> Visit<'ast> for DomainRpcTypeCollection<'ast> {
    fn visit_type(&mut self, node: &'ast Type) {
        self.types.push(node)
    }
}

impl<'ast> Visit<'ast> for DomainRpcCollection<'ast> {
    fn visit_trait_item_method(&mut self, node: &'ast TraitItemMethod) {
        self.rpcs.push(DomainRpc {
            raw: node,
            raw_types: collect_domain_rpc_types(node),
            lowered_types: Vec::new()
        })
    }
}

impl<'ast> Visit<'ast> for StructDefCollection<'ast> {
    fn visit_type_param(&mut self, node: &'ast TypeParam) {
        self.generics.push(node.ident.to_string())
    }

    fn visit_type(&mut self, node: &'ast Type) {
        self.types.push(node)
    }
}

impl<'ast> Visit<'ast> for ModItemCollection<'ast> {
    fn visit_item_trait(&mut self, node: &'ast ItemTrait) {
        let ir_node = DomainTrait {
            raw: node,
            name: node.ident.to_string(),
            methods: collect_domain_rpcs(node),
        };

        self.items.push(ModItem::DomainTrait(Box::new(ir_node)))
    }

    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        let (gens, types) = collect_struct_def(node);
        let ir_node = StructDef {
            name: node.ident.to_string(),
            raw: node,
            generic_names: gens,
            raw_types: types,
        };

        self.items.push(ModItem::StructDef(Box::new(ir_node)))
    }
}

fn collect_domain_rpc_types<'ast>(node: &'ast TraitItemMethod) -> Vec<&'ast Type> {
    let mut visit = DomainRpcTypeCollection { types: Vec::new() };
    visit.visit_trait_item_method(node);
    visit.types
}

fn collect_domain_rpcs<'ast>(node: &'ast ItemTrait) -> Vec<DomainRpc<'ast>> {
    let mut visit = DomainRpcCollection { rpcs: Vec::new() };
    visit.visit_item_trait(node);
    visit.rpcs
}

fn collect_mod_items<'ast>(node: &'ast File) -> Vec<ModItem<'ast>> {
    let mut visit = ModItemCollection { items: Vec::new() };
    visit.visit_file(node);
    visit.items
}

fn collect_struct_def<'ast>(node: &'ast ItemStruct) -> (Vec<String>, Vec<&'ast Type>) {
    let mut visit = StructDefCollection {
        generics: Vec::new(),
        types: Vec::new(),
    };
    visit.visit_item_struct(node);
    (visit.generics, visit.types)
}

pub fn produce_module<'ast>(name: &str, ast: &'ast File) -> Module<'ast> {
    Module {
        name: name.to_string(),
        submodules: Vec::new(),
        items: collect_mod_items(ast),
    }
}
