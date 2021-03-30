# New Generation Codegenerator(ngc)

A new tool to replace the [old redIDL code generation tool](../codegen-old/README.md).

# Architecture

* Single pass, single file
* Feature guard for different types of generation



# Problems that we've encountered
## Unable to compile interface and generate code without TypeIdentifiable.
* Interface dependes on rref. If rref defines TypeIdentifiable, `cargo expand` wouldn't work because
  interface depends on rref, rref depends on the not-yet-generated TypeIdentifiable, which makes
  interface failed to compile. If interface defines TypeIdentifiable, rref needs to depends on
  interface, which creates a circular dependency.
    * Solution 1: write your own `cargo expand`. This will disallow macros.
    * Solution 2: dummpy TypeIdentifiable. One extra step but should work.

## Where should be generated TypeIdentifiable live?
* Typeid can be generated with dummy TypeIdentifiable. Now, new problem: where do you put it? If we
  put TypeIdentifiable in `rref`, dependencies will not be found because the path are resolved 
  within `interface`. If we put it in `interface`, the import from `rref` will create a circular
  dependency. If we put it in a seperate crate, it needs to depend on `rref` and creates a circular
  dependency again. 
    * Solution 1:  Generate to rref. Ban renaming, ban usage of external types unless from rref.
      ngc should refactor `crate` to `interace` and `rref` to `crate`. This still doesn't work
      because it will create a circular dependency again.
    * Solution 2: everything one crate. Does `rref` belong to `interface` though?
  Decision: Put `rref` inside of `interface` and make everything one single crate. This is ugly but
  this might be the only solution we have.


# Problems with the old codegen

codegen-old was a disaster. We did [7-passes](https://github.com/mars-research/redleaf/blob/874b42c6a5f03c8b8484e2642ac35425b1acc518/interface/Makefile#L10)
to just get some sort of end-to-end proxy generation working.

One cause of this monstrosity is that the original design splits differnent kinds of generated
code (e.g., proxy, create, etc.) and the interface library to be imported by the domains 
to seperate crates. This forces the generated code to remove all non-interface definitions
and reolve the corresponding import path to the interface library.

Another cause is that codegen-old is a proc-macro based geneneration system.
Since proc-macros knows about the syntax
tree of a particular interface, It's relatively easy to proc-macros to generate most of the code
that we want so that's what we started with. However, serious problems arise as we try to have it
figure out the import paths. Since it does not know 


# Known issues
* Reference to types in structs are not supported. For example, `Self::T` or `Foo::T` is not supported.
