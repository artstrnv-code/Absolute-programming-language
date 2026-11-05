include >> windows

AVInt std::main() {
  if windows::available() != true {
    return(ERR=["windows_window.apl requires Windows"][1])
  }

  WindowConfig config = WindowConfig()
  config.title = "APL Windows library"
  config.width = 960
  config.height = 640

  VTime window = config.create()
  windows::show_window(window)
  return std::int(windows::run_message_loop())
}
