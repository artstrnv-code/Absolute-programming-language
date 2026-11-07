# Getting Started

APL is still a bootstrap project. The complete user path currently targets
64-bit Windows and still needs a Rust toolchain for the final native link.

## Build The Current Package

From the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\build_release.ps1
```

The result is under `dist\package\`. It contains the CLI, Studio, compiler
sources, standard libraries, examples, and the native runtime used by F7.

Start Studio with:

```powershell
.\dist\package\apl_studio.exe
```

For compiler work without rebuilding the complete package:

```powershell
cargo build -p apl
```

That produces `target\debug\apl.exe`. Developer output under `target\` may be
newer than the packaged Studio, so do not mix binaries and libraries from the
two trees when testing a release.

## First Program

Create `hello.apl`:

```apl
AVInt std::main() {
  out "hello from APL\n"
  return 0
}
```

Check it without running:

```powershell
.\dist\package\apl.exe --check .\hello.apl
```

`apl check <file>` remains an equivalent command for compatibility.

Run it through the portable compiler and VM:

```powershell
.\dist\package\apl.exe run .\hello.apl
```

`out` does not add a newline. Write `\n` when one is wanted.

Every executable has exactly one `AVInt std::main()` and returns a public
integer process code. File-scope declarations and functions are allowed, but
executable statements belong inside `std::main`.

## Input And Arguments

`input` values are numbered by their position in the program:

```apl
AVInt std::main() {
  AVInt left = input
  AVInt right = input
  out left + right
  out "\n"
  return 0
}
```

Pass them as numbered flags after `--`:

```powershell
.\dist\package\apl.exe run .\sum.apl -- -1 12 -2 30
```

The output is `42`. Missing numbered values are read from standard input.
`secret input` cannot be passed as a process argument; use standard input or
Studio's input pane so the value is not exposed in a process command line.

## Use A Library

Libraries are explicit:

```apl
include >> math

AVInt std::main() {
  out math::sqrt(81)
  out "\n"
  return 0
}
```

`include` makes the module available, but it does not remove its namespace.
Use `math::sqrt`, open one namespace with `using namespace math`, or use `nc`
for several libraries whose export sets do not conflict. The linker checks
those claims instead of accepting ambiguous names.

Build a library source into a neighboring `.aplmod`:

```powershell
apl build-lib .\libraries\sample.apl
```

An explicit destination is optional:

```powershell
apl build-lib .\libraries\sample.apl .\package\sample.aplmod
```

`emit-module` remains available as the low-level module-emitter command.

## Portable And Native Paths

`run` uses the APL-written frontend, linked artifact, and portable VM. In
Studio, this is F5.

F7 builds a native Windows executable and asks where to save it. The resulting
program does not need Studio or `.aplmod` files at runtime. Building it still
uses the packaged Rust runtime and an external Cargo toolchain; removing that
last bridge is unfinished compiler work.

The CLI exposes the same native path:

```powershell
.\dist\package\apl.exe compile-native .\hello.apl .\build\hello-native
```

The command prints the generated package and executable paths. It builds
offline from the runtime sources and vendored dependencies shipped with the
APL package.

## Where To Continue

- Read [the language reference](language.md) for implemented syntax.
- Read [the current status](status.md) before depending on a planned feature.
- Read [the Studio guide](studio.md) for editing, building, and debugging.
- Read [the Windows contract](windows.md) for native UI and process APIs.
- Read [installation and distribution](installation.md) for the ZIP and
  installer layouts.
