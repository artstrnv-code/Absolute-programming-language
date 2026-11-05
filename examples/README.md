# Examples

This directory contains small programs intended for APL users. Compiler and VM
development programs live under `tests/apl/`; pre-entrypoint examples are kept
under `legacy/`.

## Programs

- `std_main.apl` - minimal executable with `AVInt std::main()`.
- `nc_math.apl` - conflict-checked namespace import.
- `con_io.apl` - console output and prompted input.
- `bitops_demo.apl` - signed 64-bit bit operations.
- `datetime_demo.apl` - Gregorian calendar calculations and current UTC time.
- `random_demo.apl` - deterministic pseudo-random operations.
- `timer_demo.apl` - Rust-backed monotonic timing.
- `classes_demo.apl` - identity, fields, inheritance, override, and output hooks.
- `raw_memory_demo.apl` - tracked bytes, stable identity, and two-phase release.
- `windows_window.apl` - default Win32 window creation and message dispatch.
- `windows_font.apl` - RichEdit creation and undo-safe font configuration.
- `windows_process.apl` - hidden child process with captured output and status.
- `windows_dialog.apl` - configured native open-file dialog with cancellation.
- `windows_menu.apl` - owned native menu tree and decoded command events.

## Build The Modules

Library sources live in the repository-level `libraries/` directory. Generate
the modules used by these examples with one command:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\build_libraries.ps1
```

The generated `libraries/*.aplmod` files are ignored by Git. Developer CLI and
Studio binaries discover that workspace directory. Run an example through the
release CLI, for example:

```powershell
.\target\release\apl.exe run .\examples\datetime_demo.apl
```

Local modules beside a project take priority. Installed APL tools also search
their own `libraries/` directory, which is where the Windows package installs
the standard modules.
