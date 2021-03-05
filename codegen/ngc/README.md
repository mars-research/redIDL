# New Generation Codegenerator(ngc)

A new tool to replace the [old redIDL code generation tool](../codegen-old/README.md).

# Architecture

* Single pass, single file
* Feature guard for different types of generation



# Problems that we've encountered
* Interface dependes on rref. If rref defines TypeIdentifiable, `cargo expand` wouldn't work because
  interface depends on rref, rref depends on the not-yet-generated TypeIdentifiable, which makes
  interface failed to compile. If interface defines TypeIdentifiable, rref needs to depends on
  interface, which creates a circular dependency.

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

