func timer.start_timer() {
  return std::timer_start()
}

func timer.stop_timer() {
  return std::timer_stop()
}

func timer.return_timer(unit) {
  VTime __timer_elapsed = std::timer_elapsed()
  if unit == "ms" {
    return __timer_elapsed * 1000.0
  }
  if unit == "s" {
    return __timer_elapsed
  }
  if unit == "m" {
    return __timer_elapsed / 60.0
  }
  return NONE
}
