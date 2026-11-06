# Implementation Status

Updated: 2026-11-06.

## Implemented

- Rust lexer/parser/checker/runtime reference layers.
- APL-written lexer, parser, semantic checker, IR, linker, verifier, portable
  VM, artifact handling, and bootstrap driver.
- Portable linked artifacts and named `.aplmod` libraries.
- Canonical APL library sources for `math`, `con`, `bitops`, `datetime`,
  `random`, `timer`, `classes`, and `raw_memory`, with reproducible module
  compilation.
- Source composition with `include`, `exclude`, `using namespace`, and `nc`.
- Explicit executable entry `AVInt std::main()`.
- CLI check/run, artifact, bootstrap, native-lowering, and package commands.
- Win32 APL Studio with a RichEdit source editor, lexer-backed highlighting,
  line numbers, delimiter assistance, find/replace, diagnostic navigation,
  input/output panes, F5 portable run, F7 native EXE build, and source debugger.
- Runtime source maps and debug snapshots with breakpoints, continue/pause/stop,
  step into/over/out, call stacks, and ASV/SASV value redaction.
- Target-neutral native IR, ABI lowering, and x86-64 assembly prototypes.
- Typed absolute variables, `VTime`, protection levels, lists, indexing,
  slicing, functions, conditions, bounded loops, `pick`, and flow control.
- Qualified `std::add/get/set/pop` calls preserve dynamic list protection;
  zero-step slices fail consistently with an explicit runtime diagnostic.
- Postfix `++`/`--`, expression-valued loop limits, finite Float-to-Int
  truncation, and shortened `main` with an open `std` namespace.
- Non-fallthrough `match` with multiple values, inclusive ranges, and
  `case default`; cause-specific `while limit` and `while else` handlers.
- `return(ERR=[message][code])` propagates through calls and preserves an
  explicit process exit code in portable and native execution.
- Mutable-list protection propagates across tracked `VTime` aliases. JSON
  conversion and typed JSON input reject malformed values.
- Portable and Rust native runtimes expose monotonic timer builtins with
  accumulated start/pause/resume behavior.
- Reproducible Windows release staging with the embedded compiler, self-hosted
  sources, standard modules, bootstrap native runtime, ZIP/checksum generation,
  and a per-user Inno Setup installer definition.
- Identity-bearing classes through the `classes` module: mandatory `__init__`,
  typed fields, stable IDs and aliases, single inheritance, explicit override,
  inherited-method exclusion, polymorphic dispatch, and `__out__` hooks in
  portable and native execution.
- C-like copied structures with typed/defaulted fields, zero-argument
  construction, nested field reads and compound updates, cyclic-layout
  rejection, runtime guards for values returned by untyped functions, and
  `.aplmod` surface export in portable and native execution.
- Tracked raw memory through `include >> raw_memory`: stable non-reused IDs,
  owned and borrowed records, bounded byte access, source-aware `where`,
  executable ABI registration and `call`, two-phase `mark/free`, and
  `@marked`/`@collector`/`@flee` scopes in portable and native execution.
- Initial Windows system module through `include >> windows`: opaque DLL
  identities, capability-registered exports, typed Win64 integer/pointer calls
  with up to sixteen arguments, automatic revocation on `FreeLibrary`, UTF-16
  conversion, `MessageBoxW`, default window registration/creation, opaque
  window identities, and blocking/nonblocking message dispatch in portable and
  native paths.
- Class-aware `.aplmod` exports preserve public class fields, constructors, and
  methods across separately compiled libraries. The Windows module uses this
  for its editable `WindowConfig` and declares its `classes` dependency in
  TOML metadata.
- Imported base classes can be extended by descendants in later `.aplmod`
  layers or application source. Compiler-owned ancestor dispatch tables are
  regenerated and overlaid so inherited methods see external overrides in
  portable and native execution.
- Win32 window events cross into APL through an explicit per-thread queue and
  `windows::next_event()` rather than a reentrant callback into the VM.
  `WM_COMMAND` and `WM_NOTIFY` are decoded to stable source identity,
  notification code, and control ID without exposing native pointer values;
  `WindowEvent` supplies named APL fields and predicates.
- Initial Win32 child controls use stable APL identities and an editable
  `ControlConfig`. Static, button, edit, and RichEdit classes can be created;
  text and geometry can be updated, and `WM_COMMAND`/`WM_NOTIFY` events resolve
  their originating control to a stable identity instead of exposing `HWND`.
  Destroying a parent immediately revokes the identities of every child that
  Win32 destroys with it.
