# Test Sources

This directory separates APL test programs from user-facing examples.

- `apl/bootstrap/` - lexer, parser, checker, IR, linker, artifact, and bootstrap
  programs used during self-host development.
- `apl/runtime/` - portable VM behavior and guard programs.
- `apl/language/` - focused operator and collection programs.
- `regressions/open/` - minimized known failures; these are expected to fail
  until their linked issues are fixed.
- `regressions/fixed/` - minimized repaired failures whose contracts are
  asserted at the owning implementation layer.
- `manual/` - preserved exploratory sources that are not automated assertions.

Rust unit and integration tests remain under their owning crates.

The external edge-case generator may create large raw corpora elsewhere under
`D:\veles`. Do not copy that output wholesale into this repository. Minimize a
failure, assign its severity, record expected and current behavior, then add
only that reproduction to `regressions/open/`. Move it to `regressions/fixed/`
when tests assert the repaired behavior.
