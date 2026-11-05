# Windows Library Contract

Status: partial. `include >> windows` provides the implemented Windows system
boundary in portable execution and native Windows executables. The current
slice covers DLL lifetime, typed Win64 procedure calls, UTF-16 conversion,
basic window creation, editable window/control configuration, common child
controls, safe decoded message dispatch, and the RichEdit operations needed by
an editor: text, selection, search, foreground/background/bold formatting,
undo/redo, notification masks, scrolling, focus, font face/size management, and
synchronous child-process execution with captured standard streams. Native
open/save file dialogs are available through a small editable configuration
class. Stable owned menu trees support native menu bars and decoded commands.
Owned external `.ico` resources can be attached without exposing `HICON`, and
`wait_event()` provides an efficient blocking event step. Owned accelerator
tables translate keys in every blocking and nonblocking message-loop path.
`WindowApplication` supplies an overridable APL event-dispatch loop, including
subclasses declared outside the precompiled Windows module. Embedded PE
resources and asynchronous process control remain the next layer.

The module metadata declares `windows -> classes`. The compiler resolves this
dependency transitively and independently of source `include` order. Class
surface metadata is carried by `.aplmod`, so `WindowConfig` remains a real
typed class when consumed from the compiled Windows module; its implementation
does not need to be copied into the application.

## Capability Boundary

`windows::load_library(name)` calls `LoadLibraryW` and returns an opaque
library identity, not a module address. `windows::get_procedure(library, name,
argument_types, result_type)` resolves an export and immediately registers it
as an executable borrowed record in the shared raw-memory table. Its result is
the ordinary APL pointer descriptor:

```text
["APL_PTR_1", stable_id, address, "load"]
```

APL code cannot call the numeric address directly. `windows::call(procedure,
arguments)` accepts only a live registered descriptor/ID with the exact ABI
arity recorded by `get_procedure`. Pointer arguments may themselves be
raw-memory descriptors; the runtime resolves them to their registered address
immediately before the call.

`windows::close_library(library)` calls `FreeLibrary` and revokes every
procedure descriptor derived from that library. A later call through one of
those descriptors fails with an `unloaded` raw-memory state before control can
reach the stale address. Library and procedure identities are never reused.

## Win64 ABI Types

The implemented type names are `Int`, `I8`, `U8`, `I16`, `U16`, `I32`, `U32`,
`I64`, `U64`, `Ptr`, `Bool`, and result-only `Void`. The same strings are
available as constants such as `windows::U32` and `windows::PTR`.

Arguments are range-checked before transfer. Narrow results are truncated to
their declared width and then sign- or zero-extended into APL's signed 64-bit
integer representation. `Bool` maps zero to false and any nonzero result to
true. `Void` discards the machine result and returns zero at the low-level
boundary.

The current bootstrap dispatcher supports up to sixteen integer/pointer
arguments on Windows. It follows the platform `system` convention, which on
x86-64 places the first four integer/pointer arguments in RCX, RDX, R8, and R9
and the remainder in stack slots with the required shadow space. Floating
point, vector, aggregate-by-value, and callback signatures are not implemented
yet. Sixteen arguments are enough for the immediate Win32 bootstrap, but this
fixed match table is not the final arbitrary-signature FFI.

## Public Helpers

- `windows::available()` reports whether the current runtime target is Windows.
- `windows::last_error()` exposes the current thread's `GetLastError` value.
- `windows::utf16(value)` returns NUL-terminated UTF-16LE bytes.
- `windows::message_box(text, caption, kind)` wraps `MessageBoxW` without
  requiring callers to allocate two temporary wide strings.
- `windows::run_process(program, arguments, stdin_text, working_directory)`
  starts a hidden child process, waits for completion, and returns
  `[exit_code, stdout, stderr]`. Arguments must be a list of strings; stdin is
  supplied as UTF-8 text. An empty working directory inherits the parent
  directory. `ProcessResult(raw)` provides named `exit_code`, `stdout`, and
  `stderr` fields without exposing process or pipe handles.
- `windows::open_file_dialog(owner, title, initial_directory, file_name,
  filter_name, filter_pattern, default_extension)` and the matching
  `save_file_dialog` wrap the Unicode common-dialog API. `owner = 0` creates an
  unowned dialog; any other owner must be a live stable APL window identity.
  Success returns the selected path, cancellation returns `NONE`, and native
  common-dialog failures remain runtime errors.
- `FileDialogConfig` supplies editable dialog title, initial directory,
  suggested file name, filter label/pattern, and default extension. Its
  `open(owner)` and `save(owner)` methods keep Win32 buffers, double-NUL filter
  encoding, and `OPENFILENAMEW` details outside APL code.
- `windows::create_menu(popup)` creates a stable root or popup menu identity.
  `append_menu_item`, `append_submenu`, and `append_menu_separator` build its
  tree; command IDs must be in `1..65535` so `WM_COMMAND` never truncates them.
