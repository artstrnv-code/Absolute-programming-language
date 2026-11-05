include >> raw_memory

AVInt std::main() {
  VTime pointer = memory::malloc(8)
  memory::write(pointer, 0, std::bytes("APL"))

  out "id="
  out memory::where(pointer, 1)
  out ", name="
  out memory::where(pointer, 2)
  out ", bytes="
  out std::str(memory::read(pointer, 0, 3))
  out "\n"

  memory::mark(pointer)
  out "released="
  out memory::free()
  out "\n"
  return 0
}
