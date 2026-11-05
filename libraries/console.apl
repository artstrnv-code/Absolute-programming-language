func con.print(values, sep, end) {
  VTime count = std::len(values)
  VTime index = 0

  while (index < count) (-1) {
    if index > 0 {
      out sep
    }

    out values[index]
    index += 1
  }

  out end
}

func con.wait_in(prompt) {
  out prompt
  out "\n"

  VTime value = input
  if value == NONE {
    return(ERR=["input reached EOF"][1])
  }

  return value
}

func con.lower_char(value) {
  VTime code = std::ord(value)

  if (code >= 65) and (code <= 90) {
    return std::char(code + 32)
  }

  if ((code >= 192) and (code <= 214)) or ((code >= 216) and (code <= 222)) {
    return std::char(code + 32)
  }

  if ((code >= 913) and (code <= 929)) or ((code >= 931) and (code <= 939)) {
    return std::char(code + 32)
  }

  if (code >= 1040) and (code <= 1071) {
    return std::char(code + 32)
  }

  if code == 1025 {
    return std::char(1105)
  }

  return value
}

func con.upper_char(value) {
  VTime code = std::ord(value)

  if (code >= 97) and (code <= 122) {
    return std::char(code - 32)
  }

  if ((code >= 224) and (code <= 246)) or ((code >= 248) and (code <= 254)) {
    return std::char(code - 32)
  }

  if ((code >= 945) and (code <= 961)) or ((code >= 963) and (code <= 971)) {
    return std::char(code - 32)
  }

  if (code >= 1072) and (code <= 1103) {
    return std::char(code - 32)
  }

  if code == 1105 {
    return std::char(1025)
  }

  return value
}

func con.lower(value) {
  VTime result = ""
  VTime index = 0

  while (index < std::len(value)) (-1) {
    result += con.lower_char(value[index])
    index += 1
  }

  return result
}

func con.upper(value) {
  VTime result = ""
  VTime index = 0

  while (index < std::len(value)) (-1) {
    result += con.upper_char(value[index])
    index += 1
  }

  return result
}

func con.title(value) {
  VTime result = ""
  VTime index = 0
  VTime word_start = true

  while (index < std::len(value)) (-1) {
    VTime current = value[index]

    if current == " " {
      result += current
      word_start = true
    } else {
      if word_start {
        result += con.upper_char(current)
        word_start = false
      } else {
        result += con.lower_char(current)
      }
    }

    index += 1
  }

  return result
}

func con.split(value, separator) {
  return std::split(value, separator)
}

func con.join(values, separator) {
  return std::join(values, separator)
}

func con.contains(value, needle) {
  return std::contains(value, needle)
}

func con.replace(value, target, replacement) {
  VTime value_length = std::len(value)
  VTime target_length = std::len(target)
  VTime result = ""
  VTime index = 0

  if target_length == 0 {
    return value
  }

  while (index < value_length) (-1) {
    VTime found = false

    if index + target_length <= value_length {
      found = true
      VTime offset = 0

      while (offset < target_length) (-1) {
        if value[index + offset] != target[offset] {
          found = false
        }

        offset += 1
      }
    }

    if found {
      result += replacement
      index += target_length
    } else {
      result += value[index]
      index += 1
    }
  }

  return result
}