- `windows::set_menu(window, menu)` attaches a root menu and redraws the menu
  bar; passing zero removes the current menu. `set_menu_item_enabled` and
  `set_menu_item_checked` update command state and redraw an attached root.
  `destroy_menu(root)` detaches it if needed and recursively revokes every
  submenu identity. An owned submenu cannot be destroyed separately. Win32
  destroys an attached menu together with its window; the host mirrors that
  behavior by revoking the complete identity tree when the window closes.
- `Menu(popup)` wraps that lifecycle with `item`, `submenu`, `separator`,
  `attach`, `enable`, `check`, and explicit `destroy` methods. `submenu`
  accepts the child menu's stable `identity`.
- `windows::create_window(title, width, height)` registers APL's default class
  and returns an opaque stable window identity. The runtime supplies the module
  instance, cursor, background, `WndProc`, default position, and standard
  overlapped-window flags.
- `windows::load_icon(path, width, height)` loads an owned standalone `.ico`
  through Unicode `LoadImageW`; zero dimensions select the system icon size.
  `set_window_icon(window, icon)` applies both large and small variants, and
  passing zero clears them. `destroy_icon(icon)` first removes the icon from
  every live APL window and then releases it, so `DestroyIcon` never sees a
  resource still in use.
- `Icon(path, width, height)` wraps that resource identity with `apply(window)`
  and explicit `destroy()` methods. `WindowConfig.icon` may hold the identity
  so a configured window receives it during `create()`.
- `windows::create_accelerators(entries)` creates an owned accelerator table
  from `[key, command, modifiers]` entries. Key and command values use the
  Win32 16-bit range; modifiers may combine `ACCEL_ALT`, `ACCEL_CONTROL`,
  `ACCEL_SHIFT`, and `ACCEL_NOINVERT`. `set_accelerators(window, table)`
  attaches or clears a table, and `destroy_accelerators(table)` safely detaches
  it from every window before releasing the native handle.
- `Accelerators(entries)` wraps that identity with `attach(window)` and
  `destroy()`. `WindowConfig.accelerators` attaches a table during `create()`.
  Accelerator keys arrive as ordinary decoded menu-style command events, and
  `WindowEvent.is_accelerator(command_id)` recognizes them without exposing
  `HACCEL` or message-packing details.
- `windows::show_window(window)` shows and updates a live window.
- `windows::destroy_window(window)` destroys it and invalidates the identity.
- `windows::run_message_loop()` blocks on `GetMessageW`, translates and
  dispatches messages, and returns the `WM_QUIT` exit code.
- `windows::pump_messages()` performs a nonblocking `PeekMessageW` pass and
  returns false after `WM_QUIT`.
- `windows::next_event()` returns `NONE` when the event queue is empty or
  `[window_id, message, wparam, lparam, source_id, code, control_id]` for the
  oldest captured event. `source_id` is the stable APL identity of the
  originating child control for `WM_COMMAND` and `WM_NOTIFY`; `code` is the
  decoded notification code and `control_id` is its numeric Win32 ID. Their
  pointer-bearing native `lparam` is replaced with zero, so the queue never
  leaks a child `HWND` or a temporary `NMHDR` pointer. Other captured messages
  retain their integral `lparam`. The queue is filled by `WndProc` and consumed
  explicitly by APL; Win32 never calls back into a running VM frame.
- `windows::wait_event()` first drains an already decoded event, otherwise
  blocks in one checked `GetMessageW`/`TranslateMessage`/`DispatchMessageW`
  step until a public event exists. It returns `NONE` only after `WM_QUIT`, so
  APL window loops can sleep instead of polling `pump_messages()`.
- `WindowEvent(raw)` provides named `window`, `message`, `wparam`, `lparam`,
  `source`, `code`, and `control_id` fields over a value returned by
  `next_event()`. Its `is_message`, `is_from`, `is_command`, and
  `is_notification` methods avoid repeated list indexing.
  `is_menu(command_id)` recognizes menu-originated `WM_COMMAND` events without
  exposing their Win32 packing rules.
- `WindowApplication(window)` owns a blocking `wait_event()` loop and routes
  decoded messages through overridable `on_event`, `on_close`, `on_destroy`,
  `on_resize`, `on_menu`, `on_command`, and `on_notification` methods. Its
  `exit(code)` requests an orderly loop exit. Application source may subclass
  this class from the compiled module; inherited `run()` dispatches to the
  external overrides in portable and native builds.
- `windows::create_control(parent, class, text, id, x, y, width, height,
  style, extended_style)` creates a child control and returns an opaque stable
  identity. The shipped class constants cover `STATIC`, `BUTTON`, `EDIT`, and
  `RICHEDIT50W`; requesting RichEdit loads and retains `Msftedit.dll`
  automatically.
- `windows::set_window_text(identity, text)` and
  `windows::window_text(identity)` update/read window or control text.
- `windows::move_window(identity, x, y, width, height, repaint)` changes a
  window or control rectangle.
