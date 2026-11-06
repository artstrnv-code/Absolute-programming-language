include >> classes

# Windows system boundary. Procedure values returned by get_procedure are
# tracked raw-memory capabilities and become invalid when their library closes.

AVInt windows.NULL = 0
AVInt windows.FALSE = 0
AVInt windows.TRUE = 1

AVStr windows.INT = "Int"
AVStr windows.I8 = "I8"
AVStr windows.U8 = "U8"
AVStr windows.I16 = "I16"
AVStr windows.U16 = "U16"
AVStr windows.I32 = "I32"
AVStr windows.U32 = "U32"
AVStr windows.I64 = "I64"
AVStr windows.U64 = "U64"
AVStr windows.PTR = "Ptr"
AVStr windows.BOOL = "Bool"
AVStr windows.VOID = "Void"

AVStr windows.CALLBACK_WNDPROC = "WNDPROC"
AVStr windows.CALLBACK_ENUM_WINDOWS = "ENUM_WINDOWS"
AVStr windows.CALLBACK_TIMER = "TIMER"
AVStr windows.CALLBACK_THREAD = "THREAD"

AVInt windows.MB_OK = 0
AVInt windows.MB_OKCANCEL = 1
AVInt windows.MB_ABORTRETRYIGNORE = 2
AVInt windows.MB_YESNOCANCEL = 3
AVInt windows.MB_YESNO = 4
AVInt windows.MB_RETRYCANCEL = 5
AVInt windows.MB_CANCELTRYCONTINUE = 6
AVInt windows.MB_ICONERROR = 16
AVInt windows.MB_ICONQUESTION = 32
AVInt windows.MB_ICONWARNING = 48
AVInt windows.MB_ICONINFORMATION = 64

AVInt windows.IDOK = 1
AVInt windows.IDCANCEL = 2
AVInt windows.IDABORT = 3
AVInt windows.IDRETRY = 4
AVInt windows.IDIGNORE = 5
AVInt windows.IDYES = 6
AVInt windows.IDNO = 7
AVInt windows.IDTRYAGAIN = 10
AVInt windows.IDCONTINUE = 11

AVInt windows.WM_DESTROY = 2
AVInt windows.WM_MOVE = 3
AVInt windows.WM_SIZE = 5
AVInt windows.WM_CLOSE = 16
AVInt windows.WM_NOTIFY = 78
AVInt windows.WM_KEYDOWN = 256
AVInt windows.WM_KEYUP = 257
AVInt windows.WM_CHAR = 258
AVInt windows.WM_COMMAND = 273
AVInt windows.WM_TIMER = 275
AVInt windows.WM_MOUSEMOVE = 512
AVInt windows.WM_LBUTTONDOWN = 513
AVInt windows.WM_LBUTTONUP = 514
AVInt windows.WM_RBUTTONDOWN = 516
AVInt windows.WM_RBUTTONUP = 517

AVInt windows.BN_CLICKED = 0
AVInt windows.EN_CHANGE = 768
AVInt windows.EN_UPDATE = 1024
AVInt windows.EN_SELCHANGE = 1794

AVInt windows.ENM_CHANGE = 1
AVInt windows.ENM_UPDATE = 2
AVInt windows.ENM_SCROLL = 4
AVInt windows.ENM_SCROLLEVENTS = 8
AVInt windows.ENM_SELCHANGE = 524288

AVInt windows.WS_VISIBLE = 268435456
AVInt windows.WS_CHILD = 1073741824
AVInt windows.WS_TABSTOP = 65536
AVInt windows.WS_VSCROLL = 2097152
AVInt windows.WS_HSCROLL = 1048576
AVInt windows.ES_MULTILINE = 4
AVInt windows.ES_AUTOVSCROLL = 64
AVInt windows.ES_AUTOHSCROLL = 128

AVStr windows.CLASS_STATIC = "STATIC"
AVStr windows.CLASS_BUTTON = "BUTTON"
AVStr windows.CLASS_EDIT = "EDIT"
AVStr windows.CLASS_RICHEDIT = "RICHEDIT50W"

AVInt windows.ACCEL_NOINVERT = 2
AVInt windows.ACCEL_SHIFT = 4
AVInt windows.ACCEL_CONTROL = 8
AVInt windows.ACCEL_ALT = 16

