include >> windows

AVInt std::main() {
  if windows::available() != true {
    out "Windows API is unavailable\n"
    return 0
  }

  ProcessResult child = ProcessResult(windows::run_process("cmd.exe", ["/D", "/S", "/C", "echo APL child process"], "", ""))
  out child.stdout
  if child.stderr != "" {
    out child.stderr
  }
  return std::int(child.exit_code)
}
