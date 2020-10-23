use syn::*;

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