class WindowConfig {
  AVStr title = "APL"
  AVInt width = 960
  AVInt height = 640
  AVInt icon = 0
  AVInt accelerators = 0

  func __init__() {
  }

  func create() {
    VTime window = windows.create_window(self.title, self.width, self.height)
    if self.icon != 0 {
      windows.set_window_icon(window, self.icon)
    }
    if self.accelerators != 0 {
      windows.set_accelerators(window, self.accelerators)
    }
    return window
  }
}

class ControlConfig {
  AVStr class_name = "STATIC"
  AVStr text = ""
  AVInt id = 0
  AVInt x = 0
  AVInt y = 0
  AVInt width = 120
  AVInt height = 28
  AVInt style = 1342177280
  AVInt extended_style = 0

  func __init__() {
  }

  func create(parent) {
    return windows.create_control(parent, self.class_name, self.text, self.id, self.x, self.y, self.width, self.height, self.style, self.extended_style)
  }
}

class WindowEvent {
  AVInt window = 0
  AVInt message = 0
  AVInt wparam = 0
  AVInt lparam = 0
  AVInt source = 0
  AVInt code = 0
  AVInt control_id = 0

  func __init__(raw) {
    self.window = std::get(raw, 0)
    self.message = std::get(raw, 1)
    self.wparam = std::get(raw, 2)
    self.lparam = std::get(raw, 3)
    self.source = std::get(raw, 4)
    self.code = std::get(raw, 5)
    self.control_id = std::get(raw, 6)
  }

  func is_message(message) {
    return self.message == message
  }

  func is_from(control) {
    return self.source == control
  }

  func is_command(control, code) {
    return ((self.message == 273) and (self.source == control)) and (self.code == code)
  }

  func is_menu(command_id) {
    return (((self.message == 273) and (self.source == 0)) and (self.code == 0)) and (self.control_id == command_id)
  }

  func is_accelerator(command_id) {
    return (((self.message == 273) and (self.source == 0)) and (self.code == 1)) and (self.control_id == command_id)
  }

  func is_timer(timer_id) {
    return (self.message == 275) and (self.wparam == timer_id)
  }

  func is_notification(control, code) {
    return ((self.message == 78) and (self.source == control)) and (self.code == code)
  }
}

class ProcessResult {
  AVInt exit_code = 0
  AVStr stdout = ""
  AVStr stderr = ""

  func __init__(raw) {
    self.exit_code = std::get(raw, 0)
    self.stdout = std::get(raw, 1)
    self.stderr = std::get(raw, 2)
  }
}

class ProcessSnapshot {
  AVBool running = false
  AVInt exit_code = NONE
  AVStr stdout = ""
  AVStr stderr = ""

  func __init__(raw) {
    self.running = std::get(raw, 0)
    self.exit_code = std::get(raw, 1)
    self.stdout = std::get(raw, 2)
    self.stderr = std::get(raw, 3)
  }
}

class FileDialogConfig {
  AVStr title = ""
  AVStr initial_directory = ""
  AVStr file_name = ""
  AVStr filter_name = "APL source"
  AVStr filter_pattern = "*.apl"
  AVStr default_extension = "apl"

  func __init__() {
  }

  func open(owner) {
    return windows.open_file_dialog(owner, self.title, self.initial_directory, self.file_name, self.filter_name, self.filter_pattern, self.default_extension)
  }

  func save(owner) {
    return windows.save_file_dialog(owner, self.title, self.initial_directory, self.file_name, self.filter_name, self.filter_pattern, self.default_extension)
  }
}

class Menu {
  AVInt identity = 0
  AVBool popup = false

  func __init__(popup) {
    self.popup = popup
    self.identity = windows.create_menu(popup)
  }

  func item(command_id, text) {
    return windows.append_menu_item(self.identity, command_id, text)
  }

  func submenu(submenu_identity, text) {
    return windows.append_submenu(self.identity, submenu_identity, text)
  }

  func separator() {
    return windows.append_menu_separator(self.identity)
  }

  func attach(window) {
    return windows.set_menu(window, self.identity)
  }