- Edit/RichEdit controls expose safe selection ranges, selection replacement,
  and line counts in portable and native execution. UTF-16 buffers and
  `SendMessageW` pointers remain inside the Windows host boundary.
- RichEdit foreground/background/bold formatting and font face/size changes
  preserve both the user's selection and undo history. Undo/redo, notification
  masks, focus, vertical scrolling, and native Unicode range search remain
  behind pointer-free APL contracts.
- Windows child processes can be run without a console window using a string
  argument list, captured stdin/stdout/stderr, an optional working directory,
  and an explicit exit code. They can also be started asynchronously, polled
  for accumulated output, waited for, or terminated through stable identities.
  Win32 Job Objects clean up the owned process tree on termination or host exit.
  `ProcessResult` and `ProcessSnapshot` provide named fields without exposing
  process or pipe handles.

## Partial

- Native output is a bootstrap backend, not yet the final independent compiler
  and linker toolchain. It still represents ordinary values through the
  dynamic runtime ABI, so typed integer arithmetic, comparisons, and list
  indexing emit calls rather than specialized machine operations.
- Library composition uses TOML metadata with transitive, order-independent
  dependency resolution and cycle checks. A network package registry, version
  solver, and lockfile format are not implemented.
- The installer recipe is implemented, but release installers are not yet
  code-signed and require Inno Setup 6 to build locally. Native F7 builds no
  longer require a source checkout or network access, but still require an
  external Cargo toolchain for the final link.
- Diagnostics are useful but still differ between host validation and the
  portable frontend in some paths.
- Existing APL test programs cover many stages but are not yet driven by one
  corpus manifest.
- Studio debugging currently executes the checked host runtime representation;
  named external `.aplmod` modules do not yet carry source maps for stepping.
- The implicit-flow policy is not yet fixed: conditions and loop predicates may
  currently depend on ASV/SASV values even when their bodies have observable
  public effects. The checker must not choose rejection or program-counter
  tainting until that language contract is agreed.
- The Windows module exports editable `WindowConfig` and `ControlConfig`
  classes through class-aware `.aplmod` metadata. Generic child creation is
  implemented together with editor-grade RichEdit formatting, fonts, undo,
  event masks, scrolling, focus, search, synchronous/asynchronous process
  capture and cancellation, configured native open/save file dialogs, stable
  owned native menus, owned
  external icons, owned accelerator tables, blocking decoded event retrieval,
  an overridable `WindowApplication` loop, and linked PE build metadata with an
  optional application icon. Four statically typed Win32 callback endpoints
  queue foreign invocations without reentering the VM; direct synchronous APL
  function callbacks remain. Its current FFI supports integer/pointer
  signatures only and uses a fixed sixteen-argument bootstrap dispatcher.
  Owned window timers now feed `WM_TIMER` through the decoded event queue, and
  window/control enable plus invalidation operations cover Studio debounce and
  repaint orchestration without exposing raw HWNDs.

## Open Regressions

The compiler/runtime unit suites currently have 22 reproducible
diagnostic-contract failures: two compiler tests and twenty runtime tests
expect `semantic error at line:column`, while the portable checker currently
reports only the semantic message. The failures cover `exclude` and other
checker diagnostics but share the same missing-position symptom. Functional
core, parser, IR, native-runtime, raw-memory, crypto, and Windows unit suites
remain green. The minimized source
`tests/regressions/open/p1_missing_semantic_position.apl` reproduces the shared
symptom.

Several corpus outcomes still require an explicit language decision:
short-circuit evaluation, out-of-range `set`, empty-string splitting, and
implicit-flow security.

## Planned

- Construction syntax and optional future struct output facilities.
- Standard library package installation, version solving, and lockfiles.
- Filesystem, path, system, wall-clock time, text, formatting, containers,
  networking, foreign, build, threads, Linux, and the remaining Windows layer;
  expansion of the existing math, console, random, timer, and datetime modules.
- Native object emission, platform linking, packaging, and freestanding/OS
  profiles.

Planned items are design commitments or candidates, not accepted syntax in the
current parser.

Exception handling is not planned. Recoverable library failures use returned
values; `ERR` is an explicit, non-catchable whole-program termination.
