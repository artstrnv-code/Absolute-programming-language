# Raw Memory Library Contract

Status: implemented. `include >> raw_memory` enables the `memory::` capability
surface in portable execution and native Windows/Linux builds.

## Table Identity

A numeric address is not an APL pointer or callable target by itself. Every
memory operation resolves its target through the raw-memory table. Each record
has a unique ID that is never reused, a current address, and at least:

- `origin`: `user`, `runtime`, or `load`;
- `state`: `live`, `marked`, `freed`, or `unloaded`;
- byte range and access permissions;
- ownership information.

`user` identifies memory allocated on request by APL code. `runtime` identifies
runtime-owned memory and does not by itself expose that memory to user code.
`load` identifies an externally owned address registered explicitly with
`memory::load(address, size, permissions, abi)`.

`memory::malloc(size)` creates a zeroed, live, owned `user` record. Its result
is `["APL_PTR_1", stable_id, address, "user"]`. The stable ID is canonical and
never reused. If an address is reused by several historical records, passing
that bare address is deliberately ambiguous; the descriptor or ID remains
unambiguous.

`memory::where(source, selector)` accepts a descriptor, ID, or exact registered
address. Selector 0 returns the address, 1 the stable ID, 2 the source variable
name (or `NONE` for a non-variable expression), and 3 the source value. The
name is preserved from source identity; it is never guessed by comparing equal
values.

`memory::read(pointer, offset, count)` returns `AVBytes` and
`memory::write(pointer, offset, bytes)` writes public bytes. Both require a live
entry, matching permissions, and a range wholly inside the registered extent.
Negative and overflowing ranges fail before memory is touched.

## External Addresses

`memory::load(address, size, permissions, abi)` creates a live, borrowed `load`
record for an address not allocated by APL. Registration makes table-mediated operations legal under the
APL memory contract, but cannot prove that the operating system mapped the
address or that the underlying device will accept an access.

`memory::mark(ptr)` immediately removes a record from the table of accessible targets.
All later reads, writes, and calls through that ID or address fail. For an
owned `user` record, `mark` changes its state to `marked` and moves it to the
pending-deletion table without releasing the allocation yet. For a borrowed
`load` record, `mark` changes its state to `unloaded`; it must never free,
clear, or overwrite the external address. Runtime records can be marked only
by their runtime owner.

Parameterless `memory::free()` releases every owned allocation currently present in
the pending-deletion table and changes those records from `marked` to `freed`.
It does not release live records and does not operate on unloaded external
addresses. IDs remain historical and are never reassigned after either path.
This separates revoking access from physical reclamation and provides the
base operation used by the scoped arena facilities below.

## Scoped Memory Attributes

The following attributes may appear at the beginning of a function, main, or
`while`. They apply to the dynamic execution of that scope, including allocations
made for it by called functions. A loop scope spans all iterations and closes
only when the loop itself finishes; `continue` does not close it.

`@marked` associates allocations requested during the scope with that scope.
When the function or loop finishes, every still-live owned allocation in the
set is passed to `mark`. This revokes access immediately and queues the memory
for later reclamation. The marking step also runs on structural exits such as
`return` and `break`.

An `@marked` scope is deliberately non-escaping. It may use temporary raw
memory for work such as an iteration buffer and copy or emit the computed
contents before the scope ends, but it may not return, store outside the
scope, or otherwise leak an ID, address, slice, or alias into that memory.
There is no promotion or escape operation. A statically visible escape is a
compile-time error; indirect cases that cannot be proven statically must be
rejected by the table/lifetime check at runtime rather than becoming a stale
usable pointer.

`@collector` performs a reachability pass when the scope finishes. Owned
allocations for which no live APL root or tracked pointer remains are passed to
`mark`; reachable allocations remain live. Borrowed `load` records and hidden
runtime records are not collectible merely because user code has no visible
reference to them.

`@flee` invokes parameterless `memory::free()` when the scope finishes, physically
releasing the pending-deletion table. The spelling is intentional: it denotes
memory "fleeing" the marked table rather than duplicating the `free()`
operation's name.

When attributes are combined, exit actions have a fixed semantic order rather
than depending on source order: `@marked`, then `@collector`, then `@flee`.
Nested attributed scopes keep distinct allocation sets so that closing an
inner arena cannot accidentally mark live allocations owned by an outer one.

## Intended Usage Profiles

The library exposes no implicit background collector. Programs select one of
four explicit memory-management profiles according to the job:

1. Internal temporary buffers are allocated inside `@marked` functions or
   loops. Their contents are consumed or copied before the scope closes, then
   every allocation in the arena is marked deterministically.
2. A function executed at a logical pause, such as the boundary between
   processing rounds, may use `@collector`. Pairing it with `@flee` marks
   unreachable owned records and then physically reclaims the pending table;
   either attribute may also be used alone when the phase requires it.
3. Code requiring exact lifetime control calls `memory::mark(ptr)` and
   parameterless `memory::free()` directly. Revocation and physical reclamation remain two separate,
   visible operations.
4. Systems code registers externally owned addresses with `memory::load(...)`.
   These records are borrowed and can be unloaded but never freed by APL.

This is the meaning of "honest" memory management in APL: reclamation occurs
only at an explicit operation or annotated boundary visible in source code.

## Indirect Calls

`memory::call(pointer, arguments)` accepts only a live target registered in the table. Passing an
unregistered numeric address, a stale ID, or a `freed`/`unloaded` record is an
error before control is transferred.

Registration alone is not executable permission. A callable record must also
carry `execute` permission and a declared ABI signature: calling convention,
argument types, and result type. Ordinary `malloc` memory starts as
read/write, not executable. Runtime records are not user-callable unless the
runtime explicitly exports that capability.

An executable load must use `x` permission and an ABI descriptor
`[convention, argument_types, result_type]`. Windows accepts `win64`/`system`;
Linux accepts `sysv64`/`c`. Supported integer/pointer types are `Int`, `I8`,
`U8`, `I16`, `U16`, `I32`, `U32`, `I64`, `U64`, `Ptr`, and `Bool`; `Void` is
also valid as a result. The bootstrap supports zero to sixteen arguments on
Windows and zero to six on other targets. The runtime also verifies that the
address belongs to an executable OS mapping before transfer. Ordinary malloc
memory is never executable.

These checks define language behavior and prevent mismatched calls from
becoming accepted APL programs. The operating system can still reject an
external read/write, which is reported explicitly rather than blindly
dereferencing the address.

## Fixed Implementation Decisions

Collector roots are pointer descriptors recursively reachable from current APL
values. There are no public interior-pointer/subrange descriptors yet.
`@flee` drains the global pending table. Combined scope actions always run as
`@marked`, then `@collector`, then `@flee`, independent of source order.

Obvious attempts to return an allocation or local pointer alias from
`@marked` are compile errors. Indirect escapes that static analysis cannot
prove are revoked at runtime and become unusable. Internal `raw_*` operations
are not source-callable, and excluding a public `raw_memory` export removes the
corresponding operation.