  func enable(command_id, enabled) {
    return windows.set_menu_item_enabled(self.identity, command_id, enabled)
  }

  func check(command_id, checked) {
    return windows.set_menu_item_checked(self.identity, command_id, checked)
  }

  func destroy() {
    VTime destroyed = windows.destroy_menu(self.identity)
    self.identity = 0
    return destroyed
  }
}

class Icon {
  AVInt identity = 0
  AVStr path = ""
  AVInt width = 0
  AVInt height = 0

  func __init__(path, width, height) {
    self.path = path
    self.width = width
    self.height = height
    self.identity = windows.load_icon(path, width, height)
  }

  func apply(window) {
    return windows.set_window_icon(window, self.identity)
  }

  func destroy() {
    VTime destroyed = windows.destroy_icon(self.identity)
    self.identity = 0
    return destroyed
  }
}

class Accelerators {
  AVInt identity = 0
  List entries = []

  func __init__(entries) {
    self.entries = entries
    self.identity = windows.create_accelerators(entries)
  }

  func attach(window) {
    return windows.set_accelerators(window, self.identity)
  }

  func destroy() {
    VTime destroyed = windows.destroy_accelerators(self.identity)
    self.identity = 0
    return destroyed
  }
}

class ForeignCallback {
  List identity = []
  AVStr kind = ""
  AVInt result = 0

  func __init__(kind, result) {
    self.kind = kind
    self.result = result
    self.identity = windows.create_callback(kind, result)
  }

  func next() {
    return windows.next_callback(self.identity)
  }

  func destroy() {
    VTime destroyed = windows.destroy_callback(self.identity)
    self.identity = []
    return destroyed
  }
}

class WindowApplication {
  AVInt window = 0
  AVBool running = true
  AVBool main_destroyed = false
  AVInt exit_code = 0
  AVInt _message_destroy = 2
  AVInt _message_size = 5
  AVInt _message_close = 16
  AVInt _message_notify = 78
  AVInt _message_command = 273
  AVInt _message_timer = 275

  func __init__(window) {
    self.window = window
    windows.set_deferred_close(window, true)
  }

  func run() {
    while (self.running) (-1) {
      VTime raw = windows.wait_event()
      if raw == NONE {
        self.running = false
      } else {
        self.dispatch(raw)
      }
    }
    return self.exit_code
  }

  func exit(code) {
    self.exit_code = code
    self.running = false
    return true
  }

  func dispatch(raw) {
    WindowEvent event = WindowEvent(raw)
    self.on_event(raw)

    if event.message == self._message_destroy {
      if event.window == self.window {
        self.main_destroyed = true
        self.running = false
      }
      return self.on_destroy(raw)
    }
    if event.message == self._message_close {
      return self.on_close(raw)
    }
    if event.message == self._message_size {
      return self.on_resize(raw)
    }
    if event.message == self._message_command {
      if (event.source == 0) and (event.code == 0) {
        return self.on_menu(raw)
      }
      return self.on_command(raw)
    }
    if event.message == self._message_notify {
      return self.on_notification(raw)
    }
    if event.message == self._message_timer {
      return self.on_timer(raw)
    }
    return true
  }

  func on_close(raw) {
    WindowEvent event = WindowEvent(raw)
    if event.window == self.window {
      return windows.destroy_window(self.window)
    }
    return true
  }

  func on_destroy(raw) {
    return true
  }

  func on_resize(raw) {
    return true
  }

  func on_menu(raw) {
    return true
  }

  func on_command(raw) {
    return true
  }

  func on_notification(raw) {
    return true
  }

  func on_timer(raw) {
    return true
  }

  func on_event(raw) {
    return true
  }
}

func windows.available() {
  return std::windows_available()
}

func windows.load_library(name) {
  return std::windows_load_library(name)
}

func windows.get_procedure(library, name, argument_types, result_type) {
  return std::windows_get_procedure(library, name, argument_types, result_type)
}

func windows.create_callback(kind, result) {
  return std::windows_create_callback(kind, result)
}

func windows.next_callback(callback) {
  return std::windows_next_callback(callback)
}

func windows.destroy_callback(callback) {
  return std::windows_destroy_callback(callback)
}

func windows.call(procedure, arguments) {
  return std::windows_call(procedure, arguments)
}

