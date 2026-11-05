include >> windows

AVInt std::main() {
  if windows::available() != true {
    return(ERR=["windows_dialog.apl requires Windows"][1])
  }

  FileDialogConfig dialog = FileDialogConfig()
  dialog.title = "Open an APL source file"
  dialog.initial_directory = "."

  VTime path = dialog.open(0)
  if path != NONE {
    out path
    out "\n"
  }

  return 0
}
