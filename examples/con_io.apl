include >> console

AVInt std::main() {
  con::print(["Ready:", true, "attempts:", 3], " ", "\n")
  VTime answer = con::wait_in("Enter another name:")
  con::print(["Hello,", answer], " ", "\n")
  return 0
}
