use syn::*;

// NOTE: A lot of information is simply left in the original AST
// NOTE:

pub struct Module<'ast> {
    pub name: String, // TODO: does Ident to string heap optimization?
    pub submodules: Vec<Module<'ast>>, // Will be extended as ModuleDef nodes are processed
    pub items: Vec<ModItem<'ast>>
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
