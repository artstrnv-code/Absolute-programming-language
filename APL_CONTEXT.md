# APL Active Context

This file is the short handoff for active development. It records only the
current direction and verified project boundary. Detailed historical notes are
preserved in `docs/history/APL_CONTEXT-bootstrap-2026-10-25.md`.

## Current Phase

APL is in the Windows system-library phase after stabilization of classes,
copied structures, and raw memory. The initial open P0 regressions are closed.
The DLL/typed Win64 call foundation, default window creation, pointer-free
message dispatch, editable window/control configurations, editor-grade
RichEdit support, menus, icons, accelerators, dialogs, synchronous processes,
and the overridable `WindowApplication` loop are implemented. Remaining
Windows work is low-level callback registration, embedded PE resources, and
asynchronous process control.

Current priorities:

1. Keep one reproducible CLI, Studio, release, and installation workflow.
2. Minimize and classify failures before fixing them.
3. Keep user examples separate from implementation test programs.
4. Keep current behavior separate from proposed language design.
5. Preserve the self-hosted bootstrap while reducing dependence on the Rust
   host over time.
6. Finish the Windows acceptance layer, then remove the Rust bridge from the
   APL-written compiler path before beginning the Linux backend.

Typed native specialization is recorded in the roadmap but is intentionally
paused. The current verified backend continues to use the dynamic runtime ABI.

APL Studio now has one canonical release at `dist/package/apl_studio.exe`. Its
implemented editor/build/debug boundary is recorded in `docs/studio.md`.
Standard library sources now live under `libraries/`; release packaging and
installer behavior are recorded in `docs/installation.md`.
The portable payload includes the embedded CLI/Studio compiler, its APL source,
standard `.aplmod` files, and the bootstrap native runtime. F7 no longer
depends on a repository checkout or network access, though its final link still
uses Cargo.

## Current Executable Contract

Executable source has one explicit entry point:

```apl
AVInt std::main() {
  out "hello\n"
  return 0
}
```

Standard callable functions use `std::` unless `using namespace std` or a valid
`nc` declaration opens the namespace. Core syntax such as `out`, `return`,
operators, and security primitives is not rewritten as a standard function.
Library modules remain entrypoint-free.

## Repaired Regression Gate

The postfix increment/decrement, Float-to-Int, expression-valued `while`
limit, shortened `main`, `match`/loop-handler, and explicit-error regressions
are repaired. Their minimized sources live in `tests/regressions/fixed/`.
`tests/regressions/open/` currently contains the minimized missing-position
P1. The Rust compiler/runtime suites have 22 diagnostic-contract failures:
two compiler tests and twenty runtime tests expect `semantic error at
line:column`, while the portable checker currently emits only the semantic
message. Functional core, parser, IR, native-runtime, raw-memory, crypto, and
Windows unit suites remain green. The shared symptom is minimized in
`tests/regressions/open/p1_missing_semantic_position.apl`; this is one
stabilization item, not 22 language-semantics failures.

Implemented control-flow additions are non-fallthrough `match`, `while limit`
and initial-condition `while else`, plus non-catchable
`return(ERR=[message][code])`. APL does not plan to add exception-based
`try`/`catch`/`except`; recoverable failures remain explicit return values.

## Agreed Design And Current Boundary

- Libraries use APL source where possible and native host code only at system
  boundaries. TOML identity, namespace, description, and dependency metadata
  are implemented with transitive order-independent resolution and cycle
  checks; an ordered multi-source/load manifest remains planned.
- Namespaces use `library::symbol`; implemented `using namespace` permits exactly one
  namespace, while `nc` opens several only after link-time conflict checking.
- Identity-bearing classes are implemented through `include >> classes`, with
  mandatory `__init__`, typed fields, stable handles, single inheritance,
  explicit override/exclusion, polymorphic dispatch, and `__out__`.
- C-like `struct` values are implemented without a support include: typed
  defaulted fields, zero-argument construction, nested functional updates,
  copied assignment, cycle rejection, and `.aplmod` surface export work in
  portable and native paths.
- A construction is readable call syntax which captures typed arguments and
  forwards them to an ordinary function. The function remains the practical
  implementation; the construction owns no independent runtime behavior.
- The long-term compiler target is native PE/ELF/Mach-O output, with AppImage
  as packaging around a Linux executable and a future freestanding profile for
  OS work.
- Current native EXEs still route dynamic values, integer comparisons, and list
  access through runtime ABI calls. A future, currently paused performance pass
  specializes proven `AVInt` and homogeneous integer-list inner loops into
  direct machine arithmetic and bounds-checked load/store operations.
- APL Studio is intended to become an APL-written systems-library acceptance
  milestone after `fs`/`path`/`sys` and the `windows`/`linux` backends mature.
- The implemented `raw_memory` table distinguishes owned APL allocations from external
  addresses registered by `memory::load(...)`. Releasing an external registration
  only detaches it from the table; it never frees or clears the underlying
  memory.
- `memory::call` accepts only a live table entry with explicit execute
  permission and a declared ABI signature. Records keep `origin`
  (`user`/`runtime`/`load`) separate from lifecycle state.
- `include >> windows` now owns DLL identities and registers every resolved
  export in that same table. Closing a DLL unloads all attached procedure
  capabilities. The bootstrap Win64 dispatcher supports typed integer/pointer
  signatures with up to sixteen arguments; aggregate/callback FFI remains
  unfinished.
- Windows also owns menu, icon, and accelerator identities. The exported
  `WindowApplication` class routes blocking decoded events through overridable
  handlers, and application-defined subclasses retain polymorphism across the
  `.aplmod` boundary.
- Raw memory uses two-phase release: `memory::mark(ptr)` revokes access and queues an
  owned allocation, while parameterless `memory::free()` releases every queued
  allocation. Marking a borrowed `load` record only unregisters it.
- Implemented scope attributes are `@marked` (mark allocations made in a function
  or whole loop), `@collector` (mark unreachable owned allocations on exit),
  and intentional `@flee` (run batch `free()` on exit; "flee" is deliberate).
  Combined exit order is mark, collect, then free.
- `@marked` is a strictly non-escaping temporary arena. Raw IDs, addresses,
  slices, and aliases created inside cannot be returned or stored outside;
  only copied/computed values may leave the scope.

## Source Of Truth

- Current status: `docs/status.md`
- Implemented language surface: `docs/language.md`
- Compiler organization: `docs/architecture.md`
- Future design: `docs/roadmap.md`
- Test layout: `tests/README.md`
- Installation and releases: `docs/installation.md`
- Windows system boundary: `docs/windows.md`

When implementation changes, update the relevant focused document and this
handoff only if the project boundary or priority changed.