func windows.close_library(library) {
  return std::windows_close_library(library)
}

func windows.last_error() {
  return std::windows_last_error()
}

func windows.utf16(value) {
  return std::windows_utf16(value)
}

func windows.message_box(text, caption, kind) {
  return std::windows_message_box(text, caption, kind)
}

func windows.run_process(program, arguments, stdin_text, working_directory) {
  return std::windows_run_process(program, arguments, stdin_text, working_directory)
}

func windows.start_process(program, arguments, stdin_text, working_directory) {
  return std::windows_start_process(program, arguments, stdin_text, working_directory)
}

func windows.poll_process(process) {
  return std::windows_poll_process(process)
}

func windows.wait_process(process) {
  return std::windows_wait_process(process)
}

func windows.terminate_process(process) {
  return std::windows_terminate_process(process)
}

func windows.open_file_dialog(owner, title, initial_directory, file_name, filter_name, filter_pattern, default_extension) {
  return std::windows_open_file_dialog(owner, title, initial_directory, file_name, filter_name, filter_pattern, default_extension)
}

func windows.save_file_dialog(owner, title, initial_directory, file_name, filter_name, filter_pattern, default_extension) {
  return std::windows_save_file_dialog(owner, title, initial_directory, file_name, filter_name, filter_pattern, default_extension)
}

func windows.create_menu(popup) {
  return std::windows_create_menu(popup)
}

func windows.append_menu_item(menu, command_id, text) {
  return std::windows_append_menu_item(menu, command_id, text)
}

func windows.append_submenu(menu, submenu, text) {
  return std::windows_append_submenu(menu, submenu, text)
}

func windows.append_menu_separator(menu) {
  return std::windows_append_menu_separator(menu)
}

func windows.set_menu(window, menu) {
  return std::windows_set_menu(window, menu)
}

func windows.set_menu_item_enabled(menu, command_id, enabled) {
  return std::windows_set_menu_item_enabled(menu, command_id, enabled)
}

func windows.set_menu_item_checked(menu, command_id, checked) {
  return std::windows_set_menu_item_checked(menu, command_id, checked)
}

func windows.destroy_menu(menu) {
  return std::windows_destroy_menu(menu)
}

func windows.load_icon(path, width, height) {
  return std::windows_load_icon(path, width, height)
}

func windows.load_resource_icon(resource, width, height) {
  return std::windows_load_resource_icon(resource, width, height)
}

func windows.set_window_icon(window, icon) {
  return std::windows_set_window_icon(window, icon)
}

func windows.set_deferred_close(window, enabled) {
  return std::windows_set_deferred_close(window, enabled)
}

func windows.set_timer(window, timer_id, interval_ms) {
  return std::windows_set_timer(window, timer_id, interval_ms)
}

func windows.kill_timer(window, timer_id) {
  return std::windows_kill_timer(window, timer_id)
}

func windows.enable_window(window, enabled) {
  return std::windows_enable_window(window, enabled)
}

func windows.invalidate_window(window, erase) {
  return std::windows_invalidate_window(window, erase)
}

func windows.destroy_icon(icon) {
  return std::windows_destroy_icon(icon)
}

func windows.create_accelerators(entries) {
  return std::windows_create_accelerators(entries)
}

func windows.set_accelerators(window, accelerators) {
  return std::windows_set_accelerators(window, accelerators)
}

func windows.destroy_accelerators(accelerators) {
  return std::windows_destroy_accelerators(accelerators)
}

func windows.create_window(title, width, height) {
  return std::windows_create_window(title, width, height)
}

func windows.show_window(window) {
  return std::windows_show_window(window)
}

func windows.destroy_window(window) {
  return std::windows_destroy_window(window)
}

func windows.run_message_loop() {
  return std::windows_run_message_loop()
}

func windows.pump_messages() {
  return std::windows_pump_messages()
}

func windows.next_event() {
  return std::windows_next_event()
}

func windows.wait_event() {
  return std::windows_wait_event()
}

func windows.create_control(parent, class_name, text, id, x, y, width, height, style, extended_style) {
  return std::windows_create_control(parent, class_name, text, id, x, y, width, height, style, extended_style)
}