- `windows::selection(control)` returns `[start, end]` using full 32-bit
  UTF-16 character positions. Equal positions describe the caret.
- `windows::set_selection(control, start, end)` selects a range; `-1` retains
  the Win32 special selection behavior, while other positions must be
  nonnegative and ordered.
- `windows::replace_selection(control, text, can_undo)` inserts or replaces
  text at the current range without exposing the temporary UTF-16 pointer.
- `windows::line_count(control)` returns the logical/display line count used
  by Edit or RichEdit, including word-wrapped lines.
- `windows::rgb(red, green, blue)` validates three byte channels and returns a
  Win32 `COLORREF`, or `NONE` for an invalid channel.
- `windows::set_text_color(control, start, end, color)` applies a foreground
  color to a RichEdit range and restores the previous selection/caret. `end`
  may be `-1` to select through the end of the document.
- `windows::set_background_color(control, start, end, color)` and
  `windows::set_text_bold(control, start, end, enabled)` apply the other two
  syntax-highlighting attributes without changing the selection. All three
  formatting helpers suspend and resume RichEdit undo recording through TOM,
  so highlighting does not displace the user's most recent edit.
- `windows::set_text_font(control, start, end, face, points)` applies a
  nonempty font face of at most 31 UTF-16 units and a positive whole-point size.
  It uses the same selection restoration and undo suspension as highlighting;
  `end = -1` applies through the end of the document.
- `windows::can_undo/undo/can_redo/redo(control)` expose the RichEdit edit
  queues; `windows::clear_history(control)` deliberately empties both queues.
- `windows::event_mask(control)` and `windows::set_event_mask(control, mask)`
  read and replace the RichEdit notification mask. `enable_event` and
  `disable_event` update a single flag. Constants include `ENM_CHANGE`,
  `ENM_UPDATE`, `ENM_SCROLL`, `ENM_SCROLLEVENTS`, and `ENM_SELCHANGE`.
- `windows::first_visible_line(control)` and
  `windows::scroll_lines(control, lines)` support a synchronized line-number
  gutter. RichEdit ignores horizontal line-scroll arguments, so the APL API
  intentionally exposes only the meaningful vertical count.
- `windows::focus(window)` moves keyboard focus to a registered window or
  child control and confirms that it became the focused target.
- `windows::find_text(control, text, start, end, match_case, whole_word)`
  performs a forward Unicode RichEdit search. It returns `[start, end]` using
  UTF-16 positions or `NONE`; an empty search term is rejected so Find Next
  loops cannot stall on an empty match.
- `windows::i8/u8/i16/word/i32/dword/qword` perform checked conversion to
  common Win32 widths. `qword` covers the nonnegative half representable by
  APL `Int`; values above signed 64-bit range need a future unsigned value type.
- `windows::bool(value)` returns Win32 `TRUE` or `FALSE`.
- `WindowConfig` supplies editable `title`, `width`, `height`, and optional
  icon identity fields plus a `create()` method. It is a high-level
  configuration object and avoids
  repeating the low-level creation arguments at every call site.
- `ControlConfig` supplies editable class, text, numeric ID, position, size,
  style, and extended-style fields. Its defaults create a visible `STATIC`
  child, and `create(parent)` forwards only the resulting configuration to the
  runtime boundary.

Common message-box flags and result constants are provided under `windows::`.
The captured message constants currently include `WM_DESTROY`, `WM_MOVE`,
`WM_SIZE`, `WM_CLOSE`, `WM_NOTIFY`, `WM_COMMAND`, keyboard/character messages,
and primary mouse-button/move messages. Unknown and internal messages continue
directly to `DefWindowProcW` without filling the APL queue.
Common decoded notification constants include `BN_CLICKED`, `EN_CHANGE`,
`EN_UPDATE`, and `EN_SELCHANGE`.
`DWORD`, `WORD`, and related values intentionally remain ordinary APL integers;
handles and pointers remain capability-bearing descriptors where APL owns the
lifetime contract.

## Example

```apl
include >> windows

AVInt std::main() {
  WindowConfig config = WindowConfig()
  config.title = "APL Windows library"
  config.width = 960
  config.height = 640

  VTime window = config.create()
  windows::show_window(window)
  return std::int(windows::run_message_loop())
}
```

On non-Windows targets, pure helpers such as `utf16` remain available while
DLL and User32 operations fail explicitly rather than silently emulating
Windows behavior.

The default window procedure records the public event subset, delegates normal
handling to `DefWindowProcW`, and posts
`WM_QUIT` after the last APL window on the current thread is destroyed.
Low-level callback registration and embedded PE resources are not implemented.
High-level APL handlers use `WindowApplication` over the safe event queue;
asynchronous process streaming/cancellation also remains future work.

The host implementation follows the Win32 `EM_SETCHARFORMAT`/`CHARFORMAT2W`,
`EM_FINDTEXTEXW`/`FINDTEXTEXW`, and Text Object Model undo contracts. Their
pointer-bearing COM and message structures are intentionally not part of the
APL-facing API.
