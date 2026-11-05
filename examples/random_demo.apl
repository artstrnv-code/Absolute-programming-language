include >> random
exclude "_normalize_seed" << random
exclude "_step" << random
exclude "_draw_integer" << random

AVInt std::main() {
  out random::integer(99, 1, 200)
  out "\n"

  out random::rfloat(99, 1.0, 200.0)
  out "\n"

  out random::choose(99, ["red", "green", "blue"])
  out "\n"

  out random::boolean(99)
  out "\n"

  out random::shuffle(99, [1, 2, 3, 4, 5])
  out "\n"

  out random::sample(99, [1, 2, 3, 4, 5], 3)
  out "\n"

  return 0
}
