# APL

APL (Absolute Programming Language) is a standalone language under active
bootstrap development. The current toolchain contains a Rust host, an
APL-written portable frontend and VM path, a native-lowering prototype, a CLI,
and a native Win32 development studio.

The project is in a stabilization-led phase: agreed language additions land
with host, self-hosted, portable, native, Studio, test, and documentation
coverage kept in sync.

## Repository Map

- `crates/` - Rust host crates, CLI, runtimes, compiler bridge, and Studio.
- `std/` - APL-written lexer, parser, checker, IR, linker, VM, and bootstrap.
- `libraries/` - canonical APL sources for the standard library modules.
- `examples/` - small user-facing programs only.
- `tests/apl/` - APL bootstrap, language, and portable-runtime test programs.
- `tests/regressions/open/` - minimized known failures that are not fixed yet.
- `docs/` - current status, language notes, architecture, roadmap, and history.
- `tools/` - maintained developer tools and explicitly labelled legacy tools.
- `installer/` - reproducible Windows installer definition.
- `build/` - generated APL packages and artifacts; ignored and disposable.
- `target/` - Cargo output; ignored and disposable.

Raw output from the separate edge-case generator does not belong in this
repository. Only minimized, reproducible failures should be copied into
`tests/regressions/open/`.

## Quick Start

Build the CLI and check the current executable example:

```powershell
cargo build -p apl
.\target\debug\apl.exe check .\examples\std_main.apl
.\target\debug\apl.exe run .\examples\std_main.apl
```

Build the canonical Studio binary:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\build_studio.ps1
.\dist\package\apl_studio.exe
```

Everything under `target/` or `build/` is internal cache and may contain
development copies with different sizes. Do not select a Studio binary there.
`dist\package\apl_studio.exe` is the single user-facing release build.

Build the portable Windows package, all standard modules, and its checksum:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\build_release.ps1
```

The complete installer recipe and requirements are documented in
[docs/installation.md](docs/installation.md).

Studio uses `F5` to run through the portable path, `F6` to start debugging,
`F7` to save a native Windows executable, `F8` to continue, and `F9` to toggle
a breakpoint on the current line. See [docs/studio.md](docs/studio.md).

## Documentation

Start with [docs/README.md](docs/README.md). The short active handoff is kept in
[APL_CONTEXT.md](APL_CONTEXT.md). The former long README and context log are
preserved unchanged in `docs/history/`.

## Stabilization Status

The authoritative list of currently reproduced blockers is
`tests/regressions/open/`. The initial postfix, Float-to-Int, dynamic loop
limit, shortened-entry, `match`/loop-handler, and explicit-error regressions
are repaired and preserved under `tests/regressions/fixed/`.

The shipped library set currently contains `math`, `con`, `bitops`,
`datetime`, `random`, `timer`, `crypto`, `classes`, `raw_memory`, and the
initial `windows` system module. The network package registry, version solver,
and lockfile remain future work.

See [docs/status.md](docs/status.md) for the current implementation boundary.
