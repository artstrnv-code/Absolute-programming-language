# Capability surface for tracked raw memory. These declarations are recognized
# as runtime intrinsics only when this module is included.

func memory.malloc(size) {
  return NONE
}

func memory.load(address, size, permissions, abi) {
  return NONE
}

func memory.mark(pointer) {
  return NONE
}

func memory.free() {
  return NONE
}

func memory.where(source, selector) {
  return NONE
}

func memory.read(pointer, offset, count) {
  return NONE
}

func memory.write(pointer, offset, value) {
  return NONE
}

func memory.call(pointer, arguments) {
  return NONE
}
