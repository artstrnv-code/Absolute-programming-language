include >> timer
AVStr __timer__ = "ms"

AVInt std::main() {
  timer::start_timer()
  out timer::return_timer(__timer__)
  out "\n"
  timer::stop_timer()
  out timer::return_timer(__timer__)
  out "\n"
  timer::start_timer()
  timer::stop_timer()
  out timer::return_timer("s")
  out "\n"
  out timer::return_timer("m")
  out "\n"
  out timer::return_timer("invalid")
  return 0
}
