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

// NOTE: Tian: you can always quote! these

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
