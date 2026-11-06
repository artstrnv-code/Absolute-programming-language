# Roadmap And Design Boundary

This document contains future work. Nothing here is current syntax unless the
status document says it is implemented.

## Stabilization First

1. Keep the repaired regression gate green.
2. Add one manifest-driven runner for the APL corpus.
3. Differentially test host, portable bootstrap, Studio, and native paths.
4. Align diagnostics and remove stale compatibility behavior deliberately.
5. Resume language growth only after the regression gate is green.

The security contract still needs an explicit implicit-flow rule for `if` and
`while` controlled by ASV/SASV values. The conservative options are rejecting
secret predicates with observable bodies or tracking a protected program
counter and tainting effects performed under it.

## Near-Term Sequence

1. Completed: public `exclude "symbol" << library` end to end,
   including linker behavior, diagnostics, regression coverage, and the
   packaged CLI/Studio toolchain.
2. Completed: classes and their agreed identity, initialization, output,
   inheritance, override, and exclusion rules.
3. Completed: the tracked raw-memory library and pointer table after the
   object model supplies suitable descriptors and identities.
4. In progress: the Windows backend. Its DLL/typed-call capability layer is
   implemented together with window registration, mutable window/control
   defaults, child controls, pointer-free decoded message dispatch, and
   RichEdit text/search, foreground/background/bold formatting with preserved
   undo, fonts, notification masks, scrolling, and focus. Menus, external
   icons, accelerators, configured file dialogs, synchronous/asynchronous
   process capture and cancellation, and an overridable `WindowApplication`
   loop are also implemented. Low-level callback registration and embedded PE
   resources remain.
5. After the Windows acceptance layer, remove the Rust bridge from the
   APL-written compiler path, then implement the Linux system backend.

Each stage is closed and tested before implementation begins on the next one.

## Error Model

APL deliberately does not plan to add `try`/`catch`/`except` exceptions. A
recoverable failure is represented by an ordinary returned value and checked
by the caller. `return(ERR=[message][code])` is the explicit, non-catchable
escape for terminating the whole program with a process code. This keeps
exceptions from becoming a hidden and expensive control-flow mechanism.

## Libraries

Libraries are APL source by default, with Rust or native code at system
boundaries. Current packages use `<name>.metadata.toml` for identity,
namespace, description, and transitive dependencies. An ordered multi-source
or native-load manifest plus exported constructions remains package-format
work. Source order does not control dependency resolution.

The package manager follows the network library rather than preceding it. It
should be written primarily in APL and use the same TOML metadata consumed by
the compiler. The installer-owned library directory is immutable; user
packages live under `%LOCALAPPDATA%\APL\packages`, while projects may pin local
dependencies without changing the global store.

The first source modules are math, console, bit operations, calendar datetime,
deterministic random operations, monotonic timers, crypto, classes, tracked raw
memory, and the initial Windows system module. Planned packages still include
filesystem, path, system, wall-clock time, text, formatting, containers,
higher-level allocators, networking, foreign interfaces, build/packaging,
lightweight safe threads, and the Linux system module.

## Data And Object Facilities

Classes are implemented as identity-bearing reference objects with explicit
initialization, inheritance controls, override/polymorphism, and output hooks.
Structures are implemented as independent C-like copied values with typed
defaulted fields, nested updates, no identity, and no inheritance or methods.
Direct structure output and parameterized/generic structure types remain
future design work.

Planned class work includes explicit operator methods such as `__+__`,
`__-__`, `__*__`, `__/__`, and the corresponding hooks for the other supported
operators. They will allow domain objects to participate in ordinary
expressions, for example matrix multiplication or `car / car2`, by lowering
the operator to a class method call. Before implementation, the dispatch rules
for the left and right operands, inheritance/override behavior, permitted
return values, and the separate treatment of unary, comparison, and compound
assignment operators must be fixed. This is future object-model syntax, not a
current language feature.

Protected class variants also need an explicit method contract. Before adding
ASV/SASV forms of user-defined types, define which methods each protection
level may call and whether a class can declare a deliberately narrower method
surface for protected instances. Field reads, writes, aliases, inherited
methods, operator methods, arguments, and return values must preserve at least
the strongest protection involved and must never provide an implicit
declassification path. `__out__`, serialization, containers, FFI/native
calls, addresses, and other observable channels need separate ASV/SASV rules;
SASV objects in particular must retain the existing runtime-only boundary.
Any method that intentionally returns public information derived from a
protected object must be an explicit, narrowly specified security primitive,
not an ordinary class method. The exact declaration syntax and prohibition
table remain TODO items for design review before implementation.

