# APL Studio

Status: implemented unless a limitation is stated explicitly below.

## Editor

Studio uses the Windows RichEdit control. The source view provides lexer-backed
syntax highlighting, synchronized line numbers, automatic delimiter closing,
matching-delimiter highlighting, undo/redo, find/replace, and automatic cursor
navigation to source diagnostics. Input and output remain separate multiline
panes.

Syntax and matching-delimiter formatting preserve the active selection. While
Studio applies RichEdit character formats it suppresses `ENM_SELCHANGE`, then
restores the original range before re-enabling selection notifications. A
non-empty user selection also disables matching-delimiter formatting for that
event, so dragging a selection cannot be replaced by an internal formatting
range. Formatting is debounced briefly after edits or caret motion and the
editor is invalidated once afterward; typing never runs the full formatter on
each key and stale RichEdit caret images are cleared in one repaint.

On startup Studio registers the current executable for `.apl` under the
current user's Windows `OpenWithProgids` and application capabilities. This
makes APL Studio available in **Open with** and Default Apps without silently
changing the user's existing default. The registered command passes the file
as a quoted `%1` argument; Studio opens that path on launch.

Association maintenance runs after the main window is visible and on a worker
thread. Registry or Windows Shell delays therefore cannot block editor startup.

## Run And Build

- `F5` checks, compiles, and runs through the linked portable APL path.
- `F7` saves the source if needed, asks for an `.exe` destination, invokes the
  existing x86-64 native pipeline in a temporary package, and copies the
  release executable to that destination.
- Named `.aplmod` libraries from the source file's `libraries/` directory are
  linked into the native code during F7 compilation. The resulting executable
  does not need the module files at runtime.
- Built executables contain an `APL_BUILD_METADATA` PE resource and the marker
  `Made with APL`.

The build requires the host Rust/Cargo toolchain while native lowering remains
a bootstrap backend. The resulting program runs as a standalone executable and
does not embed APL Studio.

## Debugger

- `F6` starts a session and pauses before the first source statement.
- `F8` continues.
- `F9` toggles a breakpoint on the current editor line.
- The debug strip provides pause, stop, step into, step over, and step out.
- The right pane shows the current source coordinate, call stack, and visible
  variables. Output is updated at every pause.

The parser assigns stable structural paths to nested statements. The runtime
maps these paths to compiled opcodes, so stepping preserves live loops, scopes,
and function calls instead of replaying the program. ASV and SASV values are
redacted inside the runtime; Studio receives only `<ASV protected>` or
`<SASV protected>`.

Current limitation: named precompiled `.aplmod` libraries do not carry source
maps, so the debugger currently covers the source program and standard
builtins. Programs using `classes` or `raw_memory` therefore run and build with
F5/F7, but are not yet step-debuggable with F6. F5 execution and F7 builds
resolve named modules normally.
