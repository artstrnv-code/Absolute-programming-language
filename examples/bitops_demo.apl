include >> bitops
exclude "_binary" << bitops
exclude "_left_one" << bitops

AVInt std::main() {
  out bitops::bit_and(5, 3)
  out "\n"
  out bitops::bit_or(5, 3)
  out "\n"
  out bitops::bit_xor(5, 3)
  out "\n"
  out bitops::bit_xand(5, 3)
  out "\n"
  out bitops::bit_not(0)
  out "\n"
  out bitops::shift_left(5, 1)
  out "\n"
  out bitops::shift_right(-8, 2)
  out "\n"
  out bitops::shift_left(9223372036854775807, 1)
  out "\n"
  out bitops::bit_not(-9223372036854775807 - 1)
  out "\n"
  out bitops::bit_and(-1, 9223372036854775807)
  out "\n"
  out bitops::bit_or(-9223372036854775807 - 1, 1)
  out "\n"
  out bitops::bit_xor(-9223372036854775807 - 1, 9223372036854775807)
  out "\n"
  out bitops::bit_xand(-1, 0)
  out "\n"
  out bitops::shift_left(1, 63)
  out "\n"
  out bitops::shift_left(-9223372036854775807 - 1, 1)
  out "\n"
  out bitops::shift_right(-3, 1)
  out "\n"
  out bitops::shift_right(-9223372036854775807 - 1, 63)
  out "\n"
  out bitops::shift_right(1, 64)
  out "\n"
  out bitops::bit_and("not an integer", 1)
  return 0
}
