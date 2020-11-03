use syn::{*, visit::*};

const RESERVED_WORDS: [&str; 4] = ["RRef", "RRefArray", "RRefDequeue", "Option"];

fn is_reserved(ident: &Ident) -> bool {
    RESERVED_WORDS.iter().any(|&i| i == ident.to_string())
}

struct SanityVisitor {
    is_sane: bool
}

// Brute-force check of all possible places where a new type could be defined
impl<'ast> Visit<'ast> for SanityVisitor {
    fn visit_item_struct(&mut self, node: &ItemStruct) {
        if is_reserved(&node.ident) {
            println!("Cannot define struct as {}", node.ident);
            self.is_sane = false
        }
        
        visit_item_struct(self, node);
    }
    
    fn visit_item_trait(&mut self, node: &ItemTrait) {
        if is_reserved(&node.ident) {
            println!("Cannot define trait as {}", node.ident);
            self.is_sane = false
        }
        
        visit_item_trait(self, node);
    }
    
    fn visit_item_type(&mut self, node: &ItemType) {
        if is_reserved(&node.ident) {
            println!("Cannot define type alias as {}", node.ident);
            self.is_sane = false
        }
        
        visit_item_type(self, node);
    }
    
    fn visit_trait_item_type(&mut self, node: &TraitItemType) {
        if is_reserved(&node.ident) {
            println!("Cannot define trait-associated type as {}", node.ident);
            self.is_sane = false
        }
        
        visit_trait_item_type(self, node);
    }
    
    fn visit_impl_item_type(&mut self, node: &ImplItemType) {
        if is_reserved(&node.ident) {
            println!("Cannot define impl-associated type as {}", node.ident);
            self.is_sane = false
        }
        
        visit_impl_item_type(self, node);
    }
    
    fn visit_foreign_item_type(&mut self, node: &ForeignItemType) {
        if is_reserved(&node.ident) {
            println!("Cannot define foreign type as {}", node.ident);
            self.is_sane = false
        }
        
        visit_foreign_item_type(self, node);
    }
    
    fn visit_type_param(&mut self, node: &TypeParam) {
        if is_reserved(&node.ident) {
            println!("Cannot type parameter as {}", node.ident);
            self.is_sane = false
        }

        visit_type_param(self, node);
    }
}

pub fn sanity_check_module(ast: &File) -> bool {
    let mut visit = SanityVisitor {is_sane: true};
    visit.visit_file(ast);
    visit.is_sane
}