func windows.set_window_text(window, text) {
  return std::windows_set_window_text(window, text)
}

func windows.window_text(window) {
  return std::windows_window_text(window)
}

func windows.move_window(window, x, y, width, height, repaint) {
  return std::windows_move_window(window, x, y, width, height, repaint)
}

func windows.selection(control) {
  return std::windows_selection(control)
}

func windows.set_selection(control, start, end) {
  return std::windows_set_selection(control, start, end)
}

func windows.replace_selection(control, text, can_undo) {
  return std::windows_replace_selection(control, text, can_undo)
}

func windows.line_count(control) {
  return std::windows_line_count(control)
}

func windows.set_text_color(control, start, end, color) {
  return std::windows_set_text_color(control, start, end, color)
}

func windows.set_background_color(control, start, end, color) {
  return std::windows_set_background_color(control, start, end, color)
}

func windows.set_text_bold(control, start, end, enabled) {
  return std::windows_set_text_bold(control, start, end, enabled)
}

func windows.set_text_font(control, start, end, face, points) {
  return std::windows_set_text_font(control, start, end, face, points)
}

func windows.can_undo(control) {
  return std::windows_can_undo(control)
}

func windows.undo(control) {
  return std::windows_undo(control)
}

func windows.can_redo(control) {
  return std::windows_can_redo(control)
}

func windows.redo(control) {
  return std::windows_redo(control)
}

func windows.clear_history(control) {
  return std::windows_clear_history(control)
}

func windows.event_mask(control) {
  return std::windows_event_mask(control)
}

func windows.set_event_mask(control, mask) {
  return std::windows_set_event_mask(control, mask)
}

func windows._has_flag(value, flag) {
  if flag <= 0 {
    return false
  }
  return ((value / flag) % 2) == 1
}

func windows.enable_event(control, flag) {
  VTime mask = windows.event_mask(control)
  if windows._has_flag(mask, flag) {
    return true
  }
  windows.set_event_mask(control, mask + flag)
  return true
}

func windows.disable_event(control, flag) {
  VTime mask = windows.event_mask(control)
  if windows._has_flag(mask, flag) != true {
    return true
  }
  windows.set_event_mask(control, mask - flag)
  return true
}

func windows.first_visible_line(control) {
  return std::windows_first_visible_line(control)
}

func windows.scroll_lines(control, lines) {
  return std::windows_scroll_lines(control, lines)
}

func windows.focus(window) {
  return std::windows_focus(window)
}

func windows.find_text(control, text, start, end, match_case, whole_word) {
  return std::windows_find_text(control, text, start, end, match_case, whole_word)
}

func windows.rgb(red, green, blue) {
  VTime checked_red = windows._integer_in_range(red, 0, 255)
  VTime checked_green = windows._integer_in_range(green, 0, 255)
  VTime checked_blue = windows._integer_in_range(blue, 0, 255)
  if ((checked_red == NONE) or (checked_green == NONE)) or (checked_blue == NONE) {
    return NONE
  }
  return checked_red + (checked_green * 256) + (checked_blue * 65536)
}

func windows._integer_in_range(value, minimum, maximum) {
  VTime __windows_integer = std::int(value)
  if __windows_integer == NONE {
    return NONE
  }
  if __windows_integer < minimum {
    return NONE
  }
  if __windows_integer > maximum {
    return NONE
  }
  return __windows_integer
}

func windows.i8(value) {
  return windows._integer_in_range(value, -128, 127)
}

func windows.u8(value) {
  return windows._integer_in_range(value, 0, 255)
}

func windows.i16(value) {
  return windows._integer_in_range(value, -32768, 32767)
}

func windows.word(value) {
  return windows._integer_in_range(value, 0, 65535)
}

func windows.i32(value) {
  return windows._integer_in_range(value, -2147483648, 2147483647)
}

func windows.dword(value) {
  return windows._integer_in_range(value, 0, 4294967295)
}

func windows.qword(value) {
  VTime __windows_integer = std::int(value)
  if __windows_integer == NONE {
    return NONE
  }
  if __windows_integer < 0 {
    return NONE
  }
  return __windows_integer
}

func windows.bool(value) {
  if value {
    return windows.TRUE
  }
  return windows.FALSE
}
