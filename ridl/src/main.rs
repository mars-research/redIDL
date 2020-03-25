extern crate syn;

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

// panic!() may not be the best of ideas

// NOTE: Rust uses the leading "::" for global qualification

fn get_ast(path: &Path) -> syn::File {
    let mut file = match File::open(path) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            panic!()
        }
    };
    let mut content = String::new();
    file.read_to_string(&mut content).expect(&("[ERROR] Failed to open file: ".to_string() + path.to_str().expect("[ERROR] Failed to convert filename string")));
    return match syn::parse_file(&content) {
        Err(e) => {
            println!("[ERROR] Failed to parse because: {}", e);
            panic!()
        },
        Ok(v) => v
    }
}

// struct IDLFile {
//     ast: syn::File,
//     deps: Vec<usize>
// }

// fn find_deps(path: &Path, idl_store: &mut Vec<IDLFile>, seen: &mut Vec<PathBuf>) -> usize {
//     match seen.iter().position(|x| x == path) {
//         Some(id) => {
//             println!("\tAlready saw IDL file at {:?}", seen[id]);
//             return id
//         },
//         None => ()
//     };

//     let mut p = PathBuf::new();
//     p.push(path);
//     idl_store.push(IDLFile {ast: get_ast(path), deps: Vec::new()});
//     seen.push(p);
//     let id = idl_store.len() - 1;
    
//     let mut imports : Vec<String> = Vec::new();
//     {
//         let idl = &idl_store[id];
//         let ast = &idl.ast;
//         let mut useds : Vec<syn::ItemUse> = Vec::new();
//         for item in &ast.items {
//             match item {
//                 syn::Item::Use(used) => {
//                     println!("[INFO] Encountered use statement");
//                     useds.push(used.clone())
//                 },
//                 syn::Item::Trait(_) => (),
//                 syn::Item::Struct(_) => (),
//                 _ => {
//                     println!("[ERROR] IDL may only contain traits, structs, and use statements");
//                     panic!()
//                 }
//             }
//         }

//         for used in useds {
//             match used.tree {
//                 syn::UseTree::Name(name) => {
//                     println!("[INFO] Used {}", name.ident);
//                     imports.push(name.ident.to_string())
//                 },
//                 _ => {
//                     println!("[ERROR] Only \"use <name>;\" is supported for use statements");
//                     panic!()
//                 }
//             }
//         }
//     }    

//     let mut deps : Vec<usize> = Vec::new();
//     if imports.len() > 0 {
//         println!(
//             "[INFO] Collecting ASTs for all imports of: {}",
//             path.to_str().expect("[ERROR] Could not convert path to string"));
        
//         for imp in imports {
//             let mut buf = PathBuf::new();
//             buf.push(path.parent().unwrap_or(Path::new("")));
//             buf.push(imp + ".idl");
//             println!("\t{}", buf.to_str().expect("Could not convert from path to str"));
//             deps.push(find_deps(buf.as_path(), idl_store, seen))
//         }
//     }

//     idl_store[id].deps = deps;

//     return idl_store.len() - 1;
// }

// fn build_sym_table(tree: usize, idls_store: &Vec<IDLFile>, table: &mut Vec<String>) {
//     let idl = &idls_store[tree];
//     for dep in &idl.deps {
//         build_sym_table(*dep, idls_store, table);
//     }
//     for item in &idl.ast.items {
//         match item {
//             syn::Item::Trait(tr) => table.push(tr.ident.to_string()),
//             syn::Item::Struct(st) => table.push(st.ident.to_string()),
//             syn::Item::Use(_) => (),
//             _ => panic!()
//         }
//     }
// }

enum Condition {
    IsTrait,
    IsCopy,
    IRRefable, // Probably superseding IsCopy
    None
}

struct Require {
    id: usize,
    needs: Condition
}

struct TypeCheck {
    id: usize,
    req_start: usize,
    reqs: usize
}

struct TypeProps {
    id: usize,
    satisfies: Condition
}

impl Resolver {
    fn new() -> Resolver {
        Resolver {
            checks: Vec::new(),
            known: Vec::new(),
            requires: Vec::new(),
            ids: Vec::new()
        }
    }

    fn resolve(&mut self, path: &Path) {
        let ast = get_ast(path);
        for item in &ast.items {
            let mut check = TypeCheck {id: 0, req_start: self.requires.len(), reqs: 0};
            check.id = match item {
                syn::Item::Trait(tr) => self.process_trait(tr),
                _ => {
                    println!("[ERROR:IMPL] Not allowed");
                    panic!();
                }
            };
            check.reqs = self.requires.len() - check.req_start;
            self.checks.push(check);
        }
        for check in &self.checks {
            let ty = check.id;
            println!("{}", self.ids[ty]);
            for i in check.req_start..check.req_start + check.reqs {
                let id = self.requires[i].id;
                println!("\t\t{}", self.ids[id])
            }
        }
    }

