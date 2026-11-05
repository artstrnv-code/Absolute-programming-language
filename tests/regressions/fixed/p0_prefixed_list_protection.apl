AVInt std::main() {
  SASVStr secret_value = "hidden"
  List values = ["public"]
  std::add(values, secret_value)
  VTime selected = std::get(values, 1)
  out selected
  return 0
}

# Expected check failure:
# secret expression denied `out`
