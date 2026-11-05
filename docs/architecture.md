# Architecture

## Repository Layers

- `apl_parser` - host AST parser and executable-structure validation support.
- `apl_raw_memory` - shared tracked allocation table, address validation, and
  indirect-call boundary used by portable and native runtimes.
- `apl_core` - types, protection model, and reference semantic checks.
- `apl_ir` - host IR structures and lowering support.
- `apl_runtime` - reference and bootstrap-hosted runtime behavior.
- `apl_compiler` - bridges into the APL-written compiler, artifacts, libraries,
  native lowering, and package generation.
- `apl_cli` - command-line orchestration and process I/O.
- `apl_studio` - Windows RichEdit editor, native build shell, and source
  debugger over the shared parser/compiler/runtime layers.
- `std/*.apl` - APL-owned frontend, IR, linker, verifier, VM, ABI, assembly,
  native, artifact, and bootstrap implementation.
- `std/classes.apl` - APL-owned validation and lowering for the class surface.
- `std/structures.apl` - APL-owned copied-structure validation, surface export,
  and functional field lowering.
- `std/raw_memory.apl` - APL-owned lifetime-attribute validation and lowering.
- `libraries/classes.apl` - stable object identities, ancestry, and per-instance
  field tables shared by portable and native execution.

## Current Execution Path

For ordinary CLI and Studio execution, the host validates the source boundary,
resolves requested module files relative to the source, invokes the APL-written
portable compiler, verifies the linked artifact, and hosts the portable VM.
The source-facing namespace separator is `::`; compatibility internals may use
the older dotted symbol representation.

Class declarations are consumed by the APL-written frontend before ordinary
semantic checking. They lower to constructors, typed field accessors, and
polymorphic dispatch functions over the `classes` module. The module stores
stable handles and object records in APL data, so portable and native paths use
one object model instead of separate host object heaps.

Structure declarations are lowered earlier into tagged value constructors,
runtime type guards, and typed getter/setter functions. Setters copy the field
vector and return a rebuilt record, giving nested field assignment value
semantics in both portable and native paths without a second object heap.
Structure surfaces travel in `.aplmod` symbol tables; cyclic by-value layouts
are rejected before IR generation.

When a later module declares a descendant of an imported class, class lowering
regenerates dispatch functions for the affected imported ancestors. Program
composition overlays only matching compiler-owned `aplclass.dispatch.*`
functions from later layers before ordinary duplicate validation and
reachability linking. This lets a base method compiled in one module dispatch
to an override introduced by another without allowing general last-writer-wins
symbol replacement.

Named modules are resolved first from the source file's `libraries/` directory.
If a module is absent there, installed tools fall back to the `libraries/`
directory beside `apl.exe` or `apl_studio.exe`. Local project modules therefore
remain authoritative while release packages can ship a shared standard set.

## Studio Paths

Studio F5 uses the same linked portable compiler and VM path as normal source
execution. F7 invokes native lowering, builds the generated release package,
copies the chosen `.exe`, and adds `APL_BUILD_METADATA` as PE `RCDATA` plus a
plain `Made with APL` marker.

Debug parsing produces structural statement paths and source coordinates.
`apl_runtime` carries those coordinates beside compiled statement opcodes and
emits snapshots from the live execution state. Secret values are replaced by
protection labels inside the runtime before a snapshot reaches Studio. The
current debugger handles the source program and standard builtins; stepping
into named precompiled modules remains partial until module source maps exist.

## Native Direction

The current native path can lower linked APL into target-neutral low IR, apply
platform ABI decisions, emit x86-64 textual assembly, and construct packages.
It remains a bootstrap path. The intended end state emits platform objects and
links native executables without requiring the Rust runtime for ordinary APL
programs.

## Generated Data

`target/` is Cargo output. `build/` contains generated APL artifacts and nested
packages. Neither directory is source or documentation, both are ignored by
Git, and both may be removed with `tools/clean_workspace.ps1`. The maintained
Studio build script stages the release executable at
`dist/package/apl_studio.exe`; that is the only user-facing binary path.

`tools/build_release.ps1` stages the CLI, Studio, compiled modules, sources,
the APL compiler sources, the bootstrap native runtime, examples, and
documentation under `dist/package`, then creates a portable ZIP and SHA-256
checksum. `installer/windows/apl.iss` consumes the same stage so the portable
archive and installer cannot silently diverge. `apl-distribution.toml`
describes this installed layout for tools that need to locate components.
The developer-facing `dist/package/apl_studio.exe` uses the exact same staged
tree as the ZIP and installer, so there is no second user-facing Studio binary.
