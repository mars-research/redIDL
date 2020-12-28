# IDL Code Generation

This document will describe how the redIDL Codegen is implemented.

# Terminologies

* RPC call: Cross-domain function call
* Proxy: The middleman during an RPC(cross-domain) call.
    It is a wrapper around the callee domain. When the caller domain do a RPC call, the call will
    goes into the proxy, the proxy will move `RRef` objects' ownerships from the caller domain to 
    the callee, invokes the corresponding function in the callee domain. When the function in the 
    callee domain returns, it will move the `RRef` objects back to the caller's domain, and returns
    the result back to the caller with an `Ok`. If the callee panics, the proxy will returns an
    `Err` instead.
* cargo-expand: Merge all files in the crate and try to expand all macros as much as possible.
    If the macro is not found, it will be kept as is.

# Proxy generation

The codegen generates a proxy for each interface trait(i.e. traits marks with `#[interface]`). 
The `#[interface]` attribute is implemented as an attribute procedure macro. 

## Step 1: module resolution

Since the procedure macro only know about the syntax tree but it knows nothing about the type,
we need to let it know what is the path of the trait should it try to implement a proxy for.
For example, for the interface trait `Rv6` defined in file `src/rv6.rs` in `usr` crate, the 
proc-macro needs to know that the path for the trait is `usr::rv6::Rv6` and generate the following
code

```rust
impl usr::rv6::Rv6 for Rv6Proxy {
    // some more code here ...
}
```

To achieve this, we will add an addition pass of cargo-expand prior to the actual code generation.

