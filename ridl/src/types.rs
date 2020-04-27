use std::fs;
use std::io::Write;

pub struct TypeSystemDecls {
    functionals: Vec<String>,
    rrefables: Vec<String>,
    safe_copyables: Vec<String>,
    check_functional: Vec<String>,
    check_rrefable: Vec<String>,
    check_safe_copy: Vec<String>
}

enum FieldStatus {
    IsOptRRef,
    IsNormal,
    IsInvalid
}

fn path_is_opt_rref(pt: &syn::Path) -> bool {
    pt.segments.len() == 1 && pt.segments[0].ident.to_string() == "OptRRef"
}

fn path_is_rref(pt: &syn::Path) -> bool {
    pt.segments.len() == 1 && pt.segments[0].ident.to_string() == "RRef"
}

impl TypeSystemDecls {
    pub fn new() -> Self {
        TypeSystemDecls {
            functionals: Vec::new(),
            rrefables: Vec::new(),
            safe_copyables: Vec::new(),
            check_functional: Vec::new(),
            check_rrefable: Vec::new(),
            check_safe_copy: Vec::new()
        }
    }

    fn process_reference(&mut self, rr: &syn::TypeReference) -> bool {
        let ty = &*rr.elem;
        if let syn::Type::TraitObject(tr) = ty {
            if let None = tr.dyn_token {
                println!("[ERROR] Trait references must be dynamic");
                return false
            }

            self.check_functional.push(quote::quote!{#tr}.to_string());

            return true
        }

        println!("[ERROR] Non-trait references are not allowed");

        false
    }

    fn process_struct_field(&mut self, field: &syn::Field) -> FieldStatus {
        // Reject unsafe stuff (TODO: fuzz this)
        match field.ty {
            syn::Type::Ptr(_) => {
                println!("[ERROR] IDL does not allow pointers");
                return FieldStatus::IsInvalid
            },
            syn::Type::BareFn(_) => {
                println!("[ERROR] IDL does not allow function pointers");
                return FieldStatus::IsInvalid
            },
            _ => ()
        }

        // generate checks for field types

        let is_valid = match &field.ty {
            syn::Type::Reference(rr) => self.process_reference(rr),
            syn::Type::Path(p) => {
                let path = &p.path;

                if path_is_opt_rref(path) {
                    if let syn::PathArguments::AngleBracketed(args) = &path.segments[0].arguments {
                        let gen_arg = &args.args[0];
                        self.check_rrefable.push(quote::quote!{#gen_arg}.to_string());                        
                        return FieldStatus::IsOptRRef;
                    }
                    else {
                        return FieldStatus::IsInvalid;
                    }
                }

                self.check_safe_copy.push(quote::quote!{#path}.to_string());

                true
            }
            _ => {
                println!("[ERROR] Unsupported field type");
                false
            }
        };

        if !is_valid {
            return FieldStatus::IsInvalid
        }

        return FieldStatus::IsNormal;
    }

    fn process_arg_type(&mut self, ty: &syn::Type) -> bool {
        match &ty {
            syn::Type::Reference(rr) => self.process_reference(rr),
            syn::Type::Path(p) => {
                let path = &p.path;

                if path_is_rref(path) {
                    if let syn::PathArguments::AngleBracketed(args) = &path.segments[0].arguments {
                        let gen_arg = &args.args[0];
                        self.check_rrefable.push(quote::quote!{#gen_arg}.to_string());
                        return true
                    }
                    else {
                        return false
                    }
                }

                self.check_safe_copy.push(quote::quote!{#path}.to_string());

                true
            }
            _ => {
                println!("[ERROR] Invalid argument type");
                false
            }
        }
    }

    pub fn process_signature(&mut self, sig: &syn::Signature) -> bool {
        if let syn::ReturnType::Type(_, ty) = &sig.output {
            if !self.process_arg_type(ty) {
                return false
            }
        }

        for arg in &sig.inputs {
            if let syn::FnArg::Typed(pat) = arg {
                if !self.process_arg_type(&pat.ty) {
                    return false;
                }
            }
        }

        true
    }

    pub fn process_type(&mut self, item: &syn::Item) -> bool {
        match item {
            // Is either rrefable or safe-copy
            // safe-copy is a subset of rrefable
            syn::Item::Struct(st) => {
                let mut is_sc = true;

                for field in &st.fields {
                    let field_status = self.process_struct_field(field);
                    if let FieldStatus::IsInvalid = field_status {
                        return false;
                    }
                    if let FieldStatus::IsOptRRef = field_status {
                        is_sc = false;
                    }
                }

                if is_sc {
                    self.safe_copyables.push(st.ident.to_string());
                }
                else {
                    self.rrefables.push(st.ident.to_string());
                }

                true
            },
            // Can only be functional
            syn::Item::Trait(tr) => {
                for tr_item in &tr.items {
                    if let syn::TraitItem::Method(_) = tr_item {}
                    else {
                        println!("[ERROR] IDL traits may only have methods");
                        return false
                    }
                }

                self.functionals.push(tr.ident.to_string());

                true
            },
            _ => true
        }
    }

    pub fn write_decls(&mut self, file: &mut fs::File) {
        for f in &self.functionals {
            writeln!(file, "red_idl::declare_functional!({});", f).expect("[ERROR] Could not write to generated file");
        }

        for r in &self.rrefables {
            writeln!(file, "red_idl::declare_rrefable!({});", r).expect("[ERROR] Could not write to generated file");
        }

        for sc in &self.safe_copyables {
            // Nice way of making sure that things are only declared SafeCopy if they're also Copy
            writeln!(file, "red_idl::require_copy!({});", sc).expect("[ERROR] Could not write to generated file");
            writeln!(file, "red_idl::declare_safe_copy!({});", sc).expect("[ERROR] Could not write to generated file");
        }

        for f in &self.check_functional {
            writeln!(file, "red_idl::require_functional!({});", f).expect("[ERROR] Could not write to generated file");
        }

        for f in &self.check_rrefable {
            writeln!(file, "red_idl::require_rrefable!({});", f).expect("[ERROR] Could not write to generated file");
        }

        for f in &self.check_safe_copy {
            writeln!(file, "red_idl::require_safe_copy!({});", f).expect("[ERROR] Could not write to generated file");
        }
    }
}