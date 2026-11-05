# Installation And Distribution

APL currently targets 64-bit Windows for its complete Studio and installer
experience. The portable compiler layers also run on supported Rust hosts.

## Windows Installer

The installer places these components under the current user's local Programs
directory and does not require administrator rights:

- `apl.exe` command-line compiler and runner;
- `apl_studio.exe` Win32 editor, runner, debugger, and native build shell;
- the APL-written compiler sources used to build the embedded toolchain;
- the native runtime crate used by the current F7 bootstrap linker path;
- its vendored `apl_raw_memory`, `apl_crypto`, and `apl_windows` system-boundary
  crates, so native builds resolve the same capabilities as F5;
- compiled standard `.aplmod` libraries and their APL sources;
- examples, focused documentation, and the MIT license;
- Start menu shortcuts and an `.apl` Open With registration;
- an optional user `PATH` entry for the CLI.

The uninstaller removes the registered file type, shortcuts, installed files,
and only the exact PATH entry it added.

The installed tree carries `apl-distribution.toml`. Tools and future package
management code must use that manifest instead of guessing repository paths.
The console module is stored as `console.aplmod` and exports `con::`; `con` is
a reserved Windows device filename and cannot be a portable package identity.

## Portable Package

Build the binaries, modules, documentation bundle, ZIP archive, and checksum:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\build_release.ps1
```

Results are written under `dist/`:

- `package/` - unpacked installer payload;
- `APL-<version>-windows-x64.zip` - portable archive;
- `APL-<version>-windows-x64.zip.sha256` - checksum;
- `package/apl_studio.exe` - the single canonical Studio binary, already beside
  its CLI, compiler components, and libraries.

## Installer Build

Install Inno Setup 6, then run:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\build_installer.ps1
```

The installer is emitted to `dist/installer/`. The recipe is maintained in
`installer/windows/apl.iss`. Release executables are not code-signed yet, so
Windows may show an unknown-publisher warning.

## Library Resolution

For `include >> name`, APL searches in this order:

1. `<source directory>/libraries/name.aplmod`;
2. `<apl.exe or apl_studio.exe directory>/libraries/name.aplmod`;
3. for developer binaries inside this repository only,
   `<workspace>/build/libraries/name.aplmod`;
4. the legacy ignored `<workspace>/libraries/name.aplmod` location.

This keeps a project's pinned local module authoritative while allowing the
installer to provide standard libraries globally. Workspace builds prefer the
dedicated generated directory, so an old source-adjacent artifact cannot mask
a module rebuilt by `tools/build_libraries.ps1`.

## Native Build Requirement

Studio F7 now resolves its native runtime and vendored Rust dependencies from
`compiler/native-runtime` beside the installed tools, so it needs neither an
APL source checkout nor network access. It still invokes the external
Rust/Cargo toolchain for the final native link. F5 execution, checking,
portable artifacts, editing, and debugging do not require Cargo.

Generated native packages contain a local Cargo source configuration. Build
one manually from inside its directory so Cargo discovers that configuration;
Studio and `compile-native` set this working directory automatically.

Removing the final Cargo requirement depends on native object emission and a
platform linker; it is a compiler milestone, not an installer workaround.

## Future Package Store

The installer-owned `libraries/` directory is read-only application content.
The future package manager will use `%LOCALAPPDATA%\APL\packages` for user
packages and a project-local package directory for pinned dependencies. Its
network client and dependency resolver should be implemented in APL after the
network library exists; only the directory and manifest contracts are fixed
at this stage.
