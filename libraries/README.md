# Standard Libraries

This directory is the canonical source location for libraries shipped with
APL. Library code stays in APL wherever the language can express the required
behavior; host runtime primitives are reserved for operating-system boundaries.

Current modules:

- `math.apl` - constants, roots, logarithms, angle conversion, and trigonometry.
- `console.apl` - console formatting, input, and basic case conversion under
  the `con::` namespace. The file is not named `con` because that is a reserved
  Windows device name.
- `bitops.apl` - signed 64-bit bitwise operations and shifts.
- `datetime.apl` - Gregorian calendar calculation; system-clock access is partial.
- `random.apl` - deterministic Park-Miller pseudo-random operations.
- `timer.apl` - monotonic timing over Rust runtime primitives.
- `crypto.apl` - SHA-1, SHA-2, and SHA-3 helpers over audited host primitives.
- `classes.apl` - stable instance identities and field storage used by the
  compiler-lowered class surface.
- `raw_memory.apl` - capability-gated tracked allocation, external address
  registration, indirect calls, and explicit reclamation scopes.
- `windows.apl` - capability-gated Windows DLL loading, typed Win64 procedure
  calls, fixed-width integer helpers, UTF-16 encoding, an editable
  `WindowConfig`, stable queued events, and editable child controls through
  `ControlConfig`, including pointer-free decoded notifications, safe
  Edit/RichEdit selection, foreground/background/bold formatting with
  preserved undo history, scrolling, focus, Unicode range search, menus,
  external icons, accelerators, file dialogs, process capture, fixed-signature
  queued callback endpoints, owned window timers, control state/repaint helpers,
  client-size queries, minimum-size constraints, and the overridable
  `WindowApplication` loop.

`.aplmod` files are generated artifacts. Build them with:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\build_libraries.ps1
```

Developer builds are written to `build/libraries/`. Release modules are staged
under `dist/package/libraries/`; generated artifacts do not belong beside the
canonical `.apl` sources in this directory.
The developer CLI searches `build/libraries/` before the legacy source-adjacent
artifact location, so rebuilding cannot be masked by an older ignored module.

Library metadata lives beside the canonical source as `<name>.metadata.toml`.
It defines the library identity, namespace, description, and dependencies.
Dependencies are resolved transitively and cycle-checked before compilation;
their order in application source is irrelevant. The network package registry
and package-manager lock format remain future work.
