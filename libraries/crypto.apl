# TEMPORARY, UNSAFE RANDOM IMPLEMENTATION.
# This uses and modifies the shared monotonic timer as a weak seed and is not
# suitable for keys, tokens, OTP masks, or any security-sensitive purpose. It
# fails if the caller already has the timer running. Replace the seed source
# with OS CSPRNG output before treating this API as safe.
# The result is public VTime; callers must store it in ASVInt when needed.
func crypto.safe_rand_int(minimum, maximum) {
  VTime __crypto_minimum = std::int(minimum)
  VTime __crypto_maximum = std::int(maximum)
  if (__crypto_minimum == NONE) {
    fail "crypto::safe_rand_int minimum must be an integer"
  }
  if (__crypto_maximum == NONE) {
    fail "crypto::safe_rand_int maximum must be an integer"
  }
  if (__crypto_minimum >= __crypto_maximum) {
    fail "crypto::safe_rand_int requires minimum < maximum"
  }

  if ((__crypto_minimum < 0) and (__crypto_maximum >= 0)) {
    if (__crypto_maximum > (9223372036854775807 + __crypto_minimum)) {
      fail "crypto::safe_rand_int range is too wide"
    }
  }
  VTime __crypto_difference = __crypto_maximum - __crypto_minimum
  if __crypto_difference > 2147483646 {
    fail "crypto::safe_rand_int range exceeds temporary generator capacity"
  }

  std::timer_start()
  std::timer_stop()
  VTime __crypto_seed = std::int(std::timer_elapsed() * 1000000000.0)
  if __crypto_seed == NONE {
    fail "crypto::safe_rand_int could not derive a temporary seed"
  }
  __crypto_seed %= 2147483647
  if __crypto_seed < 0 {
    __crypto_seed += 2147483647
  }
  if __crypto_seed == 0 {
    __crypto_seed = 1
  }

  VTime __crypto_state = (__crypto_seed * 48271) % 2147483647
  VTime __crypto_width = __crypto_difference
  VTime __crypto_limit = 2147483646 - (2147483646 % __crypto_width)
  VTime __crypto_sample = __crypto_state - 1
  while (__crypto_sample >= __crypto_limit)(-1) {
    __crypto_state = (__crypto_state * 48271) % 2147483647
    __crypto_sample = __crypto_state - 1
  }

  VTime __crypto_result = __crypto_minimum + (__crypto_sample % __crypto_width)
  return __crypto_result
}

func crypto.sha1(data) {
  return std::sha1(std::bytes(data))
}

func crypto.sha256(data) {
  return std::sha256(std::bytes(data))
}

func crypto.sha512(data) {
  return std::sha512(std::bytes(data))
}

func crypto.sha3_384(data) {
  return std::sha3_384(std::bytes(data))
}

func crypto._hex(bytes_value) {
  VTime alphabet = "0123456789abcdef"
  VTime __crypto_hex_result = ""
  VTime __crypto_hex_index = 0
  VTime __crypto_hex_length = std::len(bytes_value)
  while (__crypto_hex_index < __crypto_hex_length)(-1) {
    VTime __crypto_hex_byte = std::int(bytes_value[__crypto_hex_index])
    VTime __crypto_hex_high = __crypto_hex_byte / 16
    VTime __crypto_hex_low = __crypto_hex_byte % 16
    __crypto_hex_result += alphabet[__crypto_hex_high]
    __crypto_hex_result += alphabet[__crypto_hex_low]
    __crypto_hex_index += 1
  }
  return __crypto_hex_result
}

func crypto.sha1_hex(data) {
  return crypto._hex(crypto.sha1(data))
}

func crypto.sha256_hex(data) {
  return crypto._hex(crypto.sha256(data))
}

func crypto.sha512_hex(data) {
  return crypto._hex(crypto.sha512(data))
}

func crypto.sha3_384_hex(data) {
  return crypto._hex(crypto.sha3_384(data))
}
