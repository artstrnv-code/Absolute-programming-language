using namespace std

func fail_from_library() {
  return(ERR=["boom"][7])
}

AVInt std::main() {
  fail_from_library()
  return 0
}
