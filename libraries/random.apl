func random._normalize_seed(value) {
  VTime __random_seed = std::int(value)
  if __random_seed == NONE {
    return NONE
  }

  __random_seed %= 2147483647
  if __random_seed < 0 {
    __random_seed += 2147483647
  }
  if __random_seed == 0 {
    __random_seed = 1
  }
  return __random_seed
}

func random._step(seed) {
  VTime __random_state = random._normalize_seed(seed)
  if __random_state == NONE {
    return NONE
  }
  return (__random_state * 48271) % 2147483647
}

func random._draw_integer(seed, minimum, maximum) {
  VTime __random_minimum = std::int(minimum)
  VTime __random_maximum = std::int(maximum)
  if __random_minimum == NONE {
    return NONE
  }
  if __random_maximum == NONE {
    return NONE
  }
  if __random_minimum > __random_maximum {
    return NONE
  }

  if ((__random_minimum < 0) and (__random_maximum >= 0)) {
    if __random_maximum > (9223372036854775807 + __random_minimum) {
      return NONE
    }
  }

  VTime __random_difference = __random_maximum - __random_minimum
  if __random_difference >= 2147483646 {
    return NONE
  }

  VTime __random_state = random._step(seed)
  if __random_state == NONE {
    return NONE
  }

  VTime __random_width = __random_difference + 1
  VTime __random_limit = 2147483646 - (2147483646 % __random_width)
  VTime __random_sample = __random_state - 1

  while (__random_sample >= __random_limit) (-1) {
    __random_state = random._step(__random_state)
    __random_sample = __random_state - 1
  }

  VTime __random_value = __random_minimum + (__random_sample % __random_width)
  return [__random_value, __random_state]
}

func random.integer(seed, minimum, maximum) {
  VTime __random_integer_draw = random._draw_integer(seed, minimum, maximum)
  if __random_integer_draw == NONE {
    return NONE
  }
  return __random_integer_draw[0]
}

func random.boolean(seed) {
  VTime __random_bool_draw = random._draw_integer(seed, 0, 1)
  if __random_bool_draw == NONE {
    return NONE
  }
  return __random_bool_draw[0] == 1
}

func random.rfloat(seed, minimum, maximum) {
  VTime __random_minimum = std::float(minimum)
  VTime __random_maximum = std::float(maximum)
  if __random_minimum == NONE {
    return NONE
  }
  if __random_maximum == NONE {
    return NONE
  }
  if __random_minimum >= __random_maximum {
    return NONE
  }

  VTime __random_state = random._step(seed)
  if __random_state == NONE {
    return NONE
  }

  VTime __random_unit = std::float(__random_state - 1) / 2147483646.0
  return __random_minimum + ((__random_maximum - __random_minimum) * __random_unit)
}

func random.choose(seed, variants) {
  VTime __random_count = std::len(variants)
  if __random_count == NONE {
    return NONE
  }
  if __random_count == 0 {
    return NONE
  }

  VTime __random_index = random.integer(seed, 0, __random_count - 1)
  if __random_index == NONE {
    return NONE
  }
  return variants[__random_index]
}

func random.shuffle(seed, variants) {
  VTime __random_count = std::len(variants)
  if __random_count == NONE {
    return NONE
  }

  VTime __random_shuffle_output = []
  VTime __random_copy_index = 0
  while (__random_copy_index < __random_count) (-1) {
    std::add(__random_shuffle_output, variants[__random_copy_index])
    __random_copy_index += 1
  }

  VTime __random_state = random._normalize_seed(seed)
  if __random_state == NONE {
    return NONE
  }
  VTime __random_index = __random_count - 1
  while (__random_index > 0) (-1) {
    VTime __random_draw = random._draw_integer(__random_state, 0, __random_index)
    if __random_draw == NONE {
      return NONE
    }

    VTime __random_swap_index = __random_draw[0]
    __random_state = __random_draw[1]
    VTime __random_current = __random_shuffle_output[__random_index]
    VTime __random_swap = __random_shuffle_output[__random_swap_index]
    std::set(__random_shuffle_output, __random_index, __random_swap)
    std::set(__random_shuffle_output, __random_swap_index, __random_current)
    __random_index -= 1
  }
  return __random_shuffle_output
}

func random.sample(seed, variants, count) {
  VTime __random_sample_list_count = std::len(variants)
  VTime __random_sample_count = std::int(count)
  if __random_sample_list_count == NONE {
    return NONE
  }
  if __random_sample_count == NONE {
    return NONE
  }
  if __random_sample_count < 0 {
    return NONE
  }
  if __random_sample_count > __random_sample_list_count {
    return NONE
  }

  VTime __random_sample_shuffled = random.shuffle(seed, variants)
  if __random_sample_shuffled == NONE {
    return NONE
  }

  VTime __random_sample_output = []
  VTime __random_sample_index = 0
  while (__random_sample_index < __random_sample_count) (-1) {
    std::add(__random_sample_output, __random_sample_shuffled[__random_sample_index])
    __random_sample_index += 1
  }
  return __random_sample_output
}
