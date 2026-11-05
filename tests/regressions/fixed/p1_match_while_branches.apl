using namespace std

AVInt std::main() {
  AVInt selected = 2

  match selected {
    case 0 {
      out "zero"
    }
    case 1, 2 {
      out "two"
    }
    case 3..5 {
      out "range"
    }
    case default {
      out "default"
    }
  }

  AVInt limited = 0
  while (limited < 10) (2) {
    limited++
  } limit {
    out "limit"
  } else {
    out "wrong-else"
  }

  AVInt skipped = 1
  while (skipped < 0) (2) {
    skipped++
  } limit {
    out "wrong-limit"
  } else {
    out "else"
  }

  AVInt normal = 0
  while (normal < 1) (3) {
    normal++
  } limit {
    out "wrong-normal-limit"
  } else {
    out "wrong-normal-else"
  }

  return 0
}
