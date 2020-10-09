# IDL Design Docs

## Cross-domain calls

These are the important ones. Also termed RPCs; these are the members of domain traits.
Their arguments obey a simple marshaling scheme:
- `RRef<>`-like types, which may be passed in two ways
	- By being moved, in which case their owner must be switched to the callee domain
	- By being immutably borrowed, in which case their refcount must be incremented
- References (mutable or immutable) to domain traits, which must be replaced by a designated
proxy implementation
- `Copy` types, which may be simply bitwise-copied (passed directly)

## Notion of `SafeCopy`

Some `Copy` types may still be invariant-breaking, notably: references, which may refer to private heap data;
pointers, which have the same issue; and function pointers, which require their own proxying system to be made safe.
Therefore, when we speak of `Copy` types, we refer to the restricted subset of said types that exclude the
aforementioned cases.

## Composites

Composite types, to avoid having to deal with type structures, must be `Copy` if they are to be passed
directly.

## RRef

RRefs, which refer to data allocated on the shared heap, have different rules. The data itself needs no marshaling, but
must still respect these invariants. For the same reasons as above, a type so referred to must usually be `Copy`.
An exception is made, however: `RRef<>`-ed named composites (more precisiely, composites which allow `impl` blocks
and access specifiers) are permitted to contain both immutable references to `RRef<>`s and `Optional<RRef<>>`s.
The latter construction is necessary to express how an `RRef<>`---which is owning---may be moved. This construction
is subject to change, but the semantics remain identical. To enforce the manner in which these are moved, the field
is kept private, and getters and setters are generated to automate the taking and relinquishment of direct ownership
over the `RRef<>`. The borrow-count mechanism is automatically enforced. We refer to these chained, shared-heap
structures as `RRef<>`-trees.

`RRef<>`-like types, such as `RRefArray<>` or `RRefDequeue<>`, implement their borrow-counting automatically,
and possess the same `move_to()` mechanism.

# Practical Matters

- The `syn` crate has a major flaw, thanks to `proc_macro`: we can't really get `Span` information outside of a macro,
so we have to manually piece together context-based diagnostics.

- The type's syntactic structure does somewhat mirror its semantic layout, but there is unecessary information and
unneeded subtrees. These need to be progressively folded / pruned, which necessitates the laborious task of writing
a custom tree structure to support this.
