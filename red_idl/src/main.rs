extern crate syn;

use std::fs;
use std::env;

use syn::visit;
use visit::Visit;

/*
    Major issues with syn:
    - No way to abort an AST walk
    - Source location information not usable from outside a procedural macro
*/

// Things we reject: bare function types, pointers, etc.
struct PruningVisitor {
    context: Vec<String>
}

impl PruningVisitor {
    fn get_context(&self) -> String {
        if self.context.len() == 0 {
            "<global scope>".to_string()
        }
        else {
            let mut msg = format!("{}", self.context[0]);
            for extra in &self.context[1..] {
                msg = format!("{}, {}", msg, extra)
            }

            msg
        }
    }

    fn new() -> Self {
        Self {
            context: Vec::new()
        }
    }
}

// The approach to error handling is effectively just listing the chain of enclosing scopes
// Right now, that's traits, structs, methods, and fields
impl<'ast> Visit<'ast> for PruningVisitor {
    fn visit_type_bare_fn(&mut self, i: &'ast syn::TypeBareFn) {
        println!("\x1b[31merror:\x1b[0m Bare function types are not permitted");
        println!("at: {}", self.get_context());
        visit::visit_type_bare_fn(self, i);
    }

    fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
        self.context.push(format!("struct {}", i.ident.to_string()));
        visit::visit_item_struct(self, i);
        self.context.pop();
    }

    fn visit_field(&mut self, i: &'ast syn::Field) {
        match &i.ident {
            Some(id) => self.context.push(format!("field {}", id.to_string())),
            None => self.context.push("<unnamed field>".to_string())
        }

        visit::visit_field(self, i);
        self.context.pop();
    }

    fn visit_item_trait(&mut self, i: &'ast syn::ItemTrait) {
        self.context.push(format!("trait {}", i.ident.to_string()));
        visit::visit_item_trait(self, i);
        self.context.pop();
    }

    fn visit_signature(&mut self, i: &'ast syn::Signature) {
        self.context.push(format!("fn {}", i.ident.to_string()));
        visit::visit_signature(self, i);
        self.context.pop();
    }

    fn visit_expr_closure(&mut self, i: &'ast syn::ExprClosure) {
        println!("\x1b[31merror:\x1b[0m Closures are not permitted");
        println!("at: {}", self.get_context());
        visit::visit_expr_closure(self, i);
    }

    fn visit_item_const(&mut self, i: &'ast syn::ItemConst) {
        self.context.push(format!("const {}", i.ident.to_string()));
        visit::visit_item_const(self, i);
        self.context.pop();
    }
}

fn main() {
    let args : Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage (unstable interface): red_idl <test-path>");
        return
    }

    let content = match fs::read_to_string(&args[1]) {
        Ok(text) => text,
        Err(error) => {
            println!("Couldn't open file: {}", error);
            return
        }
    };

    let ast = match syn::parse_file(&content) {
        Ok(ast) => ast,
        Err(error) => {
            println!("Couldn't parse file: {}", error);
            return
        }
    };

    let mut rejector = PruningVisitor::new();
    rejector.visit_file(&ast);
}
