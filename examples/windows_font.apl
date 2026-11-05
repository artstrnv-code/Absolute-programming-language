include >> windows

AVInt std::main() {
  if windows::available() != true {
    return 0
  }

  VTime window = windows::create_window("APL font example", 480, 240)
  ControlConfig editor = ControlConfig()
  editor.class_name = windows::CLASS_RICHEDIT
  editor.text = "Consolas 12"
  editor.style += windows::ES_MULTILINE
  VTime control = editor.create(window)
  windows::set_text_font(control, 0, -1, "Consolas", 12)
  windows::destroy_window(window)
  return 0
}