### Typed Security Signatures

Planned function and method signatures should explicitly declare the value
type and protection requirement of every parameter and of the return value.
The preferred final contract makes these declarations mandatory rather than
optional, including for class methods and public library APIs, so the checker
and `.aplmod` interface can validate calls without inspecting an
implementation body. Migration syntax for existing untyped functions remains
to be designed.

The planned `anys` keyword means any secrecy level, not any value type. An
`anys` parameter accepts AV, ASV, or SASV while retaining the caller's actual
protection label inside the function. It does not declassify the value. An
`anys` return likewise preserves the computed protection level. Returning an
AV result from data accepted as `anys` must be rejected unless the operation is
an explicitly declared and narrowly audited security primitive. Exact syntax,
generic interaction, override compatibility, and whether inferred lambdas are
the only exception to mandatory signatures remain TODO design decisions.

`VTime` is not planned as an entrypoint-local type. The semantic checker must
reject direct `VTime` declarations inside `std::main`; executable boundaries
should use explicitly typed AV/ASV/SASV values and typed function signatures.
`VTime` remains available for dynamic internal temporaries where the language
contract permits it. The current compiler still accepts `VTime` in
`std::main`, so enforcing this boundary is an explicit compatibility-breaking
TODO rather than documented current behavior.

## Constructions

A construction is planned as readable syntax over function arguments. It
captures typed slots from a phrase and forwards them to an explicitly selected
ordinary function. It has no independent algorithm, state, or return model;
the target function remains the practical API and source of type/security
behavior. Construction overloading is not planned initially.

## Native And Systems Work

Long-term work includes object emission, PE/ELF/Mach-O linking, AppImage and ISO
packaging, hardware/system libraries, higher-level allocation policies, and a
freestanding runtime suitable for operating-system development.

The next backend performance milestone, currently paused, is typed
specialization. When the
checker proves an `AVInt`, arithmetic and comparisons should lower to direct
machine operations instead of `apl_rt_binary` calls. Proven homogeneous
integer-list indexing and replacement should lower to bounds-checked native
loads/stores instead of `get`/`set` ABI calls. Dynamic `VTime`, mixed lists,
and protected values retain the general runtime path. A worst-case 600-element
bubble-sort benchmark in `examples/bubble_sort_datetime.apl` is the initial
inspection/performance regression: release output must remain sorted, and its
assembly should progressively lose calls from the inner loop.

### Tracked External Addresses

The implemented memory library registers an address that was not allocated by
APL using
`load(address, ...)`. Registration creates a table ID and
turns later pointer operations through that ID into legal APL memory requests.
It does not prove that the operating system has mapped the address, transfer
ownership to APL, or make an otherwise invalid address safe to access.

Memory-table records therefore carry an ownership kind. An APL allocation is
owned: releasing it calls the allocator and leaves its stable ID as a dead
historical record. A loaded address is external/borrowed: releasing it only
removes its live registration from the table and must never free, overwrite,
or clear the underlying address. This distinction also prevents an accidental
release of special addresses such as `0x00` from becoming a write operation.

The current API checks the registered byte range, read/write/execute
permissions, duplicate historical registrations, and explicit call ABI.
Sub-address descriptors and concurrent unloading remain future extensions;
neither is implied permission to dereference an arbitrary numeric address.

## Studio Self-Hosting

APL Studio should ultimately be rewritten in APL as a systems-library
acceptance milestone. The path starts with `fs`, `path`, `sys`, and process
support, followed by platform libraries such as `windows` and `linux`. The
Windows backend must expose window registration, the message loop, RichEdit,
dialogs, resources, and process execution without forcing ordinary APL code to
repeat raw Win32 defaults. A small native launcher may remain temporarily, but
editor, build, run, and debug orchestration should move into APL.

This rewrite begins only after the system-library contracts and native linker
boundary are stable; it is not a reason to extend the current Rust Studio in
two competing directions.
