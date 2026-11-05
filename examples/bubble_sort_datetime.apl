include >> datetime

func utc_milliseconds() {
  VTime current = std::now_utc()
  VTime days = datetime::to_day_number(
    std::get(current, 0),
    std::get(current, 1),
    std::get(current, 2)
  )
  return days * 86400000 + std::get(current, 3) * 3600000 + std::get(current, 4) * 60000 + std::get(current, 5) * 1000 + std::get(current, 6)
}

AVInt std::main() {
  List values = []
  AVInt value = 600

  while (value > 0)(600) {
    std::add(values, value)
    value--
  }

  VTime started = utc_milliseconds()
  VTime size = std::len(values)
  VTime outer = 0

  while (outer < size - 1)(-1) {
    VTime inner = 0
    VTime last = size - outer - 1

    while (inner < last)(-1) {
      VTime left = std::get(values, inner)
      VTime right = std::get(values, inner + 1)

      if left > right {
        std::set(values, inner, right)
        std::set(values, inner + 1, left)
      }

      inner += 1
    }

    outer += 1
  }

  VTime elapsed = utc_milliseconds() - started

  out "elements: "
  out std::len(values)
  out "\ntime: "
  out elapsed
  out " ms\n"
  out "first: "
  out std::get(values, 0)
  out ", last: "
  out std::get(values, std::len(values) - 1)
  out "\n"
  return 0
}