    fn process_trait(&mut self, tr: &syn::ItemTrait) -> usize {
        let tr_id = self.get_id(&tr.ident.to_string());
        self.known.push(TypeProps {id: tr_id, satisfies: Condition::IsTrait});
        for item in &tr.items {
            match item {
                syn::TraitItem::Method(m) => self.process_method(m),
                _ => {
                    println!("[ERROR] IDL traits may only have methods");
                    panic!()
                }
            }
        }
        tr_id
    }

    fn process_method(&mut self, m: &syn::TraitItemMethod) {
        for arg in &m.sig.inputs {
            match arg {
                syn::FnArg::Typed(ty) => self.add_require(&ty.ty),
                syn::FnArg::Receiver(_) => ()
            }
        }
        match &m.sig.output {
            syn::ReturnType::Default => (),
            syn::ReturnType::Type(_, ty) => self.add_require(&ty)
        }
    }

    fn add_require(&mut self, ty: &syn::Type) {
        match ty {
            syn::Type::Reference(r) => self.add_trait_ref(&r), // Since at this point it must be a trait
            syn::Type::Path(p) => self.add_path(&p),
            _ => println!("[WARNING] Type checks only added for traits so far")
        }
    }

    fn add_trait_ref(&mut self, r: &syn::TypeReference) {
        let ty : &syn::Type = &r.elem;
        match ty {
            syn::Type::TraitObject(tr) => {
                if tr.dyn_token.is_none() {
                    println!("[ERROR] Only dynamic traits allowed");
                    panic!()
                }
                if tr.bounds.len() != 1 {
                    println!("[ERROR] Cannot bind multiple traits");
                    panic!()
                }
                let name = match &tr.bounds[0] {
                    syn::TypeParamBound::Trait(b) => {
                        if b.path.segments.len() != 1 {
                            println!("[ERROR:IMPL] Paths not supported at this time");
                            panic!()
                        }
                        &b.path.segments[0]
                    },
                    _ => {
                        println!("[ERROR] Must bind trait, not lifetime");
                        panic!()
                    }
                };
                let id = self.get_id(&name.ident.to_string());
                self.requires.push(Require {id: id, needs: Condition::IsTrait})
            },
            _ => {
                println!("[ERROR:IMPL] Now that's just wrong. . .")
            }
        }
    }

    fn get_id(&mut self, id: &String) -> usize {
        let index = self.ids.iter().position(|v| v == id);
        match index {
            Some(v) => v,
            None => {
                self.ids.push(id.clone());
                self.ids.len() - 1
            }
        }
    }

    fn add_path(&mut self, p: &syn::TypePath) {
        // The whole expression must be copy

    }
}

struct Resolver {
    checks: Vec<TypeCheck>,
    known: Vec<TypeProps>,
    requires: Vec<Require>,
    ids: Vec<String>
}

/* 
    We're doing this wrong. Summary of the rules:
        - You can pass and return &dyn FooTrait, provided the trait only has functions
        - You can pass and return any type that is Copy (must check that its innards are, in fact, Copy)
        - You can pass and return RRefs to everything else, but those types must use OptRRefs inside, and not RRefs.
    
    Internally, types should only be referred to by globally-qualified names, i.e.: "::module::Type"
    These can obviously only be declared once the actual type definition is seen

    Possibly generate constraints in terms of macros
*/

// Does the given trait consist purely of functions?
fn is_functional(tr: &syn::ItemTrait) -> bool {
    for item in &tr.items {
        match item {
            syn::TraitItem::Method(_) => continue,
            _ => return false
        }
    }
    true
}

// Only checks if its declared as copy, if we just copy the struct verbatim to the crates, compilation will fail if it actually isn't
fn is_copy() -> bool {false}

// Really just return false if we find that it has a plain RRef in it
fn is_rrefable() -> bool {false}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: <invocation> <filepath>");
        return ()
    }
    let path = Path::new(&args[1]);
    let mut resolver = Resolver::new();
    resolver.resolve(path);
    // let mut table : Vec<String> = Vec::new();
    // let mut idl_store : Vec<IDLFile> = Vec::new();
    // let mut seen : Vec<PathBuf> = Vec::new();
    // let tree = find_deps(Path::new(&args[1]), &mut idl_store, &mut seen);
    //build_sym_table(tree, &idl_store, &mut table);
    //println!("Types found in tree: {:?}", table)
}