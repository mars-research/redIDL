# redIDL

Redleaf IPC IDL compiler.

# Syntax
Same as rust. But for cross-domain interface traits, we mark then with `#[interface]`. And we 
mark domain create traits with `#[domain_creation]`. See __test.ridl__ in __../data/__ for example. 

# Constrains

* All modules must be public.
* Identifiers starts with `RRef` will be reserved.
* No super trait allow.
