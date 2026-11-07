# Documentation

The active documentation is deliberately split by responsibility:

- `getting-started.md` - build the current package and run a first portable or
  native program.
- `status.md` - what is implemented, partial, broken, or only planned.
- `language.md` - concise reference for the implemented language surface.
- `architecture.md` - current compiler and runtime paths.
- `studio.md` - editor, build, and debugger controls and implementation limits.
- `installation.md` - portable releases, Windows installer, and library lookup.
- `raw-memory.md` - implemented tracked-memory contract and runtime limits.
- `windows.md` - current Windows DLL, Win64 ABI, and capability contract.
- `edge-case-generator-spec.md` - technical machine contract for generated tests.
- `roadmap.md` - accepted and exploratory future design.
- `manual/` - official Russian user guide, written as sequential chapters.
- `history/` - preserved bootstrap-era journals; not normative.

`APL_CONTEXT.md` at the repository root is the short development handoff.

Documentation rule: never describe a designed feature as implemented. Use one
of these labels when status is not obvious: `implemented`, `partial`, `broken`,
or `planned`.
