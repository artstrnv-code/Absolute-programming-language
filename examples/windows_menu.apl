include >> windows

AVInt std::main() {
  if windows::available() != true {
    return(ERR=["windows_menu.apl requires Windows"][1])
  }

  WindowConfig config = WindowConfig()
  config.title = "APL native menu"
  config.width = 640
  config.height = 420
  VTime window = config.create()

  Menu menu_bar = Menu(false)
  Menu file_menu = Menu(true)
  file_menu.item(1001, "Exit")
  menu_bar.submenu(file_menu.identity, "File")
  menu_bar.attach(window)
  windows::show_window(window)

  AVBool running = true
  while (running) (-1) {
    VTime raw = windows::wait_event()
    if raw == NONE {
      running = false
    } else {
      WindowEvent event = WindowEvent(raw)
      if event.is_menu(1001) {
        menu_bar.destroy()
        windows::destroy_window(window)
      }
    }
  }

  return 0
}
