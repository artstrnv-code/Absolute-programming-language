func bitops._binary(left, right, operation) {
  VTime __bitops_left = std::int(left)
  VTime __bitops_right = std::int(right)
  if __bitops_left == NONE {
    return NONE
  }
  if __bitops_right == NONE {
    return NONE
  }
  VTime __bitops_index = 0
  VTime __bitops_weight = 1
  VTime __bitops_low = 0
  VTime __bitops_msb = 0

  while (__bitops_index < 64)(-1) {
    VTime __bitops_left_bit = __bitops_left % 2
    VTime __bitops_right_bit = __bitops_right % 2
    if (__bitops_left_bit < 0) {
      __bitops_left_bit += 2
    }
    if (__bitops_right_bit < 0) {
      __bitops_right_bit += 2
    }

    VTime __bitops_result_bit = 0
    if operation == 0 {
      if ((__bitops_left_bit == 1) and (__bitops_right_bit == 1)) {
        __bitops_result_bit = 1
      }
    } else if operation == 1 {
      if ((__bitops_left_bit == 1) or (__bitops_right_bit == 1)) {
        __bitops_result_bit = 1
      }
    } else if operation == 2 {
      if __bitops_left_bit != __bitops_right_bit {
        __bitops_result_bit = 1
      }
    } else {
      if ((__bitops_left_bit == 1) and (__bitops_right_bit == 0)) {
        __bitops_result_bit = 1
      }
    }

    if __bitops_index < 63 {
      if __bitops_result_bit == 1 {
        __bitops_low += __bitops_weight
      }
      if __bitops_index < 62 {
        __bitops_weight *= 2
      }
    } else {
      __bitops_msb = __bitops_result_bit
    }

    __bitops_left = (__bitops_left - __bitops_left_bit) / 2
    __bitops_right = (__bitops_right - __bitops_right_bit) / 2
    __bitops_index += 1
  }

  if __bitops_msb == 1 {
    return __bitops_low + (0 - 9223372036854775807 - 1)
  }
  return __bitops_low
}

func bitops._left_one(value) {
  VTime __bitops_value = std::int(value)
  VTime __bitops_two62 = 4611686018427387904
  VTime __bitops_max = 9223372036854775807
  VTime __bitops_min = 0 - __bitops_max - 1

  if __bitops_value >= 0 {
    if __bitops_value >= __bitops_two62 {
      return (__bitops_value - __bitops_two62) * 2 + __bitops_min
    }
    return __bitops_value * 2
  }

  if __bitops_value < (0 - __bitops_two62) {
    return (__bitops_value + __bitops_two62) * 2 + 1 + __bitops_max
  }
  return __bitops_value * 2
}

func bitops.bit_and(left, right) {
  return bitops._binary(left, right, 0)
}

func bitops.bit_or(left, right) {
  return bitops._binary(left, right, 1)
}

func bitops.bit_xor(left, right) {
  return bitops._binary(left, right, 2)
}

func bitops.bit_xand(left, right) {
  return bitops._binary(left, right, 3)
}

func bitops.bit_not(value) {
  VTime __bitops_value = std::int(value)
  if __bitops_value == NONE {
    return NONE
  }
  return (0 - 1) - __bitops_value
}

func bitops.shift_left(value, count) {
  VTime __bitops_value = std::int(value)
  VTime __bitops_count = std::int(count)
  if __bitops_value == NONE {
    return NONE
  }
  if __bitops_count == NONE {
    return NONE
  }
  if (__bitops_count < 0) or (__bitops_count > 63) {
    return NONE
  }

  VTime __bitops_index = 0
  while (__bitops_index < __bitops_count)(-1) {
    __bitops_value = bitops._left_one(__bitops_value)
    __bitops_index += 1
  }
  return __bitops_value
}

func bitops.shift_right(value, count) {
  VTime __bitops_value = std::int(value)
  VTime __bitops_count = std::int(count)
  if __bitops_value == NONE {
    return NONE
  }
  if __bitops_count == NONE {
    return NONE
  }
  if (__bitops_count < 0) or (__bitops_count > 63) {
    return NONE
  }

  VTime __bitops_index = 0
  while (__bitops_index < __bitops_count)(-1) {
    VTime __bitops_remainder = __bitops_value % 2
    __bitops_value = __bitops_value / 2
    if (__bitops_remainder < 0) {
      __bitops_value -= 1
    }
    __bitops_index += 1
  }
  return __bitops_value
}
