AVInt std::main() {
  List values = ["a", "b", "c"]
  VTime invalid_slice = values[::0]
  out std::len(invalid_slice)
  return 0
}

# Expected runtime failure:
# zero slice step
