# Type resolution
The job of the type resolution is gathering type information the in the IDL files for the codegen.
There are two objectives that we want to achieve in type resolution.
1.  The ability to get all unique types that are being `RRef`ed, i.e. all `T` in `RRef<T>`,
    `RRefVec`, and `RRefDeque<T>`. This information will later be used to generate unique type_id
    for every `RRef`ed type.
1.  The ability to check if a type is what it actually looks like. For example, when there's 
    `RRef<u8>` in the code, we would like to know whether it is the remote reference type for
    inter-domain communication in Redleaf or it's some other types with the same name that the user
    added. This information is needed when generating the proxy because the proxy is responsible of
    managing the ownerships of all `RRef`ed types.


# Architecture
To figure out all unique `RRef`ed types, we put the fully-qualified paths of all `RRef`ed types
into a hashset, this gives us all the unique ones. This leaves us to figure out how to get the
fully-qualified path of a type. Let say we have an IDL file like this.

```rust
mod crate {
    pub use extern_lib::Foo;
    mod inner_a {
        pub struct Foo {}
        pub use crate::Foo as Bar;

        type _ = RRef<(Foo, Bar)>;
    }
    mod inner_b {
        use super::inner_a::Foo;
        use crate::inner_a as renamed_a;
        use extern_lib::Baz;

        type _ = RRef<(Foo, renamed_a::Bar, Baz)>;
    }
}
```

First, we do one pass over the file to construct a module tree. Each node in the tree represents a
module. Each node contains a hashmap that maps each the symbol in the module to its relative path.
We mark the symbol as terminal if it is the definition of the symbol itself or if it's from an
external crate. Terminal symbols need no further resolution. The reason that we don't further
resolve symbols from external crates is because resolving them requiring us pulling all the external
dependencies and parse them. This will greatly increase the complexity of this project. 

```
// Module tree
crate -> inner_a
    ↳ inner_b 

// Module `crate` mapping
{
    <pub Foo, extern_lib::Foo, terminal>,
}

// Module `inner_a` mapping
{
    <pub Foo, Foo, terminal>,
    <pub Bar, crate::Foo>,
}

// Module `inner_b` mapping
{
    <Foo, super::inner_a::Foo>,
    <reanmed_a, crate::inner_a>,
    <Baz, extern_lib::Baz>
}
```

Then, we use all information in the moduel and try connecting all pieces together until all the
symbols have a terminal mapping. For example, `Foo` in `crate::inner_b` will get resolved from
`crate::inner_a::Foo`.

