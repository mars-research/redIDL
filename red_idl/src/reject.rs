use syn::*;
use visit::Visit;

pub struct RejectPass {
    pub is_legal: bool
}

impl<'ast> Visit<'ast> for RejectPass {
    fn visit_type_bare_fn(&mut self, node: &'ast TypeBareFn) {
        println!("IDL does not allow bare function types");
        visit::visit_type_bare_fn(self, node);
        self.is_legal = false;
    }

    fn visit_type_ptr(&mut self, node: &'ast TypePtr) {
        println!("IDL does not allow pointer types");
        visit::visit_type_ptr(self, node);
        self.is_legal = false;
    }

    fn visit_type_reference(&mut self, node: &'ast TypeReference) {
        println!("IDL does not allow ref types");
        visit::visit_type_reference(self, node);
        self.is_legal = false;
    }
}

pub fn _reject_types(type_tree: &Type) -> bool {
    let mut rejector = RejectPass {is_legal: true};
    rejector.visit_type(type_tree);
    rejector.is_legal
}
