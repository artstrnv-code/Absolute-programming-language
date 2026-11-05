func math.pi() {
  return 3.141592653589793
}

func math.e() {
  return 2.718281828459045
}

func math.fcl(number) {
  VTime __math_fcl_int = std::int(number)
  if (__math_fcl_int == NONE) {
    return NONE
  }
  if (__math_fcl_int < 0) {
    return NONE
  }
  if (number != __math_fcl_int) {
    return NONE
  }

  VTime __math_fcl_result = 1
  while (__math_fcl_int > 1)(-1) {
    if (__math_fcl_result > 9223372036854775807 / __math_fcl_int) {
      return NONE
    }
    __math_fcl_result *= __math_fcl_int
    __math_fcl_int -= 1
  }
  return __math_fcl_result
}

func math.abs(value) {
  VTime __math_abs_value = std::float(value)
  if (__math_abs_value < 0.0) {
    return 0.0 - __math_abs_value
  }
  return __math_abs_value
}

func math.ln(value) {
  VTime __math_ln_value = std::float(value)
  if (__math_ln_value <= 0.0) {
    return NONE
  }

  VTime __math_ln_power = 0.0
  while (__math_ln_value >= 2.0)(2048) {
    __math_ln_value /= 2.0
    __math_ln_power += 1.0
  } limit {
    return NONE
  }
  while (__math_ln_value < 1.0)(2048) {
    __math_ln_value *= 2.0
    __math_ln_power -= 1.0
  } limit {
    return NONE
  }

  VTime __math_ln_y = (__math_ln_value - 1.0) / (__math_ln_value + 1.0)
  VTime __math_ln_y2 = __math_ln_y * __math_ln_y
  VTime __math_ln_term = __math_ln_y
  VTime __math_ln_sum = __math_ln_y
  VTime __math_ln_denominator = 3.0
  VTime __math_ln_index = 0
  while (__math_ln_index < 20)(-1) {
    __math_ln_term *= __math_ln_y2
    __math_ln_sum += __math_ln_term / __math_ln_denominator
    __math_ln_denominator += 2.0
    __math_ln_index += 1
  }
  return 2.0 * __math_ln_sum + __math_ln_power * 0.6931471805599453
}

func math.log(base, number) {
  VTime __math_log_base = std::float(base)
  VTime __math_log_number = std::float(number)
  if (__math_log_base <= 0.0) {
    return NONE
  }
  if (__math_log_base == 1.0) {
    return NONE
  }
  if (__math_log_number <= 0.0) {
    return NONE
  }

  VTime __math_log_base_ln = math::ln(__math_log_base)
  VTime __math_log_number_ln = math::ln(__math_log_number)
  if (__math_log_base_ln == NONE) {
    return NONE
  }
  if (__math_log_number_ln == NONE) {
    return NONE
  }
  return __math_log_number_ln / __math_log_base_ln
}

func math.sqrt(value) {
  VTime __math_sqrt_value = std::float(value)
  if (__math_sqrt_value < 0.0) {
    return NONE
  }
  if (__math_sqrt_value == 0.0) {
    return 0.0
  }

  VTime __math_sqrt_scale = 1.0
  while (__math_sqrt_value >= 4.0)(1024) {
    __math_sqrt_value /= 4.0
    __math_sqrt_scale *= 2.0
  } limit {
    return NONE
  }
  while (__math_sqrt_value < 1.0)(1024) {
    __math_sqrt_value *= 4.0
    __math_sqrt_scale /= 2.0
  } limit {
    return NONE
  }

  VTime __math_sqrt_result = 1.5
  VTime __math_sqrt_next = 0.0
  VTime __math_sqrt_index = 0
  while (__math_sqrt_index < 64)(-1) {
    __math_sqrt_next = (__math_sqrt_result + __math_sqrt_value / __math_sqrt_result) / 2.0
    if (math::abs(__math_sqrt_next - __math_sqrt_result) < 0.000000000000001 * __math_sqrt_next) {
      return __math_sqrt_next * __math_sqrt_scale
    }
    __math_sqrt_result = __math_sqrt_next
    __math_sqrt_index += 1
  }
  return __math_sqrt_next * __math_sqrt_scale
}

func math.cbrt(value) {
  VTime __math_cbrt_value = std::float(value)
  VTime __math_cbrt_sign = 1.0
  if (__math_cbrt_value < 0.0) {
    __math_cbrt_sign = 0.0 - 1.0
    __math_cbrt_value = 0.0 - __math_cbrt_value
  }
  if (__math_cbrt_value == 0.0) {
    return 0.0
  }

  VTime __math_cbrt_scale = 1.0
  while (__math_cbrt_value >= 8.0)(1024) {
    __math_cbrt_value /= 8.0
    __math_cbrt_scale *= 2.0
  } limit {
    return NONE
  }
  while (__math_cbrt_value < 1.0)(1024) {
    __math_cbrt_value *= 8.0
    __math_cbrt_scale /= 2.0
  } limit {
    return NONE
  }

  VTime __math_cbrt_result = 2.0
  VTime __math_cbrt_next = 0.0
  VTime __math_cbrt_index = 0
  while (__math_cbrt_index < 64)(-1) {
    __math_cbrt_next = (2.0 * __math_cbrt_result + __math_cbrt_value / (__math_cbrt_result * __math_cbrt_result)) / 3.0
    if (math::abs(__math_cbrt_next - __math_cbrt_result) < 0.000000000000001 * __math_cbrt_next) {
      return __math_cbrt_sign * __math_cbrt_next * __math_cbrt_scale
    }
    __math_cbrt_result = __math_cbrt_next
    __math_cbrt_index += 1
  }
  return __math_cbrt_sign * __math_cbrt_next * __math_cbrt_scale
}

func math.rad(degrees) {
  return std::float(degrees) * (math::pi() / 180.0)
}

func math.deg(radians) {
  return std::float(radians) * (180.0 / math::pi())
}

func math.sin(value) {
  VTime __math_sin_x = std::float(value) % (2.0 * math::pi())
  if (__math_sin_x > math::pi()) {
    __math_sin_x -= 2.0 * math::pi()
  }
  if (__math_sin_x < (0.0 - math::pi())) {
    __math_sin_x += 2.0 * math::pi()
  }

  VTime __math_sin_term = __math_sin_x
  VTime __math_sin_result = __math_sin_x
  VTime __math_sin_x2 = __math_sin_x * __math_sin_x
  VTime __math_sin_index = 1.0
  while (__math_sin_index < 13.0)(-1) {
    VTime __math_sin_denominator = (2.0 * __math_sin_index) * (2.0 * __math_sin_index + 1.0)
    __math_sin_term = (0.0 - __math_sin_term) * __math_sin_x2 / __math_sin_denominator
    __math_sin_result += __math_sin_term
    __math_sin_index += 1.0
  }
  return __math_sin_result
}

func math.cos(value) {
  VTime __math_cos_x = std::float(value) % (2.0 * math::pi())
  if (__math_cos_x > math::pi()) {
    __math_cos_x -= 2.0 * math::pi()
  }
  if (__math_cos_x < (0.0 - math::pi())) {
    __math_cos_x += 2.0 * math::pi()
  }

  VTime __math_cos_term = 1.0
  VTime __math_cos_result = 1.0
  VTime __math_cos_x2 = __math_cos_x * __math_cos_x
  VTime __math_cos_index = 1.0
  while (__math_cos_index < 13.0)(-1) {
    VTime __math_cos_denominator = (2.0 * __math_cos_index - 1.0) * (2.0 * __math_cos_index)
    __math_cos_term = (0.0 - __math_cos_term) * __math_cos_x2 / __math_cos_denominator
    __math_cos_result += __math_cos_term
    __math_cos_index += 1.0
  }
  return __math_cos_result
}

func math.tan(value) {
  VTime __math_tan_sin = math::sin(value)
  VTime __math_tan_cos = math::cos(value)
  if (__math_tan_cos == 0.0) {
    return NONE
  }
  return __math_tan_sin / __math_tan_cos
}

func math.ctg(value) {
  VTime __math_ctg_sin = math::sin(value)
  if (__math_ctg_sin == 0.0) {
    return NONE
  }
  return math::cos(value) / __math_ctg_sin
}

func math.atan_series(value) {
  VTime __math_atan_series_x2 = value * value
  VTime __math_atan_series_term = value
  VTime __math_atan_series_result = value
  VTime __math_atan_series_divisor = 3.0
  VTime __math_atan_series_index = 0
  while (__math_atan_series_index < 30)(-1) {
    __math_atan_series_term = (0.0 - __math_atan_series_term) * __math_atan_series_x2
    __math_atan_series_result += __math_atan_series_term / __math_atan_series_divisor
    __math_atan_series_divisor += 2.0
    __math_atan_series_index += 1
  }
  return __math_atan_series_result
}

func math.atan(value) {
  VTime __math_atan_x = std::float(value)
  VTime __math_atan_sign = 1.0
  if (__math_atan_x < 0.0) {
    __math_atan_sign = 0.0 - 1.0
    __math_atan_x = 0.0 - __math_atan_x
  }

  VTime __math_atan_result = 0.0
  if (__math_atan_x > 1.0) {
    __math_atan_result = math::pi() / 2.0 - math::atan_series(1.0 / __math_atan_x)
  } else {
    if (__math_atan_x > 0.41421356237309503) {
      __math_atan_result = math::pi() / 4.0 + math::atan_series((__math_atan_x - 1.0) / (__math_atan_x + 1.0))
    } else {
      __math_atan_result = math::atan_series(__math_atan_x)
    }
  }
  return __math_atan_sign * __math_atan_result
}

func math.asin(value) {
  VTime __math_asin_x = std::float(value)
  if (__math_asin_x < (0.0 - 1.0)) {
    return NONE
  }
  if (__math_asin_x > 1.0) {
    return NONE
  }
  if (__math_asin_x == 1.0) {
    return math::pi() / 2.0
  }
  if (__math_asin_x == (0.0 - 1.0)) {
    return 0.0 - math::pi() / 2.0
  }
  return math::atan(__math_asin_x / math::sqrt(1.0 - __math_asin_x * __math_asin_x))
}

func math.acos(value) {
  VTime __math_acos_asin = math::asin(value)
  if (__math_acos_asin == NONE) {
    return NONE
  }
  return math::pi() / 2.0 - __math_acos_asin
}

func math.actg(value) {
  return math::pi() / 2.0 - math::atan(value)
}
